//! Bookmarks of a CLOSED profile, written straight into Firefox's `places.sqlite` (CM-3.4).
//!
//! [`sync`] makes the toolbar folder "Camoufox Manager" (fixed guid [`FOLDER_GUID`]) hold exactly
//! the profile's bookmark list: missing entries are created, entries no longer listed are removed,
//! titles and order are updated. Only rows inside that folder are ever touched, so bookmarks the
//! user made by hand in the browser stay. `launch` calls it before the browser opens.
//!
//! First launch: a profile that never ran has no `places.sqlite` (Firefox creates and migrates the
//! whole schema itself). Re-creating that schema here would be fragile, so the sync is simply
//! skipped ([`Outcome::NoDb`]) and happens on the next launch. A freshly created profile therefore
//! shows its bookmarks from the second start on.
//!
//! Schema notes: `moz_places` needs `url_hash` (see [`url_hash`]; a wrong value only makes Firefox
//! add its own row for the same URL when you visit it) and `moz_origins` is filled when the table
//! exists. Frecency is left at -1 with `recalc_frecency` set, so Firefox computes it.
//! `foreign_count` is kept right by Firefox's own triggers when they exist, otherwise adjusted here.
//! Columns of `moz_places` are discovered with `PRAGMA table_info`, so added columns do not break
//! the insert.

use anyhow::Result;
use cm_core::Bookmark;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Guid of the managed toolbar folder (12 chars, the length Firefox uses).
pub const FOLDER_GUID: &str = "cmBookmarks_";
pub const FOLDER_TITLE: &str = "Camoufox Manager";
const TOOLBAR_GUID: &str = "toolbar_____";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// `places.sqlite` does not exist yet (first launch): nothing done.
    NoDb,
    /// The database has no toolbar root (unexpected schema): nothing done.
    NoToolbar,
    Synced,
}

fn hash_string(s: &[u8]) -> u32 {
    s.iter().fold(0u32, |h, &c| (h.rotate_left(5) ^ u32::from(c)).wrapping_mul(0x9E37_79B9))
}

/// Firefox's `url_hash`: 16 bits of the scheme hash above the 32-bit hash of the whole URL.
pub fn url_hash(url: &str) -> i64 {
    let prefix = url.find("://").map(|i| &url[..i]).unwrap_or("");
    ((i64::from(hash_string(prefix.as_bytes())) & 0xFFFF) << 32) + i64::from(hash_string(url.as_bytes()))
}

/// `a.b` -> `b.a.` (Firefox's `rev_host`).
fn rev_host(host: &str) -> String {
    let mut r: String = host.chars().rev().collect();
    r.push('.');
    r
}

fn split_url(url: &str) -> (String, String) {
    let (scheme, rest) = url.split_once("://").unwrap_or(("", url));
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    (format!("{scheme}://"), host.to_string())
}

/// `https://example.com` -> `https://example.com/` (how Firefox stores a bare origin).
fn normalize(url: &str) -> String {
    match url.split_once("://") {
        Some((_, rest)) if !rest.contains(['/', '?', '#']) => format!("{url}/"),
        _ => url.to_string(),
    }
}

fn now_us() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros() as i64).unwrap_or(0)
}

fn new_guid(seed: &mut u64) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut s = String::from("cm");
    for _ in 0..10 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s.push(A[((*seed >> 33) % 64) as usize] as char);
    }
    s
}

fn folder_guid(name: &str) -> String {
    format!("cmf{:08x}_", hash_string(name.as_bytes()))
}

/// Sync the managed folder of `<user_data>/places.sqlite`. Call only while the browser is closed.
pub fn sync(user_data: &Path, bookmarks: &[Bookmark]) -> Result<Outcome> {
    let path = user_data.join("places.sqlite");
    if !path.exists() {
        return Ok(Outcome::NoDb);
    }
    let mut conn = Connection::open(&path)?;
    conn.busy_timeout(Duration::from_secs(3))?;
    apply(&mut conn, bookmarks)
}

fn columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut st = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let v = st.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(v)
}

struct Ctx<'a> {
    tx: &'a rusqlite::Transaction<'a>,
    triggers: bool,
    place_cols: Vec<String>,
    origins: bool,
    now: i64,
    seed: u64,
}

