use rusqlite::{Connection, OpenFlags};
const KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

#[test]
fn cloud_check() {
    let path = format!("{}/Library/Pioneer/rekordbox/master.db", std::env::var("HOME").unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_update(None, "key", KEY).unwrap();
    conn.pragma_update(None, "cipher_compatibility", 4i32).unwrap();

    let last: i64 = conn.query_row("SELECT int_1 FROM agentRegistry WHERE registry_id='lastUpdateCount'", [], |r| r.get(0)).unwrap_or(0);
    let local: i64 = conn.query_row("SELECT int_1 FROM agentRegistry WHERE registry_id='localUpdateCount'", [], |r| r.get(0)).unwrap_or(0);
    eprintln!("lastUpdateCount={last} localUpdateCount={local}");

    for table in ["djmdContent", "djmdSongMyTag", "djmdMyTag"] {
        let max_usn: i64 = conn.query_row(&format!("SELECT COALESCE(MAX(usn),0) FROM {table}"), [], |r| r.get(0)).unwrap();
        let max_local: i64 = conn.query_row(&format!("SELECT COALESCE(MAX(rb_local_usn),0) FROM {table}"), [], |r| r.get(0)).unwrap();
        eprintln!("{table}: max usn={max_usn} max rb_local_usn={max_local}");
    }

    // SongMyTag where usn == rb_local_usn
    let same: i64 = conn.query_row("SELECT COUNT(*) FROM djmdSongMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND usn=rb_local_usn", [], |r| r.get(0)).unwrap();
    let diff: i64 = conn.query_row("SELECT COUNT(*) FROM djmdSongMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND usn!=rb_local_usn", [], |r| r.get(0)).unwrap();
    eprintln!("SongMyTag usn==local: {same} usn!=local: {diff}");
}
