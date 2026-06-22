use rusqlite::{Connection, OpenFlags};
const KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

#[test]
fn sections() {
    let path = format!("{}/Library/Pioneer/rekordbox/master.db", std::env::var("HOME").unwrap());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_update(None, "key", KEY).unwrap();
    conn.pragma_update(None, "cipher_compatibility", 4i32).unwrap();

    // Compare uuid vs numeric songmytag rows for section 2
    eprintln!("=== Section 2 SongMyTag: numeric mytag ===");
    let mut s = conn.prepare(
        "SELECT c.Title, t.Name, sm.* FROM djmdSongMyTag sm
         JOIN djmdMyTag t ON sm.MyTagID=t.ID JOIN djmdContent c ON c.ID=sm.ContentID
         WHERE COALESCE(sm.rb_local_deleted,0)=0 AND t.ParentID='2'
         AND t.ID GLOB '[0-9]*' AND t.ID NOT GLOB '*[^0-9]*' LIMIT 1"
    ).unwrap();
    print_row(&s);

    eprintln!("\n=== Section 2 SongMyTag: uuid mytag ===");
    let mut s = conn.prepare(
        "SELECT c.Title, t.Name, sm.* FROM djmdSongMyTag sm
         JOIN djmdMyTag t ON sm.MyTagID=t.ID JOIN djmdContent c ON c.ID=sm.ContentID
         WHERE COALESCE(sm.rb_local_deleted,0)=0 AND t.ParentID='2'
         AND NOT (t.ID GLOB '[0-9]*' AND t.ID NOT GLOB '*[^0-9]*') LIMIT 1"
    ).unwrap();
    print_row(&s);

    eprintln!("\n=== Section 1 numeric (working genre) ===");
    let mut s = conn.prepare(
        "SELECT c.Title, t.Name, sm.* FROM djmdSongMyTag sm
         JOIN djmdMyTag t ON sm.MyTagID=t.ID JOIN djmdContent c ON c.ID=sm.ContentID
         WHERE COALESCE(sm.rb_local_deleted,0)=0 AND t.ParentID='1'
         AND t.ID GLOB '[0-9]*' AND t.ID NOT GLOB '*[^0-9]*' LIMIT 1"
    ).unwrap();
    print_row(&s);
}

fn print_row(s: &rusqlite::Statement) {
    if let Ok((title, tag, row)) = s.query_row([], |r| {
        let title: String = r.get(0)?;
        let tag: String = r.get(1)?;
        let mut fields = vec![];
        for i in 2..r.as_ref().column_count() {
            fields.push(format!("{}={:?}", r.as_ref().column_name(i).unwrap(), r.get::<_, rusqlite::types::Value>(i).ok()));
        }
        Ok((title, tag, fields))
    }) {
        eprintln!("{title} > {tag}");
        for f in row { eprintln!("  {f}"); }
    }
}
