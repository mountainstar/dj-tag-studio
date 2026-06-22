use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

pub const DB_KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";
const DB_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Rekordbox database not found")]
    NotFound,
    #[error("Failed to open database: {0}")]
    Open(String),
    #[error("SQL error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("Rekordbox is running — close it before writing tags")]
    RekordboxRunning,
    #[error("{0}")]
    Other(String),
}

pub struct RekordboxDb {
    pub conn: Connection,
    pub path: PathBuf,
    pub mode: DatabaseMode,
}

pub fn detect_master_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let path = dirs_home()?.join("Library/Pioneer/rekordbox/master.db");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let path = PathBuf::from(appdata)
                .join("Pioneer")
                .join("rekordbox")
                .join("master.db");
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let path = dirs_home()?.join(".rekordbox/master.db");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn open_database(path: &Path, mode: DatabaseMode) -> Result<RekordboxDb, DbError> {
    if !path.exists() {
        return Err(DbError::NotFound);
    }

    let flags = match mode {
        DatabaseMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        DatabaseMode::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE,
    };

    let conn = Connection::open_with_flags(path, flags)
        .map_err(|e| DbError::Open(e.to_string()))?;

    conn.pragma_update(None, "key", DB_KEY)
        .map_err(|e| DbError::Open(e.to_string()))?;
    conn.pragma_update(None, "cipher_compatibility", 4i32)
        .map_err(|e| DbError::Open(e.to_string()))?;
    conn.busy_timeout(DB_BUSY_TIMEOUT)
        .map_err(|e| DbError::Open(e.to_string()))?;

    // Verify decryption worked
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|e| DbError::Open(format!("Database key invalid or corrupt: {e}")))?;

    Ok(RekordboxDb {
        conn,
        path: path.to_path_buf(),
        mode,
    })
}

pub fn next_usn(conn: &Connection) -> Result<i64, DbError> {
    next_local_usn(conn)
}

pub fn next_local_usn(conn: &Connection) -> Result<i64, DbError> {
    bump_registry_counter(conn, "localUpdateCount", max_local_usn(conn)?)
}

pub fn next_cloud_usn(conn: &Connection) -> Result<i64, DbError> {
    bump_registry_counter(conn, "lastUpdateCount", max_cloud_usn(conn)?)
}

pub fn next_usn_pair(conn: &Connection) -> Result<(i64, i64), DbError> {
    Ok((next_cloud_usn(conn)?, next_local_usn(conn)?))
}

