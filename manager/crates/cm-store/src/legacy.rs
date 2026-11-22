//! One-time import of the files used before SQLite (D2):
//!
//! - `<data_dir>/proxies.txt` (or `PROXY_FILE`): `host:port:user:pass` per line → `proxies`
//! - `<data_dir>/state.json`: `{"assignments": {profileId: index}}` (index into the proxy list)
//!   → `profiles.proxy_id`
//! - `<data_dir>/camoufox/profiles/<id>/profile.json` → `profiles` (user-data stays in place)
//!
//! Everything goes in one transaction, then the marker `legacy.imported` is written. After the
//! commit the originals are renamed to `*.bak` (a proxy file outside the data dir is left alone).
//! The marker makes the import run at most once per database.
//!
//! [`import_proxy_file`] additionally picks up a `proxies.txt` dropped in later (until phase 2's
//! proxy library UI exists).

use crate::{Db, profiles, proxies, settings};
use anyhow::{Context, Result};
use cm_core::{
    Clock, NewProxy, Os, Profile, ProfileSettings, ProxyId, ProxyScheme, SystemClock, profile::valid_id, settings::LEGACY_IMPORTED,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// What the import did.
#[derive(Debug, Default, PartialEq)]
pub struct Report {
    pub proxies: usize,
    pub profiles: usize,
    pub assignments: usize,
    /// Files renamed to `*.bak`.
    pub renamed: Vec<PathBuf>,
    /// Human-readable problems (bad lines/files); the import still succeeds.
    pub warnings: Vec<String>,
}

/// `profile.json` written by the pre-SQLite Camoufox provider.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldProfile {
    id: String,
    name: String,
    os: String,
    created_at: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    last_activity: Option<String>,
    config: Value,
}

#[derive(Deserialize, Default)]
struct OldState {
    #[serde(default)]
    assignments: HashMap<String, usize>,
}

/// Parse one proxies.txt line: `host:port:user:pass` or `host:port`.
fn parse_line(line: &str) -> Option<NewProxy> {
    let p: Vec<&str> = line.trim().split(':').collect();
    let (host, port) = (p.first()?.trim(), p.get(1)?.trim().parse().ok()?);
    if host.is_empty() {
        return None;
    }
    let (username, password) = match p.len() {
        2 => (None, None),
        4 => (Some(p[2].to_string()), Some(p[3].to_string())),
        _ => return None,
    };
    Some(NewProxy {
        name: format!("{host}:{port}"),
        scheme: ProxyScheme::Http,
        host: host.into(),
        port,
        username,
        password,
        ..Default::default()
    })
}

/// Insert every valid line of a proxies.txt; a line equal to a stored proxy (scheme, host, port,
/// username) reuses it. Returns the id per valid line (old list index → id) and how many were new.
fn read_proxy_file(c: &Connection, file: &Path, warnings: &mut Vec<String>) -> Result<(Vec<ProxyId>, usize)> {
    let text = fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
    let (mut ids, mut added) = (Vec::new(), 0);
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some(p) = parse_line(line) else {
            warnings.push(format!("{}:{}: not host:port[:user:pass], skipped", file.display(), n + 1));
            continue;
        };
        let existing: Option<ProxyId> = c
            .query_row(
                "SELECT id FROM proxies WHERE scheme = ?1 AND host = ?2 AND port = ?3 AND username IS ?4",
                params![p.scheme.as_str(), p.host, p.port, p.username],
                |r| r.get(0),
            )
            .optional()?;
        ids.push(match existing {
            Some(id) => id,
            None => {
                added += 1;
                proxies::insert(c, &p)?
            }
        });
    }
    Ok((ids, added))
}

fn rename_bak(f: &Path, report: &mut Report) {
    let mut bak = f.as_os_str().to_owned();
    bak.push(".bak");
    match fs::rename(f, &bak) {
        Ok(()) => report.renamed.push(PathBuf::from(bak)),
        Err(e) => report.warnings.push(format!("cannot rename {}: {e}", f.display())),
    }
}

/// Bridge until the proxy library UI exists (phase 2): a `proxies.txt` dropped into the data
/// dir after the first import is imported on the next start (duplicates skipped) and renamed
/// to `.bak`. `None` = no such file.
pub fn import_proxy_file(db: &Db, data_dir: &Path) -> Result<Option<Report>> {
    let file = data_dir.join("proxies.txt");
    if !file.is_file() {
        return Ok(None);
    }
    let mut report = Report::default();
    db.with(|c| {
        let tx = c.transaction()?;
        report.proxies = read_proxy_file(&tx, &file, &mut report.warnings)?.1;
        tx.commit()?;
        Ok(())
    })?;
    rename_bak(&file, &mut report);
    Ok(Some(report))
}