impl Ctx<'_> {
    fn adjust(&self, place: i64, delta: i64) -> Result<()> {
        if !self.triggers && place > 0 {
            self.tx.execute("UPDATE moz_places SET foreign_count = MAX(0, foreign_count + ?1) WHERE id = ?2", params![delta, place])?;
        }
        Ok(())
    }

    fn delete_tree(&self, id: i64) -> Result<()> {
        let kids: Vec<i64> = {
            let mut st = self.tx.prepare("SELECT id FROM moz_bookmarks WHERE parent = ?1")?;
            st.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for k in kids {
            self.delete_tree(k)?;
        }
        let fk: Option<i64> = self.tx.query_row("SELECT fk FROM moz_bookmarks WHERE id = ?1", [id], |r| r.get(0)).optional()?.flatten();
        self.tx.execute("DELETE FROM moz_bookmarks WHERE id = ?1", [id])?;
        self.adjust(fk.unwrap_or(0), -1)
    }

    fn origin(&self, url: &str) -> Result<Option<i64>> {
        if !self.origins {
            return Ok(None);
        }
        let (prefix, host) = split_url(url);
        if let Some(id) =
            self.tx.query_row("SELECT id FROM moz_origins WHERE prefix = ?1 AND host = ?2", [&prefix, &host], |r| r.get(0)).optional()?
        {
            return Ok(Some(id));
        }
        self.tx.execute("INSERT INTO moz_origins (prefix, host, frecency) VALUES (?1, ?2, 0)", [&prefix, &host])?;
        Ok(Some(self.tx.last_insert_rowid()))
    }

    /// Id of the `moz_places` row for `url`, created when missing.
    fn place(&mut self, url: &str, title: &str) -> Result<i64> {
        if let Some(id) = self.tx.query_row("SELECT id FROM moz_places WHERE url = ?1", [url], |r| r.get(0)).optional()? {
            return Ok(id);
        }
        let (_, host) = split_url(url);
        let guid = new_guid(&mut self.seed);
        let origin = self.origin(url)?;
        let mut cols: Vec<(&str, rusqlite::types::Value)> = vec![
            ("url", url.to_string().into()),
            ("title", title.to_string().into()),
            ("rev_host", rev_host(&host).into()),
            ("visit_count", 0i64.into()),
            ("hidden", 0i64.into()),
            ("typed", 0i64.into()),
            ("frecency", (-1i64).into()),
            ("guid", guid.into()),
            ("foreign_count", 0i64.into()),
            ("url_hash", url_hash(url).into()),
            ("recalc_frecency", 1i64.into()),
        ];
        if let Some(o) = origin {
            cols.push(("origin_id", o.into()));
        }
        cols.retain(|(c, _)| self.place_cols.iter().any(|x| x == c));
        let names: Vec<&str> = cols.iter().map(|c| c.0).collect();
        let marks: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
        self.tx.execute(
            &format!("INSERT INTO moz_places ({}) VALUES ({})", names.join(","), marks.join(",")),
            rusqlite::params_from_iter(cols.iter().map(|c| &c.1)),
        )?;
        Ok(self.tx.last_insert_rowid())
    }

    fn insert_bookmark(&mut self, kind: i64, fk: Option<i64>, parent: i64, pos: i64, title: &str, guid: &str) -> Result<i64> {
        self.tx.execute(
            "INSERT INTO moz_bookmarks (type, fk, parent, position, title, dateAdded, lastModified, guid, syncStatus, syncChangeCounter) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, 1, 1)",
            params![kind, fk, parent, pos, title, self.now, guid],
        )?;
        let id = self.tx.last_insert_rowid();
        self.adjust(fk.unwrap_or(0), 1)?;
        Ok(id)
    }

    fn set_pos(&self, id: i64, pos: i64, title: &str) -> Result<()> {
        self.tx.execute(
            "UPDATE moz_bookmarks SET lastModified = CASE WHEN position = ?1 AND title IS ?2 THEN lastModified ELSE ?3 END, position = ?1, title = ?2 \
             WHERE id = ?4",
            params![pos, title, self.now, id],
        )?;
        Ok(())
    }
}

