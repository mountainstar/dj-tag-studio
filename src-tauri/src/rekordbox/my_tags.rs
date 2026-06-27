use rusqlite::{params, OptionalExtension};

use crate::types::{MyTagDef, PendingChange, TagGroup, TagPack};

use super::db::{
    mark_my_tag_merge_needed, new_mytag_id, new_song_mytag_row, next_usn_pair, now_timestamp,
    sync_agent_registry, touch_content, DbError, RekordboxDb,
};

/// Rekordbox sets this on existing `djmdSongMyTag` rows; required for the app to pick them up.
const RB_DATA_STATUS_ACTIVE: i64 = 256;
/// Rekordbox tombstone flag for soft-deleted `djmdSongMyTag` rows.
const RB_DATA_STATUS_DELETED: i64 = 262;

fn is_native_numeric_mytag_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

fn find_native_mytag_id(
    conn: &rusqlite::Connection,
    parent_id: &str,
    name: &str,
) -> Result<Option<String>, DbError> {
    conn.query_row(
        "SELECT ID FROM djmdMyTag
         WHERE ParentID = ?1 AND Name = ?2 COLLATE NOCASE
           AND COALESCE(rb_local_deleted, 0) = 0
           AND ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*'
         LIMIT 1",
        params![parent_id, name],
        |row| row.get(0),
    )
    .optional()
    .map_err(DbError::from)
}