/// Run the import if it has not run on this database yet. `None` = already done before.
pub fn import(db: &Db, data_dir: &Path, proxy_file: Option<&Path>) -> Result<Option<Report>> {
    db.with(|c| {
        if settings::get(c, LEGACY_IMPORTED)?.is_some() {
            return Ok(None);
        }
        let mut report = Report::default();
        let mut to_rename: Vec<PathBuf> = Vec::new();
        let tx = c.transaction()?;

        // 1. proxies: list index (as the old app counted: valid lines only) → new id
        let file = proxy_file.map(Path::to_path_buf).unwrap_or(data_dir.join("proxies.txt"));
        let mut by_index: Vec<ProxyId> = Vec::new();
        if file.is_file() {
            let (ids, added) = read_proxy_file(&tx, &file, &mut report.warnings)?;
            (by_index, report.proxies) = (ids, added);
            if file.starts_with(data_dir) {
                to_rename.push(file);
            }
        }

        // 2. assignments
        let state_path = data_dir.join("state.json");
        let state: OldState = match fs::read_to_string(&state_path) {
            Ok(t) => {
                to_rename.push(state_path.clone());
                serde_json::from_str(&t).unwrap_or_else(|e| {
                    report.warnings.push(format!("{}: {e}", state_path.display()));
                    OldState::default()
                })
            }
            Err(_) => OldState::default(),
        };

        // 3. profiles
        let dir = data_dir.join("camoufox/profiles");
        let mut entries: Vec<PathBuf> =
            fs::read_dir(&dir).map(|rd| rd.flatten().map(|e| e.path().join("profile.json")).filter(|p| p.is_file()).collect()).unwrap_or_default();
        entries.sort();
        for path in entries {
            let old: OldProfile = match fs::read_to_string(&path).map_err(anyhow::Error::from).and_then(|t| Ok(serde_json::from_str(&t)?)) {
                Ok(p) => p,
                Err(e) => {
                    report.warnings.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            if !valid_id(&old.id) || path.parent().and_then(Path::file_name).and_then(|n| n.to_str()) != Some(old.id.as_str()) {
                report.warnings.push(format!("{}: id does not match its folder, skipped", path.display()));
                continue;
            }
            let proxy_id = state.assignments.get(&old.id).and_then(|&i| by_index.get(i).copied());
            report.assignments += usize::from(proxy_id.is_some());
            let p = Profile {
                id: old.id,
                name: old.name,
                os: Os::parse(&old.os).unwrap_or(Os::Win),
                notes: old.notes,
                updated_at: old.created_at.clone(),
                created_at: old.created_at,
                last_activity: old.last_activity,
                proxy_id,
                machine: old.config,
                settings: ProfileSettings::default(),
            };
            profiles::insert(&tx, &p)?;
            report.profiles += 1;
            to_rename.push(path);
        }

        settings::set(
            &tx,
            LEGACY_IMPORTED,
            &json!({ "at": SystemClock.now_iso(), "proxies": report.proxies, "profiles": report.profiles, "assignments": report.assignments }),
        )?;
        tx.commit()?;

        for f in to_rename {
            rename_bak(&f, &mut report);
        }
        Ok(Some(report))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqliteProfiles, SqliteProxies};
    use cm_core::{ProfileRepo, ProxyRepo};

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("cm-legacy-{}-{}", std::process::id(), SystemClock.now()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn lines() {
        let p = parse_line("1.2.3.4:8080:u:p").unwrap();
        assert_eq!((p.host.as_str(), p.port, p.username.as_deref(), p.password.as_deref()), ("1.2.3.4", 8080, Some("u"), Some("p")));
        assert!(parse_line("1.2.3.4:8080").unwrap().username.is_none());
        assert!(parse_line("garbage").is_none());
        assert!(parse_line("1.2.3.4:x:u:p").is_none());
    }

    #[tokio::test]
    async fn imports_once_and_renames() {
        let d = tmp();
        fs::write(d.join("proxies.txt"), "1.2.3.4:8080:u:p\nbad line\n5.6.7.8:3128:a:b\n").unwrap();
        let id = "0123456789abcdef01234567";
        fs::write(d.join("state.json"), format!(r#"{{"assignments":{{"{id}":1}}}}"#)).unwrap();
        let pdir = d.join("camoufox/profiles").join(id);
        fs::create_dir_all(pdir.join("user-data")).unwrap();
        fs::write(
            pdir.join("profile.json"),
            format!(
                r#"{{"id":"{id}","name":"Shop","os":"mac","createdAt":"2024-01-01T00:00:00Z","config":{{"navigator.userAgent":"UA"}}}}"#
            ),
        )
        .unwrap();

        let db = Db::memory().unwrap();
        let r = import(&db, &d, None).unwrap().unwrap();
        assert_eq!((r.proxies, r.profiles, r.assignments), (2, 1, 1));
        assert_eq!(r.warnings.len(), 1);
        assert!(d.join("proxies.txt.bak").is_file() && d.join("state.json.bak").is_file() && pdir.join("profile.json.bak").is_file());
        assert!(pdir.join("user-data").is_dir());

        let px = SqliteProxies::new(db.clone()).list().await.unwrap();
        let p = SqliteProfiles::new(db.clone()).get(id).await.unwrap().unwrap();
        assert_eq!(p.proxy_id, Some(px[1].id));
        assert_eq!(p.machine["navigator.userAgent"], "UA");
        assert_eq!(p.os, Os::Mac);

        assert!(import(&db, &d, None).unwrap().is_none(), "second run is a no-op");

        // later drop-in: duplicates skipped, new line added, file renamed
        assert!(import_proxy_file(&db, &d).unwrap().is_none());
        fs::write(d.join("proxies.txt"), "1.2.3.4:8080:u:p\n9.9.9.9:1080\n").unwrap();
        let r = import_proxy_file(&db, &d).unwrap().unwrap();
        assert_eq!(r.proxies, 1);
        assert_eq!(SqliteProxies::new(db.clone()).list().await.unwrap().len(), 3);
        assert!(!d.join("proxies.txt").exists());
        let _ = fs::remove_dir_all(&d);
    }
}
