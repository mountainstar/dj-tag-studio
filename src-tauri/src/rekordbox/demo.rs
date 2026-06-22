use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};

use crate::types::{Playlist, TagGroup, Track};

pub fn open_demo_db() -> Connection {
    let conn = Connection::open_in_memory().expect("demo db");
    conn.execute_batch(
        r#"
        CREATE TABLE djmdMyTag (
            ID TEXT PRIMARY KEY, UUID TEXT, rb_data_status INTEGER DEFAULT 0,
            rb_local_data_status INTEGER DEFAULT 0, rb_local_deleted INTEGER DEFAULT 0,
            rb_local_synced INTEGER DEFAULT 0, usn INTEGER DEFAULT 1, rb_local_usn INTEGER DEFAULT 1,
            created_at TEXT, updated_at TEXT, Seq INTEGER, Name TEXT, Attribute INTEGER, ParentID TEXT
        );
        CREATE TABLE djmdSongMyTag (
            ID TEXT PRIMARY KEY, UUID TEXT, rb_data_status INTEGER DEFAULT 0,
            rb_local_data_status INTEGER DEFAULT 0, rb_local_deleted INTEGER DEFAULT 0,
            rb_local_synced INTEGER DEFAULT 0, usn INTEGER DEFAULT 1, rb_local_usn INTEGER DEFAULT 1,
            created_at TEXT, updated_at TEXT, MyTagID TEXT, ContentID TEXT, TrackNo INTEGER
        );
        CREATE TABLE djmdContent (
            ID TEXT PRIMARY KEY, Title TEXT, ArtistID TEXT, AlbumID TEXT, GenreID TEXT,
            BPM INTEGER, FolderPath TEXT, Rating INTEGER, Commnt TEXT, rb_local_deleted INTEGER DEFAULT 0
        );
        CREATE TABLE djmdArtist (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdAlbum (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdGenre (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdPlaylist (
            ID TEXT PRIMARY KEY, UUID TEXT, rb_data_status INTEGER DEFAULT 0,
            rb_local_data_status INTEGER DEFAULT 0, rb_local_deleted INTEGER DEFAULT 0,
            rb_local_synced INTEGER DEFAULT 0, usn INTEGER DEFAULT 1, rb_local_usn INTEGER DEFAULT 1,
            created_at TEXT, updated_at TEXT, Seq INTEGER, Name TEXT, Attribute INTEGER, ParentID TEXT,
            SmartList TEXT
        );
        CREATE TABLE djmdSongPlaylist (
            ID TEXT PRIMARY KEY, UUID TEXT, rb_data_status INTEGER DEFAULT 0,
            rb_local_data_status INTEGER DEFAULT 0, rb_local_deleted INTEGER DEFAULT 0,
            rb_local_synced INTEGER DEFAULT 0, usn INTEGER DEFAULT 1, rb_local_usn INTEGER DEFAULT 1,
            created_at TEXT, updated_at TEXT, PlaylistID TEXT, ContentID TEXT, TrackNo INTEGER
        );
        "#,
    )
    .expect("demo schema");

    let pack: crate::types::TagPack = serde_json::from_str(include_str!(
        "../../../tag-packs/rekordbox-default.json"
    ))
    .expect("default tag pack");

    let ts = "2026-01-01 00:00:00.000 +00:00";
    for (gidx, group) in pack.groups.iter().enumerate() {
        let section_id = format!("section-{gidx}");
        conn.execute(
            "INSERT INTO djmdMyTag (ID, UUID, Seq, Name, Attribute, ParentID, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, '', ?5, ?5)",
            params![section_id, format!("uuid-{section_id}"), (gidx + 1) as i64, group.name, ts],
        ).unwrap();
        for (tidx, tag) in group.tags.iter().enumerate() {
            let tag_id = format!("{section_id}-tag-{tidx}");
            conn.execute(
                "INSERT INTO djmdMyTag (ID, UUID, Seq, Name, Attribute, ParentID, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)",
                params![tag_id, format!("uuid-{tag_id}"), (tidx + 1) as i64, tag, section_id, ts],
            ).unwrap();
        }
    }

    let demos = [
        ("1", "Espresso (Remix)", "Sabrina Carpenter", "Pop", 12400, "/Music/espresso.mp3", 204, "Pop Disco Vocals"),
        ("2", "Talk To You", "ANOTR", "House", 12600, "/Music/talk.mp3", 102, "House Driving Vocals"),
        ("3", "Free", "Ultra Nate", "House", 12800, "/Music/free.mp3", 255, "House Anthemic Peak"),
        ("4", "Strobe", "deadmau5", "Techno", 12800, "/Music/strobe.mp3", 255, "Techno Progressive No-Vocals"),
        ("5", "One More Time", "Daft Punk", "Disco", 12300, "/Music/omt.mp3", 204, "Disco French Vocals"),
    ];

    for (id, title, artist, genre, bpm, path, rating, comment) in demos {
        conn.execute(
            "INSERT INTO djmdArtist (ID, Name) VALUES (?1, ?2)",
            params![format!("a-{id}"), artist],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO djmdGenre (ID, Name) VALUES (?1, ?2)",
            params![format!("g-{id}"), genre],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO djmdAlbum (ID, Name) VALUES (?1, ?2)",
            params![format!("al-{id}"), "Demo Album"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO djmdContent (ID, Title, ArtistID, AlbumID, GenreID, BPM, FolderPath, Rating, Commnt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, title, format!("a-{id}"), format!("al-{id}"), format!("g-{id}"), bpm, path, rating, comment],
        )
        .unwrap();
    }

    conn.execute(
        "INSERT INTO djmdSongMyTag (ID, UUID, MyTagID, ContentID, TrackNo, created_at, updated_at) VALUES ('sm1', 'u1', 'section-0-tag-3', '1', 1, ?1, ?1)",
        params![ts],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO djmdPlaylist (ID, UUID, Seq, Name, Attribute, ParentID, created_at, updated_at) VALUES ('pl-folder', 'u-pl-folder', 1, 'Demo Sets', 1, '', ?1, ?1)",
        params![ts],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO djmdPlaylist (ID, UUID, Seq, Name, Attribute, ParentID, created_at, updated_at) VALUES ('pl-house', 'u-pl-house', 1, 'House Warmup', 0, 'pl-folder', ?1, ?1)",
        params![ts],
    )
    .unwrap();
    for (idx, track_id) in ["2", "3", "5"].iter().enumerate() {
        conn.execute(
            "INSERT INTO djmdSongPlaylist (ID, UUID, PlaylistID, ContentID, TrackNo, created_at, updated_at) VALUES (?1, ?2, 'pl-house', ?3, ?4, ?5, ?5)",
            params![
                format!("sp-{track_id}"),
                format!("u-sp-{track_id}"),
                track_id,
                (idx + 1) as i64,
                ts
            ],
        )
        .unwrap();
    }

    conn
}

pub fn demo_library() -> (
    Vec<TagGroup>,
    Vec<Track>,
    Vec<Playlist>,
    HashMap<String, HashSet<String>>,
) {
    let conn = open_demo_db();
    let db = super::db::RekordboxDb {
        conn,
        path: "demo".into(),
        mode: super::db::DatabaseMode::ReadOnly,
    };
    let groups = super::my_tags::load_tag_groups(&db).unwrap();
    let tracks = super::content::load_tracks(&db).unwrap();
    let (playlists, playlist_tracks) = super::playlists::load_playlists(&db).unwrap();
    (groups, tracks, playlists, playlist_tracks)
}
