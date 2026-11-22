//! Cookie store of a CLOSED profile, read and written directly.
//!
//! Firefox keeps cookies in `<user-data>/cookies.sqlite` (table `moz_cookies`). No browser is
//! started, so this works without the Camoufox browser, in milliseconds, and cannot disturb the
//! pinned fingerprint. The columns are discovered with `PRAGMA table_info`, so a Firefox that adds
//! or drops a column does not break it. `domain` follows the browser-extension convention: a
//! leading dot unless `host_only`.

use crate::closed::firefox_markers;
use anyhow::{Result, anyhow};
use cm_core::{
    CmError,
    cookies::{Cookie, CookieImportMode},
};
use rusqlite::{Connection, params_from_iter, types::Value};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Schema written by Camoufox's Firefox (cookies.sqlite user_version 17, read from a real profile),
/// used only when the profile has never been started and so has no cookie store yet; Firefox
/// migrates other versions on its own.
const SCHEMA: &str = "
CREATE TABLE moz_cookies (
  id INTEGER PRIMARY KEY,
  originAttributes TEXT NOT NULL DEFAULT '',
  name TEXT,
  value TEXT,
  host TEXT,
  path TEXT,
  expiry INTEGER,
  lastAccessed INTEGER,
  creationTime INTEGER,
  isSecure INTEGER,
  isHttpOnly INTEGER,
  inBrowserElement INTEGER DEFAULT 0,
  sameSite INTEGER DEFAULT 0,
  schemeMap INTEGER DEFAULT 0,
  isPartitionedAttributeSet INTEGER DEFAULT 0,
  updateTime INTEGER,
  CONSTRAINT moz_uniqueid UNIQUE (name, host, path, originAttributes)
);
PRAGMA user_version = 17;
";

/// Firefox sameSite value for "no attribute".
const SAMESITE_UNSET: i64 = 256;
/// Expiries above this are milliseconds (Firefox since 2021); older profiles hold seconds.
const MS_THRESHOLD: i64 = 100_000_000_000;

fn refuse_if_running(user_data: &Path) -> Result<()> {
    if firefox_markers(user_data).is_empty() {
        Ok(())
    } else {
        Err(CmError::Conflict("the profile is running; stop it first".into()).into())
    }
}

fn columns(db: &Connection) -> Result<HashSet<String>> {
    let mut st = db.prepare("PRAGMA table_info(moz_cookies)")?;
    let cols = st.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<HashSet<_>>>()?;
    Ok(cols)
}

fn open(user_data: &Path, create: bool) -> Result<Option<Connection>> {
    let path = user_data.join("cookies.sqlite");
    if !path.exists() {
        if !create {
            return Ok(None);
        }
        fs::create_dir_all(user_data)?;
    }
    let db = Connection::open(&path)?;
    db.busy_timeout(Duration::from_secs(10))?;
    if columns(&db)?.is_empty() {
        if !create {
            return Ok(None);
        }
        db.execute_batch(SCHEMA)?;
    }
    Ok(Some(db))
}

fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// Unexpired cookies of the profile, ordered by host and name. No store yet = empty list (the
/// file is not created).
pub fn export(user_data: &Path) -> Result<Vec<Cookie>> {
    refuse_if_running(user_data)?;
    let Some(db) = open(user_data, false)? else { return Ok(Vec::new()) };
    let same = if columns(&db)?.contains("sameSite") { "sameSite" } else { "0" };
    let mut st =
        db.prepare(&format!("SELECT name, value, host, path, expiry, isSecure, isHttpOnly, {same} FROM moz_cookies ORDER BY host, name"))?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, Option<i64>>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, Option<i64>>(7)?,
        ))
    })?;
    let now = now_secs();
    let mut out = Vec::new();
    for row in rows {
        let (name, value, host, path, expiry, secure, http_only, ss) = row?;
        let exp = expiry.filter(|&e| e != 0).map(|e| if e > MS_THRESHOLD { e as f64 / 1000.0 } else { e as f64 });
        if exp.is_some_and(|e| e < now) {
            continue; // expired, Firefox will purge it
        }
        let host = host.unwrap_or_default();
        out.push(Cookie {
            name: name.unwrap_or_default(),
            value: value.unwrap_or_default(),
            host_only: !host.starts_with('.'),
            domain: host,
            path: path.filter(|p| !p.is_empty()).unwrap_or_else(|| "/".into()),
            expires: exp,
            http_only: http_only.unwrap_or(0) != 0,
            secure: secure.unwrap_or(0) != 0,
            same_site: match ss {
                Some(0) => Some("None".into()),
                Some(1) => Some("Lax".into()),
                Some(2) => Some("Strict".into()),
                _ => None,
            },
        });
    }
    Ok(out)
}

