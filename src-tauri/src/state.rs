//! Shared application state.

use crate::storage::{self, AppConfig};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Manager};

/// Global state managed by Tauri.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    paused: AtomicBool,
}

impl AppState {
    /// Load config and initialize the SQLite database in the app data directory.
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        let config = match AppConfig::load(&data_dir) {
            Ok(config) => config,
            Err(err) => {
                log::warn!("{err}");
                AppConfig::default()
            }
        };
        let db_path = storage::db_path(&data_dir);
        storage::init_schema(&db_path)?;

        Ok(Self {
            config: Mutex::new(config),
            data_dir,
            db_path,
            paused: AtomicBool::new(false),
        })
    }

    pub fn config_snapshot(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|config| config.clone())
            .map_err(|e| e.to_string())
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }
}
