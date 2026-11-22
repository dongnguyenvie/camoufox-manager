//! Site data of a CLOSED profile, removed before launch when the profile does not keep site storage.
//!
//! Firefox's clear-on-shutdown prefs do not reliably clear localStorage, so [`clear`] deletes it from
//! `user-data` directly: `storage/default` (except extension storage, `moz-extension+++*`),
//! `storage/temporary`, `webappsstore.sqlite*` and `serviceworker.txt`. Cookies, `storage/permanent`
//! and anything outside `user-data` are never touched.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Extension storage lives in `storage/default` too; it must survive (extensions keep their data).
const EXTENSION_PREFIX: &str = "moz-extension+++";

/// Delete everything in `user_data` that holds site data. Returns the number of entries removed.
pub fn clear(user_data: &Path) -> io::Result<usize> {
    let mut removed = 0;
    let default = user_data.join("storage").join("default");
    for entry in read_dir_or_empty(&default)? {
        if entry.file_name().to_string_lossy().starts_with(EXTENSION_PREFIX) {
            continue;
        }
        remove_path(&entry.path())?;
        removed += 1;
    }
    let temporary = user_data.join("storage").join("temporary");
    for entry in read_dir_or_empty(&temporary)? {
        remove_path(&entry.path())?;
        removed += 1;
    }
    for entry in read_dir_or_empty(user_data)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("webappsstore.sqlite") || name == "serviceworker.txt" {
            remove_path(&entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn read_dir_or_empty(dir: &Path) -> io::Result<Vec<fs::DirEntry>> {
    match fs::read_dir(dir) {
        Ok(rd) => rd.collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

fn remove_path(p: &PathBuf) -> io::Result<()> {
    let meta = fs::symlink_metadata(p)?;
    if meta.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clears_site_data_and_keeps_the_rest() {
        let d = std::env::temp_dir().join(format!("cm-sitedata-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let ud = d.join("user-data");
        let touch = |p: PathBuf| {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, b"x").unwrap();
        };
        // Site storage: removed.
        touch(ud.join("storage/default/https+++example.com/ls/data.sqlite"));
        touch(ud.join("storage/default/http+++localhost/ls/data.sqlite"));
        // Temporary storage: contents removed, directory stays.
        touch(ud.join("storage/temporary/https+++example.com/idb/x.sqlite"));
        touch(ud.join("webappsstore.sqlite"));
        touch(ud.join("webappsstore.sqlite-wal"));
        touch(ud.join("serviceworker.txt"));
        // Kept: extension storage, permanent storage, cookies, anything else.
        touch(ud.join("storage/default/moz-extension+++abc/idb/y.sqlite"));
        touch(ud.join("storage/permanent/chrome/x.sqlite"));
        touch(ud.join("cookies.sqlite"));
        touch(ud.join("places.sqlite"));
        touch(d.join("outside.txt"));

        let n = clear(&ud).unwrap();
        assert_eq!(n, 6);
        assert!(!ud.join("storage/default/https+++example.com").exists());
        assert!(!ud.join("storage/default/http+++localhost").exists());
        assert!(ud.join("storage/temporary").is_dir());
        assert!(!ud.join("storage/temporary/https+++example.com").exists());
        assert!(!ud.join("webappsstore.sqlite").exists());
        assert!(!ud.join("webappsstore.sqlite-wal").exists());
        assert!(!ud.join("serviceworker.txt").exists());
        assert!(ud.join("storage/default/moz-extension+++abc/idb/y.sqlite").exists());
        assert!(ud.join("storage/permanent/chrome/x.sqlite").exists());
        assert!(ud.join("cookies.sqlite").exists());
        assert!(ud.join("places.sqlite").exists());
        assert!(d.join("outside.txt").exists());

        // Missing directories are fine (fresh profile).
        let empty = d.join("fresh");
        assert_eq!(clear(&empty).unwrap(), 0);
        let _ = fs::remove_dir_all(&d);
    }
}
