use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::rekordbox::db::{detect_master_db_path, open_database, DatabaseMode, DbError};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    /// Override path to Rekordbox `master.db`. Empty = use default location.
    #[serde(default)]
    pub custom_master_db_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsView {
    pub custom_master_db_path: Option<String>,
    pub default_master_db_path: Option<String>,
    pub resolved_master_db_path: Option<String>,
    pub settings_file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionTest {
    pub ok: bool,
    pub path: String,
    pub track_count: Option<usize>,
    pub message: String,
}

impl AppSettings {
    pub fn load() -> Self {
        settings_file_path()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn to_view(&self) -> SettingsView {
        SettingsView {
            custom_master_db_path: self.custom_master_db_path.clone(),
            default_master_db_path: default_master_db_path().map(|p| p.display().to_string()),
            resolved_master_db_path: self.resolve_master_db_path().map(|p| p.display().to_string()),
            settings_file_path: settings_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| e),
        }
    }

    pub fn resolve_master_db_path(&self) -> Option<PathBuf> {
        if let Some(custom) = self.custom_master_db_path.as_ref() {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                let path = PathBuf::from(trimmed);
                if path.is_file() {
                    return Some(path);
                }
                return None;
            }
        }
        detect_master_db_path()
    }
}

pub fn default_master_db_path() -> Option<PathBuf> {
    detect_master_db_path()
}

pub fn test_db_connection(path: &str) -> DbConnectionTest {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return DbConnectionTest {
            ok: false,
            path: String::new(),
            track_count: None,
            message: "Enter a path to master.db first.".into(),
        };
    }

    let path_buf = PathBuf::from(trimmed);
    if !path_buf.is_file() {
        return DbConnectionTest {
            ok: false,
            path: trimmed.to_string(),
            track_count: None,
            message: "File not found. Choose your Rekordbox master.db file.".into(),
        };
    }

    match open_database(&path_buf, DatabaseMode::ReadOnly) {
        Ok(db) => match db.conn.query_row(
            "SELECT COUNT(*) FROM djmdContent WHERE COALESCE(rb_local_deleted, 0) = 0",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => DbConnectionTest {
                ok: true,
                path: trimmed.to_string(),
                track_count: Some(count as usize),
                message: format!("Connected — {count} tracks in library."),
            },
            Err(e) => DbConnectionTest {
                ok: false,
                path: trimmed.to_string(),
                track_count: None,
                message: format!("Opened database but query failed: {e}"),
            },
        },
        Err(DbError::Open(msg)) if msg.contains("key invalid") => DbConnectionTest {
            ok: false,
            path: trimmed.to_string(),
            track_count: None,
            message: "File found but it is not a valid Rekordbox master.db.".into(),
        },
        Err(e) => DbConnectionTest {
            ok: false,
            path: trimmed.to_string(),
            track_count: None,
            message: e.to_string(),
        },
    }
}

fn settings_file_path() -> Result<PathBuf, String> {
    let base = config_dir().ok_or("Could not determine settings directory")?;
    Ok(base.join("settings.json"))
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs_home().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("dj-tag-studio")
        })
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|appdata| {
            PathBuf::from(appdata).join("dj-tag-studio")
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs_home().map(|home| home.join(".config").join("dj-tag-studio"))
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_custom_uses_default_detection() {
        let settings = AppSettings::default();
        assert_eq!(
            settings.resolve_master_db_path().as_deref(),
            detect_master_db_path().as_deref()
        );
    }

    #[test]
    fn invalid_custom_returns_none() {
        let settings = AppSettings {
            custom_master_db_path: Some("/no/such/master.db".into()),
        };
        assert!(settings.resolve_master_db_path().is_none());
    }
}
