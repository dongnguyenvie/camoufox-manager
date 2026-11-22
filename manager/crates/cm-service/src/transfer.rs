//! Profile transfer (phase 6): export a CLOSED profile to one zip, import it, clone, bulk create.
//!
//! Zip layout (also the D9 device-sync snapshot):
//! ```text
//! snapshot.json        manifest: schema, app/Camoufox build, host OS + arch, profile record
//!                      (pinned machine + settings + proxy LABEL only), per-file size + SHA-256
//! data/<rel path>      the user-data dir without lock files and caches
//! ```
//! Everything is streamed (256 KiB buffers): no file or archive is ever held in memory.
//! Import never trusts the archive: manifest validated first, every entry checked against it
//! (names, types, sizes), content hashed while it is written into a staging dir.

use crate::{
    Manager,
    journal::{self, Crash, State},
    profiles::ProfileView,
    profiles::new_id,
    profiles::validate_settings,
};
use anyhow::{Context, Result, bail};
use cm_core::{
    CmError, NewProfile, Profile,
    profile::valid_id,
    snapshot::{
        DATA_PREFIX, FileEntry, MANIFEST_NAME, MAX_ENTRIES, MAX_MANIFEST_BYTES, MAX_TOTAL_BYTES, ProxyLabel, SNAPSHOT_SCHEMA,
        SnapshotManifest, SnapshotProfile, check_rel_path, is_excluded,
    },
    transfer::{BulkCreateRequest, CloneRequest, ImportIdMode},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

/// Set unix permission bits; a no-op on Windows (the per-user profile ACL applies).
pub(crate) fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}

/// `OpenOptions::mode` stand-in so the call sites stay identical on Windows.
#[cfg(not(unix))]
pub(crate) trait OpenOptionsExt {
    fn mode(&mut self, _mode: u32) -> &mut Self;
}
#[cfg(not(unix))]
impl OpenOptionsExt for fs::OpenOptions {
    fn mode(&mut self, _mode: u32) -> &mut Self {
        self
    }
}

fn nlink_is_one(md: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        md.nlink() == 1
    }
    #[cfg(not(unix))]
    {
        let _ = md;
        true
    }
}

const BUF: usize = 256 * 1024;
/// Most profiles a single bulk request may create.
pub const MAX_BULK: u32 = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Absolute path of the zip (inside `<data dir>/exports`).
    pub path: String,
    pub file_name: String,
    pub size: u64,
    pub files: usize,
    /// Entries left out because they are not plain files (symlinks, sockets, hard links).
    pub skipped: Vec<String>,
}

#[derive(Serialize)]
pub struct Warning {
    pub code: &'static str,
    pub detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub profile: ProfileView,
    /// The archived id was free and kept.
    pub kept_id: bool,
    pub files: usize,
    /// Proxy the profile had on the source machine (label only, not assigned here).
    pub proxy: Option<ProxyLabel>,
    pub warnings: Vec<Warning>,
}

#[derive(Serialize)]
pub struct BulkResult {
    pub created: Vec<ProfileView>,
    /// Set when creation stopped early; `created` still lists what was made.
    pub error: Option<String>,
}

fn invalid(msg: impl Into<String>) -> anyhow::Error {
    CmError::Invalid(msg.into()).into()
}

// ---------------------------------------------------------------------------------------------
// File-level building blocks (blocking; called inside spawn_blocking)
// ---------------------------------------------------------------------------------------------

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn sha256_file(p: &Path) -> Result<String> {
    let mut f = File::open(p).with_context(|| format!("cannot read {}", p.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; BUF];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

#[cfg(unix)]
fn mtime_ns(md: &fs::Metadata) -> i128 {
    i128::from(md.mtime()) * 1_000_000_000 + i128::from(md.mtime_nsec())
}

#[cfg(not(unix))]
fn mtime_ns(md: &fs::Metadata) -> i128 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| i128::try_from(d.as_nanos()).unwrap_or(0))
}

pub(crate) struct Found {
    pub rel: String,
    pub abs: PathBuf,
    pub size: u64,
    /// Modification time (ns since the epoch): the capture re-check compares it.
    pub mtime: i128,
}