fn apply(conn: &mut Connection, list: &[Bookmark]) -> Result<Outcome> {
    let place_cols = columns(conn, "moz_places")?;
    let origins =
        conn.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name='moz_origins'", [], |_| Ok(())).optional()?.is_some();
    let triggers = conn
        .query_row("SELECT 1 FROM sqlite_master WHERE type='trigger' AND name LIKE 'moz_bookmarks_foreign_count%'", [], |_| Ok(()))
        .optional()?
        .is_some();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let toolbar: Option<i64> = tx.query_row("SELECT id FROM moz_bookmarks WHERE guid = ?1", [TOOLBAR_GUID], |r| r.get(0)).optional()?;
    let Some(toolbar) = toolbar else { return Ok(Outcome::NoToolbar) };
    let mut cx = Ctx { tx: &tx, triggers, place_cols, origins, now: now_us(), seed: now_us() as u64 ^ 0x9E37_79B9_7F4A_7C15 };

    let root: Option<i64> = tx.query_row("SELECT id FROM moz_bookmarks WHERE guid = ?1", [FOLDER_GUID], |r| r.get(0)).optional()?;
    if list.is_empty() {
        if let Some(r) = root {
            cx.delete_tree(r)?;
        }
        tx.commit()?;
        return Ok(Outcome::Synced);
    }
    let root = match root {
        Some(r) => r,
        None => {
            let pos: i64 =
                tx.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM moz_bookmarks WHERE parent = ?1", [toolbar], |r| r.get(0))?;
            cx.insert_bookmark(2, None, toolbar, pos, FOLDER_TITLE, FOLDER_GUID)?
        }
    };

    // Desired layout: loose bookmarks first, then sub-folders (each with its bookmarks), in list order.
    let folder_of = |b: &Bookmark| b.folder.as_deref().map(str::trim).filter(|f| !f.is_empty()).map(str::to_string);
    let mut folder_names: Vec<String> = Vec::new();
    for b in list {
        if let Some(f) = folder_of(b)
            && !folder_names.contains(&f)
        {
            folder_names.push(f);
        }
    }

    // Existing sub-folders: keep the ones we want, drop the rest.
    let existing: Vec<(i64, String)> = {
        let mut st = tx.prepare("SELECT id, COALESCE(title,'') FROM moz_bookmarks WHERE parent = ?1 AND type = 2")?;
        st.query_map([root], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?
    };
    let mut folder_ids: HashMap<String, i64> = HashMap::new();
    for (id, title) in existing {
        if folder_names.contains(&title) && !folder_ids.contains_key(&title) {
            folder_ids.insert(title, id);
        } else {
            cx.delete_tree(id)?;
        }
    }
    for (i, name) in folder_names.iter().enumerate() {
        let pos = (i + 1000) as i64;
        let id = match folder_ids.get(name) {
            Some(id) => *id,
            None => cx.insert_bookmark(2, None, root, pos, name, &folder_guid(name))?,
        };
        cx.set_pos(id, pos, name)?;
        folder_ids.insert(name.clone(), id);
    }

    // Bookmarks, per parent.
    let mut parents: Vec<(i64, Option<String>)> = vec![(root, None)];
    parents.extend(folder_names.iter().map(|n| (folder_ids[n], Some(n.clone()))));
    for (parent, folder) in parents {
        let want: Vec<(&Bookmark, String)> = list.iter().filter(|b| folder_of(b) == folder).map(|b| (b, normalize(b.url.trim()))).collect();
        let mut have: Vec<(i64, String)> = {
            let mut st = tx.prepare(
                "SELECT b.id, p.url FROM moz_bookmarks b JOIN moz_places p ON p.id = b.fk WHERE b.parent = ?1 AND b.type = 1 ORDER BY b.position",
            )?;
            st.query_map([parent], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?
        };
        // Non-url rows (separators, odd types) inside our folder are not ours to keep either.
        let stray: Vec<i64> = {
            let mut st = tx.prepare("SELECT id FROM moz_bookmarks WHERE parent = ?1 AND type NOT IN (1, 2)")?;
            st.query_map([parent], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for id in stray {
            cx.delete_tree(id)?;
        }
        for (pos, (b, url)) in want.iter().enumerate() {
            let title = if b.title.trim().is_empty() { url.as_str() } else { b.title.trim() };
            let pos = pos as i64;
            if let Some(i) = have.iter().position(|h| &h.1 == url) {
                let (id, _) = have.remove(i);
                cx.set_pos(id, pos, title)?;
            } else {
                let place = cx.place(url, title)?;
                let guid = new_guid(&mut cx.seed);
                cx.insert_bookmark(1, Some(place), parent, pos, title, &guid)?;
            }
        }
        for (id, _) in have {
            cx.delete_tree(id)?;
        }
    }
    tx.execute("UPDATE moz_bookmarks SET lastModified = ?1 WHERE id IN (?2, ?3)", params![cx.now, root, toolbar])?;
    tx.commit()?;
    Ok(Outcome::Synced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SCHEMA: &str = "
CREATE TABLE moz_origins (id INTEGER PRIMARY KEY, prefix TEXT NOT NULL, host TEXT NOT NULL, frecency INTEGER NOT NULL, UNIQUE(prefix, host));
CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR, rev_host LONGVARCHAR, visit_count INTEGER DEFAULT 0,
  hidden INTEGER DEFAULT 0 NOT NULL, typed INTEGER DEFAULT 0 NOT NULL, frecency INTEGER DEFAULT -1 NOT NULL, last_visit_date INTEGER,
  guid TEXT, foreign_count INTEGER DEFAULT 0 NOT NULL, url_hash INTEGER DEFAULT 0 NOT NULL, origin_id INTEGER);
CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER DEFAULT NULL, parent INTEGER, position INTEGER, title LONGVARCHAR,
  keyword_id INTEGER, folder_type TEXT, dateAdded INTEGER, lastModified INTEGER, guid TEXT, syncStatus INTEGER NOT NULL DEFAULT 0,
  syncChangeCounter INTEGER NOT NULL DEFAULT 1);
CREATE TRIGGER moz_bookmarks_foreign_count_afterinsert_trigger AFTER INSERT ON moz_bookmarks BEGIN
  UPDATE moz_places SET foreign_count = foreign_count + 1 WHERE id = NEW.fk; END;
CREATE TRIGGER moz_bookmarks_foreign_count_afterdelete_trigger AFTER DELETE ON moz_bookmarks BEGIN
  UPDATE moz_places SET foreign_count = foreign_count - 1 WHERE id = OLD.fk; END;
INSERT INTO moz_bookmarks (id, type, parent, position, title, guid) VALUES (1, 2, 0, 0, '', 'root________');
INSERT INTO moz_bookmarks (id, type, parent, position, title, guid) VALUES (2, 2, 1, 0, 'toolbar', 'toolbar_____');
INSERT INTO moz_bookmarks (id, type, parent, position, title, guid) VALUES (3, 2, 1, 1, 'menu', 'menu________');
";

    /// Same schema without the origins table, the triggers and `origin_id`.
    const SCHEMA_PLAIN: &str = "
CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR, rev_host LONGVARCHAR, visit_count INTEGER DEFAULT 0,
  hidden INTEGER DEFAULT 0 NOT NULL, typed INTEGER DEFAULT 0 NOT NULL, frecency INTEGER DEFAULT -1 NOT NULL, guid TEXT,
  foreign_count INTEGER DEFAULT 0 NOT NULL, url_hash INTEGER DEFAULT 0 NOT NULL);
CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER DEFAULT NULL, parent INTEGER, position INTEGER, title LONGVARCHAR,
  dateAdded INTEGER, lastModified INTEGER, guid TEXT, syncStatus INTEGER NOT NULL DEFAULT 0, syncChangeCounter INTEGER NOT NULL DEFAULT 1);
INSERT INTO moz_bookmarks (id, type, parent, position, title, guid) VALUES (1, 2, 0, 0, '', 'root________');
INSERT INTO moz_bookmarks (id, type, parent, position, title, guid) VALUES (2, 2, 1, 0, 'toolbar', 'toolbar_____');
";

    fn setup(name: &str, schema: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cm-places-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Connection::open(d.join("places.sqlite")).unwrap().execute_batch(schema).unwrap();
        d
    }

    fn open(d: &Path) -> Connection {
        Connection::open(d.join("places.sqlite")).unwrap()
    }

    fn bm(title: &str, url: &str, folder: Option<&str>) -> Bookmark {
        Bookmark { title: title.into(), url: url.into(), folder: folder.map(String::from) }
    }

    /// (title, url, parent folder title) of every url bookmark below the managed folder.
    fn managed(d: &Path) -> Vec<(String, String, String)> {
        let c = open(d);
        let mut st = c
            .prepare(
                "SELECT COALESCE(b.title,''), p.url, f.title FROM moz_bookmarks b JOIN moz_places p ON p.id = b.fk \
                 JOIN moz_bookmarks f ON f.id = b.parent WHERE b.type = 1 AND (f.guid = 'cmBookmarks_' OR f.parent = \
                 (SELECT id FROM moz_bookmarks WHERE guid = 'cmBookmarks_')) ORDER BY f.id, b.position",
            )
            .unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(Result::unwrap).collect()
    }

    fn fc(c: &Connection, url: &str) -> i64 {
        c.query_row("SELECT foreign_count FROM moz_places WHERE url = ?1", [url], |r| r.get(0)).unwrap()
    }

    #[test]
    fn no_db_is_skipped() {
        let d = std::env::temp_dir().join(format!("cm-places-none-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        let _ = fs::remove_file(d.join("places.sqlite"));
        assert_eq!(sync(&d, &[bm("a", "https://a.test/", None)]).unwrap(), Outcome::NoDb);
        assert!(!d.join("places.sqlite").exists());
    }

    #[test]
    fn creates_updates_and_removes_only_inside_the_folder() {
        let d = setup("sync", SCHEMA);
        open(&d)
            .execute_batch(
                "INSERT INTO moz_places (id, url, guid) VALUES (100, 'https://mine.test/', 'mineguid0001');
                 INSERT INTO moz_bookmarks (type, fk, parent, position, title, guid) VALUES (1, 100, 2, 5, 'mine', 'minebmk00001');",
            )
            .unwrap();
        let list =
            vec![bm("A", "https://a.test", None), bm("B", "https://b.test/x", Some("Work")), bm("C", "https://c.test/", Some("Work"))];
        assert_eq!(sync(&d, &list).unwrap(), Outcome::Synced);
        let t = managed(&d);
        assert_eq!(t.len(), 3);
        assert!(t.contains(&("A".into(), "https://a.test/".into(), FOLDER_TITLE.into())));
        assert!(t.contains(&("B".into(), "https://b.test/x".into(), "Work".into())));
        let c = open(&d);
        let (guid, parent): (String, i64) =
            c.query_row("SELECT guid, parent FROM moz_bookmarks WHERE title = ?1", [FOLDER_TITLE], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((guid.as_str(), parent), (FOLDER_GUID, 2));
        let (hash, origin): (i64, Option<i64>) = c
            .query_row("SELECT url_hash, origin_id FROM moz_places WHERE url = 'https://a.test/'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(hash, url_hash("https://a.test/"));
        assert!(origin.is_some());
        assert_eq!(fc(&c, "https://a.test/"), 1);
        drop(c);

        // second sync: rename A, drop C, add D; running it twice changes nothing more
        let list2 =
            vec![bm("A2", "https://a.test/", None), bm("B", "https://b.test/x", Some("Work")), bm("D", "https://d.test/", Some("Play"))];
        sync(&d, &list2).unwrap();
        sync(&d, &list2).unwrap();
        let t = managed(&d);
        assert_eq!(t.len(), 3);
        assert!(t.contains(&("A2".into(), "https://a.test/".into(), FOLDER_TITLE.into())));
        assert!(t.contains(&("D".into(), "https://d.test/".into(), "Play".into())));
        assert!(!t.iter().any(|r| r.1 == "https://c.test/"));
        let c = open(&d);
        assert_eq!(fc(&c, "https://c.test/"), 0);
        assert_eq!(fc(&c, "https://a.test/"), 1);
        let n: i64 = c.query_row("SELECT COUNT(*) FROM moz_bookmarks WHERE guid = 'cmBookmarks_'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        drop(c);

        // an empty list removes the folder and everything in it, nothing else
        sync(&d, &[]).unwrap();
        let c = open(&d);
        assert!(managed(&d).is_empty());
        let mine: i64 = c.query_row("SELECT COUNT(*) FROM moz_bookmarks WHERE guid = 'minebmk00001'", [], |r| r.get(0)).unwrap();
        assert_eq!(mine, 1);
        let folders: i64 = c.query_row("SELECT COUNT(*) FROM moz_bookmarks WHERE type = 2 AND id > 3", [], |r| r.get(0)).unwrap();
        assert_eq!(folders, 0);
    }

    #[test]
    fn works_without_triggers_and_origins() {
        let d = setup("plain", SCHEMA_PLAIN);
        assert_eq!(sync(&d, &[bm("", "https://a.test/", None)]).unwrap(), Outcome::Synced);
        let c = open(&d);
        assert_eq!(fc(&c, "https://a.test/"), 1);
        let title: String = c.query_row("SELECT title FROM moz_bookmarks WHERE fk IS NOT NULL", [], |r| r.get(0)).unwrap();
        assert_eq!(title, "https://a.test/"); // empty title falls back to the url
        drop(c);
        sync(&d, &[]).unwrap();
        assert_eq!(fc(&open(&d), "https://a.test/"), 0);
    }

    #[test]
    fn missing_toolbar_is_skipped() {
        let d = setup(
            "notb",
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT); CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, guid TEXT);",
        );
        assert_eq!(sync(&d, &[bm("a", "https://a.test/", None)]).unwrap(), Outcome::NoToolbar);
    }

    #[test]
    fn helpers() {
        assert_ne!(url_hash("https://a.test/"), url_hash("http://a.test/"));
        assert_eq!(rev_host("a.test"), "tset.a.");
        assert_eq!(normalize("https://x.y"), "https://x.y/");
        assert_eq!(normalize("https://x.y/p"), "https://x.y/p");
        assert_eq!(folder_guid("Work").len(), 12);
    }
}
