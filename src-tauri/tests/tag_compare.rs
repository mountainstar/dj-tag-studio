use rusqlite::{Connection, OpenFlags, OptionalExtension};

const DB_KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

fn open_db() -> Connection {
    let path = format!("{}/Library/Pioneer/rekordbox/master.db", std::env::var("HOME").unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_update(None, "key", DB_KEY).unwrap();
    conn.pragma_update(None, "cipher_compatibility", 4i32).unwrap();
    conn
}

fn dump_song_tags(conn: &Connection, content_id: &str, label: &str) {
    eprintln!("\n=== {label} ({content_id}) ===");
    let content: (String, Option<i64>, Option<i64>, Option<i64>, String) = conn.query_row(
        "SELECT COALESCE(Title,''), rb_local_usn, rb_data_status, rb_local_data_status, COALESCE(updated_at,'') FROM djmdContent WHERE ID=?1",
        rusqlite::params![content_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    ).unwrap_or(("?".into(), None, None, None, "?".into()));
    eprintln!("Content: title={} usn={:?} rds={:?} lds={:?} updated={}", content.0, content.1, content.2, content.3, content.4);

    let mut s = conn.prepare(
        "SELECT sm.ID, sm.UUID, sm.MyTagID, t.Name, t.ParentID, t.Attribute, sm.TrackNo, sm.rb_data_status, sm.rb_local_data_status, sm.usn, sm.rb_local_usn, sm.created_at
         FROM djmdSongMyTag sm JOIN djmdMyTag t ON sm.MyTagID=t.ID
         WHERE sm.ContentID=?1 AND COALESCE(sm.rb_local_deleted,0)=0 ORDER BY sm.TrackNo"
    ).unwrap();
    for row in s.query_map(rusqlite::params![content_id], |r| {
        Ok(format!(
            "  tag={} mytag_id={} parent={} attr={} trackno={} id={} uuid={} rds={} usn={}/{}",
            r.get::<_,String>(3)?, r.get::<_,String>(2)?, r.get::<_,String>(4)?, r.get::<_,i64>(5)?,
            r.get::<_,i64>(6)?, r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(7)?,
            r.get::<_,i64>(9)?, r.get::<_,i64>(10)?
        ))
    }).unwrap().flatten() { eprintln!("{row}"); }
}

#[test]
fn compare_tagged_tracks() {
    let conn = open_db();
    let local: i64 = conn.query_row("SELECT int_1 FROM agentRegistry WHERE registry_id='localUpdateCount'", [], |r| r.get(0)).unwrap_or(0);
    let merge: i64 = conn.query_row("SELECT int_1 FROM agentRegistry WHERE registry_id='needsToMergeMyTag'", [], |r| r.get(0)).unwrap_or(0);
    eprintln!("localUpdateCount={local} needsToMergeMyTag={merge}");

    dump_song_tags(&conn, "163589926", "Au Revoir (app-tagged)");
    dump_song_tags(&conn, "190784198", "Less Is More (native?)");
    dump_song_tags(&conn, "69744661", "Only (native?)");

    // find track tagged only before June 21 app session - oldest song mytag update
    let oldest: Option<(String,String)> = conn.query_row(
        "SELECT sm.ContentID, c.Title FROM djmdSongMyTag sm JOIN djmdContent c ON c.ID=sm.ContentID
         WHERE COALESCE(sm.rb_local_deleted,0)=0 AND sm.created_at < '2026-06-21'
         ORDER BY sm.created_at ASC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?))
    ).optional().ok().flatten();
    if let Some((id, title)) = oldest {
        dump_song_tags(&conn, &id, &format!("Oldest pre-app tag: {title}"));
    }

    // MyTag sections structure
    eprintln!("\n=== MyTag sections ===");
    let mut s = conn.prepare("SELECT ID, Name, Attribute, ParentID, Seq FROM djmdMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND (ParentID='' OR ParentID='root' OR Attribute=1) ORDER BY Seq").unwrap();
    for row in s.query_map([], |r| Ok(format!("section id={} name={} attr={} parent={} seq={}", r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?, r.get::<_,String>(3)?, r.get::<_,i64>(4)?))).unwrap().flatten() {
        eprintln!("{row}");
    }
}

#[test]
fn deep_investigate() {
    use rusqlite::OptionalExtension;
    let conn = open_db();
    
    for id in ["190784198", "69744661", "78320487", "163589926"] {
        let all: i64 = conn.query_row("SELECT COUNT(*) FROM djmdSongMyTag WHERE ContentID=?1", rusqlite::params![id], |r| r.get(0)).unwrap();
        let active: i64 = conn.query_row("SELECT COUNT(*) FROM djmdSongMyTag WHERE ContentID=?1 AND COALESCE(rb_local_deleted,0)=0", rusqlite::params![id], |r| r.get(0)).unwrap();
        eprintln!("{id}: all={all} active={active}");
    }

    // Check UuidIDMap for song mytag rows on Au Revoir
    let has_uuid_map: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='UuidIDMap'", [], |r| r.get(0)).unwrap();
    eprintln!("UuidIDMap exists: {has_uuid_map}");
    if has_uuid_map > 0 {
        let cols: Vec<String> = conn.prepare("PRAGMA table_info(UuidIDMap)").unwrap().query_map([], |r| r.get(1)).unwrap().filter_map(|x| x.ok()).collect();
        eprintln!("UuidIDMap cols: {cols:?}");
        let mut s = conn.prepare(
            "SELECT u.* FROM UuidIDMap u JOIN djmdSongMyTag sm ON sm.UUID=u.UUID OR sm.ID=u.ID WHERE sm.ContentID='163589926' LIMIT 5"
        );
        if s.is_ok() {
            // try simpler
        }
        let sample: Vec<String> = conn.prepare("SELECT ID, UUID, table_name FROM UuidIDMap LIMIT 3").unwrap_or_else(|_| conn.prepare("SELECT ID, UUID FROM UuidIDMap LIMIT 3").unwrap())
            .query_map([], |r| Ok(format!("{:?}|{:?}", r.get::<_,String>(0).ok(), r.get::<_,String>(1).ok()))).unwrap().filter_map(|x| x.ok()).collect();
        eprintln!("UuidIDMap sample: {sample:?}");
    }

    // Menu items for My Tag column
    let menu_cols: Vec<String> = conn.prepare("PRAGMA table_info(djmdMenuItems)").unwrap().query_map([], |r| r.get(1)).unwrap().filter_map(|x| x.ok()).collect();
    eprintln!("MenuItems cols: {menu_cols:?}");
    let mut s = conn.prepare("SELECT * FROM djmdMenuItems WHERE Name LIKE '%Tag%' OR Name LIKE '%MY%' LIMIT 10").unwrap();
    let n = s.column_count();
    for row in s.query_map([], |r| {
        let mut p = vec![];
        for i in 0..n {
            p.push(format!(
                "{}={:?}",
                s.column_name(i).unwrap_or("?"),
                r.get::<_, rusqlite::types::Value>(i).ok()
            ));
        }
        Ok(p.join(" | "))
    }).unwrap().flatten() {
        eprintln!("Menu: {row}");
    }

    // content Tag field sample
    let tag_nonempty: i64 = conn.query_row("SELECT COUNT(*) FROM djmdContent WHERE COALESCE(Tag,'') != ''", [], |r| r.get(0)).unwrap();
    eprintln!("Content rows with Tag field set: {tag_nonempty}");
}
