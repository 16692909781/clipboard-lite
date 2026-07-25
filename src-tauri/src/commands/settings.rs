//! Settings and license commands.

use crate::{
    hotkey, license, plugin_loader, state::AppState, storage, storage::AppConfig, window_state,
};
use tauri::{AppHandle, State};

/// Read current app settings from config file.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state.config_snapshot()
}

/// Mark first-launch onboarding as complete after the main window has rendered.
#[tauri::command]
pub async fn complete_first_launch(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    if config.is_first_launch {
        config.is_first_launch = false;
        config.save(&state.data_dir)?;
    }
    Ok(config.clone())
}

/// Persist and immediately apply the main panel pinned/always-on-top state.
#[tauri::command]
pub async fn set_panel_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    pinned: bool,
) -> Result<AppConfig, String> {
    window_state::apply_pinned_mode(&app, pinned)?;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.is_pinned = pinned;
    config.save(&state.data_dir)?;
    Ok(config.clone())
}

/// Persist and apply the main panel position after a user drag without touching size.
#[tauri::command]
pub async fn set_window_position(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<AppConfig, String> {
    let (x, y) = window_state::apply_window_position(&app, x, y)?;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.window_x = Some(x);
    config.window_y = Some(y);
    config.save(&state.data_dir)?;
    Ok(config.clone())
}

/// Persist and apply the main panel geometry after proportional resize.
#[tauri::command]
pub async fn set_window_geometry(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<AppConfig, String> {
    let (x, y, width, height) = window_state::apply_window_geometry(&app, x, y, width, height)?;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.window_x = Some(x);
    config.window_y = Some(y);
    config.window_width = width;
    config.window_height = height;
    config.save(&state.data_dir)?;
    Ok(config.clone())
}

/// Persist settings and apply hotkey/autostart changes.
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppConfig,
) -> Result<(), String> {
    let previous = state.config_snapshot()?;
    let settings = settings.normalized();

    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();
    if let Err(err) = hotkey::register_hotkey(&app, &settings.hotkey) {
        let _ = hotkey::register_hotkey(&app, &previous.hotkey);
        return Err(format!("快捷键注册失败：{err}"));
    }

    apply_autostart(&app, settings.autostart)?;
    window_state::apply_pinned_mode(&app, settings.is_pinned)?;
    window_state::apply_saved_geometry(&app, &settings)?;

    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        *config = settings.clone();
        config.save(&state.data_dir)?;
    }
    storage::trim_to_config(&state.db_path, &settings)?;

    log::info!("settings saved");
    Ok(())
}

/// Activate Pro license with offline validation.
#[tauri::command]
pub async fn activate_license(app: AppHandle, code: String) -> Result<bool, String> {
    let activated = license::activate(&app, &code)?;
    if activated {
        if let Err(err) = plugin_loader::load_pro_plugin(&app) {
            log::warn!("pro plugin activation succeeded but load failed: {err}");
        }
    }
    Ok(activated)
}

/// Query Pro license status.
#[tauri::command]
pub async fn is_pro_licensed(app: AppHandle) -> Result<bool, String> {
    license::is_pro_licensed(&app)
}

/// Return the local device fingerprint used by offline Pro activation.
#[tauri::command]
pub async fn get_device_fingerprint() -> Result<String, String> {
    Ok(license::device_fingerprint())
}

/// Toggle main panel visibility (called from frontend on hotkey event).
#[tauri::command]
pub async fn toggle_panel(app: AppHandle) -> Result<(), String> {
    window_state::toggle_main_panel(&app);
    Ok(())
}

/// Pause or resume clipboard watching from tray/menu flows.
#[tauri::command]
pub async fn set_watcher_paused(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state.set_paused(paused);
    Ok(())
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}
