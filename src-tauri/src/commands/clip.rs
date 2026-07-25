//! Clipboard history commands.

use crate::{clipboard, state::AppState, storage};
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};

/// Fetch clipboard history with optional search filter.
#[tauri::command]
pub async fn get_clips(
    state: State<'_, AppState>,
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
) -> Result<Vec<storage::ClipRecord>, String> {
    storage::get_clips(
        &state.db_path,
        limit.unwrap_or(100),
        offset.unwrap_or(0),
        search,
    )
}

/// Delete a single clip by id.
#[tauri::command]
pub async fn delete_clip(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    storage::delete_clip(&state.db_path, id)
}

/// Clear all clipboard history.
#[tauri::command]
pub async fn clear_all_clips(state: State<'_, AppState>) -> Result<(), String> {
    storage::clear_all_clips(&state.db_path)
}

/// Toggle favorite status for a clip.
#[tauri::command]
pub async fn toggle_favorite(state: State<'_, AppState>, id: i64) -> Result<bool, String> {
    storage::toggle_favorite(&state.db_path, id)
}

/// Write clip to system clipboard, hide the panel, then simulate Ctrl+V.
#[tauri::command]
pub async fn paste_clip(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let clip = storage::get_clip(&state.db_path, id)?;
    clipboard::write_clip_to_system(&clip)?;

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    thread::sleep(Duration::from_millis(80));
    if let Err(err) = clipboard::simulate_paste() {
        log::warn!("paste simulation failed after clipboard write: {err}");
        let _ = app.emit(
            "app-warning",
            "已写入系统剪贴板，但自动粘贴失败，可手动 Ctrl+V。",
        );
    }
    Ok(())
}
