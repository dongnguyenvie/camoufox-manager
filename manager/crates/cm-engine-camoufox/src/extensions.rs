//! Firefox extensions (`.xpi`) for a persistent profile.
//!
//! [`sync`] makes `<user-data>/extensions/` hold exactly the given add-ons (plus anything the
//! user put there themselves). `launch` calls it before the browser opens, so the assignment of
//! the app is always what Firefox sees.
//!
//! How it loads: Firefox scans `<profile>/extensions/<addon-id>.xpi` at startup (a "sideload"
//! into the profile scope). Two things make that work for unsigned / user-uploaded add-ons:
//!   * the file name must be exactly the add-on id from the manifest
//!     (`browser_specific_settings.gecko.id` or `applications.gecko.id`), so an add-on without an
//!     id cannot be installed;
//!   * the helper's launch sets `extensions.autoDisableScopes = 0` (otherwise Firefox installs
//!     sideloaded add-ons as disabled and waits for a prompt that never comes),
//!     `extensions.enabledScopes = 15`, and `xpinstall.signatures.required = false` (Camoufox is
//!     not a release-signed build, so unsigned add-ons are accepted). Measured with Camoufox
//!     0.5.9: without these prefs the add-on is ignored. The engine passes `--extension-prefs`
//!     to the helper when at least one add-on is installed.
//!
//! What is ours is recorded in `<user-data>/extensions/.cm-managed.json`; only those files are
//! ever removed, so add-ons installed by hand inside the browser are left alone. Removing the
//! file makes Firefox uninstall the add-on at the next start.
//!
//! Limits: only a Manifest V2 content-script add-on was tested (Camoufox 0.5.9); add-ons are
//! installed at launch, so assignment changes need a restart of the profile; WebExtension APIs
//! Camoufox does not ship do nothing; an add-on without a Gecko id in its manifest is refused.

use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MANAGED: &str = ".cm-managed.json";

