//! Snapshot format (phase 6). One zip = `snapshot.json` + `data/<relative path>` for every file of
//! the user-data dir. The same document is the device-sync snapshot (D9, phases 15-17): keep it
//! additive (new optional fields only) and bump [`SNAPSHOT_SCHEMA`] for breaking changes.

use crate::{
    CmError, Result,
    profile::{Os, ProfileSettings, valid_id},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const SNAPSHOT_SCHEMA: u32 = 1;
pub const MANIFEST_NAME: &str = "snapshot.json";
pub const DATA_PREFIX: &str = "data/";
/// Import limits (phase 15 may tighten): entries, total uncompressed bytes, manifest size.
pub const MAX_ENTRIES: usize = 20_000;
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: u64 = 32 * 1024 * 1024;

/// Proxy assignment as a label only: never credentials (the proxy library stays on this machine).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProxyLabel {
    pub name: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProfile {
    pub id: String,
    pub name: String,
    pub os: Os,
    #[serde(default)]
    pub notes: Option<String>,
    pub created_at: String,
    /// Pinned machine (opaque, engine-defined).
    pub machine: Value,
    #[serde(default)]
    pub settings: ProfileSettings,
    #[serde(default)]
    pub proxy: Option<ProxyLabel>,
}

/// One file of the user-data dir. `path` is relative to the data dir and `/`-separated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    /// Lowercase hex SHA-256 of the content.
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotManifest {
    pub schema: u32,
    /// Version of the app that wrote it.
    pub app_version: String,
    /// ISO 8601 UTC.
    pub created_at: String,
    /// Camoufox build when known.
    #[serde(default)]
    pub camoufox_build: Option<String>,
    /// Host OS (`std::env::consts::OS`) and CPU arch of the exporting machine.
    pub os: String,
    pub arch: String,
    pub profile: SnapshotProfile,
    pub files: Vec<FileEntry>,
}

fn invalid(msg: impl Into<String>) -> anyhow::Error {
    CmError::Invalid(msg.into()).into()
}

/// A relative path inside the data dir: `/`-separated, no empty / `.` / `..` component, no
/// absolute path, backslash, drive prefix or control character, bounded length.
pub fn check_rel_path(p: &str) -> Result<()> {
    let bad = || invalid(format!("unsafe snapshot path {p:?}"));
    if p.is_empty() || p.len() > 1024 {
        return Err(invalid("snapshot path is empty or too long"));
    }
    if p.starts_with('/') || p.contains('\\') || p.chars().any(char::is_control) {
        return Err(bad());
    }
    if p.split('/').any(|c| c.is_empty() || c == "." || c == ".." || c.len() > 255) {
        return Err(bad());
    }
    if p.split('/').next().is_some_and(|c| c.len() >= 2 && c.as_bytes()[1] == b':' && c.as_bytes()[0].is_ascii_alphabetic()) {
        return Err(bad());
    }
    Ok(())
}

pub fn is_hex_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

const EXCLUDED_FILES: &[&str] = &["lock", "app.lock", ".parentlock", "parent.lock", "SingletonLock", "SingletonCookie", "SingletonSocket"];
const EXCLUDED_DIRS: &[&str] = &[
    "cache2",
    "startupCache",
    "shader-cache",
    "GPUCache",
    "OfflineCache",
    "jumpListCache",
    "thumbnails",
    "crashes",
    "minidumps",
    "saved-telemetry-pings",
];

/// Lock files and caches are never exported or cloned (`name` = one path component).
pub fn is_excluded(name: &str, is_dir: bool) -> bool {
    if is_dir { EXCLUDED_DIRS.contains(&name) } else { EXCLUDED_FILES.contains(&name) }
}

impl SnapshotManifest {
    /// Structural validation of a manifest read from an untrusted archive.
    pub fn validate(&self) -> Result<()> {
        if self.schema != SNAPSHOT_SCHEMA {
            return Err(invalid(format!("unsupported snapshot schema {} (this app reads {SNAPSHOT_SCHEMA})", self.schema)));
        }
        let p = &self.profile;
        if !valid_id(&p.id) {
            return Err(invalid("snapshot profile id is invalid"));
        }
        if p.name.trim().is_empty() || p.name.chars().count() > 80 {
            return Err(invalid("snapshot profile name must be 1-80 characters"));
        }
        if !p.machine.is_object() {
            return Err(invalid("snapshot has no pinned machine"));
        }
        if self.files.len() > MAX_ENTRIES {
            return Err(invalid(format!("snapshot has more than {MAX_ENTRIES} files")));
        }
        let mut seen = HashSet::new();
        let mut total = 0u64;
        for f in &self.files {
            check_rel_path(&f.path)?;
            if !is_hex_sha256(&f.sha256) {
                return Err(invalid(format!("bad checksum for {}", f.path)));
            }
            // Case-insensitive: macOS volumes would merge two names that differ only by case.
            if !seen.insert(f.path.to_lowercase()) {
                return Err(invalid(format!("duplicate snapshot path {}", f.path)));
            }
            total = total.saturating_add(f.size);
        }
        if total > MAX_TOTAL_BYTES {
            return Err(invalid("snapshot is larger than 2 GiB"));
        }
        // A file may not also be a parent folder of another file.
        for f in &self.files {
            for (i, _) in f.path.match_indices('/') {
                if seen.contains(&f.path[..i].to_lowercase()) {
                    return Err(invalid(format!("snapshot path {} is both a file and a folder", &f.path[..i])));
                }
            }
        }
        Ok(())
    }
}

/// Expand `Shop {n}` for index `n`. The pattern must contain `{n}`.
pub fn expand_pattern(pattern: &str, n: u32) -> Result<String> {
    if !pattern.contains("{n}") {
        return Err(invalid("name pattern must contain {n}, e.g. Shop {n}"));
    }
    let name = pattern.replace("{n}", &n.to_string()).trim().to_string();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(invalid("profile name must be 1-80 characters"));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(files: Vec<(&str, u64)>) -> SnapshotManifest {
        SnapshotManifest {
            schema: SNAPSHOT_SCHEMA,
            app_version: "0".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            camoufox_build: None,
            os: "macos".into(),
            arch: "aarch64".into(),
            profile: SnapshotProfile {
                id: "aabbccddeeff001122334455".into(),
                name: "Shop".into(),
                os: Os::Mac,
                notes: None,
                created_at: String::new(),
                machine: serde_json::json!({ "a": 1 }),
                settings: Default::default(),
                proxy: None,
            },
            files: files.into_iter().map(|(p, size)| FileEntry { path: p.into(), size, sha256: "a".repeat(64) }).collect(),
        }
    }

    #[test]
    fn paths() {
        for ok in ["a", "a/b.sqlite", "x y/z"] {
            assert!(check_rel_path(ok).is_ok(), "{ok}");
        }
        for bad in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a/./b", "a\\b", "C:/x", "a/", "./a", "a\0b"] {
            assert!(check_rel_path(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn manifest_validation() {
        assert!(manifest(vec![("a", 1), ("d/b", 2)]).validate().is_ok());
        assert!(manifest(vec![("a", 1), ("A", 1)]).validate().is_err(), "case-insensitive duplicate");
        assert!(manifest(vec![("a", 1), ("a/b", 1)]).validate().is_err(), "file and folder");
        assert!(manifest(vec![("../a", 1)]).validate().is_err());
        assert!(manifest(vec![("a", MAX_TOTAL_BYTES + 1)]).validate().is_err());
        let mut m = manifest(vec![("a", 1)]);
        m.schema = 99;
        assert!(m.validate().is_err());
        let mut m = manifest(vec![("a", 1)]);
        m.files[0].sha256 = "xyz".into();
        assert!(m.validate().is_err());
        let mut m = manifest(vec![]);
        m.profile.id = "../x".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn exclusions_and_pattern() {
        assert!(is_excluded("lock", false) && is_excluded(".parentlock", false) && is_excluded("cache2", true));
        assert!(!is_excluded("cookies.sqlite", false) && !is_excluded("cache2", false));
        assert_eq!(expand_pattern("Shop {n}", 3).unwrap(), "Shop 3");
        assert!(expand_pattern("Shop", 1).is_err());
        assert!(expand_pattern(&"x".repeat(90), 1).is_err());
    }
}
