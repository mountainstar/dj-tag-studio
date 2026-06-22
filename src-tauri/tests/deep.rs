use rusqlite::{Connection, OpenFlags};
const KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";
#[test]
fn deep() {
    let path = format!("{}/Library/Pioneer/rekordbox/master.db", std::env::var("HOME").unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_update(None, "key", KEY).unwrap();
    conn.pragma_update(None, "cipher_compatibility", 4i32).unwrap();
    let numeric: i64 = conn.query_row("SELECT COUNT(*) FROM djmdMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*'", [], |r| r.get(0)).unwrap();
    let uuidish: i64 = conn.query_row("SELECT COUNT(*) FROM djmdMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND NOT (ID GLOB '[0-9]*' AND ID NOT GLOB '*[^0-9]*')", [], |r| r.get(0)).unwrap();
    eprintln!("MyTag ID numeric={numeric} uuidish={uuidish}");
    let status0: i64 = conn.query_row("SELECT COUNT(*) FROM djmdMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND rb_data_status=0", [], |r| r.get(0)).unwrap();
    let status256: i64 = conn.query_row("SELECT COUNT(*) FROM djmdMyTag WHERE COALESCE(rb_local_deleted,0)=0 AND rb_data_status=256", [], |r| r.get(0)).unwrap();
    eprintln!("MyTag rb_data_status 0={status0} 256={status256}");
}
