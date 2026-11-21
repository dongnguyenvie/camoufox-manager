//! Apply journal (phase 15): crash-safe "replace the data of an existing profile".
//!
//! Replacing a profile's user-data (import `replace`, device sync pull) must never leave a
//! half-written dir. The apply is a small state machine persisted in SQLite
//! (`snapshot_journal`, migration `0008_journal.sql`) so that a crash or power loss at any point
//! is repaired on the next startup:
//!
//! ```text
//! preparing   a staging dir is being filled from the archive (nothing live was touched)
//! prepared    staging is complete and verified against the manifest; inventory + new profile
//!             record are stored in the journal row
//! publishing  swap: data -> backup, then staging -> data          (two renames, same volume)
//! complete    swapped, profile record updated, journal closed
//! aborted     rolled back; the old data is live again
//! ```
//!
//! Recovery ([`resolve`]) of an unresolved row:
//! - `preparing` / `prepared`: delete staging, `aborted` (the live data was never touched).
//! - `publishing`: **roll forward** when the swap already happened, or when the staging dir is
//!   still there and verifies against the stored inventory; otherwise **roll back** (put the
//!   backup back, drop staging).
//!
//! Until a row is resolved the profile cannot start ([`Manager::journal_check_start`]). Rows are
//! kept after resolution as an audit trail; `user-data.backup-<id>` stays on disk as an undo copy
//! until the next successful apply of that profile prunes it.
//!
//! The table lives in the app's main database file, but this module opens its own short-lived
//! connection (the tables are independent), so it needs nothing in `Deps`.

use crate::Manager;
use crate::transfer::{sha256_file, walk};
use anyhow::{Context, Result, bail};
use cm_core::{CmError, Profile, snapshot::FileEntry};
use rusqlite::{Connection, params};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SCHEMA: &str = include_str!("../../cm-store/src/migrations/0008_journal.sql");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Preparing,
    Prepared,
    Publishing,
    Complete,
    Aborted,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Preparing => "preparing",
            State::Prepared => "prepared",
            State::Publishing => "publishing",
            State::Complete => "complete",
            State::Aborted => "aborted",
        }
    }

    fn parse(s: &str) -> Option<State> {
        Some(match s {
            "preparing" => State::Preparing,
            "prepared" => State::Prepared,
            "publishing" => State::Publishing,
            "complete" => State::Complete,
            "aborted" => State::Aborted,
            _ => return None,
        })
    }

    pub fn is_unresolved(self) -> bool {
        !matches!(self, State::Complete | State::Aborted)
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub profile_id: String,
    pub state: State,
    pub staging: PathBuf,
    pub backup: PathBuf,
    pub inventory: Vec<FileEntry>,
    /// The profile record to store once the data is swapped (set at `prepared`).
    pub record: Option<Profile>,
    pub error: Option<String>,
}

/// Where a test (or nothing) simulates a crash: the apply stops right after the step, leaving
/// disk and journal exactly as a dead process would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum Crash {
    /// Staging half written, journal `preparing`.
    DuringPreparing,
    /// Journal `prepared`, nothing swapped.
    AfterPrepared,
    /// Journal `publishing`, nothing swapped yet.
    BeforeSwap,
    /// data renamed to backup, staging not yet renamed to data.
    BetweenRenames,
    /// Swapped, profile record not updated, journal still `publishing`.
    AfterSwap,
}

pub struct Journal {
    conn: Connection,
}

impl Journal {
    pub fn open(db: &Path) -> Result<Journal> {
        if let Some(dir) = db.parent() {
            fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(db).with_context(|| format!("cannot open {}", db.display()))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch(SCHEMA)?; // IF NOT EXISTS: a no-op once cm-store migrated the file
        Ok(Journal { conn })
    }

    pub fn begin(&self, id: &str, profile_id: &str, staging: &Path, backup: &Path, now: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO snapshot_journal (id, profile_id, state, staging, backup, created_at, updated_at) VALUES (?1, ?2, 'preparing', ?3, ?4, ?5, ?5)",
            params![id, profile_id, staging.to_string_lossy(), backup.to_string_lossy(), now],
        )?;
        Ok(())
    }

