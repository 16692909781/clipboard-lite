//! Clipboard watcher and paste helpers.

use crate::{state::AppState, storage};
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::Path, thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager};

const POLL_INTERVAL: Duration = Duration::from_millis(350);
const MAX_TEXT_BYTES: usize = 512 * 1024;
const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Debug, Clone)]
struct ClipboardSnapshot {
    clip_type: &'static str,
    content: String,
    preview: String,
    source_app: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageClip {
    width: usize,
    height: usize,
    rgba_base64: String,
}

/// Start the clipboard polling loop. Tauri keeps this on a background thread.
pub fn start_watcher(app: AppHandle) -> Result<(), String> {
    log::info!("clipboard watcher started");
    let mut last_hash: Option<String> = None;

    loop {
        thread::sleep(POLL_INTERVAL);

        let state = app.state::<AppState>();
        if state.is_paused() {
            continue;
        }

        let config = match state.config_snapshot() {
            Ok(config) => config,
            Err(err) => {
                log::warn!("config unavailable for clipboard watcher: {err}");
                continue;
            }
        };

        let source_app = active_source_app();
        if should_ignore_source(source_app.as_deref(), &config.ignored_apps) {
            continue;
        }

        let Some(snapshot) = read_current_clipboard(source_app) else {
            continue;
        };

        let hash = storage::content_hash(snapshot.clip_type, &snapshot.content);
        if last_hash.as_deref() == Some(hash.as_str()) {
            continue;
        }
        last_hash = Some(hash);

        match storage::upsert_clip(
            &state.db_path,
            snapshot.clip_type,
            &snapshot.content,
            &snapshot.preview,
            snapshot.source_app.as_deref(),
            &config,
        ) {
            Ok(clip) => emit_clip_added(&app, &clip)?,
            Err(err) => log::warn!("failed to persist clipboard item: {err}"),
        }
    }
}

/// Write a stored record back to the system clipboard.
pub fn write_clip_to_system(clip: &storage::ClipRecord) -> Result<(), String> {
    match clip.clip_type.as_str() {
        "files" => write_files_to_clipboard(&clip.content),
        "image" => write_image_to_clipboard(&clip.content),
        _ => write_text_to_clipboard(&clip.content),
    }
}

/// Simulate Ctrl+V after the panel is hidden. If this fails, the content is
/// still already in the system clipboard and the user can paste manually.
pub fn simulate_paste() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
        };

        fn input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: 0,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }

        let v_key = VIRTUAL_KEY(0x56);
        let inputs = [
            input(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
            input(v_key, KEYBD_EVENT_FLAGS(0)),
            input(v_key, KEYEVENTF_KEYUP),
            input(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize == inputs.len() {
            Ok(())
        } else {
            Err(format!("SendInput sent {sent}/{} events", inputs.len()))
        }
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Notify frontend that a new clip was added.
pub fn emit_clip_added(app: &AppHandle, clip: &storage::ClipRecord) -> Result<(), String> {
    app.emit("clip-added", clip).map_err(|e| e.to_string())
}

fn read_current_clipboard(source_app: Option<String>) -> Option<ClipboardSnapshot> {
    if let Some(files) = read_files_from_clipboard() {
        if !files.is_empty() {
            return Some(files_snapshot(files, source_app));
        }
    }

    if let Some(snapshot) = read_image_from_clipboard(source_app.clone()) {
        return Some(snapshot);
    }

    let text = read_text_from_clipboard()?;
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return None;
    }

    if let Some(files) = parse_file_paths_text(&text) {
        return Some(files_snapshot(files, source_app));
    }

    Some(ClipboardSnapshot {
        clip_type: "text",
        preview: make_text_preview(&text),
        content: text,
        source_app,
    })
}

fn read_text_from_clipboard() -> Option<String> {
    #[cfg(windows)]
    {
        use clipboard_win::{formats::Unicode, get_clipboard};
        get_clipboard(Unicode).ok()
    }

    #[cfg(not(windows))]
    {
        Clipboard::new().ok()?.get_text().ok()
    }
}

fn write_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| e.to_string())
}