fn bump_registry_counter(
    conn: &Connection,
    registry_id: &str,
    max_table: i64,
) -> Result<i64, DbError> {
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(int_1, 0) FROM agentRegistry WHERE registry_id = ?1",
            rusqlite::params![registry_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let next = current.max(max_table) + 1;
    let ts = now_timestamp();
    conn.execute(
        "UPDATE agentRegistry SET int_1 = ?1, updated_at = ?2 WHERE registry_id = ?3",
        rusqlite::params![next, ts, registry_id],
    )?;
    Ok(next)
}

pub fn sync_agent_registry(conn: &Connection) -> Result<(), DbError> {
    sync_registry_counter(conn, "localUpdateCount", max_local_usn(conn)?)?;
    sync_registry_counter(conn, "lastUpdateCount", max_cloud_usn(conn)?)?;
    Ok(())
}

fn sync_registry_counter(
    conn: &Connection,
    registry_id: &str,
    max_table: i64,
) -> Result<(), DbError> {
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(int_1, 0) FROM agentRegistry WHERE registry_id = ?1",
            rusqlite::params![registry_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if max_table > current {
        let ts = now_timestamp();
        conn.execute(
            "UPDATE agentRegistry SET int_1 = ?1, updated_at = ?2 WHERE registry_id = ?3",
            rusqlite::params![max_table, ts, registry_id],
        )?;
    }
    Ok(())
}

fn max_cloud_usn(conn: &Connection) -> Result<i64, DbError> {
    let mut max_usn = 0i64;
    for table in ["djmdContent", "djmdSongMyTag", "djmdMyTag"] {
        let table_max: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(MAX(usn), 0) FROM {table}"),
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        max_usn = max_usn.max(table_max);
    }
    Ok(max_usn)
}

fn max_local_usn(conn: &Connection) -> Result<i64, DbError> {
    let mut max_usn = 0i64;
    for table in ["djmdContent", "djmdSongMyTag", "djmdMyTag"] {
        let table_max: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(MAX(rb_local_usn), 0) FROM {table}"),
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        max_usn = max_usn.max(table_max);
    }
    Ok(max_usn)
}

pub fn touch_content(db: &RekordboxDb, content_id: &str) -> Result<(), DbError> {
    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    db.conn.execute(
        "UPDATE djmdContent SET usn = ?1, rb_local_usn = ?2, rb_data_status = 256, updated_at = ?3 WHERE ID = ?4",
        rusqlite::params![cloud_usn, local_usn, ts, content_id],
    )?;
    Ok(())
}

pub fn now_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f +00:00").to_string()
}

pub fn new_id() -> String {
    format!(
        "{}",
        (uuid::Uuid::new_v4().as_u128() % 2_900_000_000) + 1_000_000_000
    )
}

/// Rekordbox native My Tags use numeric string IDs, not UUID hex.
pub fn new_mytag_id(conn: &Connection) -> Result<String, DbError> {
    for _ in 0..128 {
        let n = uuid::Uuid::new_v4().as_u128();
        let candidate = format!("{}", (n % 2_900_000_000) + 1_000_000_000);
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM djmdMyTag WHERE ID = ?1",
            rusqlite::params![candidate],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(candidate);
        }
    }
    Err(DbError::Other("Could not allocate unique MyTag ID".into()))
}

/// Rekordbox uses dashed UUIDs for `djmdSongMyTag.ID` (not the hex form used elsewhere).
pub fn new_row_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Create a SongMyTag row id/uuid pair the way Rekordbox does (same dashed UUID).
pub fn new_song_mytag_row() -> (String, String) {
    let id = new_row_uuid();
    (id.clone(), id)
}

/// Rekordbox reads this flag on startup to rebuild in-memory My Tag state from `master.db`.
pub fn mark_my_tag_merge_needed(conn: &Connection) -> Result<(), DbError> {
    let ts = now_timestamp();
    conn.execute(
        "UPDATE agentRegistry SET int_1 = 1, updated_at = ?1 WHERE registry_id = 'needsToMergeMyTag'",
        rusqlite::params![ts],
    )?;
    Ok(())
}

/// Fix SongMyTag rows written in an older format (non-dashed ID, mismatched UUID).
pub fn repair_song_my_tag_rows(conn: &Connection) -> Result<(usize, usize, usize), DbError> {
    // Native Rekordbox rows use NULL TrackNo even with multiple tags per category.
    let track_no_fixed = conn.execute(
        "UPDATE djmdSongMyTag SET TrackNo = NULL WHERE TrackNo IS NOT NULL",
        [],
    )?;

    let mut stmt = conn.prepare(
        "SELECT ID FROM djmdSongMyTag WHERE ID NOT LIKE '%-%' AND length(ID) = 32",
    )?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut id_fixed = 0usize;
    for id in ids {
        if let Some(dashed) = dash_uuid_32(&id) {
            id_fixed += conn.execute(
                "UPDATE djmdSongMyTag SET ID = ?1 WHERE ID = ?2",
                rusqlite::params![dashed, id],
            )?;
        }
    }

    let uuid_synced = conn.execute(
        "UPDATE djmdSongMyTag SET UUID = ID WHERE COALESCE(rb_local_deleted, 0) = 0 AND UUID != ID",
        [],
    )?;

    Ok((track_no_fixed, id_fixed, uuid_synced))
}

fn dash_uuid_32(hex: &str) -> Option<String> {
    if hex.len() != 32 || hex.contains('-') {
        return None;
    }
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}