/// Write `cookies` into the profile (creating the store when absent). Returns how many were written.
pub fn import(user_data: &Path, cookies: &[Cookie], mode: CookieImportMode) -> Result<usize> {
    refuse_if_running(user_data)?;
    let mut db = open(user_data, true)?.ok_or_else(|| anyhow!("cannot open the cookie store"))?;
    let has = columns(&db)?;
    let micros = (now_secs() * 1_000_000.0) as i64;
    // Millisecond expiry is the Firefox >= 2021 convention. Detect it from the existing data and
    // fall back to milliseconds for an empty store (Camoufox ships a recent Firefox).
    let max: Option<i64> = db.query_row("SELECT MAX(expiry) FROM moz_cookies", [], |r| r.get(0))?;
    let ms = !max.is_some_and(|m| m != 0 && m < MS_THRESHOLD);
    let tx = db.transaction()?;
    if mode == CookieImportMode::All {
        tx.execute("DELETE FROM moz_cookies", [])?;
    } else {
        let hosts: BTreeSet<&str> = cookies.iter().map(|c| c.domain.trim_start_matches('.')).collect();
        for host in hosts {
            tx.execute("DELETE FROM moz_cookies WHERE host IN (?1, ?2)", [host, &format!(".{host}")])?;
        }
    }
    for c in cookies {
        let exp = c.expires.unwrap_or(0.0) as i64;
        let ss = match c.same_site.as_deref() {
            Some("None") => 0,
            Some("Lax") => 1,
            Some("Strict") => 2,
            _ => SAMESITE_UNSET,
        };
        let int = Value::Integer;
        let vals: Vec<(&str, Value)> = vec![
            ("originAttributes", Value::Text(String::new())),
            ("name", Value::Text(c.name.clone())),
            ("value", Value::Text(c.value.clone())),
            ("host", Value::Text(c.domain.clone())),
            ("path", Value::Text(if c.path.is_empty() { "/".into() } else { c.path.clone() })),
            ("expiry", int(if ms { exp * 1000 } else { exp })),
            ("lastAccessed", int(micros)),
            ("creationTime", int(micros)),
            ("updateTime", int(micros)),
            ("isSecure", int(i64::from(c.secure))),
            ("isHttpOnly", int(i64::from(c.http_only))),
            ("inBrowserElement", int(0)),
            ("sameSite", int(ss)),
            ("rawSameSite", int(ss)),
            ("schemeMap", int(0)), // what Playwright's add_cookies writes too
            ("isPartitionedAttributeSet", int(0)),
        ];
        let used: Vec<&(&str, Value)> = vals.iter().filter(|(k, _)| has.contains(*k)).collect();
        let names: Vec<&str> = used.iter().map(|(k, _)| *k).collect();
        let marks = vec!["?"; used.len()].join(", ");
        tx.execute(
            &format!("INSERT OR REPLACE INTO moz_cookies ({}) VALUES ({marks})", names.join(", ")),
            params_from_iter(used.iter().map(|(_, v)| v)),
        )?;
    }
    tx.commit()?;
    db.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    Ok(cookies.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cm-cookies-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn cookie(name: &str, domain: &str, expires: Option<f64>) -> Cookie {
        Cookie {
            name: name.into(),
            value: "v".into(),
            domain: domain.into(),
            path: "/".into(),
            expires,
            http_only: true,
            secure: true,
            same_site: Some("Lax".into()),
            host_only: !domain.starts_with('.'),
        }
    }

    fn future() -> f64 {
        now_secs() + 86_400.0
    }

    #[test]
    fn missing_store_exports_empty_and_creates_nothing() {
        let d = tmp("missing");
        assert!(export(&d).unwrap().is_empty());
        assert!(!d.join("cookies.sqlite").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn roundtrip_and_user_version() {
        let root = tmp("round");
        let d = root.join("nested");
        let exp = future().floor();
        let list = vec![cookie("b", ".example.com", Some(exp)), cookie("a", "host.test", None)];
        assert_eq!(import(&d, &list, CookieImportMode::Domains).unwrap(), 2);
        let db = Connection::open(d.join("cookies.sqlite")).unwrap();
        assert_eq!(db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 17);
        let got = export(&d).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], list[0]); // ORDER BY host: ".example.com" sorts first
        assert_eq!(got[1], list[1]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn domains_mode_keeps_other_hosts_all_mode_clears() {
        let d = tmp("modes");
        let e = Some(future());
        import(&d, &[cookie("a", ".one.com", e), cookie("b", "two.com", e)], CookieImportMode::Domains).unwrap();
        import(&d, &[cookie("c", "one.com", e)], CookieImportMode::Domains).unwrap();
        let mut names: Vec<String> = export(&d).unwrap().into_iter().map(|c| c.name).collect();
        names.sort();
        assert_eq!(names, ["b", "c"]); // one.com replaced (".one.com" removed too), two.com stays
        import(&d, &[cookie("z", "x.com", e)], CookieImportMode::All).unwrap();
        let names: Vec<String> = export(&d).unwrap().into_iter().map(|c| c.name).collect();
        assert_eq!(names, ["z"]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn expired_skipped_and_ms_vs_seconds() {
        let d = tmp("expiry");
        import(&d, &[cookie("ok", "a.com", Some(future()))], CookieImportMode::All).unwrap();
        let db = Connection::open(d.join("cookies.sqlite")).unwrap();
        let ms: i64 = db.query_row("SELECT expiry FROM moz_cookies", [], |r| r.get(0)).unwrap();
        assert!(ms > MS_THRESHOLD, "empty store defaults to milliseconds");
        db.execute(
            "INSERT INTO moz_cookies (name, host, path, expiry, originAttributes) VALUES ('old', 'b.com', '/', 1000000000000, '')",
            [],
        )
        .unwrap();
        drop(db);
        let names: Vec<String> = export(&d).unwrap().into_iter().map(|c| c.name).collect();
        assert_eq!(names, ["ok"]);
        // seconds convention detected from MAX(expiry)
        let db = Connection::open(d.join("cookies.sqlite")).unwrap();
        db.execute("DELETE FROM moz_cookies", []).unwrap();
        db.execute("INSERT INTO moz_cookies (name, host, path, expiry, originAttributes) VALUES ('s', 'c.com', '/', 2000000000, '')", [])
            .unwrap();
        drop(db);
        import(&d, &[cookie("n", "d.com", Some(2_000_000_100.0))], CookieImportMode::Domains).unwrap();
        let db = Connection::open(d.join("cookies.sqlite")).unwrap();
        let e: i64 = db.query_row("SELECT expiry FROM moz_cookies WHERE name = 'n'", [], |r| r.get(0)).unwrap();
        assert_eq!(e, 2_000_000_100);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_samesite_column_is_tolerated() {
        let d = tmp("nocol");
        let db = Connection::open(d.join("cookies.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE moz_cookies (id INTEGER PRIMARY KEY, originAttributes TEXT NOT NULL DEFAULT '', name TEXT, value TEXT, host TEXT, path TEXT, expiry INTEGER, isSecure INTEGER, isHttpOnly INTEGER, CONSTRAINT u UNIQUE (name, host, path, originAttributes));",
        )
        .unwrap();
        drop(db);
        import(&d, &[cookie("a", "a.com", Some(future()))], CookieImportMode::Domains).unwrap();
        let got = export(&d).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].same_site.as_deref(), Some("None")); // no column: reported as 0
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn running_profile_is_refused() {
        let d = tmp("running");
        #[cfg(unix)]
        std::os::unix::fs::symlink(format!("127.0.0.1:+{}", std::process::id()), d.join("lock")).unwrap();
        #[cfg(windows)]
        let _held = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::write(d.join("parent.lock"), b"").unwrap();
            fs::OpenOptions::new().read(true).write(true).share_mode(0).open(d.join("parent.lock")).unwrap()
        };
        for r in [export(&d).map(|_| ()), import(&d, &[], CookieImportMode::All).map(|_| ())] {
            let e = r.unwrap_err();
            assert!(matches!(e.downcast_ref::<CmError>(), Some(CmError::Conflict(_))), "{e}");
        }
        assert!(!d.join("cookies.sqlite").exists());
        let _ = fs::remove_dir_all(&d);
    }
}