    pub fn set_state(&self, id: &str, state: State, error: Option<&str>, now: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE snapshot_journal SET state = ?2, error = COALESCE(?3, error), updated_at = ?4 WHERE id = ?1",
            params![id, state.as_str(), error, now],
        )?;
        Ok(())
    }

    /// Store what a roll-forward needs, together with the move to `prepared`.
    pub fn prepared(&self, id: &str, inventory: &[FileEntry], record: &Profile, now: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE snapshot_journal SET state = 'prepared', inventory = ?2, record = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, serde_json::to_string(inventory)?, serde_json::to_string(record)?, now],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<Entry>> {
        self.query("WHERE id = ?1", params![id]).map(|mut v| v.pop())
    }

    /// Rows that are not `complete`/`aborted`, optionally of one profile (oldest first).
    pub fn unresolved(&self, profile_id: Option<&str>) -> Result<Vec<Entry>> {
        match profile_id {
            Some(p) => {
                self.query("WHERE state IN ('preparing','prepared','publishing') AND profile_id = ?1 ORDER BY created_at, id", params![p])
            }
            None => self.query("WHERE state IN ('preparing','prepared','publishing') ORDER BY created_at, id", params![]),
        }
    }

    fn query(&self, clause: &str, p: impl rusqlite::Params) -> Result<Vec<Entry>> {
        let mut st = self
            .conn
            .prepare(&format!("SELECT id, profile_id, state, staging, backup, inventory, record, error FROM snapshot_journal {clause}"))?;
        let rows = st
            .query_map(p, |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, profile_id, state, staging, backup, inv, record, error)| {
                Ok(Entry {
                    id,
                    profile_id,
                    state: State::parse(&state).context("unknown journal state")?,
                    staging: staging.into(),
                    backup: backup.into(),
                    inventory: serde_json::from_str(&inv)?,
                    record: record.map(|r| serde_json::from_str(&r)).transpose()?,
                    error,
                })
            })
            .collect()
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.conn.query_row("SELECT COUNT(*) FROM snapshot_journal", [], |r| r.get::<_, i64>(0)).unwrap() as usize
    }
}

