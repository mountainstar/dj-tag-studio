use std::path::{Path, PathBuf};

use chrono::Local;

use super::db::DbError;

pub fn backup_master_db(db_path: &Path) -> Result<PathBuf, DbError> {
    let parent = db_path
        .parent()
        .ok_or_else(|| DbError::Other("Invalid database path".into()))?;
    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = parent.join(format!(
        "master.db.dj-tag-studio-backup-{}.db",
        stamp
    ));
    std::fs::copy(db_path, &backup_path)
        .map_err(|e| DbError::Other(format!("Backup failed: {e}")))?;
    Ok(backup_path)
}
