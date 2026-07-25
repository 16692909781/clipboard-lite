//! Main window placement, sizing, and visibility helpers.

use crate::{
    state::AppState,
    storage::{self, AppConfig},
};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

/// Apply the stored always-on-top state to the main panel.
pub fn apply_pinned_mode(app: &AppHandle, pinned: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(pinned)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Apply the stored size and, when present, the stored position.
/// Returns true when a saved position was restored.
pub fn apply_saved_geometry(app: &AppHandle, config: &AppConfig) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        apply_geometry_to_window(&window, config)
    } else {
        Ok(false)
    }
}

pub fn apply_window_geometry(
    app: &AppHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(i32, i32, u32, u32), String> {
    let (width, height) = storage::normalize_window_geometry(width, height);
    let mut next_x = x;
    let mut next_y = y;

    if let Some(window) = app.get_webview_window("main") {
        let (clamped_x, clamped_y) = clamp_position_for_window(&window, x, y, width, height);
        next_x = clamped_x;
        next_y = clamped_y;

        window
            .set_size(PhysicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        window
            .set_position(PhysicalPosition::new(clamped_x, clamped_y))
            .map_err(|e| e.to_string())?;
    }

    Ok((next_x, next_y, width, height))
}

pub fn apply_window_position(app: &AppHandle, x: i32, y: i32) -> Result<(i32, i32), String> {
    let mut next_x = x;
    let mut next_y = y;

    if let Some(window) = app.get_webview_window("main") {
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let (clamped_x, clamped_y) =
            clamp_position_for_window(&window, x, y, size.width, size.height);
        next_x = clamped_x;
        next_y = clamped_y;

        window
            .set_position(PhysicalPosition::new(clamped_x, clamped_y))
            .map_err(|e| e.to_string())?;
    }

    Ok((next_x, next_y))
}

pub fn show_main_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let config = app
            .state::<AppState>()
            .config_snapshot()
            .unwrap_or_else(|_| AppConfig::default());

        let _ = window.set_always_on_top(config.is_pinned);
        let _ = window.unminimize();

        match apply_geometry_to_window(&window, &config) {
            Ok(true) => {}
            _ => {
                let _ = window.center();
            }
        }

        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_main_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn toggle_main_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => hide_main_panel(app),
            _ => show_main_panel(app),
        }
    }
}

fn apply_geometry_to_window<R: Runtime>(
    window: &WebviewWindow<R>,
    config: &AppConfig,
) -> Result<bool, String> {
    let (width, height) =
        storage::normalize_window_geometry(config.window_width, config.window_height);

    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;

    if let (Some(x), Some(y)) = (config.window_x, config.window_y) {
        let (x, y) = clamp_position_for_window(window, x, y, width, height);
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn clamp_position_for_window<R: Runtime>(
    window: &WebviewWindow<R>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> (i32, i32) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return (x, y);
    };

    let work_area = monitor.work_area();
    let min_x = work_area.position.x;
    let min_y = work_area.position.y;
    let max_x = min_x
        .saturating_add(work_area.size.width as i32)
        .saturating_sub(width as i32);
    let max_y = min_y
        .saturating_add(work_area.size.height as i32)
        .saturating_sub(height as i32);

    (clamp_axis(x, min_x, max_x), clamp_axis(y, min_y, max_y))
}

fn clamp_axis(value: i32, min: i32, max: i32) -> i32 {
    if max < min {
        min
    } else {
        value.clamp(min, max)
    }
}
