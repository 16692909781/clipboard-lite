//! SQLite storage layer for clipboard history and app config.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const CONFIG_FILE: &str = "config.json";
const DB_FILE: &str = "clipboard.db";
pub const DEFAULT_WINDOW_WIDTH: u32 = 420;
pub const DEFAULT_WINDOW_HEIGHT: u32 = 520;
pub const MIN_WINDOW_WIDTH: u32 = 320;

/// A single clipboard history record returned to the Vue UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipRecord {
    pub id: i64,
    pub content: String,
    pub content_hash: String,
    pub clip_type: String,
    pub preview: String,
    pub is_favorite: bool,
    pub source_app: Option<String>,
    pub created_at: i64,
}

/// User-configurable settings stored as JSON in the app data directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub max_count: u32,
    pub hotkey: String,
    pub theme: String,
    pub retention_days: u32,
    pub ignored_apps: Vec<String>,
    pub autostart: bool,
    #[serde(default = "default_first_launch", rename = "is_first_launch")]
    pub is_first_launch: bool,
    #[serde(default, rename = "is_pinned")]
    pub is_pinned: bool,
    #[serde(default, rename = "window_x")]
    pub window_x: Option<i32>,
    #[serde(default, rename = "window_y")]
    pub window_y: Option<i32>,
    #[serde(default = "default_window_width", rename = "window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height", rename = "window_height")]
    pub window_height: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_count: 500,
            hotkey: "Ctrl+Shift+V".to_string(),
            theme: "system".to_string(),
            retention_days: 30,
            ignored_apps: vec![],
            autostart: false,
            is_first_launch: true,
            is_pinned: false,
            window_x: None,
            window_y: None,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
        }
    }
}

fn default_first_launch() -> bool {
    true
}

fn default_window_width() -> u32 {
    DEFAULT_WINDOW_WIDTH
}

fn default_window_height() -> u32 {
    DEFAULT_WINDOW_HEIGHT
}

pub fn normalize_window_geometry(width: u32, height: u32) -> (u32, u32) {
    let width = if width == 0 && height > 0 {
        ((height as f64) * (DEFAULT_WINDOW_WIDTH as f64) / (DEFAULT_WINDOW_HEIGHT as f64)).round()
            as u32
    } else {
        width
    }
    .max(MIN_WINDOW_WIDTH);

    let height = ((width as f64) * (DEFAULT_WINDOW_HEIGHT as f64) / (DEFAULT_WINDOW_WIDTH as f64))
        .round() as u32;

    (width, height.max(1))
}

impl AppConfig {
    /// Load config from `{data_dir}/config.json`, creating defaults if missing.
    /// Corrupt JSON is backed up and replaced with defaults so the tray app can
    /// still start.
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join(CONFIG_FILE);
        if !path.exists() {
            let config = Self::default();
            config.save(data_dir)?;
            return Ok(config);
        }

        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        match serde_json::from_str::<Self>(&raw) {
            Ok(mut config) => {
                config.normalize();
                config.save(data_dir)?;
                Ok(config)
            }
            Err(err) => {
                let backup = data_dir.join(format!(
                    "config.corrupt-{}.json",
                    Utc::now().timestamp_millis()
                ));
                let _ = fs::rename(&path, backup);
                let config = Self::default();
                config.save(data_dir)?;
                Err(format!("config reset after parse failure: {err}"))
            }
        }
    }

    /// Persist config to `{data_dir}/config.json`.
    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        let path = data_dir.join(CONFIG_FILE);
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, raw).map_err(|e| e.to_string())
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    fn normalize(&mut self) {
        self.max_count = self.max_count.clamp(50, 5_000);
        if self.hotkey.trim().is_empty() {
            self.hotkey = "Ctrl+Shift+V".to_string();
        }
        if !matches!(self.theme.as_str(), "light" | "dark" | "system") {
            self.theme = "system".to_string();
        }
        self.ignored_apps.retain(|app| !app.trim().is_empty());
        self.ignored_apps.sort();
        self.ignored_apps.dedup();

        let (window_width, window_height) =
            normalize_window_geometry(self.window_width, self.window_height);
        self.window_width = window_width;
        self.window_height = window_height;
    }
}

pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DB_FILE)
}