/// Plain files of a data dir (sorted), minus lock files and caches. Symlinks, sockets, devices and
/// hard-linked files are skipped and named in the second list.
pub(crate) fn walk(root: &Path) -> Result<(Vec<Found>, Vec<String>)> {
    let (mut out, mut skipped) = (Vec::new(), Vec::new());
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let mut entries: Vec<_> =
            fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let Ok(name) = e.file_name().into_string() else {
                skipped.push(format!("{rel}{}", e.file_name().to_string_lossy()));
                continue;
            };
            let path_rel = format!("{rel}{name}");
            let md = fs::symlink_metadata(e.path())?;
            let ft = md.file_type();
            if ft.is_dir() {
                if !is_excluded(&name, true) {
                    stack.push((e.path(), format!("{path_rel}/")));
                }
            } else if is_excluded(&name, false) {
                continue;
            } else if ft.is_file() && nlink_is_one(&md) && check_rel_path(&path_rel).is_ok() {
                out.push(Found { rel: path_rel, abs: e.path(), size: md.len(), mtime: mtime_ns(&md) });
            } else {
                skipped.push(path_rel);
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok((out, skipped))
}

fn changed(f: &str) -> CmError {
    CmError::Conflict(format!("{f} changed while exporting; close the profile and retry"))
}

/// Capture re-check: the tree must be exactly what the inventory saw (same files, sizes, mtimes).
/// Anything created, removed or touched meanwhile means the profile was not closed: abort.
fn recheck_capture(data: &Path, before: &[Found]) -> Result<()> {
    if !data.is_dir() {
        return if before.is_empty() { Ok(()) } else { Err(changed("the data folder").into()) };
    }
    let (after, _) = walk(data)?;
    if after.len() != before.len() {
        bail!(changed("the file list"));
    }
    for (a, b) in after.iter().zip(before) {
        if a.rel != b.rel || a.size != b.size || a.mtime != b.mtime {
            bail!(changed(&b.rel));
        }
    }
    Ok(())
}

/// Build the archive: manifest first, then every file, hashing again while copying and failing if
/// a file changed since the inventory (the profile must stay closed).
fn write_zip(data: &Path, manifest: SnapshotManifest, out: &Path) -> Result<(usize, Vec<String>)> {
    write_zip_with(data, manifest, out, CompressionMethod::Deflated)
}

fn write_zip_with(data: &Path, mut manifest: SnapshotManifest, out: &Path, method: CompressionMethod) -> Result<(usize, Vec<String>)> {
    let (found, skipped) = if data.is_dir() { walk(data)? } else { (Vec::new(), Vec::new()) };
    manifest.files =
        found.iter().map(|f| Ok(FileEntry { path: f.rel.clone(), size: f.size, sha256: sha256_file(&f.abs)? })).collect::<Result<_>>()?;
    manifest.seal();
    manifest.validate()?;

    let part = out.with_extension("zip.part");
    let res = (|| -> Result<()> {
        let file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&part)?;
        let mut zip = ZipWriter::new(BufWriter::with_capacity(BUF, file));
        let opts = SimpleFileOptions::default().compression_method(method).unix_permissions(0o600);
        zip.start_file(MANIFEST_NAME, opts)?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
        let mut buf = vec![0u8; BUF];
        for (f, entry) in found.iter().zip(&manifest.files) {
            zip.start_file(format!("{DATA_PREFIX}{}", f.rel), opts.large_file(f.size >= u32::MAX as u64))?;
            let mut src = File::open(&f.abs)?;
            let (mut h, mut n_total) = (Sha256::new(), 0u64);
            loop {
                let n = src.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
                zip.write_all(&buf[..n])?;
                n_total += n as u64;
            }
            if n_total != entry.size || hex(&h.finalize()) != entry.sha256 {
                bail!(changed(&f.rel));
            }
        }
        zip.finish()?.flush()?;
        // Capture re-check: the tree must be exactly what the inventory saw (same files, sizes,
        // mtimes). Anything created, removed or touched meanwhile means the profile was not closed.
        recheck_capture(data, &found)?;
        Ok(())
    })();
    if let Err(e) = res {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    fs::rename(&part, out)?;
    Ok((found.len(), skipped))
}

fn le16(b: &[u8], at: usize) -> usize {
    u16::from_le_bytes([b[at], b[at + 1]]) as usize
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Refuse ZIP64 in any form (EOCD64 record/locator, saturated header fields, 0x0001 extra
/// fields): snapshots stay under 2 GiB and 20k entries, so a legitimate archive never needs it, and
/// size/offset fields that live elsewhere are a classic way to make two parsers disagree. Parses
/// the end-of-central-directory and the central directory itself, independent of the zip crate.
fn check_no_zip64(path: &Path) -> Result<()> {
    let bad = |what: &str| invalid(format!("not a valid snapshot archive: {what}"));
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let tail_len = len.min(22 + 65_535 + 20);
    f.seek(SeekFrom::Start(len - tail_len))?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail)?;
    let eocd = tail.windows(4).rposition(|w| w == b"PK\x05\x06").ok_or_else(|| bad("no end of central directory"))?;
    if eocd + 22 > tail.len() {
        bail!(bad("truncated end of central directory"));
    }
    if eocd >= 20 && &tail[eocd - 20..eocd - 16] == b"PK\x06\x07" {
        bail!(bad("ZIP64 is not accepted"));
    }
    let (entries, cd_size, cd_off) = (le16(&tail, eocd + 10), le32(&tail, eocd + 12), le32(&tail, eocd + 16));
    if entries == 0xFFFF || cd_size == u32::MAX || cd_off == u32::MAX {
        bail!(bad("ZIP64 is not accepted"));
    }
    if entries > MAX_ENTRIES + 1 {
        bail!(invalid(format!("archive has more than {MAX_ENTRIES} entries")));
    }
    if u64::from(cd_off) + u64::from(cd_size) > len || cd_size > 64 * 1024 * 1024 {
        bail!(bad("central directory out of range"));
    }
    f.seek(SeekFrom::Start(u64::from(cd_off)))?;
    let mut cd = vec![0u8; cd_size as usize];
    f.read_exact(&mut cd)?;
    let mut at = 0usize;
    for _ in 0..entries {
        if at + 46 > cd.len() || &cd[at..at + 4] != b"PK\x01\x02" {
            bail!(bad("damaged central directory"));
        }
        let (name, extra, comment) = (le16(&cd, at + 28), le16(&cd, at + 30), le16(&cd, at + 32));
        // compressed size, size, local header offset
        if le32(&cd, at + 20) == u32::MAX || le32(&cd, at + 24) == u32::MAX || le32(&cd, at + 42) == u32::MAX {
            bail!(bad("ZIP64 is not accepted"));
        }
        let (e0, e1) = (at + 46 + name, at + 46 + name + extra);
        if e1 + comment > cd.len() {
            bail!(bad("damaged central directory"));
        }
        let mut x = e0;
        while x + 4 <= e1 {
            let (id, sz) = (le16(&cd, x), le16(&cd, x + 2));
            if id == 0x0001 {
                bail!(bad("ZIP64 is not accepted"));
            }
            x += 4 + sz;
        }
        at = e1 + comment;
    }
    Ok(())
}

/// Open an untrusted archive and return its validated manifest. Every entry must be a plain
/// file/dir named `snapshot.json` or `data/<safe path>`, listed in the manifest exactly once.
pub(crate) fn read_manifest(path: &Path) -> Result<SnapshotManifest> {
    check_no_zip64(path)?;
    let mut zip = ZipArchive::new(BufReader::new(File::open(path)?)).map_err(|e| invalid(format!("not a valid zip archive: {e}")))?;
    // Files + snapshot.json; a directory entry counts too.
    if zip.len() > MAX_ENTRIES + 1 {
        bail!(invalid(format!("archive has more than {MAX_ENTRIES} entries")));
    }
    let manifest: SnapshotManifest = {
        let m = zip.by_name(MANIFEST_NAME).map_err(|_| invalid("archive has no snapshot.json"))?;
        if m.size() > MAX_MANIFEST_BYTES {
            bail!(invalid("snapshot.json is too large"));
        }
        serde_json::from_reader(m.take(MAX_MANIFEST_BYTES)).map_err(|e| invalid(format!("snapshot.json is not valid: {e}")))?
    };
    manifest.validate()?;
    let sizes: HashMap<&str, u64> = manifest.files.iter().map(|f| (f.path.as_str(), f.size)).collect();
    let (mut seen, mut names) = (HashSet::new(), HashSet::new());
    let mut manifests = 0;
    for i in 0..zip.len() {
        let f = zip.by_index_raw(i).map_err(|e| invalid(format!("bad archive entry: {e}")))?;
        let name = f.name().to_string();
        if !names.insert(name.clone()) {
            bail!(invalid(format!("duplicate archive entry {name:?}")));
        }
        if f.encrypted() || !matches!(f.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            bail!(invalid(format!("unsupported entry {name:?} (encrypted or unusual compression)")));
        }
        // Unix type bits: only regular files and directories. Symlinks, hard-link markers,
        // devices, sockets and FIFOs are refused outright.
        if let Some(mode) = f.unix_mode() {
            let kind = mode & 0o170000;
            if f.is_dir() {
                if kind != 0 && kind != 0o040000 {
                    bail!(invalid(format!("entry {name:?} is not a plain directory")));
                }
            } else if kind != 0 && kind != 0o100000 {
                bail!(invalid(format!("entry {name:?} is a link or special file; refusing it")));
            }
        }
        if f.is_symlink() {
            bail!(invalid(format!("entry {name:?} is a symlink; refusing it")));
        }
        if name == MANIFEST_NAME {
            manifests += 1;
            continue;
        }
        let Some(rel) = name.strip_prefix(DATA_PREFIX) else { bail!(invalid(format!("unexpected archive entry {name:?}"))) };
        if f.is_dir() {
            let rel = rel.trim_end_matches('/');
            if !rel.is_empty() {
                check_rel_path(rel)?;
            }
            continue;
        }
        check_rel_path(rel)?;
        match sizes.get(rel) {
            Some(&sz) if sz == f.size() => {
                seen.insert(rel.to_string());
            }
            Some(_) => bail!(invalid(format!("size of {rel} differs from the manifest"))),
            None => bail!(invalid(format!("{rel} is not listed in the manifest"))),
        }
    }
    if manifests != 1 {
        bail!(invalid("archive must contain exactly one snapshot.json"));
    }
    if let Some(missing) = manifest.files.iter().find(|f| !seen.contains(&f.path)) {
        bail!(invalid(format!("{} is listed in the manifest but missing from the archive", missing.path)));
    }
    Ok(manifest)
}

/// Extract `data/*` into the fresh directory `dest`, hashing while writing. Any mismatch aborts.
fn extract(path: &Path, manifest: &SnapshotManifest, dest: &Path) -> Result<()> {
    let mut zip = ZipArchive::new(BufReader::new(File::open(path)?))?;
    fs::create_dir_all(dest)?;
    set_mode(dest, 0o700)?;
    let mut buf = vec![0u8; BUF];
    let mut total = 0u64;
    for entry in &manifest.files {
        // `entry.path` passed `check_rel_path`: components are plain names, nothing escapes `dest`.
        let target = dest.join(&entry.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut src =
            zip.by_name(&format!("{DATA_PREFIX}{}", entry.path)).map_err(|_| invalid(format!("{} missing from archive", entry.path)))?;
        let mut out = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&target)?;
        let (mut h, mut n_total) = (Sha256::new(), 0u64);
        loop {
            let n = (&mut src).take(entry.size + 1 - n_total).read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            n_total += n as u64;
            if n_total > entry.size {
                bail!(invalid(format!("{} is larger than the manifest says", entry.path)));
            }
        }
        total += n_total;
        if total > MAX_TOTAL_BYTES {
            bail!(invalid("archive is larger than 2 GiB"));
        }
        if n_total != entry.size {
            bail!(invalid(format!("{} is shorter than the manifest says", entry.path)));
        }
        if hex(&h.finalize()) != entry.sha256 {
            bail!(invalid(format!("checksum mismatch for {}: the archive was modified or damaged", entry.path)));
        }
    }
    Ok(())
}

/// Copy a data dir (clone): same exclusions as export, plain files only.
fn copy_tree(src: &Path, dst: &Path) -> Result<usize> {
    fs::create_dir_all(dst)?;
    set_mode(dst, 0o700)?;
    if !src.is_dir() {
        return Ok(0);
    }
    let (found, _) = walk(src)?;
    for f in &found {
        let target = dst.join(&f.rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut input = File::open(&f.abs)?;
        let mut out = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&target)?;
        std::io::copy(&mut input, &mut out)?;
    }
    Ok(found.len())
}

/// Screen of the pinned machine as (w, h), for the "window fits the screen" check.
fn pinned_screen(m: &Manager, p: &Profile) -> Option<(u32, u32)> {
    let s = m.d.engine.summarize(&p.machine).resolution?;
    let (w, h) = s.to_ascii_lowercase().split_once('x').map(|(w, h)| (w.trim().to_string(), h.trim().to_string()))?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn file_stem(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let s = s.trim_matches('-');
    if s.is_empty() { "profile".into() } else { s.chars().take(40).collect() }
}

// ---------------------------------------------------------------------------------------------
// Use-cases
// ---------------------------------------------------------------------------------------------

impl Manager {
    /// Platform / Camoufox build differences between the exporting machine and this one.
    async fn import_warnings(&self, manifest: &SnapshotManifest) -> Vec<Warning> {
        let mut warnings = Vec::new();
        if manifest.os != std::env::consts::OS || manifest.arch != std::env::consts::ARCH {
            warnings.push(Warning {
                code: "platform",
                detail: format!(
                    "exported on {}/{}, this machine is {}/{}",
                    manifest.os,
                    manifest.arch,
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
            });
        }
        if let (Some(theirs), Some(ours)) = (&manifest.camoufox_build, self.d.engine.status().await.version)
            && *theirs != ours
        {
            warnings.push(Warning { code: "build", detail: format!("exported with Camoufox {theirs}, installed is {ours}") });
        }
        warnings
    }

    /// Fail unless the profile is provably closed (not started by this app, engine does not see it).
    pub(crate) async fn ensure_closed(&self, id: &str) -> Result<()> {
        if self.running.lock().await.contains_key(id) || self.d.engine.alive(id).await == Some(true) {
            bail!(CmError::Conflict("stop the profile first; it must be closed".into()));
        }
        Ok(())
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.cfg.data_dir.join("exports")
    }

    fn tmp_dir(&self) -> PathBuf {
        self.cfg.data_dir.join("tmp")
    }

    /// Scratch file path for an upload in progress (caller deletes it).
    pub fn upload_path(&self) -> Result<PathBuf> {
        let dir = self.tmp_dir();
        fs::create_dir_all(&dir)?;
        set_mode(&dir, 0o700)?;
        Ok(dir.join(format!("import-{}.zip", new_id()?)))
    }

    /// Existing export by file name (no path separators), for download.
    pub fn export_file(&self, name: &str) -> Result<PathBuf> {
        let ok = name.ends_with(".zip") && !name.contains(['/', '\\']) && !name.starts_with('.') && !name.chars().any(char::is_control);
        let p = self.exports_dir().join(name);
        if !ok || !p.is_file() {
            bail!(CmError::NotFound(format!("export {name}")));
        }
        Ok(p)
    }

    /// Export a closed profile to `<data dir>/exports/<name>-<id>.zip` (mode 0600: it holds cookies).
    pub async fn export_profile(&self, id: &str) -> Result<ExportResult> {
        let p = self.profile(id).await?;
        self.ensure_closed(id).await?;
        self.journal_check_start(id).await?; // an unresolved import means the data is in between
        // Closed proof BEFORE the capture; the guard (app.lock flock) is held until the end so the
        // profile cannot start mid-export.
        let _closed = self.d.engine.hold_closed(id).await?;
        let proxy = match p.proxy_id {
            Some(pid) => self.d.proxies.get(pid).await?.map(|x| ProxyLabel {
                name: x.name.clone(),
                scheme: x.scheme.as_str().into(),
                host: x.host.clone(),
                port: x.port,
            }),
            None => None,
        };
        let manifest = SnapshotManifest {
            schema: SNAPSHOT_SCHEMA,
            app_version: env!("CARGO_PKG_VERSION").into(),
            created_at: self.d.clock.now_iso(),
            camoufox_build: self.d.engine.status().await.version,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            profile: SnapshotProfile {
                id: p.id.clone(),
                name: p.name.clone(),
                os: p.os,
                notes: p.notes.clone(),
                created_at: p.created_at.clone(),
                machine: p.machine.clone(),
                settings: p.settings.clone(),
                proxy,
            },
            files: Vec::new(),
            integrity: None,
        };
        let dir = self.exports_dir();
        fs::create_dir_all(&dir)?;
        set_mode(&dir, 0o700)?;
        let file_name = format!("{}-{}.zip", file_stem(&p.name), p.id);
        let out = dir.join(&file_name);
        let data = self.d.engine.data_dir(id);
        let o2 = out.clone();
        let (files, skipped) = tokio::task::spawn_blocking(move || write_zip(&data, manifest, &o2)).await??;
        // Closed-proof again after the capture: a profile started meanwhile makes the copy unreliable.
        if let Err(e) = self.ensure_closed(id).await {
            let _ = fs::remove_file(&out);
            return Err(e.context("the profile was started during the export; nothing was written"));
        }
        let size = fs::metadata(&out)?.len();
        Ok(ExportResult { path: out.display().to_string(), file_name, size, files, skipped })
    }

    /// Import a snapshot zip as a new profile (`mode` = `New`) or under its archived id when free.
    /// `name` overrides the archived name.
    pub async fn import_snapshot(&self, zip: &Path, mode: ImportIdMode, name: Option<String>) -> Result<ImportResult> {
        if mode == ImportIdMode::Replace {
            bail!(invalid("replace needs the id of the profile to overwrite"));
        }
        let z = zip.to_path_buf();
        let manifest = tokio::task::spawn_blocking(move || read_manifest(&z)).await??;
        let sp = manifest.profile.clone();
        let name = match name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            Some(n) if n.chars().count() > 80 => bail!(invalid("profile name must be 1-80 characters")),
            Some(n) => n,
            None => sp.name.clone(),
        };
        let kept = mode == ImportIdMode::KeepIfFree
            && valid_id(&sp.id)
            && self.d.profiles.get(&sp.id).await?.is_none()
            && !self.d.engine.data_dir(&sp.id).exists();
        let id = if kept { sp.id.clone() } else { new_id()? };

        let now = self.d.clock.now_iso();
        let mut p = Profile {
            id: id.clone(),
            name,
            os: sp.os,
            notes: sp.notes.clone(),
            created_at: now.clone(),
            updated_at: now,
            last_activity: None,
            proxy_id: None,
            machine: sp.machine.clone(),
            settings: Default::default(),
        };
        p.settings = validate_settings(sp.settings.clone(), pinned_screen(self, &p))?;

        let data = self.d.engine.data_dir(&id);
        let staging = data.with_file_name("user-data.importing");
        let profile_dir = data.parent().map(Path::to_path_buf).context("bad data dir")?;
        let (z, m, st) = (zip.to_path_buf(), manifest.clone(), staging.clone());
        let extracted = tokio::task::spawn_blocking(move || extract(&z, &m, &st)).await?;
        let cleanup = |dir: &Path| {
            let _ = fs::remove_dir_all(dir);
        };
        if let Err(e) = extracted {
            cleanup(&profile_dir);
            return Err(e);
        }
        if let Err(e) = fs::rename(&staging, &data) {
            cleanup(&profile_dir);
            return Err(e.into());
        }
        if let Err(e) = self.d.profiles.insert(&p).await {
            cleanup(&profile_dir);
            return Err(e);
        }

        let warnings = self.import_warnings(&manifest).await;
        Ok(ImportResult { profile: self.profile_view(&id).await?, kept_id: kept, files: manifest.files.len(), proxy: sp.proxy, warnings })
    }

    /// Import in `replace` mode: overwrite the data, pinned machine, settings and notes of the
    /// existing profile `target` with the snapshot's. The local id, name (unless `name` is given),
    /// group/tags, schedules and proxy assignment stay. Crash-safe, see [`crate::journal`].
    pub async fn replace_from_snapshot(&self, zip: &Path, target: &str, name: Option<String>) -> Result<ImportResult> {
        self.replace_inner(zip, target, name, None).await
    }

    pub(crate) async fn replace_inner(&self, zip: &Path, target: &str, name: Option<String>, crash: Option<Crash>) -> Result<ImportResult> {
        let z = zip.to_path_buf();
        let manifest = tokio::task::spawn_blocking(move || read_manifest(&z)).await??;
        let current = self.profile(target).await?;
        self.journal_check_start(target).await?;
        self.ensure_closed(target).await?;
        // Hold the closed proof for the whole apply: the profile cannot start between staging and swap.
        let _closed = self.d.engine.hold_closed(target).await?;

        let sp = &manifest.profile;
        let now = self.d.clock.now_iso();
        let name = match name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            Some(n) if n.chars().count() > 80 => bail!(invalid("profile name must be 1-80 characters")),
            Some(n) => n,
            None => current.name.clone(),
        };
        let mut record =
            Profile { name, os: sp.os, notes: sp.notes.clone(), updated_at: now.clone(), machine: sp.machine.clone(), ..current.clone() };
        record.settings = validate_settings(sp.settings.clone(), pinned_screen(self, &record))?;

        let data = self.d.engine.data_dir(target);
        let profile_dir = data.parent().map(Path::to_path_buf).context("bad data dir")?;
        fs::create_dir_all(&profile_dir)?;
        let jid = new_id()?;
        let staging = profile_dir.join(format!("user-data.staging-{jid}"));
        let backup = profile_dir.join(format!("user-data.backup-{jid}"));
        self.journal()?.begin(&jid, target, &staging, &backup, &now)?;

        let sim = |c: Crash| crash == Some(c);
        let crashed = || anyhow::anyhow!("simulated crash");
        // Phases up to the swap never touch live data: on failure drop the staging dir and abort.
        let staged = async {
            let (z, m, st) = (zip.to_path_buf(), manifest.clone(), staging.clone());
            if sim(Crash::DuringPreparing) {
                // a half-written staging dir, as left by a dead process
                tokio::task::spawn_blocking(move || -> Result<()> {
                    fs::create_dir_all(&st)?;
                    fs::write(st.join("partial"), b"x")?;
                    Ok(())
                })
                .await??;
                return Err(crashed());
            }
            tokio::task::spawn_blocking(move || extract(&z, &m, &st)).await??;
            let (st, inv) = (staging.clone(), manifest.files.clone());
            tokio::task::spawn_blocking(move || journal::verify_tree(&st, &inv)).await??;
            self.journal()?.prepared(&jid, &manifest.files, &record, &now)?;
            if sim(Crash::AfterPrepared) {
                return Err(crashed());
            }
            Ok(())
        }
        .await;
        if let Err(e) = staged {
            if crash.is_none() {
                let _ = fs::remove_dir_all(&staging);
                let _ = self.journal()?.set_state(&jid, State::Aborted, Some(&format!("{e:#}")), &now);
            }
            return Err(e);
        }

        // Publishing: data -> backup, staging -> data. `resolve` can finish or undo it from any point.
        self.journal()?.set_state(&jid, State::Publishing, None, &now)?;
        let swap = (|| -> Result<()> {
            if sim(Crash::BeforeSwap) {
                return Err(crashed());
            }
            if data.exists() {
                fs::rename(&data, &backup).context("cannot move the current data to the backup")?;
            }
            if sim(Crash::BetweenRenames) {
                return Err(crashed());
            }
            fs::rename(&staging, &data).context("cannot publish the staged data")?;
            if sim(Crash::AfterSwap) {
                return Err(crashed());
            }
            Ok(())
        })();
        if let Err(e) = swap {
            if crash.is_some() {
                return Err(e); // leave everything as a crash would
            }
            // Real failure: undo to a consistent state right away.
            let entry = self.journal()?.get(&jid)?.context("journal row vanished")?;
            let _ = journal::resolve(&entry, &data);
            let _ = self.journal()?.set_state(&jid, State::Aborted, Some(&format!("{e:#}")), &now);
            return Err(e);
        }
        // Past the swap the new data is live: the record update must not leave the journal open.
        let rec = self.journal()?.get(&jid)?.and_then(|e| e.record).context("journal lost the profile record")?;
        let updated = self.d.profiles.update(&rec).await;
        self.journal()?.set_state(&jid, State::Complete, updated.as_ref().err().map(|e| format!("{e:#}")).as_deref(), &now)?;
        journal::prune_backups(&profile_dir, &backup);
        updated?;

        let warnings = self.import_warnings(&manifest).await;
        Ok(ImportResult {
            profile: self.profile_view(target).await?,
            kept_id: true,
            files: manifest.files.len(),
            proxy: manifest.profile.proxy.clone(),
            warnings,
        })
    }

    /// Clone a closed profile: new id, copied settings and user data (no lock files/caches),
    /// same proxy assignment. `new_machine` pins a fresh machine of the same OS instead of copying it.
    pub async fn clone_profile(&self, id: &str, req: CloneRequest) -> Result<ProfileView> {
        let src = self.profile(id).await?;
        self.ensure_closed(id).await?;
        self.journal_check_start(id).await?;
        let _closed = self.d.engine.hold_closed(id).await?;
        let name = req.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 80 {
            bail!(invalid("profile name must be 1-80 characters"));
        }
        let new = new_id()?;
        let now = self.d.clock.now_iso();
        let mut p = Profile { id: new.clone(), name, created_at: now.clone(), updated_at: now, last_activity: None, ..src.clone() };
        if req.new_machine {
            p.machine = self.d.engine.generate(src.os).await?;
            // Overrides that fitted the old screen may not fit the new one.
            if validate_settings(p.settings.clone(), pinned_screen(self, &p)).is_err() {
                p.settings.screen = None;
                p.settings.window = None;
            }
        }
        let data = self.d.engine.data_dir(&new);
        let profile_dir = data.parent().map(Path::to_path_buf).context("bad data dir")?;
        let staging = data.with_file_name("user-data.importing");
        let from = self.d.engine.data_dir(id);
        let st = staging.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || copy_tree(&from, &st)).await? {
            let _ = fs::remove_dir_all(&profile_dir);
            return Err(e);
        }
        if let Err(e) = self.ensure_closed(id).await {
            let _ = fs::remove_dir_all(&profile_dir);
            return Err(e.context("the profile was started during the clone"));
        }
        if let Err(e) = fs::rename(&staging, &data) {
            let _ = fs::remove_dir_all(&profile_dir);
            return Err(e.into());
        }
        if let Err(e) = self.d.profiles.insert(&p).await {
            let _ = fs::remove_dir_all(&profile_dir);
            return Err(e);
        }
        self.profile_view(&new).await
    }

    /// Create `count` profiles named from `pattern` (`Shop {n}` → `Shop 1`, `Shop 2`, …).
    pub async fn bulk_create(&self, req: BulkCreateRequest) -> Result<BulkResult> {
        if req.count == 0 || req.count > MAX_BULK {
            bail!(invalid(format!("count must be 1-{MAX_BULK}")));
        }
        let start = req.start.unwrap_or(1);
        let end = start.checked_add(req.count).context("number out of range")?;
        // Validate every name before creating anything.
        let names: Vec<String> = (start..end).map(|n| cm_core::snapshot::expand_pattern(&req.pattern, n)).collect::<Result<_>>()?;
        let mut created = Vec::new();
        for name in names {
            match self.create_profile(NewProfile { name, os: req.os.code().into() }).await {
                Ok(v) => created.push(v),
                Err(e) => return Ok(BulkResult { created, error: Some(format!("{e:#}")) }),
            }
        }
        Ok(BulkResult { created, error: None })
    }
}

/// Test hook: build a snapshot zip from a data dir (other modules' tests).
#[cfg(test)]
pub(crate) fn write_zip_for_tests(data: &Path, manifest: SnapshotManifest, out: &Path) -> Result<()> {
    write_zip_with(data, manifest, out, CompressionMethod::Stored).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::{Os, profile::ProfileSettings};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("cm-transfer-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn base_manifest() -> SnapshotManifest {
        SnapshotManifest {
            schema: SNAPSHOT_SCHEMA,
            app_version: "t".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            camoufox_build: Some("1".into()),
            os: "macos".into(),
            arch: "aarch64".into(),
            profile: SnapshotProfile {
                id: "aabbccddeeff001122334455".into(),
                name: "Shop".into(),
                os: Os::Mac,
                notes: None,
                created_at: String::new(),
                machine: serde_json::json!({ "a": 1 }),
                settings: ProfileSettings::default(),
                proxy: None,
            },
            files: vec![],
            integrity: None,
        }
    }

    fn sample_data(root: &Path) {
        fs::create_dir_all(root.join("cache2/entries")).unwrap();
        fs::create_dir_all(root.join("storage/default")).unwrap();
        fs::write(root.join("cookies.sqlite"), b"cookie-bytes").unwrap();
        fs::write(root.join("storage/default/ls.sqlite"), vec![7u8; 700_000]).unwrap();
        fs::write(root.join("cache2/entries/x"), b"cache").unwrap();
        fs::write(root.join(".parentlock"), b"").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", root.join("lock")).unwrap();
            std::os::unix::fs::symlink("/etc/hosts", root.join("evil-link")).unwrap();
        }
    }

    #[test]
    fn export_then_import_roundtrip() {
        let t = tmp();
        let data = t.join("data");
        sample_data(&data);
        let zip = t.join("out.zip");
        let (n, skipped) = write_zip(&data, base_manifest(), &zip).unwrap();
        assert_eq!(n, 2, "lock files and caches excluded");
        assert_eq!(skipped, vec!["evil-link".to_string()]);
        let m = read_manifest(&zip).unwrap();
        assert_eq!(m.files.len(), 2);
        assert!(m.files.iter().all(|f| f.sha256.len() == 64));
        let dest = t.join("dest");
        extract(&zip, &m, &dest).unwrap();
        assert_eq!(fs::read(dest.join("cookies.sqlite")).unwrap(), b"cookie-bytes");
        assert!(!dest.join("cache2").exists() && !dest.join(".parentlock").exists());
        // the clone helper follows the same rules
        let c = t.join("clone");
        assert_eq!(copy_tree(&data, &c).unwrap(), 2);
        assert!(c.join("storage/default/ls.sqlite").is_file() && !c.join("evil-link").exists());
        let _ = fs::remove_dir_all(&t);
    }

    /// Build a zip by hand: (name, bytes, unix mode). The mode is written straight into the central
    /// directory (the zip crate would normalise it to a regular file).
    fn hand_zip(path: &Path, manifest: &SnapshotManifest, entries: &[(&str, &[u8], u32)]) {
        let mut z = ZipWriter::new(File::create(path).unwrap());
        let o = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        z.start_file(MANIFEST_NAME, o).unwrap();
        z.write_all(&serde_json::to_vec(manifest).unwrap()).unwrap();
        for (n, b, _) in entries {
            z.start_file(*n, o).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
        let mut bytes = fs::read(path).unwrap();
        let mut idx = 0;
        let mut at = 0;
        while let Some(i) = bytes[at..].windows(4).position(|w| w == b"PK\x01\x02") {
            let h = at + i;
            if idx > 0 {
                let mode = entries[idx - 1].2;
                bytes[h + 5] = 3; // made by: Unix
                bytes[h + 38..h + 42].copy_from_slice(&(mode << 16).to_le_bytes());
            }
            idx += 1;
            at = h + 4;
        }
        fs::write(path, bytes).unwrap();
    }

    fn entry(path: &str, bytes: &[u8]) -> FileEntry {
        FileEntry { path: path.into(), size: bytes.len() as u64, sha256: hex(&Sha256::digest(bytes)) }
    }

    #[test]
    fn rejects_hostile_archives() {
        let t = tmp();
        let z = t.join("a.zip");
        let mut m = base_manifest();
        m.files = vec![entry("a.txt", b"hello")];
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        assert!(read_manifest(&z).is_ok());

        // tampered content: sizes agree, hash does not
        hand_zip(&z, &m, &[("data/a.txt", b"HELLO", 0o100644)]);
        let mm = read_manifest(&z).unwrap();
        let err = extract(&z, &mm, &t.join("d1")).unwrap_err().to_string();
        assert!(err.contains("checksum mismatch"), "{err}");

        // size differs from the manifest
        hand_zip(&z, &m, &[("data/a.txt", b"hello!", 0o100644)]);
        assert!(read_manifest(&z).is_err());
        // symlink entry (mode 0o120000)
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o120777)]);
        assert!(read_manifest(&z).is_err());
        // special file (fifo)
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o010644)]);
        assert!(read_manifest(&z).is_err());
        // zip-slip and absolute names, in the archive...
        for bad in ["data/../evil", "/etc/evil", "data//x", "evil.txt", "data/a\\b"] {
            hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644), (bad, b"x", 0o100644)]);
            assert!(read_manifest(&z).is_err(), "{bad}");
        }
        // ...and in the manifest
        let mut slip = base_manifest();
        slip.files = vec![entry("../evil", b"x")];
        hand_zip(&z, &slip, &[("data/../evil", b"x", 0o100644)]);
        assert!(read_manifest(&z).is_err());
        // unlisted extra file / missing file
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644), ("data/extra", b"x", 0o100644)]);
        assert!(read_manifest(&z).is_err());
        hand_zip(&z, &m, &[]);
        assert!(read_manifest(&z).is_err());
        // not a zip at all
        fs::write(&z, b"hello").unwrap();
        assert!(read_manifest(&z).is_err());
        let _ = fs::remove_dir_all(&t);
    }

    /// Patch the central-directory header of entry `idx` (0 = snapshot.json) in a finished zip.
    fn patch_central(path: &Path, idx: usize, f: impl Fn(&mut [u8])) {
        let mut bytes = fs::read(path).unwrap();
        let (mut at, mut n) = (0, 0);
        while let Some(i) = bytes[at..].windows(4).position(|w| w == b"PK\x01\x02") {
            let h = at + i;
            if n == idx {
                f(&mut bytes[h..h + 46]);
            }
            n += 1;
            at = h + 4;
        }
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn limits_entries_zip64_compression_encryption() {
        let t = tmp();
        let z = t.join("a.zip");
        let mut m = base_manifest();
        m.files = vec![entry("a.txt", b"hello")];
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        assert!(read_manifest(&z).is_ok());

        // more than 20k entries
        let mut w = ZipWriter::new(File::create(&z).unwrap());
        let o = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        w.start_file(MANIFEST_NAME, o).unwrap();
        w.write_all(&serde_json::to_vec(&base_manifest()).unwrap()).unwrap();
        for i in 0..MAX_ENTRIES + 1 {
            w.start_file(format!("data/f{i}"), o).unwrap();
        }
        w.finish().unwrap();
        let e = read_manifest(&z).unwrap_err().to_string();
        assert!(e.contains("more than"), "{e}");

        // ZIP64 extra field (the zip crate writes one for large_file)
        let mut w = ZipWriter::new(File::create(&z).unwrap());
        w.start_file(MANIFEST_NAME, o.large_file(true)).unwrap();
        w.write_all(&serde_json::to_vec(&base_manifest()).unwrap()).unwrap();
        w.finish().unwrap();
        let e = read_manifest(&z).unwrap_err().to_string();
        assert!(e.contains("ZIP64"), "{e}");

        // saturated size field = ZIP64 marker
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        patch_central(&z, 1, |h| h[24..28].copy_from_slice(&u32::MAX.to_le_bytes()));
        assert!(read_manifest(&z).unwrap_err().to_string().contains("ZIP64"));
        // ZIP64 locator right before the EOCD
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        let bytes = fs::read(&z).unwrap();
        let eocd = bytes.windows(4).rposition(|w| w == b"PK\x05\x06").unwrap();
        let mut with_loc = bytes[..eocd].to_vec();
        with_loc.extend_from_slice(b"PK\x06\x07");
        with_loc.extend_from_slice(&[0u8; 16]);
        with_loc.extend_from_slice(&bytes[eocd..]);
        fs::write(&z, with_loc).unwrap();
        assert!(read_manifest(&z).unwrap_err().to_string().contains("ZIP64"));

        // compression other than stored/deflate (12 = bzip2)
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        patch_central(&z, 1, |h| h[10..12].copy_from_slice(&12u16.to_le_bytes()));
        assert!(read_manifest(&z).is_err(), "bzip2 refused");
        // encrypted entry (general purpose flag bit 0)
        hand_zip(&z, &m, &[("data/a.txt", b"hello", 0o100644)]);
        patch_central(&z, 1, |h| h[8] |= 1);
        assert!(read_manifest(&z).is_err(), "encrypted refused");
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn rejects_non_nfc_case_collisions_and_oversize() {
        let t = tmp();
        let z = t.join("a.zip");
        let mut m = base_manifest();
        m.files = vec![entry("cafe\u{301}", b"x")];
        hand_zip(&z, &m, &[("data/cafe\u{301}", b"x", 0o100644)]);
        assert!(read_manifest(&z).unwrap_err().to_string().contains("NFC"));
        m.files = vec![entry("caf\u{e9}", b"x")];
        hand_zip(&z, &m, &[("data/caf\u{e9}", b"x", 0o100644)]);
        assert!(read_manifest(&z).is_ok());
        m.files = vec![entry("a", b"x"), entry("A", b"x")];
        hand_zip(&z, &m, &[("data/a", b"x", 0o100644), ("data/A", b"x", 0o100644)]);
        assert!(read_manifest(&z).is_err());
        m.files = vec![FileEntry { path: "big".into(), size: MAX_TOTAL_BYTES, sha256: "a".repeat(64) }];
        hand_zip(&z, &m, &[]);
        assert!(read_manifest(&z).is_err());
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn capture_recheck_detects_any_change() {
        let t = tmp();
        let data = t.join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("a"), b"1").unwrap();
        fs::write(data.join("b"), b"2").unwrap();
        let (before, _) = walk(&data).unwrap();
        assert!(recheck_capture(&data, &before).is_ok());
        let f = File::options().write(true).open(data.join("a")).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
        assert!(recheck_capture(&data, &before).is_err(), "mtime");
        let (before, _) = walk(&data).unwrap();
        fs::write(data.join("b"), b"22").unwrap();
        assert!(recheck_capture(&data, &before).is_err(), "size");
        let (before, _) = walk(&data).unwrap();
        fs::write(data.join("c"), b"3").unwrap();
        assert!(recheck_capture(&data, &before).is_err(), "new file");
        let (before, _) = walk(&data).unwrap();
        fs::remove_file(data.join("c")).unwrap();
        assert!(recheck_capture(&data, &before).is_err(), "removed file");
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn stem() {
        assert_eq!(file_stem("Shop 01 / é"), "Shop-01");
        assert_eq!(file_stem("///"), "profile");
    }
}
