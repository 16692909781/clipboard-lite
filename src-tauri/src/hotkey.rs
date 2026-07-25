//! Global hotkey registration.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const DEFAULT_HOTKEY: &str = "Ctrl+Shift+V";

/// Register the default global shortcut to toggle the main panel.
pub fn register_default(app: &AppHandle) -> Result<(), String> {
    register_hotkey(app, DEFAULT_HOTKEY)
}

/// Register a custom hotkey string like `Ctrl+Shift+V`.
pub fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut = parse_hotkey(hotkey)?;

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::window_state::toggle_main_panel(&app_handle);
            }
        })
        .map_err(|e| e.to_string())?;

    log::info!("registered hotkey: {hotkey}");
    Ok(())
}

/// Parse a simple hotkey string into Tauri Shortcut.
fn parse_hotkey(raw: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err("empty hotkey".into());
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            key if key.len() == 1 => {
                key_code = Some(char_to_code(key.chars().next().unwrap())?);
            }
            other => return Err(format!("unsupported hotkey segment: {other}")),
        }
    }

    let code = key_code.ok_or_else(|| "hotkey missing key".to_string())?;
    Ok(Shortcut::new(Some(modifiers), code))
}

fn char_to_code(ch: char) -> Result<Code, String> {
    match ch.to_ascii_uppercase() {
        'A' => Ok(Code::KeyA),
        'B' => Ok(Code::KeyB),
        'C' => Ok(Code::KeyC),
        'D' => Ok(Code::KeyD),
        'E' => Ok(Code::KeyE),
        'F' => Ok(Code::KeyF),
        'G' => Ok(Code::KeyG),
        'H' => Ok(Code::KeyH),
        'I' => Ok(Code::KeyI),
        'J' => Ok(Code::KeyJ),
        'K' => Ok(Code::KeyK),
        'L' => Ok(Code::KeyL),
        'M' => Ok(Code::KeyM),
        'N' => Ok(Code::KeyN),
        'O' => Ok(Code::KeyO),
        'P' => Ok(Code::KeyP),
        'Q' => Ok(Code::KeyQ),
        'R' => Ok(Code::KeyR),
        'S' => Ok(Code::KeyS),
        'T' => Ok(Code::KeyT),
        'U' => Ok(Code::KeyU),
        'V' => Ok(Code::KeyV),
        'W' => Ok(Code::KeyW),
        'X' => Ok(Code::KeyX),
        'Y' => Ok(Code::KeyY),
        'Z' => Ok(Code::KeyZ),
        '0' => Ok(Code::Digit0),
        '1' => Ok(Code::Digit1),
        '2' => Ok(Code::Digit2),
        '3' => Ok(Code::Digit3),
        '4' => Ok(Code::Digit4),
        '5' => Ok(Code::Digit5),
        '6' => Ok(Code::Digit6),
        '7' => Ok(Code::Digit7),
        '8' => Ok(Code::Digit8),
        '9' => Ok(Code::Digit9),
        _ => Err(format!("unsupported key: {ch}")),
    }
}