fn file_name(xpi: &Path) -> String {
    xpi.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Add-on id from the manifest of an `.xpi` (zip); an error when it cannot be used.
pub fn addon_id(xpi: &Path) -> Result<String> {
    let name = file_name(xpi);
    let file = fs::File::open(xpi).map_err(|e| anyhow!("{name}: not a valid extension ({e})"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| anyhow!("{name}: not a valid extension ({e})"))?;
    let mut raw = Vec::new();
    match zip.by_name("manifest.json") {
        Ok(mut f) => {
            f.read_to_end(&mut raw).map_err(|e| anyhow!("{name}: not a valid extension ({e})"))?;
        }
        Err(zip::result::ZipError::FileNotFound) => bail!("{name}: no manifest.json"),
        Err(e) => bail!("{name}: not a valid extension ({e})"),
    }
    let text = std::str::from_utf8(&raw).map_err(|e| anyhow!("{name}: not a valid extension ({e})"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let manifest: Value = serde_json::from_str(text).map_err(|e| anyhow!("{name}: not a valid extension ({e})"))?;
    for key in ["browser_specific_settings", "applications"] {
        if let Some(id) = manifest[key]["gecko"]["id"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
            if id.contains('/') || id.contains('\\') || id.starts_with('.') {
                bail!("{name}: unusable add-on id {id:?}");
            }
            return Ok(id.to_string());
        }
    }
    bail!("{name}: manifest has no browser_specific_settings.gecko.id")
}

fn same(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) else { return false };
    ma.len() == mb.len() && matches!((fs::read(a), fs::read(b)), (Ok(x), Ok(y)) if x == y)
}

/// Make `<user_data>/extensions/` hold `xpis` (managed copies). Returns (installed, removed) ids.
pub fn sync(user_data: &Path, xpis: &[PathBuf]) -> Result<(Vec<String>, Vec<String>)> {
    let ext_dir = user_data.join("extensions");
    let record = ext_dir.join(MANAGED);
    let managed: Vec<String> = fs::read(&record)
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<Value>>(&b).ok())
        .map(|v| v.into_iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut wanted: BTreeMap<String, &PathBuf> = BTreeMap::new();
    for x in xpis {
        wanted.insert(addon_id(x)?, x);
    }
    if wanted.is_empty() && managed.is_empty() {
        return Ok((vec![], vec![]));
    }
    fs::create_dir_all(&ext_dir)?;
    let mut removed = Vec::new();
    for id in managed {
        if !wanted.contains_key(&id) {
            match fs::remove_file(ext_dir.join(format!("{id}.xpi"))) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            removed.push(id);
        }
    }
    for (id, src) in &wanted {
        let dest = ext_dir.join(format!("{id}.xpi"));
        if !same(src, &dest) {
            // Keep the mtime when unchanged: Firefox re-reads the add-on on change.
            let tmp = ext_dir.join(format!("{id}.xpi.tmp"));
            fs::copy(src, &tmp)?;
            fs::rename(&tmp, &dest)?;
        }
    }
    let ids: Vec<String> = wanted.keys().cloned().collect(); // BTreeMap: already sorted
    let tmp = ext_dir.join(format!("{MANAGED}.tmp"));
    fs::write(&tmp, serde_json::to_vec(&ids)?)?;
    fs::rename(&tmp, &record)?;
    Ok((ids, removed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cm-ext-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn xpi(dir: &Path, file: &str, manifest: Option<&str>) -> PathBuf {
        let p = dir.join(file);
        let mut z = zip::ZipWriter::new(fs::File::create(&p).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        match manifest {
            Some(m) => {
                z.start_file("manifest.json", o).unwrap();
                z.write_all(m.as_bytes()).unwrap();
            }
            None => {
                z.start_file("other.txt", o).unwrap();
            }
        }
        z.finish().unwrap();
        p
    }

    #[test]
    fn addon_id_rules() {
        let d = tmp("id");
        let a = xpi(&d, "a.xpi", Some(r#"{"browser_specific_settings":{"gecko":{"id":" a@x "}}}"#));
        assert_eq!(addon_id(&a).unwrap(), "a@x");
        let b = xpi(&d, "b.xpi", Some("\u{feff}{\"applications\":{\"gecko\":{\"id\":\"b@x\"}}}"));
        assert_eq!(addon_id(&b).unwrap(), "b@x");
        let c = xpi(&d, "c.xpi", Some(r#"{"name":"x"}"#));
        assert_eq!(addon_id(&c).unwrap_err().to_string(), "c.xpi: manifest has no browser_specific_settings.gecko.id");
        let e = xpi(&d, "e.xpi", Some(r#"{"applications":{"gecko":{"id":"a/b"}}}"#));
        assert!(addon_id(&e).unwrap_err().to_string().contains("unusable add-on id"));
        let n = xpi(&d, "n.xpi", None);
        assert_eq!(addon_id(&n).unwrap_err().to_string(), "n.xpi: no manifest.json");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn sync_installs_keeps_mtime_and_removes_only_managed() {
        let d = tmp("sync");
        let ud = d.join("ud");
        let a = xpi(&d, "a.xpi", Some(r#"{"browser_specific_settings":{"gecko":{"id":"a@x"}}}"#));
        let b = xpi(&d, "b.xpi", Some(r#"{"browser_specific_settings":{"gecko":{"id":"b@x"}}}"#));
        assert_eq!(sync(&ud, &[]).unwrap(), (vec![], vec![]));
        assert!(!ud.join("extensions").exists());
        let (inst, rem) = sync(&ud, &[b.clone(), a.clone()]).unwrap();
        assert_eq!((inst, rem), (vec!["a@x".to_string(), "b@x".to_string()], vec![]));
        let ext = ud.join("extensions");
        fs::write(ext.join("mine@x.xpi"), b"user").unwrap();
        let m1 = fs::metadata(ext.join("a@x.xpi")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (inst, rem) = sync(&ud, &[a]).unwrap();
        assert_eq!((inst, rem), (vec!["a@x".to_string()], vec!["b@x".to_string()]));
        assert_eq!(fs::metadata(ext.join("a@x.xpi")).unwrap().modified().unwrap(), m1);
        assert!(!ext.join("b@x.xpi").exists());
        assert!(ext.join("mine@x.xpi").exists());
        assert_eq!(fs::read_to_string(ext.join(MANAGED)).unwrap(), r#"["a@x"]"#);
        let _ = fs::remove_dir_all(&d);
    }
}