/// Prefer Rekordbox-native numeric My Tag IDs over UUID-style pack duplicates.
pub fn prefer_native_mytag_id(db: &RekordboxDb, tag_id: &str) -> Result<String, DbError> {
    if is_native_numeric_mytag_id(tag_id) {
        return Ok(tag_id.to_string());
    }

    let (name, parent_id): (String, String) = db.conn.query_row(
        "SELECT Name, ParentID FROM djmdMyTag WHERE ID = ?1 AND COALESCE(rb_local_deleted, 0) = 0",
        params![tag_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if let Some(native_id) = find_native_mytag_id(&db.conn, &parent_id, &name)? {
        return Ok(native_id);
    }

    Err(DbError::Other(format!(
        "Tag \"{name}\" is not a native Rekordbox tag — reload the library and pick tags from your Rekordbox list (not the old default layout pack)."
    )))
}

fn prefer_native_tags_for_display(tags: Vec<MyTagDef>) -> Vec<MyTagDef> {
    let mut out: Vec<MyTagDef> = tags
        .into_iter()
        .filter(|tag| is_native_numeric_mytag_id(&tag.id))
        .collect();
    out.sort_by_key(|t| t.seq);
    out
}

pub fn repair_duplicate_mytag_defs(db: &RekordboxDb) -> Result<(usize, usize, usize), DbError> {
    let remapped = remap_song_mytags_to_native(db)?;
    let links_removed = remove_pack_mytag_links(db)?;
    let removed = soft_delete_all_pack_mytag_defs(db)?;
    Ok((remapped, links_removed, removed))
}

fn remove_pack_mytag_links(db: &RekordboxDb) -> Result<usize, DbError> {
    let removed = db.conn.execute(
        "DELETE FROM djmdSongMyTag
         WHERE MyTagID IN (
           SELECT ID FROM djmdMyTag
           WHERE ParentID IN ('1', '2', '3', '4')
             AND NOT (ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*')
         )",
        [],
    )?;
    Ok(removed)
}

fn soft_delete_all_pack_mytag_defs(db: &RekordboxDb) -> Result<usize, DbError> {
    let ids: Vec<String> = db
        .conn
        .prepare(
            "SELECT ID FROM djmdMyTag
             WHERE COALESCE(rb_local_deleted, 0) = 0
               AND ParentID IN ('1', '2', '3', '4')
               AND NOT (ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*')",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut removed = 0usize;
    for id in ids {
        let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
        let ts = now_timestamp();
        removed += db.conn.execute(
            "UPDATE djmdMyTag SET rb_local_deleted = 1, usn = ?1, rb_local_usn = ?2, updated_at = ?3 WHERE ID = ?4",
            params![cloud_usn, local_usn, ts, id],
        )?;
    }
    Ok(removed)
}

pub fn purge_track_tombstones(db: &RekordboxDb, content_id: &str) -> Result<usize, DbError> {
    let removed = db.conn.execute(
        "DELETE FROM djmdSongMyTag WHERE ContentID = ?1 AND COALESCE(rb_local_deleted, 0) = 1",
        params![content_id],
    )?;
    Ok(removed)
}

fn dedupe_all_active_track_tags(db: &RekordboxDb, content_id: &str) -> Result<(), DbError> {
    let mut stmt = db.conn.prepare(
        "SELECT DISTINCT MyTagID FROM djmdSongMyTag
         WHERE ContentID = ?1 AND COALESCE(rb_local_deleted, 0) = 0",
    )?;
    let tag_ids: Vec<String> = stmt
        .query_map(params![content_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for tag_id in tag_ids {
        dedupe_active_song_mytags(db, content_id, &tag_id)?;
    }
    Ok(())
}

fn remap_song_mytags_to_native(db: &RekordboxDb) -> Result<usize, DbError> {
    let mut stmt = db.conn.prepare(
        "SELECT sm.ID, sm.ContentID, sm.MyTagID, t.Name, t.ParentID
         FROM djmdSongMyTag sm
         JOIN djmdMyTag t ON t.ID = sm.MyTagID
         WHERE COALESCE(sm.rb_local_deleted, 0) = 0
           AND NOT (sm.MyTagID GLOB '[0-9]*' AND sm.MyTagID NOT GLOB '*[^0-9]*')",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut remapped = 0usize;
    for (row_id, content_id, mytag_id, name, parent_id) in rows {
        let Some(native_id) = find_native_mytag_id(&db.conn, &parent_id, &name)? else {
            continue;
        };
        if native_id == mytag_id {
            continue;
        }

        let native_exists: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM djmdSongMyTag
             WHERE ContentID = ?1 AND MyTagID = ?2 AND COALESCE(rb_local_deleted, 0) = 0",
            params![content_id, native_id],
            |row| row.get(0),
        )?;

        let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
        let ts = now_timestamp();

        if native_exists > 0 {
            db.conn.execute(
                "UPDATE djmdSongMyTag SET rb_local_deleted = 1, rb_data_status = ?1, usn = ?2, rb_local_usn = ?3, updated_at = ?4 WHERE ID = ?5",
                params![RB_DATA_STATUS_DELETED, cloud_usn, local_usn, ts, row_id],
            )?;
        } else {
            db.conn.execute(
                "UPDATE djmdSongMyTag SET MyTagID = ?1, usn = ?2, rb_local_usn = ?3, updated_at = ?4 WHERE ID = ?5",
                params![native_id, cloud_usn, local_usn, ts, row_id],
            )?;
        }
        remapped += 1;
    }

    Ok(remapped)
}

fn dedupe_active_song_mytags(
    db: &RekordboxDb,
    content_id: &str,
    tag_id: &str,
) -> Result<(), DbError> {
    let mut stmt = db.conn.prepare(
        "SELECT ID FROM djmdSongMyTag
         WHERE ContentID = ?1 AND MyTagID = ?2 AND COALESCE(rb_local_deleted, 0) = 0
         ORDER BY updated_at DESC",
    )?;
    let ids: Vec<String> = stmt
        .query_map(params![content_id, tag_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if ids.len() <= 1 {
        return Ok(());
    }

    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    for dup_id in ids.iter().skip(1) {
        db.conn.execute(
            "UPDATE djmdSongMyTag SET rb_local_deleted = 1, rb_data_status = ?1, usn = ?2, rb_local_usn = ?3, updated_at = ?4 WHERE ID = ?5",
            params![RB_DATA_STATUS_DELETED, cloud_usn, local_usn, ts, dup_id],
        )?;
    }
    Ok(())
}

pub fn load_tag_groups(db: &RekordboxDb) -> Result<Vec<TagGroup>, DbError> {
    let mut stmt = db.conn.prepare(
        r#"
        SELECT ID, Name, Seq, ParentID, Attribute
        FROM djmdMyTag
        WHERE COALESCE(rb_local_deleted, 0) = 0
        ORDER BY Seq ASC
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let sections: Vec<(String, String, i64)> = rows
        .iter()
        .filter(|(_, _, _, parent, attr)| {
            parent.is_empty() || parent == "root" || *attr == 1
        })
        .map(|(id, name, seq, _, _)| (id.clone(), name.clone(), *seq))
        .collect();

    let mut groups = Vec::new();
    for (section_id, section_name, section_seq) in sections {
        let tags = prefer_native_tags_for_display(
            rows.iter()
                .filter(|(_, _, _, parent, _)| parent == &section_id)
                .map(|(id, name, seq, group_id, _)| MyTagDef {
                    id: id.clone(),
                    name: name.clone(),
                    group_id: group_id.clone(),
                    seq: *seq,
                })
                .collect(),
        );

        groups.push(TagGroup {
            id: section_id,
            name: section_name,
            seq: section_seq,
            tags,
        });
    }

    Ok(groups)
}

pub fn apply_tag_pack(db: &mut RekordboxDb, pack: &TagPack) -> Result<Vec<TagGroup>, DbError> {
    if pack.groups.len() > 4 {
        return Err(DbError::Other(
            "Rekordbox supports at most 4 My Tag groups".into(),
        ));
    }

    db.conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = apply_tag_pack_inner(db, pack);
    if result.is_ok() {
        db.conn.execute_batch("COMMIT")?;
    } else {
        let _ = db.conn.execute_batch("ROLLBACK");
    }
    result
}

fn apply_tag_pack_inner(db: &mut RekordboxDb, pack: &TagPack) -> Result<Vec<TagGroup>, DbError> {
    let existing = load_tag_groups(db)?;
    let mut section_ids: Vec<String> = existing.iter().map(|g| g.id.clone()).collect();

    while section_ids.len() < pack.groups.len() {
        section_ids.push(create_section(db, &format!("Group {}", section_ids.len() + 1))?);
    }
    section_ids.truncate(pack.groups.len());

    for (idx, group) in pack.groups.iter().enumerate() {
        let section_id = &section_ids[idx];
        rename_tag(db, section_id, &group.name)?;
        sync_subtags(db, section_id, &group.tags)?;
    }

    load_tag_groups(db)
}

fn create_section(db: &RekordboxDb, name: &str) -> Result<String, DbError> {
    let id = new_mytag_id(&db.conn)?;
    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    let seq: i64 = db.conn.query_row(
        "SELECT COALESCE(MAX(Seq), 0) + 1 FROM djmdMyTag WHERE ParentID = '' OR ParentID IS NULL OR ParentID = 'root'",
        [],
        |row| row.get(0),
    ).unwrap_or(1);

    db.conn.execute(
        r#"
        INSERT INTO djmdMyTag (
            ID, UUID, rb_data_status, rb_local_data_status, rb_local_deleted, rb_local_synced,
            usn, rb_local_usn, created_at, updated_at, Seq, Name, Attribute, ParentID
        ) VALUES (?1, ?2, ?3, 0, 0, 0, ?4, ?5, ?6, ?6, ?7, ?8, 1, '')
        "#,
        params![
            id,
            uuid::Uuid::new_v4().to_string(),
            RB_DATA_STATUS_ACTIVE,
            cloud_usn,
            local_usn,
            ts,
            seq,
            name
        ],
    )?;
    Ok(id)
}

fn rename_tag(db: &RekordboxDb, id: &str, name: &str) -> Result<(), DbError> {
    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    db.conn.execute(
        "UPDATE djmdMyTag SET Name = ?1, usn = ?2, rb_local_usn = ?3, updated_at = ?4 WHERE ID = ?5",
        params![name, cloud_usn, local_usn, ts, id],
    )?;
    Ok(())
}

fn sync_subtags(db: &RekordboxDb, section_id: &str, desired: &[String]) -> Result<(), DbError> {
    let mut stmt = db.conn.prepare(
        "SELECT ID, Name FROM djmdMyTag WHERE ParentID = ?1 AND COALESCE(rb_local_deleted, 0) = 0",
    )?;
    let existing: Vec<(String, String)> = stmt
        .query_map(params![section_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (idx, tag_name) in desired.iter().enumerate() {
        let existing_id = existing
            .iter()
            .find(|(id, name)| name == tag_name && is_native_numeric_mytag_id(id))
            .map(|(id, _)| id.clone())
            .or_else(|| {
                existing
                    .iter()
                    .find(|(_, name)| name == tag_name)
                    .and_then(|(id, _)| {
                        if is_native_numeric_mytag_id(id) {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
            });

        if let Some(id) = existing_id {
            db.conn.execute(
                "UPDATE djmdMyTag SET Seq = ?1 WHERE ID = ?2",
                params![(idx + 1) as i64, id],
            )?;
        } else if find_native_mytag_id(&db.conn, section_id, tag_name)?.is_some() {
            let _ = create_subtag(db, section_id, tag_name, (idx + 1) as i64)?;
        }
    }
    Ok(())
}

fn create_subtag(db: &RekordboxDb, section_id: &str, name: &str, seq: i64) -> Result<String, DbError> {
    let id = new_mytag_id(&db.conn)?;
    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    db.conn.execute(
        r#"
        INSERT INTO djmdMyTag (
            ID, UUID, rb_data_status, rb_local_data_status, rb_local_deleted, rb_local_synced,
            usn, rb_local_usn, created_at, updated_at, Seq, Name, Attribute, ParentID
        ) VALUES (?1, ?2, ?3, 0, 0, 0, ?4, ?5, ?6, ?6, ?7, ?8, 0, ?9)
        "#,
        params![
            id,
            uuid::Uuid::new_v4().to_string(),
            RB_DATA_STATUS_ACTIVE,
            cloud_usn,
            local_usn,
            ts,
            seq,
            name,
            section_id
        ],
    )?;
    Ok(id)
}

pub fn add_custom_subtag(
    db: &RekordboxDb,
    group_id: &str,
    name: &str,
) -> Result<MyTagDef, DbError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(DbError::Other("Tag name cannot be empty".into()));
    }

    let seq: i64 = db.conn.query_row(
        "SELECT COALESCE(MAX(Seq), 0) + 1 FROM djmdMyTag WHERE ParentID = ?1 AND COALESCE(rb_local_deleted, 0) = 0",
        params![group_id],
        |row| row.get(0),
    ).unwrap_or(1);

    let id = create_subtag(db, group_id, trimmed, seq)?;
    Ok(MyTagDef {
        id,
        name: trimmed.to_string(),
        group_id: group_id.to_string(),
        seq,
    })
}

pub fn delete_custom_subtag(db: &RekordboxDb, tag_id: &str) -> Result<(), DbError> {
    let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
    let ts = now_timestamp();
    db.conn.execute(
        "UPDATE djmdMyTag SET rb_local_deleted = 1, usn = ?1, rb_local_usn = ?2, updated_at = ?3 WHERE ID = ?4",
        params![cloud_usn, local_usn, ts, tag_id],
    )?;
    Ok(())
}

pub fn set_track_tag(
    db: &RekordboxDb,
    content_id: &str,
    tag_id: &str,
    enabled: bool,
) -> Result<(), DbError> {
    let tag_id = prefer_native_mytag_id(db, tag_id)?;

    if enabled {
        dedupe_active_song_mytags(db, content_id, &tag_id)?;

        let exists: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM djmdSongMyTag WHERE ContentID = ?1 AND MyTagID = ?2 AND COALESCE(rb_local_deleted, 0) = 0",
            params![content_id, tag_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
            let ts = now_timestamp();
            let (id, row_uuid) = new_song_mytag_row();

            db.conn.execute(
                r#"
                INSERT INTO djmdSongMyTag (
                    ID, UUID, rb_data_status, rb_local_data_status, rb_local_deleted, rb_local_synced,
                    usn, rb_local_usn, created_at, updated_at, MyTagID, ContentID, TrackNo
                ) VALUES (?1, ?2, ?3, 0, 0, 0, ?4, ?5, ?6, ?6, ?7, ?8, NULL)
                "#,
                params![
                    id,
                    row_uuid,
                    RB_DATA_STATUS_ACTIVE,
                    cloud_usn,
                    local_usn,
                    ts,
                    tag_id,
                    content_id,
                ],
            )?;
            dedupe_active_song_mytags(db, content_id, &tag_id)?;
        }
    } else {
        let (cloud_usn, local_usn) = next_usn_pair(&db.conn)?;
        let ts = now_timestamp();
        db.conn.execute(
            "UPDATE djmdSongMyTag SET rb_local_deleted = 1, rb_data_status = ?1, usn = ?2, rb_local_usn = ?3, updated_at = ?4
             WHERE ContentID = ?5 AND MyTagID = ?6 AND COALESCE(rb_local_deleted, 0) = 0",
            params![RB_DATA_STATUS_DELETED, cloud_usn, local_usn, ts, content_id, tag_id],
        )?;
        purge_track_tombstones(db, content_id)?;
    }
    Ok(())
}

pub fn find_tag_id_in_group(
    groups: &[TagGroup],
    group_id: &str,
    name: &str,
) -> Option<String> {
    groups.iter().find(|g| g.id == group_id).and_then(|g| {
        g.tags
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
            .map(|t| t.id.clone())
    })
}

pub fn find_tag_id_in_db(
    db: &RekordboxDb,
    group_id: &str,
    name: &str,
) -> Result<Option<String>, DbError> {
    let trimmed = name.trim();
    if let Some(id) = db
        .conn
        .query_row(
            "SELECT ID FROM djmdMyTag
             WHERE ParentID = ?1 AND Name = ?2 COLLATE NOCASE
               AND COALESCE(rb_local_deleted, 0) = 0
               AND ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*'
             LIMIT 1",
            params![group_id, trimmed],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(Some(id));
    }

    // Prefer native Rekordbox tag names over default-pack aliases (e.g. Vocal vs Vocals).
    for alias in tag_name_aliases(trimmed) {
        if let Some(id) = db
            .conn
            .query_row(
                "SELECT ID FROM djmdMyTag
                 WHERE ParentID = ?1 AND Name = ?2 COLLATE NOCASE
                   AND COALESCE(rb_local_deleted, 0) = 0
                   AND ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*'
                 LIMIT 1",
                params![group_id, alias],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(Some(id));
        }
    }

    Ok(None)
}

fn tag_name_aliases(name: &str) -> Vec<&str> {
    match name.to_ascii_lowercase().as_str() {
        "vocals" => vec!["Vocal", "Vocals"],
        "vocal" => vec!["Vocals", "Vocal"],
        "no-vocals" => vec!["No-Vocals", "No Vocals", "Instrumental"],
        "peak-time" => vec!["Peak-Time", "Peak", "Peak Time"],
        "peak" => vec!["Peak", "Peak-Time"],
        "opening-set" => vec!["Opening-Set", "Opening Set", "Warm-Up"],
        "warm-up" => vec!["Warm-Up", "Warm Up", "Opening-Set"],
        "hip-hop" => vec!["Hip-Hop", "Hip Hop", "Hip House"],
        _ => vec![],
    }
}

pub fn resolve_tag_id(
    db: &RekordboxDb,
    groups: &[TagGroup],
    change: &PendingChange,
) -> Result<String, DbError> {
    if !change.tag_id.is_empty() {
        return prefer_native_mytag_id(db, &change.tag_id);
    }
    let group_id = change
        .group_id
        .as_ref()
        .ok_or_else(|| DbError::Other("Missing group for new tag".into()))?;
    let name = change
        .tag_name
        .as_ref()
        .ok_or_else(|| DbError::Other("Missing tag name for new tag".into()))?;

    if let Some(id) = find_tag_id_in_group(groups, group_id, name) {
        return Ok(id);
    }
    if let Some(id) = find_tag_id_in_db(db, group_id, name)? {
        return Ok(id);
    }
    add_custom_subtag(db, group_id, name).map(|def| def.id)
}

pub fn commit_changes(
    db: &mut RekordboxDb,
    pending: &[PendingChange],
    groups: &[TagGroup],
) -> Result<(usize, usize, usize), DbError> {
    db.conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = {
        sync_agent_registry(&db.conn)?;
        let _ = repair_duplicate_mytag_defs(db);
        let summary = commit_changes_inner(db, pending, groups)?;
        mark_my_tag_merge_needed(&db.conn)?;
        Ok(summary)
    };
    if result.is_ok() {
        sync_agent_registry(&db.conn)?;
        db.conn.execute_batch("COMMIT")?;
    } else {
        let _ = db.conn.execute_batch("ROLLBACK");
    }
    result
}

fn commit_changes_inner(
    db: &mut RekordboxDb,
    pending: &[PendingChange],
    groups: &[TagGroup],
) -> Result<(usize, usize, usize), DbError> {
    use std::collections::{HashMap, HashSet};

    let mut tracks = HashSet::new();
    let mut added = 0usize;
    let mut removed = 0usize;

    let mut by_track: HashMap<String, Vec<&PendingChange>> = HashMap::new();
    for change in pending {
        by_track
            .entry(change.track_id.clone())
            .or_default()
            .push(change);
    }

    for (track_id, changes) in by_track {
        tracks.insert(track_id.clone());
        for change in changes {
            let tag_id = resolve_tag_id(db, groups, change)?;
            if change.enabled {
                let before: i64 = db.conn.query_row(
                    "SELECT COUNT(*) FROM djmdSongMyTag WHERE ContentID = ?1 AND MyTagID = ?2 AND COALESCE(rb_local_deleted, 0) = 0",
                    params![track_id, tag_id],
                    |row| row.get(0),
                )?;
                set_track_tag(db, &track_id, &tag_id, true)?;
                if before == 0 {
                    added += 1;
                }
            } else {
                let before: i64 = db.conn.query_row(
                    "SELECT COUNT(*) FROM djmdSongMyTag WHERE ContentID = ?1 AND MyTagID = ?2 AND COALESCE(rb_local_deleted, 0) = 0",
                    params![track_id, tag_id],
                    |row| row.get(0),
                )?;
                set_track_tag(db, &track_id, &tag_id, false)?;
                if before > 0 {
                    removed += 1;
                }
            }
        }
        touch_content(db, &track_id)?;
        purge_track_tombstones(db, &track_id)?;
        dedupe_all_active_track_tags(db, &track_id)?;
    }

    Ok((tracks.len(), added, removed))
}