/// Build the stable dedupe hash for a record type and content payload.
pub fn content_hash(clip_type: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(clip_type.as_bytes());
    hasher.update([0]);
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Initialize database schema and pragmas. A database that cannot be opened is
/// moved aside and recreated so a broken history file does not brick startup.
pub fn init_schema(db_path: &Path) -> Result<(), String> {
    match init_schema_inner(db_path) {
        Ok(()) => Ok(()),
        Err(first_err) => {
            if db_path.exists() {
                let backup =
                    db_path.with_extension(format!("corrupt-{}.db", Utc::now().timestamp_millis()));
                let _ = fs::rename(db_path, backup);
                init_schema_inner(db_path).map_err(|second_err| {
                    format!("database recovery failed: {first_err}; retry: {second_err}")
                })
            } else {
                Err(first_err)
            }
        }
    }
}

fn init_schema_inner(db_path: &Path) -> Result<(), String> {
    let conn = open(db_path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        CREATE TABLE IF NOT EXISTS clips (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            content       TEXT NOT NULL,
            content_hash  TEXT NOT NULL UNIQUE,
            clip_type     TEXT NOT NULL,
            preview       TEXT NOT NULL DEFAULT '',
            is_favorite   INTEGER NOT NULL DEFAULT 0,
            source_app    TEXT,
            created_at    INTEGER NOT NULL,
            updated_at    INTEGER NOT NULL,
            expires_at    INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_clips_created
            ON clips(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_favorite
            ON clips(is_favorite DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_type
            ON clips(clip_type);
        "#,
    )
    .map_err(|e| e.to_string())
}

fn open(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(2))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

/// Insert a new clip or move an existing duplicate to the top.
pub fn upsert_clip(
    db_path: &Path,
    clip_type: &str,
    content: &str,
    preview: &str,
    source_app: Option<&str>,
    config: &AppConfig,
) -> Result<ClipRecord, String> {
    let conn = open(db_path)?;
    let now = Utc::now().timestamp_millis();
    let hash = content_hash(clip_type, content);
    let expires_at = if config.retention_days == 0 {
        None
    } else {
        Some(now + i64::from(config.retention_days) * 24 * 60 * 60 * 1_000)
    };

    conn.execute(
        r#"
        INSERT INTO clips (
            content, content_hash, clip_type, preview, source_app,
            created_at, updated_at, expires_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)
        ON CONFLICT(content_hash) DO UPDATE SET
            content = excluded.content,
            clip_type = excluded.clip_type,
            preview = excluded.preview,
            source_app = excluded.source_app,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            expires_at = excluded.expires_at
        "#,
        params![content, hash, clip_type, preview, source_app, now, expires_at],
    )
    .map_err(|e| e.to_string())?;

    cleanup(&conn, config)?;
    get_clip_by_hash(&conn, &hash)
}

pub fn get_clips(
    db_path: &Path,
    limit: u32,
    offset: u32,
    search: Option<String>,
) -> Result<Vec<ClipRecord>, String> {
    let conn = open(db_path)?;
    cleanup_expired(&conn)?;

    let limit = i64::from(limit.clamp(1, 500));
    let offset = i64::from(offset.min(10_000));
    let trimmed = search.unwrap_or_default();
    let query = trimmed.trim();

    if query.is_empty() {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, content, content_hash, clip_type, preview, is_favorite,
                       source_app, created_at
                FROM clips
                ORDER BY is_favorite DESC, created_at DESC
                LIMIT ?1 OFFSET ?2
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit, offset], row_to_clip)
            .map_err(|e| e.to_string())?;
        collect_rows(rows)
    } else {
        let like = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, content, content_hash, clip_type, preview, is_favorite,
                       source_app, created_at
                FROM clips
                WHERE preview LIKE ?1 OR content LIKE ?1 OR source_app LIKE ?1
                ORDER BY is_favorite DESC, created_at DESC
                LIMIT ?2 OFFSET ?3
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![like, limit, offset], row_to_clip)
            .map_err(|e| e.to_string())?;
        collect_rows(rows)
    }
}

pub fn get_clip(db_path: &Path, id: i64) -> Result<ClipRecord, String> {
    let conn = open(db_path)?;
    conn.query_row(
        r#"
        SELECT id, content, content_hash, clip_type, preview, is_favorite,
               source_app, created_at
        FROM clips
        WHERE id = ?1
        "#,
        params![id],
        row_to_clip,
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("clip not found: {id}"))
}

pub fn delete_clip(db_path: &Path, id: i64) -> Result<(), String> {
    let conn = open(db_path)?;
    conn.execute("DELETE FROM clips WHERE id = ?1", params![id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn clear_all_clips(db_path: &Path) -> Result<(), String> {
    let conn = open(db_path)?;
    conn.execute("DELETE FROM clips", [])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn toggle_favorite(db_path: &Path, id: i64) -> Result<bool, String> {
    let conn = open(db_path)?;
    let current = conn
        .query_row(
            "SELECT is_favorite FROM clips WHERE id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("clip not found: {id}"))?;
    let next = if current == 0 { 1 } else { 0 };
    conn.execute(
        "UPDATE clips SET is_favorite = ?1, updated_at = ?2 WHERE id = ?3",
        params![next, Utc::now().timestamp_millis(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(next == 1)
}

pub fn trim_to_config(db_path: &Path, config: &AppConfig) -> Result<(), String> {
    let conn = open(db_path)?;
    cleanup(&conn, config)
}

fn cleanup(conn: &Connection, config: &AppConfig) -> Result<(), String> {
    cleanup_expired(conn)?;
    let max_count = i64::from(config.max_count.clamp(50, 5_000));
    conn.execute(
        r#"
        DELETE FROM clips
        WHERE is_favorite = 0
          AND id NOT IN (
            SELECT id
            FROM clips
            ORDER BY is_favorite DESC, created_at DESC
            LIMIT ?1
          )
        "#,
        params![max_count],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn cleanup_expired(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM clips WHERE is_favorite = 0 AND expires_at IS NOT NULL AND expires_at < ?1",
        params![Utc::now().timestamp_millis()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn get_clip_by_hash(conn: &Connection, hash: &str) -> Result<ClipRecord, String> {
    conn.query_row(
        r#"
        SELECT id, content, content_hash, clip_type, preview, is_favorite,
               source_app, created_at
        FROM clips
        WHERE content_hash = ?1
        "#,
        params![hash],
        row_to_clip,
    )
    .map_err(|e| e.to_string())
}

fn collect_rows<F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<ClipRecord>, String>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<ClipRecord>,
{
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn row_to_clip(row: &Row<'_>) -> rusqlite::Result<ClipRecord> {
    Ok(ClipRecord {
        id: row.get(0)?,
        content: row.get(1)?,
        content_hash: row.get(2)?,
        clip_type: row.get(3)?,
        preview: row.get(4)?,
        is_favorite: row.get::<_, i64>(5)? != 0,
        source_app: row.get(6)?,
        created_at: row.get(7)?,
    })
}