/// Check `dir` holds exactly the inventory: same files, sizes and SHA-256, nothing extra.
pub(crate) fn verify_tree(dir: &Path, inventory: &[FileEntry]) -> Result<()> {
    let (found, skipped) = walk(dir)?;
    if !skipped.is_empty() {
        bail!(CmError::Invalid(format!("staged data has unexpected entries: {}", skipped.join(", "))));
    }
    if found.len() != inventory.len() {
        bail!(CmError::Invalid(format!("staged data has {} files, manifest lists {}", found.len(), inventory.len())));
    }
    // Both sides are sorted by path.
    let mut want: Vec<&FileEntry> = inventory.iter().collect();
    want.sort_by(|a, b| a.path.cmp(&b.path));
    for (f, w) in found.iter().zip(want) {
        if f.rel != w.path || f.size != w.size || sha256_file(&f.abs)? != w.sha256 {
            bail!(CmError::Invalid(format!("staged file {} does not match the manifest", w.path)));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// The new data is live.
    Forward,
    /// The old data is live.
    Back,
}

fn remove_dir(p: &Path) {
    let _ = fs::remove_dir_all(p);
}

/// Bring the disk of an unresolved entry to a consistent state: all-new or all-old. Idempotent,
/// safe to run again after a crash during recovery itself.
pub(crate) fn resolve(e: &Entry, data: &Path) -> Result<Resolution> {
    let (has_data, has_backup, has_staging) = (data.exists(), e.backup.exists(), e.staging.exists());
    if e.state != State::Publishing {
        // The live data was never touched.
        remove_dir(&e.staging);
        return Ok(Resolution::Back);
    }
    if !has_staging && has_data {
        return Ok(Resolution::Forward); // swap finished
    }
    if has_staging && verify_tree(&e.staging, &e.inventory).is_ok() {
        match (has_data, has_backup) {
            (true, false) => fs::rename(data, &e.backup).context("cannot move the current data to the backup")?,
            (true, true) => bail!("journal {}: data, backup and staging all exist; resolve by hand", e.id),
            (false, _) => {}
        }
        fs::rename(&e.staging, data).context("cannot publish the staged data")?;
        return Ok(Resolution::Forward);
    }
    // Roll back.
    if has_backup && !has_data {
        fs::rename(&e.backup, data).context("cannot restore the backup")?;
    }
    remove_dir(&e.staging);
    Ok(Resolution::Back)
}

/// Delete the undo copies of `profile_dir` except `keep`.
pub(crate) fn prune_backups(profile_dir: &Path, keep: &Path) {
    let Ok(rd) = fs::read_dir(profile_dir) else { return };
    for e in rd.flatten() {
        if e.file_name().to_string_lossy().starts_with("user-data.backup-") && e.path() != keep {
            remove_dir(&e.path());
        }
    }
}

impl Manager {
    /// The app database file that holds `snapshot_journal`.
    pub(crate) fn db_path(&self) -> PathBuf {
        self.cfg.db_path.clone().unwrap_or_else(|| self.cfg.data_dir.join(cm_core_db_file()))
    }

    pub(crate) fn journal(&self) -> Result<Journal> {
        Journal::open(&self.db_path())
    }

    /// Resolve every unresolved journal row (startup / first use). Returns how many were repaired.
    /// The app calls this once at startup; it also runs lazily before the first start or apply.
    pub async fn recover_journals(&self) -> Result<usize> {
        self.journal_ready
            .get_or_try_init(|| async {
                let j = self.journal()?;
                let mut n = 0;
                for e in j.unresolved(None)? {
                    let now = self.d.clock.now_iso();
                    let data = self.d.engine.data_dir(&e.profile_id);
                    match resolve(&e, &data) {
                        Ok(Resolution::Forward) => {
                            if let Some(rec) = &e.record
                                && self.d.profiles.get(&rec.id).await?.is_some()
                            {
                                self.d.profiles.update(rec).await?;
                            }
                            if let Some(dir) = data.parent() {
                                prune_backups(dir, &e.backup);
                            }
                            j.set_state(&e.id, State::Complete, Some("rolled forward after a crash"), &now)?;
                        }
                        Ok(Resolution::Back) => j.set_state(&e.id, State::Aborted, Some("rolled back after a crash"), &now)?,
                        Err(err) => {
                            // Leave it unresolved (the profile stays blocked) and say why.
                            eprintln!("Snapshot journal {} for profile {} could not be recovered: {err:#}", e.id, e.profile_id);
                            let _ = j.set_state(&e.id, e.state, Some(&format!("{err:#}")), &now);
                            continue;
                        }
                    }
                    println!("Recovered an interrupted snapshot apply of profile {} ({})", e.profile_id, e.id);
                    n += 1;
                }
                Ok::<usize, anyhow::Error>(n)
            })
            .await
            .copied()
    }

    /// Refuse while the profile has an unresolved journal (also recovers leftovers first).
    pub(crate) async fn journal_check_start(&self, id: &str) -> Result<()> {
        let _ = self.recover_journals().await;
        if !self.journal()?.unresolved(Some(id))?.is_empty() {
            bail!(CmError::Conflict(
                "an import into this profile is unfinished or could not be recovered; the profile cannot start".into()
            ));
        }
        Ok(())
    }
}

fn cm_core_db_file() -> &'static str {
    "camoufox-manager.db"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Deps, transfer::write_zip_for_tests};
    use async_trait::async_trait;
    use cm_core::{
        Engine, EngineStatus, LaunchOptions, MachineSummary, NewProfile, Os, Proxy, ProxyCheck, ProxyChecker, Session, SystemClock,
        snapshot::{SNAPSHOT_SCHEMA, SnapshotManifest, SnapshotProfile},
    };
    use cm_store::*;
    use serde_json::{Value, json};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    };

    struct TestEngine {
        root: PathBuf,
        refuse_closed: AtomicBool,
        launches: AtomicU32,
    }

    #[async_trait]
    impl Engine for TestEngine {
        fn name(&self) -> &'static str {
            "Test"
        }
        fn ws_protocol(&self) -> &'static str {
            "Playwright"
        }
        async fn status(&self) -> EngineStatus {
            EngineStatus { ready: true, hint: None, version: None }
        }
        async fn generate(&self, os: Os) -> Result<Value> {
            Ok(json!({ "os": os.code() }))
        }
        fn summarize(&self, _m: &Value) -> MachineSummary {
            MachineSummary::default()
        }
        async fn launch(&self, _p: &Profile, _o: &LaunchOptions) -> Result<Session> {
            self.launches.fetch_add(1, Ordering::SeqCst);
            Ok(Session { ws_url: "ws://127.0.0.1:1/x".into() })
        }
        async fn stop(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn hold_closed(&self, _id: &str) -> Result<Box<dyn std::any::Any + Send>> {
            if self.refuse_closed.load(Ordering::SeqCst) {
                bail!(CmError::Conflict("profile is running; close it first".into()));
            }
            Ok(Box::new(()))
        }
        fn data_dir(&self, id: &str) -> PathBuf {
            self.root.join(id).join("user-data")
        }
        async fn delete_data(&self, _id: &str) -> Result<()> {
            Ok(())
        }
    }

    struct NoCheck;
    #[async_trait]
    impl ProxyChecker for NoCheck {
        async fn check(&self, _p: &Proxy) -> Result<ProxyCheck> {
            bail!("no network in tests")
        }
    }

    /// A restartable app: the same files and database, fresh in-memory state per `boot`.
    struct Rig {
        root: PathBuf,
        db: Db,
        engine: Arc<TestEngine>,
    }

    impl Rig {
        fn new(name: &str) -> Rig {
            let root = std::env::temp_dir().join(format!("cm-journal-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let db = Db::open(&root).unwrap();
            let engine =
                Arc::new(TestEngine { root: root.join("profiles"), refuse_closed: AtomicBool::new(false), launches: AtomicU32::new(0) });
            Rig { root, db, engine }
        }

        fn boot(&self) -> Manager {
            let db = self.db.clone();
            let deps = Deps {
                engine: self.engine.clone(),
                profiles: Arc::new(SqliteProfiles::new(db.clone())),
                proxies: Arc::new(SqliteProxies::new(db.clone())),
                checker: Arc::new(NoCheck),
                tags: Arc::new(SqliteTags::new(db.clone())),
                groups: Arc::new(SqliteGroups::new(db.clone())),
                views: Arc::new(SqliteSavedViews::new(db.clone())),
                templates: Arc::new(SqliteTemplates::new(db.clone())),
                extensions: Arc::new(SqliteExtensions::new(db.clone())),
                schedules: Arc::new(SqliteSchedules::new(db.clone())),
                settings: Arc::new(SqliteSettings::new(db.clone())),
                api_keys: Arc::new(cm_store::SqliteApiKeys::new(db.clone())),
                clock: Arc::new(SystemClock),
            };
            Manager::new(deps, Config { proxy_base_port: 45999, data_dir: self.root.clone(), db_path: db.path().map(Path::to_path_buf) })
        }

        fn data(&self, id: &str) -> PathBuf {
            self.engine.data_dir(id)
        }
    }

    fn write_data(dir: &Path, cookies: &str) {
        fs::create_dir_all(dir.join("storage")).unwrap();
        fs::write(dir.join("cookies.sqlite"), cookies).unwrap();
        fs::write(dir.join("storage/ls"), format!("ls-{cookies}")).unwrap();
    }

    fn cookies(dir: &Path) -> String {
        fs::read_to_string(dir.join("cookies.sqlite")).unwrap()
    }

    fn manifest_for(p: &Profile) -> SnapshotManifest {
        SnapshotManifest {
            schema: SNAPSHOT_SCHEMA,
            app_version: "t".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            camoufox_build: None,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            profile: SnapshotProfile {
                id: p.id.clone(),
                name: "Remote name".into(),
                os: p.os,
                notes: Some("from snapshot".into()),
                created_at: p.created_at.clone(),
                machine: json!({ "from": "snapshot" }),
                settings: Default::default(),
                proxy: None,
            },
            files: vec![],
            integrity: None,
        }
    }

    /// A profile with OLD data on disk and a snapshot zip holding NEW data.
    async fn setup(rig: &Rig, m: &Manager) -> (Profile, PathBuf) {
        let v = m.create_profile(NewProfile { name: "Local".into(), os: "mac".into() }).await.unwrap();
        let p = m.profile(&v.id).await.unwrap();
        write_data(&rig.data(&p.id), "OLD");
        let newdata = rig.root.join("newdata");
        write_data(&newdata, "NEW");
        let zip = rig.root.join("new.zip");
        write_zip_for_tests(&newdata, manifest_for(&p), &zip).unwrap();
        (p, zip)
    }

    fn err<T>(r: Result<T>) -> anyhow::Error {
        match r {
            Err(e) => e,
            Ok(_) => panic!("expected an error"),
        }
    }

    fn dir_names(p: &Path) -> Vec<String> {
        fs::read_dir(p).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect()
    }

    #[tokio::test]
    async fn replace_swaps_data_keeps_identity_and_leaves_a_backup() {
        let rig = Rig::new("replace");
        let m = rig.boot();
        let (p, zip) = setup(&rig, &m).await;
        let r = m.replace_from_snapshot(&zip, &p.id, None).await.unwrap();
        assert_eq!(r.profile.id, p.id);
        assert_eq!(cookies(&rig.data(&p.id)), "NEW");
        let after = m.profile(&p.id).await.unwrap();
        assert_eq!(after.name, "Local", "local name kept");
        assert_eq!(after.notes.as_deref(), Some("from snapshot"));
        assert_eq!(after.machine, json!({ "from": "snapshot" }));
        assert_eq!(after.created_at, p.created_at);
        let dir = rig.data(&p.id).parent().unwrap().to_path_buf();
        let names = dir_names(&dir);
        assert!(!names.iter().any(|n| n.contains("staging")), "{names:?}");
        let b = names.iter().find(|n| n.starts_with("user-data.backup-")).expect("backup kept");
        assert_eq!(cookies(&dir.join(b)), "OLD");
        let j = m.journal().unwrap();
        assert!(j.unresolved(None).unwrap().is_empty());
        assert_eq!(j.count(), 1);
        // a second replace prunes the first backup and may rename
        m.replace_from_snapshot(&zip, &p.id, Some("Renamed".into())).await.unwrap();
        assert_eq!(dir_names(&dir).iter().filter(|n| n.starts_with("user-data.backup-")).count(), 1);
        assert_eq!(m.profile(&p.id).await.unwrap().name, "Renamed");
        let _ = fs::remove_dir_all(&rig.root);
    }

    #[tokio::test]
    async fn replace_refused_when_not_closed_or_bad_archive_and_leaves_data_alone() {
        let rig = Rig::new("refuse");
        let m = rig.boot();
        let (p, zip) = setup(&rig, &m).await;
        rig.engine.refuse_closed.store(true, Ordering::SeqCst);
        let e = err(m.replace_from_snapshot(&zip, &p.id, None).await);
        assert!(matches!(e.downcast_ref::<CmError>(), Some(CmError::Conflict(_))));
        assert_eq!(cookies(&rig.data(&p.id)), "OLD");
        rig.engine.refuse_closed.store(false, Ordering::SeqCst);
        // tampered archive: fails while staging, aborts, live data untouched, no leftovers
        let mut bytes = fs::read(&zip).unwrap();
        let at = bytes.windows(3).position(|w| w == b"NEW").unwrap();
        bytes[at] = b'X';
        let bad = rig.root.join("bad.zip");
        fs::write(&bad, bytes).unwrap();
        assert!(m.replace_from_snapshot(&bad, &p.id, None).await.is_err());
        assert_eq!(cookies(&rig.data(&p.id)), "OLD");
        let dir = rig.data(&p.id).parent().unwrap().to_path_buf();
        assert!(dir_names(&dir).iter().all(|n| !n.contains("staging")));
        assert!(m.journal().unwrap().unresolved(None).unwrap().is_empty(), "aborted, not left open");
        assert!(m.replace_from_snapshot(&zip, "00ff00ff00ff00ff00ff00ff", None).await.is_err(), "unknown target");
        assert!(m.import_snapshot(&zip, cm_core::transfer::ImportIdMode::Replace, None).await.is_err(), "replace needs a target");
        let _ = fs::remove_dir_all(&rig.root);
    }

    /// Crash at `at` (the process dies), then a restart; returns the restarted manager.
    async fn crash_then_restart(rig: &Rig, at: Crash, tamper_staging: bool) -> (Manager, Profile) {
        let m = rig.boot();
        let (p, zip) = setup(rig, &m).await;
        let e = err(m.replace_inner(&zip, &p.id, None, Some(at)).await);
        assert!(e.to_string().contains("simulated crash"), "{e:#}");
        // While the row is unresolved the profile cannot start.
        let blocked = err(m.start(&p.id).await);
        assert!(matches!(blocked.downcast_ref::<CmError>(), Some(CmError::Conflict(_))), "{blocked:#}");
        assert_eq!(rig.engine.launches.load(Ordering::SeqCst), 0);
        if tamper_staging {
            let st = m.journal().unwrap().unresolved(Some(&p.id)).unwrap().remove(0).staging;
            fs::write(st.join("cookies.sqlite"), "XXX").unwrap();
        }
        let m2 = rig.boot();
        assert_eq!(m2.recover_journals().await.unwrap(), 1);
        (m2, p)
    }

    async fn assert_consistent(rig: &Rig, m: &Manager, p: &Profile, new_live: bool) {
        assert_eq!(cookies(&rig.data(&p.id)), if new_live { "NEW" } else { "OLD" });
        let dir = rig.data(&p.id).parent().unwrap().to_path_buf();
        assert!(dir_names(&dir).iter().all(|n| !n.contains("staging")), "staging gone");
        let rec = m.profile(&p.id).await.unwrap();
        assert_eq!(rec.notes.as_deref(), if new_live { Some("from snapshot") } else { None }, "record matches data");
        assert!(m.journal().unwrap().unresolved(None).unwrap().is_empty());
        m.start(&p.id).await.unwrap(); // and the profile starts again
    }

    #[tokio::test]
    async fn crash_while_preparing_or_prepared_rolls_back() {
        for (name, at) in [("c1", Crash::DuringPreparing), ("c2", Crash::AfterPrepared)] {
            let rig = Rig::new(name);
            let (m2, p) = crash_then_restart(&rig, at, false).await;
            assert_consistent(&rig, &m2, &p, false).await;
            let _ = fs::remove_dir_all(&rig.root);
        }
    }

    #[tokio::test]
    async fn crash_while_publishing_rolls_forward_with_verified_staging() {
        for (name, at) in [("c3", Crash::BeforeSwap), ("c4", Crash::BetweenRenames), ("c5", Crash::AfterSwap)] {
            let rig = Rig::new(name);
            let (m2, p) = crash_then_restart(&rig, at, false).await;
            assert_consistent(&rig, &m2, &p, true).await;
            let dir = rig.data(&p.id).parent().unwrap().to_path_buf();
            let b = dir_names(&dir).into_iter().find(|n| n.starts_with("user-data.backup-")).expect("old data kept as backup");
            assert_eq!(cookies(&dir.join(b)), "OLD");
            let _ = fs::remove_dir_all(&rig.root);
        }
    }

    #[tokio::test]
    async fn crash_while_publishing_with_damaged_staging_rolls_back() {
        for (name, at) in [("c6", Crash::BeforeSwap), ("c7", Crash::BetweenRenames)] {
            let rig = Rig::new(name);
            let (m2, p) = crash_then_restart(&rig, at, true).await;
            assert_consistent(&rig, &m2, &p, false).await;
            let _ = fs::remove_dir_all(&rig.root);
        }
    }

    #[tokio::test]
    async fn a_second_restart_finds_nothing_to_recover() {
        let rig = Rig::new("idem");
        let (_m2, p) = crash_then_restart(&rig, Crash::BetweenRenames, false).await;
        assert_eq!(cookies(&rig.data(&p.id)), "NEW");
        assert_eq!(rig.boot().recover_journals().await.unwrap(), 0);
        let _ = fs::remove_dir_all(&rig.root);
    }

    #[tokio::test]
    async fn export_needs_the_closed_proof_and_no_unresolved_journal() {
        let rig = Rig::new("export");
        let m = rig.boot();
        let (p, zip) = setup(&rig, &m).await;
        rig.engine.refuse_closed.store(true, Ordering::SeqCst);
        let e = err(m.export_profile(&p.id).await);
        assert!(matches!(e.downcast_ref::<CmError>(), Some(CmError::Conflict(_))));
        assert!(!m.exports_dir().join(format!("Local-{}.zip", p.id)).exists(), "nothing written");
        rig.engine.refuse_closed.store(false, Ordering::SeqCst);
        m.export_profile(&p.id).await.unwrap();
        let _ = m.replace_inner(&zip, &p.id, None, Some(Crash::AfterPrepared)).await;
        assert!(m.export_profile(&p.id).await.is_err(), "unresolved journal blocks export");
        let _ = fs::remove_dir_all(&rig.root);
    }

    #[test]
    fn states_roundtrip_and_unresolved_flag() {
        for s in [State::Preparing, State::Prepared, State::Publishing, State::Complete, State::Aborted] {
            assert_eq!(State::parse(s.as_str()), Some(s));
        }
        assert!(State::Preparing.is_unresolved() && State::Publishing.is_unresolved());
        assert!(!State::Complete.is_unresolved() && !State::Aborted.is_unresolved());
        assert_eq!(State::parse("bogus"), None);
    }
}