fn read_files_from_clipboard() -> Option<Vec<String>> {
    #[cfg(windows)]
    {
        use clipboard_win::{
            formats::{FileList, Format},
            get_clipboard,
        };

        if !FileList.is_format_avail() {
            return None;
        }
        let files: Vec<String> = get_clipboard(FileList).ok()?;
        Some(files)
    }

    #[cfg(not(windows))]
    {
        None
    }
}

fn write_files_to_clipboard(content: &str) -> Result<(), String> {
    let files: Vec<String> = serde_json::from_str(content).map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        use clipboard_win::{formats::FileList, Clipboard as WinClipboard, Setter};
        let _clipboard = WinClipboard::new_attempts(10).map_err(|e| e.to_string())?;
        FileList
            .write_clipboard(files.as_slice())
            .map_err(|e| e.to_string())
    }

    #[cfg(not(windows))]
    {
        write_text_to_clipboard(&files.join("\n"))
    }
}

fn read_image_from_clipboard(source_app: Option<String>) -> Option<ClipboardSnapshot> {
    let mut clipboard = Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    if image.bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }

    let payload = ImageClip {
        width: image.width,
        height: image.height,
        rgba_base64: STANDARD.encode(image.bytes.as_ref()),
    };
    let content = serde_json::to_string(&payload).ok()?;
    Some(ClipboardSnapshot {
        clip_type: "image",
        preview: format!("[Image {}x{}]", image.width, image.height),
        content,
        source_app,
    })
}

fn write_image_to_clipboard(content: &str) -> Result<(), String> {
    let payload: ImageClip = serde_json::from_str(content).map_err(|e| e.to_string())?;
    let bytes = STANDARD
        .decode(payload.rgba_base64)
        .map_err(|e| e.to_string())?;
    let image = ImageData {
        width: payload.width,
        height: payload.height,
        bytes: Cow::Owned(bytes),
    };
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_image(image).map_err(|e| e.to_string())
}

fn files_snapshot(files: Vec<String>, source_app: Option<String>) -> ClipboardSnapshot {
    let preview = match files.len() {
        0 => "[Files]".to_string(),
        1 => format!("[File] {}", files[0]),
        count => format!("[Files] {count} items"),
    };

    ClipboardSnapshot {
        clip_type: "files",
        content: serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string()),
        preview,
        source_app,
    }
}

fn parse_file_paths_text(text: &str) -> Option<Vec<String>> {
    let paths = text
        .lines()
        .map(|line| line.trim().trim_matches('"'))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if paths.is_empty() || paths.len() > 64 {
        return None;
    }

    let all_paths = paths.iter().all(|path| {
        let path_ref = Path::new(path);
        path_ref.is_absolute() && (path_ref.exists() || looks_like_windows_path(path))
    });

    all_paths.then_some(paths)
}

fn looks_like_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() > 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

fn make_text_preview(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() > 160 {
        single_line.chars().take(160).collect::<String>() + "..."
    } else {
        single_line
    }
}

fn should_ignore_source(source_app: Option<&str>, ignored_apps: &[String]) -> bool {
    let Some(source) = source_app else {
        return false;
    };
    let source = source.to_ascii_lowercase();

    const SENSITIVE_HINTS: &[&str] = &[
        "password",
        "passcode",
        "pin",
        "1password",
        "bitwarden",
        "keepass",
        "lastpass",
        "dashlane",
        "login",
        "sign in",
        "signin",
        "credential",
    ];

    SENSITIVE_HINTS.iter().any(|hint| source.contains(hint))
        || ignored_apps
            .iter()
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .any(|ignored| source.contains(&ignored))
}

fn active_source_app() -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        };

        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return None;
        }

        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; len as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        if copied <= 0 {
            return None;
        }

        let title = String::from_utf16_lossy(&buffer[..copied as usize]);
        (!title.trim().is_empty()).then_some(title)
    }

    #[cfg(not(windows))]
    {
        None
    }
}
