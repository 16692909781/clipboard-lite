//! Clipboard Lite — Rust backend entry modules.

pub mod clipboard;
pub mod commands;
pub mod hotkey;
pub mod license;
pub mod plugin_loader;
pub mod state;
pub mod storage;
pub mod window_state;

use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

/// Initialize logging, database, tray, hotkeys, and clipboard watcher.
fn setup_app(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let handle = app.handle().clone();
    let app_state = AppState::new(&handle)?;
    let startup_config = app_state.config_snapshot()?;
    let configured_hotkey = startup_config.hotkey.clone();
    let is_first_launch = startup_config.is_first_launch;
    let is_pinned = startup_config.is_pinned;
    app.manage(app_state);
    let _ = window_state::apply_pinned_mode(&handle, is_pinned);
    let _ = window_state::apply_saved_geometry(&handle, &startup_config);

    // System tray
    let show_item = MenuItem::with_id(app, "show", "打开面板", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", "暂停/恢复监听", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&show_item, &settings_item, &pause_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .tooltip("Clipboard Lite")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window_state::show_main_panel(app),
            "settings" => open_settings(app),
            "pause" => {
                let state = app.state::<AppState>();
                state.set_paused(!state.is_paused());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                window_state::toggle_main_panel(&app);
            }
        })
        .build(app)?;

    // Global shortcut (default Ctrl+Shift+V). A conflict should not prevent
    // the tray app and panel from starting.
    if let Err(err) = hotkey::register_hotkey(&handle, &configured_hotkey) {
        log::warn!("global hotkey registration skipped: {err}");
        let _ = handle.emit(
            "app-warning",
            format!("全局快捷键注册失败：{err}。可在设置中换一个快捷键。"),
        );
    }

    // Start clipboard watcher in background thread
    let watcher_handle = handle.clone();
    std::thread::spawn(move || {
        if let Err(err) = clipboard::start_watcher(watcher_handle) {
            log::error!("clipboard watcher failed: {err}");
        }
    });

    // Pro plugin load (only when licensed)
    if license::is_pro_licensed(&handle)? {
        if let Err(err) = plugin_loader::load_pro_plugin(&handle) {
            log::warn!("pro plugin load skipped: {err}");
        }
    }

    if is_first_launch {
        window_state::show_main_panel(&handle);
    }

    Ok(())
}

fn open_settings(app: &tauri::AppHandle) {
    window_state::show_main_panel(app);
    let _ = app.emit("navigate", "/settings");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::clip::get_clips,
            commands::clip::delete_clip,
            commands::clip::clear_all_clips,
            commands::clip::toggle_favorite,
            commands::clip::paste_clip,
            commands::settings::get_settings,
            commands::settings::complete_first_launch,
            commands::settings::set_panel_pinned,
            commands::settings::set_window_position,
            commands::settings::set_window_geometry,
            commands::settings::save_settings,
            commands::settings::activate_license,
            commands::settings::is_pro_licensed,
            commands::settings::get_device_fingerprint,
            commands::settings::toggle_panel,
            commands::settings::set_watcher_paused,
        ])
        .run(tauri::generate_context!())
        .expect("error while running clipboard-lite");
}
