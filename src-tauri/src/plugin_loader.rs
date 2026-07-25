//! Dynamic Pro plugin loader.
//!
//! The open-source core only defines a tiny, stable C ABI. Closed-source Pro
//! plugins can be built separately and dropped into the bundled `plugins/`
//! directory. Missing or incompatible plugins never block core startup.

use libloading::Library;
use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
};
use tauri::{AppHandle, Manager};

pub const PLUGIN_API_VERSION: u32 = 1;

#[repr(C)]
pub struct HostApi {
    pub api_version: u32,
}

#[repr(C)]
pub struct PluginInfo {
    pub api_version: u32,
    pub name: *const c_char,
    pub version: *const c_char,
    pub feature_flags: u64,
}

type PluginInfoFn = unsafe extern "C" fn() -> PluginInfo;
type PluginLoadFn = unsafe extern "C" fn(host: *const HostApi) -> i32;

/// Attempt to load the Pro plugin dynamic library from the plugins directory.
pub fn load_pro_plugin(app: &AppHandle) -> Result<(), String> {
    let plugin_path = resolve_plugin_path(app)?;
    if !plugin_path.exists() {
        return Err(format!("pro plugin not found: {}", plugin_path.display()));
    }

    let library = unsafe { Library::new(&plugin_path) }.map_err(|e| e.to_string())?;

    let info = unsafe {
        let get_info = library
            .get::<PluginInfoFn>(b"clipboard_lite_plugin_info\0")
            .map_err(|e| e.to_string())?;
        get_info()
    };

    if info.api_version != PLUGIN_API_VERSION {
        return Err(format!(
            "plugin api mismatch: host={}, plugin={}",
            PLUGIN_API_VERSION, info.api_version
        ));
    }

    let name = cstr_to_string(info.name).unwrap_or_else(|| "Unnamed Pro Plugin".to_string());
    let version = cstr_to_string(info.version).unwrap_or_else(|| "unknown".to_string());

    unsafe {
        let on_load = library
            .get::<PluginLoadFn>(b"clipboard_lite_plugin_load\0")
            .map_err(|e| e.to_string())?;
        let host = HostApi {
            api_version: PLUGIN_API_VERSION,
        };
        let code = on_load(&host);
        if code != 0 {
            return Err(format!("plugin load returned error code {code}"));
        }
    }

    // Keep the dynamic library resident for the lifetime of the process. The
    // MVP ABI does not yet support unloading registered menu/pages safely.
    std::mem::forget(library);
    log::info!("loaded pro plugin: {name} {version}");
    Ok(())
}

fn resolve_plugin_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    let filename = "pro-plugin.dll";
    #[cfg(target_os = "macos")]
    let filename = "libpro-plugin.dylib";
    #[cfg(target_os = "linux")]
    let filename = "libpro-plugin.so";

    Ok(resource_dir.join("plugins").join(filename))
}

fn cstr_to_string(raw: *const c_char) -> Option<String> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }
        .to_str()
        .ok()
        .map(ToOwned::to_owned)
}
