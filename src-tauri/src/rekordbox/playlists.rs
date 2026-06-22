use std::collections::{HashMap, HashSet};

use crate::types::{Playlist, Track};

use super::db::{DbError, RekordboxDb};

/// Rekordbox `djmdPlaylist.Attribute` values (pyrekordbox / reklawdbox).
const ATTR_PLAYLIST: i64 = 0;
const ATTR_FOLDER: i64 = 1;
const ATTR_SMART_PLAYLIST: i64 = 4;

struct RawPlaylist {
    id: String,
    name: String,
    parent_id: String,
    attribute: i64,
    seq: i64,
}

pub fn load_playlists(db: &RekordboxDb) -> Result<(Vec<Playlist>, HashMap<String, HashSet<String>>), DbError> {
    let mut stmt = db.conn.prepare(
        r#"
        SELECT ID, COALESCE(Name, ''), COALESCE(ParentID, ''), COALESCE(Attribute, 0), COALESCE(Seq, 0)
        FROM djmdPlaylist
        WHERE COALESCE(rb_local_deleted, 0) = 0
        ORDER BY COALESCE(Seq, 0), Name COLLATE NOCASE
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RawPlaylist {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                attribute: row.get(3)?,
                seq: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut membership_stmt = db.conn.prepare(
        r#"
        SELECT PlaylistID, ContentID
        FROM djmdSongPlaylist
        WHERE COALESCE(rb_local_deleted, 0) = 0
        "#,
    )?;
    let membership_rows = membership_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut membership: HashMap<String, HashSet<String>> = HashMap::new();
    for (playlist_id, content_id) in membership_rows {
        membership
            .entry(playlist_id)
            .or_default()
            .insert(content_id);
    }

    let by_id: HashMap<String, &RawPlaylist> = rows.iter().map(|p| (p.id.clone(), p)).collect();
    let mut playlists: Vec<Playlist> = rows
        .iter()
        .filter(|p| is_user_playlist(p.attribute))
        .map(|p| {
            let track_count = membership.get(&p.id).map(|s| s.len()).unwrap_or(0);
            Playlist {
                id: p.id.clone(),
                name: p.name.clone(),
                path: build_playlist_path(p, &by_id),
                attribute: p.attribute,
                track_count,
            }
        })
        .collect();

    playlists.sort_by(|a, b| {
        a.path
            .to_lowercase()
            .cmp(&b.path.to_lowercase())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok((playlists, membership))
}

fn build_playlist_path(playlist: &RawPlaylist, by_id: &HashMap<String, &RawPlaylist>) -> String {
    let mut parts = vec![playlist.name.clone()];
    let mut parent_id = playlist.parent_id.clone();
    let mut guard = 0;

    while !parent_id.is_empty() && guard < 32 {
        guard += 1;
        if let Some(parent) = by_id.get(&parent_id) {
            if parent.attribute == ATTR_FOLDER {
                parts.push(parent.name.clone());
            }
            parent_id = parent.parent_id.clone();
        } else {
            break;
        }
    }

    parts.reverse();
    parts.join(" / ")
}

fn is_user_playlist(attribute: i64) -> bool {
    attribute == ATTR_PLAYLIST
}

pub fn filter_by_playlist(
    tracks: &[Track],
    membership: &HashMap<String, HashSet<String>>,
    playlist_id: &str,
) -> Vec<Track> {
    let Some(ids) = membership.get(playlist_id) else {
        return Vec::new();
    };
    tracks
        .iter()
        .filter(|t| ids.contains(&t.id))
        .cloned()
        .collect()
}

pub fn sort_tracks(tracks: &mut [Track], sort_by: &str, sort_dir: &str) {
    let asc = sort_dir != "desc";
    tracks.sort_by(|a, b| {
        let ord = match sort_by {
            "artist" => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
            "genre" => a.genre.to_lowercase().cmp(&b.genre.to_lowercase()),
            "bpm" => a
                .bpm
                .partial_cmp(&b.bpm)
                .unwrap_or(std::cmp::Ordering::Equal),
            "tags" => a.tag_ids.len().cmp(&b.tag_ids.len()),
            "rating" => a.rating.cmp(&b.rating),
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        };
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_playlist_includes_normal_only() {
        assert!(is_user_playlist(ATTR_PLAYLIST));
        assert!(!is_user_playlist(ATTR_FOLDER));
        assert!(!is_user_playlist(ATTR_SMART_PLAYLIST));
    }
}
