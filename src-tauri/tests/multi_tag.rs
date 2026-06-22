use rusqlite::{Connection, OpenFlags};
const KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

#[test]
fn multi_tag() {
    let path = format!("{}/Library/Pioneer/rekordbox/master.db", std::env::var("HOME").unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_update(None, "key", KEY).unwrap();
    conn.pragma_update(None, "cipher_compatibility", 4i32).unwrap();

    eprintln!("=== TrackNo distribution (active SongMyTag) ===");
    let mut s = conn.prepare(
        "SELECT CASE WHEN TrackNo IS NULL THEN 'NULL' ELSE CAST(TrackNo AS TEXT) END, COUNT(*)
         FROM djmdSongMyTag WHERE COALESCE(rb_local_deleted,0)=0 GROUP BY 1 ORDER BY 2 DESC LIMIT 10"
    ).unwrap();
    for row in s.query_map([], |r| Ok(format!("TrackNo={} count={}", r.get::<_,String>(0)?, r.get::<_,i64>(1)?))).unwrap().flatten() {
        eprintln!("{row}");
    }

    eprintln!("\n=== Multi-tag tracks with any non-null TrackNo ===");
    let c: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT ContentID) FROM djmdSongMyTag
         WHERE COALESCE(rb_local_deleted,0)=0 AND TrackNo IS NOT NULL", [], |r| r.get(0)).unwrap();
    eprintln!("tracks with non-null TrackNo: {c}");

    eprintln!("\n=== Less Is More full rows ===");
    let mut s = conn.prepare("SELECT t.Name, t.ParentID, sm.TrackNo, sm.ID FROM djmdSongMyTag sm JOIN djmdMyTag t ON sm.MyTagID=t.ID WHERE sm.ContentID='190784198' AND COALESCE(sm.rb_local_deleted,0)=0").unwrap();
    for row in s.query_map([], |r| Ok(format!("tag={} parent={} trackno={:?}", r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,Option<i64>>(2).ok().flatten()))).unwrap().flatten() {
        eprintln!("{row}");
    }

    // ParentID section seq mapping
    eprintln!("\n=== Section roots ===");
    let mut s = conn.prepare("SELECT ID, Name, Seq, Attribute, ParentID FROM djmdMyTag WHERE ParentID IN ('','root') OR Attribute=1 ORDER BY Seq").unwrap();
    for row in s.query_map([], |r| Ok(format!("id={} name={} seq={} attr={} parent={:?}", r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?, r.get::<_,i64>(3)?, r.get::<_,String>(4).ok()))).unwrap().flatten() {
        eprintln!("{row}");
    }
}
