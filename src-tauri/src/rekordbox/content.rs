use std::collections::HashMap;

use crate::types::Track;

use super::db::{DbError, RekordboxDb};
use super::paths::{rekordbox_share_root, resolve_playback_path};

pub fn load_tracks(db: &RekordboxDb) -> Result<Vec<Track>, DbError> {
    let share_root = rekordbox_share_root();

    let mut stmt = db.conn.prepare(
        r#"
        SELECT
            c.ID,
            COALESCE(c.Title, ''),
            COALESCE(a.Name, ''),
            COALESCE(al.Name, ''),
            COALESCE(g.Name, ''),
            COALESCE(c.BPM, 0),
            COALESCE(c.FolderPath, ''),
            COALESCE(c.OrgFolderPath, ''),
            COALESCE(c.rb_LocalFolderPath, ''),
            COALESCE(c.Rating, 0),
            COALESCE(c.Commnt, '')
        FROM djmdContent c
        LEFT JOIN djmdArtist a ON c.ArtistID = a.ID
        LEFT JOIN djmdAlbum al ON c.AlbumID = al.ID
        LEFT JOIN djmdGenre g ON c.GenreID = g.ID
        WHERE COALESCE(c.rb_local_deleted, 0) = 0
        ORDER BY c.Title COLLATE NOCASE
        "#,
    )?;

    let track_rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut tag_stmt = db.conn.prepare(
        "SELECT ContentID, MyTagID FROM djmdSongMyTag WHERE COALESCE(rb_local_deleted, 0) = 0",
    )?;
    let tag_rows = tag_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut tags_by_track: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (content_id, tag_id) in tag_rows {
        tags_by_track
            .entry(content_id)
            .or_default()
            .push(tag_id);
    }

    Ok(track_rows
        .into_iter()
        .map(
            |(id, title, artist, album, genre, bpm_raw, folder_path, org_path, local_path, rating, comment)| {
                let playback = resolve_playback_path(
                    &folder_path,
                    &org_path,
                    &local_path,
                    share_root.as_deref(),
                );
                Track {
                    id: id.clone(),
                    title,
                    artist,
                    album,
                    genre,
                    bpm: bpm_raw as f64 / 100.0,
                    path: folder_path,
                    playback_path: playback.path,
                    playback_available: playback.available,
                    playback_note: playback.note,
                    rating,
                    comment,
                    tag_ids: tags_by_track.remove(&id).unwrap_or_default(),
                }
            },
        )
        .collect())
}

pub fn load_tag_ids_for_tracks(
    db: &RekordboxDb,
    track_ids: &[String],
) -> Result<HashMap<String, Vec<String>>, DbError> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    if track_ids.is_empty() {
        return Ok(out);
    }

    for chunk in track_ids.chunks(200) {
        let placeholders = chunk
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT ContentID, MyTagID FROM djmdSongMyTag WHERE COALESCE(rb_local_deleted, 0) = 0 AND ContentID IN ({placeholders})"
        );
        let mut stmt = db.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = chunk
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (content_id, tag_id) in rows {
            out.entry(content_id).or_default().push(tag_id);
        }
    }

    Ok(out)
}

pub fn search_tracks(tracks: &[Track], query: &str) -> Vec<Track> {
    if query.trim().is_empty() {
        return tracks.to_vec();
    }
    let q = query.to_lowercase();
    tracks
        .iter()
        .filter(|t| {
            t.title.to_lowercase().contains(&q)
                || t.artist.to_lowercase().contains(&q)
                || t.album.to_lowercase().contains(&q)
                || t.genre.to_lowercase().contains(&q)
                || t.path.to_lowercase().contains(&q)
                || t.playback_path.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

pub fn filter_untagged(tracks: &[Track]) -> Vec<Track> {
    tracks
        .iter()
        .filter(|t| t.tag_ids.is_empty())
        .cloned()
        .collect()
}

pub fn filter_missing_group(tracks: &[Track], group_tag_ids: &[String]) -> Vec<Track> {
    tracks
        .iter()
        .filter(|t| !t.tag_ids.iter().any(|id| group_tag_ids.contains(id)))
        .cloned()
        .collect()
}
