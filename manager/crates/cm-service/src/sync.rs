//! Device sync engine (phase 17, D9): manifest, diff, chunked transfer, journalled apply.
//!
//! Two roles in the same code:
//!
//! - **Peer side** (`sync_*` methods called by the signed `/sync/v1/…` routes of `cm-api`): list
//!   my profiles, export a closed profile for the caller to pull (`prepare`/`chunk`/`done`),
//!   receive a profile the caller pushes (`begin`/`put`/`commit`).
//! - **Local side** (called by the UI routes): [`Manager::sync_plan`] diffs the two machines,
//!   [`Manager::sync_start`] runs a job (one profile at a time, progress + cancel) that talks to
//!   the other device through [`cm_sync::PeerClient`].
//!
//! The data always travels as the phase 6 snapshot zip and is applied through the phase 15 journal
//! (`import_snapshot` / `replace_from_snapshot`): the previous profile stays intact on any failure
//! or cancel. Nothing is merged: when both sides changed, the user chooses.
//!
//! State kept in `sync_state` (migration 0011): per profile its current content hash and a
//! revision counter (`device_id = ''`), and per paired device what was agreed at the last sync.
//! Files of a transfer in progress live in `<data>/sync/{out,in}/` (0700, deleted after 24 h).

use crate::{
    Manager,
    profiles::new_id,
    transfer::{read_manifest, sha256_file, walk},
};
use anyhow::{Context, Result, bail};
use cm_core::{
    CmError, NewProxy, ProxyScheme,
    snapshot::{MAX_TOTAL_BYTES, is_hex_sha256},
    transfer::ImportIdMode,
};
use cm_sync::{
    Action, BeginRequest, BeginResponse, ChunkAck, CommitResponse, Direction, DoneRequest, Env, LinkInfo, ManifestEntry, PeerClient,
    PeerError, PeerManifest, PrepareRequest, PrepareResponse, ProxyInfo, Row,
    apply::copy_name,
    diff::build_rows,
    transfer::{
        CHUNK_SHA_HEADER, ChunkSink, ChunkSource, MAX_CHUNK, Meta, PartFile, Store, TransferError, chunk_size_from_env, pull_file,
        push_file, read_chunk, sha256_hex,
    },
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Same SQL as `cm-store` migration 0011 (idempotent), like the journal: this module opens its
/// own short-lived connection to the app database.
const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS sync_state (
    device_id TEXT NOT NULL, profile_id TEXT NOT NULL, rev INTEGER NOT NULL DEFAULT 0,
    content_hash TEXT NOT NULL DEFAULT '', last_synced_hash TEXT, peer_profile_id TEXT,
    updated_at TEXT NOT NULL, PRIMARY KEY (device_id, profile_id));
    CREATE INDEX IF NOT EXISTS sync_state_peer ON sync_state (device_id, peer_profile_id);";

/// Unfinished transfers are dropped after this many seconds.
const KEEP_SECS: u64 = 24 * 3600;
/// Per-request timeout towards the peer (a chunk is 4-8 MiB).
const CHUNK_TIMEOUT: Duration = Duration::from_secs(120);
/// For requests where the peer verifies / applies gigabytes before answering.
const APPLY_TIMEOUT: Duration = Duration::from_secs(900);
const MAX_JOBS_KEPT: usize = 20;
const HASH_CACHE_MAX: usize = 100_000;

// ---------------------------------------------------------------------------------------------
// Errors the HTTP layer maps to a status + stable code
// ---------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct SyncError {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
    /// For `bad_offset`: the offset the receiver confirmed.
    pub received: Option<u64>,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SyncError {}

fn refuse(status: u16, code: &'static str, message: impl Into<String>) -> anyhow::Error {
    SyncError { status, code, message: message.into(), received: None }.into()
}

fn from_transfer(e: TransferError) -> anyhow::Error {
    match e {
        TransferError::Offset { expected, .. } => {
            SyncError { status: 409, code: "bad_offset", message: e.to_string(), received: Some(expected) }.into()
        }
        TransferError::ChunkHash(_) => refuse(422, "bad_chunk", e.to_string()),
        TransferError::WholeHash | TransferError::Size { .. } => refuse(422, "bad_file", e.to_string()),
        other => anyhow::anyhow!(other.to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

type HashCache = Arc<Mutex<HashMap<(PathBuf, u64, i128), String>>>;

#[derive(Default)]
pub struct SyncState {
    jobs: Mutex<Vec<Arc<Job>>>,
    hashes: HashCache,
}

/// A link row: what was agreed with one device about one local profile.
#[derive(Clone, Debug)]
struct LinkRow {
    peer_profile_id: Option<String>,
    last_synced_hash: Option<String>,
}

fn state_conn(m: &Manager) -> Result<Connection> {
    let db = m.db_path();
    if let Some(dir) = db.parent() {
        fs::create_dir_all(dir)?;
    }
    let c = Connection::open(&db).with_context(|| format!("cannot open {}", db.display()))?;
    c.execute_batch("PRAGMA busy_timeout = 5000;")?;
    c.execute_batch(SCHEMA)?;
    Ok(c)
}

// ---------------------------------------------------------------------------------------------
// Content hash
// ---------------------------------------------------------------------------------------------

/// Hash of everything a sync moves: pinned machine, settings, notes, OS and every data file
/// (path, size, SHA-256). Not the name, group, schedules or proxy assignment: those stay local.
fn compute_hash(data: &Path, record: &serde_json::Value, cache: &HashCache) -> Result<(String, u64)> {
    let (found, _) = if data.is_dir() { walk(data)? } else { (Vec::new(), Vec::new()) };
    let mut h = Sha256::new();
    h.update(b"cm-sync-content-v1\n");
    h.update(serde_json::to_vec(record)?);
    h.update(b"\n");
    let mut total = 0u64;
    for f in &found {
        let key = (f.abs.clone(), f.size, f.mtime);
        let cached = cache.lock().unwrap().get(&key).cloned();
        let sha = match cached {
            Some(s) => s,
            None => {
                let s = sha256_file(&f.abs)?;
                let mut c = cache.lock().unwrap();
                if c.len() > HASH_CACHE_MAX {
                    c.clear();
                }
                c.insert(key, s.clone());
                s
            }
        };
        h.update(format!("{}\0{}\0{}\n", f.rel, f.size, sha).as_bytes());
        total += f.size;
    }
    Ok((cm_sync::transfer::hex(&h.finalize()), total))
}

// ---------------------------------------------------------------------------------------------
// Jobs (local side)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartItem {
    /// Profile id on the source device.
    pub profile_id: String,
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub direction: Direction,
    pub items: Vec<StartItem>,
    /// Include the proxy assigned to each profile (host, port, scheme, user). Default off.
    #[serde(default)]
    pub include_proxy: bool,
    /// Also include the proxy password. Only honoured together with `include_proxy`.
    #[serde(default)]
    pub include_proxy_password: bool,
    /// The user saw the warning that the same logins will exist on both machines.
    #[serde(default)]
    pub acknowledged: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub profile_id: String,
    pub name: String,
    pub action: Action,
    /// `queued` | `preparing` | `transferring` | `verifying` | `applying` | `done` | `failed` | `skipped` | `cancelled`
    pub state: &'static str,
    pub bytes: u64,
    pub total: u64,
    pub message: Option<String>,
    pub code: Option<String>,
    /// Profile id on the destination once applied.
    pub result_profile_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub direction: Direction,
    /// `running` | `done` | `cancelled`
    pub state: &'static str,
    pub cancelling: bool,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub items: Vec<ItemView>,
    pub done: usize,
    pub failed: usize,
    pub skipped: usize,
}

struct Job {
    cancel: AtomicBool,
    view: Mutex<JobView>,
}

impl Job {
    fn snapshot(&self) -> JobView {
        let mut v = self.view.lock().unwrap().clone();
        v.cancelling = self.cancel.load(Ordering::SeqCst) && v.state == "running";
        v.done = v.items.iter().filter(|i| i.state == "done").count();
        v.failed = v.items.iter().filter(|i| i.state == "failed").count();
        v.skipped = v.items.iter().filter(|i| matches!(i.state, "skipped" | "cancelled")).count();
        v
    }

    fn item(&self, idx: usize, f: impl FnOnce(&mut ItemView)) {
        if let Some(i) = self.view.lock().unwrap().items.get_mut(idx) {
            f(i);
        }
    }

    fn set(&self, idx: usize, state: &'static str) {
        self.item(idx, |i| i.state = state);
    }
}

/// A row's plan after validation against the fresh diff.
struct Planned {
    source_id: String,
    action: Action,
    target_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPeer {
    pub id: String,
    pub name: String,
    pub env: Env,
}

/// Answer of the "what would sync do" listing.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub direction: Direction,
    pub peer: PlanPeer,
    pub me: Env,
    /// Why nothing can be copied between the two machines (same OS + arch + Camoufox build needed).
    pub incompatible: Option<String>,
    pub rows: Vec<Row>,
}

// ---------------------------------------------------------------------------------------------
// Manager: shared helpers
// ---------------------------------------------------------------------------------------------

fn proxy_info(p: &cm_core::Proxy, with_password: bool) -> ProxyInfo {
    ProxyInfo {
        name: p.name.clone(),
        scheme: p.scheme.as_str().into(),
        host: p.host.clone(),
        port: p.port,
        username: p.username.clone(),
        password: if with_password { p.password.clone().filter(|s| !s.is_empty()) } else { None },
    }
}

fn action_str(a: Action) -> &'static str {
    match a {
        Action::Create => "create",
        Action::Replace => "replace",
        Action::Copy => "copy",
        Action::Skip => "skip",
    }
}

fn parse_action(s: Option<&str>) -> Option<Action> {
    match s? {
        "create" => Some(Action::Create),
        "replace" => Some(Action::Replace),
        "copy" => Some(Action::Copy),
        _ => None,
    }
}

impl Manager {
    fn sync_dir(&self) -> PathBuf {
        self.cfg.data_dir.join("sync")
    }

    fn out_store(&self) -> Store {
        Store::new(self.sync_dir().join("out"), "zip")
    }

    fn in_store(&self) -> Store {
        Store::new(self.sync_dir().join("in"), "part")
    }

    /// OS, CPU arch and Camoufox build of this machine: what decides whether snapshots fit.
    pub async fn sync_env(&self) -> Env {
        Env { os: std::env::consts::OS.into(), arch: std::env::consts::ARCH.into(), camoufox_build: self.d.engine.status().await.version }
    }

    async fn is_closed(&self, id: &str) -> bool {
        !self.running.lock().await.contains_key(id)
            && self.d.engine.alive(id).await != Some(true)
            && self.journal().and_then(|j| j.unresolved(Some(id))).map(|v| v.is_empty()).unwrap_or(false)
    }

    /// Content hash + data size of a profile.
    pub async fn sync_content_hash(&self, id: &str) -> Result<(String, u64)> {
        let p = self.profile_or_trashed(id).await?;
        let record = serde_json::json!({ "os": p.os, "notes": p.notes, "machine": p.machine, "settings": p.settings });
        let data = self.d.engine.data_dir(id);
        let cache = self.sync.hashes.clone();
        tokio::task::spawn_blocking(move || compute_hash(&data, &record, &cache)).await?
    }

    // --- sync_state ----------------------------------------------------------------------------

    /// Current revision of a profile, bumped when its content hash differs from the last seen.
    fn touch_local(&self, profile: &str, hash: &str) -> Result<i64> {
        let c = state_conn(self)?;
        let now = self.d.clock.now_iso();
        let row: Option<(i64, String)> = c
            .query_row("SELECT rev, content_hash FROM sync_state WHERE device_id = '' AND profile_id = ?1", [profile], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        match row {
            Some((rev, h)) if h == hash => Ok(rev),
            Some((rev, _)) => {
                c.execute(
                    "UPDATE sync_state SET rev = ?2, content_hash = ?3, updated_at = ?4 WHERE device_id = '' AND profile_id = ?1",
                    params![profile, rev + 1, hash, now],
                )?;
                Ok(rev + 1)
            }
            None => {
                c.execute(
                    "INSERT INTO sync_state (device_id, profile_id, rev, content_hash, updated_at) VALUES ('', ?1, 1, ?2, ?3)",
                    params![profile, hash, now],
                )?;
                Ok(1)
            }
        }
    }

    /// Drop every sync bookkeeping row of a profile that is gone for good (its own state and what
    /// was agreed with each peer).
    pub(crate) fn sync_forget(&self, profile: &str) -> Result<()> {
        let c = state_conn(self)?;
        c.execute("DELETE FROM sync_state WHERE profile_id = ?1", [profile])?;
        Ok(())
    }

    fn links(&self, device: &str) -> Result<HashMap<String, LinkRow>> {
        let c = state_conn(self)?;
        let mut st = c.prepare("SELECT profile_id, peer_profile_id, last_synced_hash FROM sync_state WHERE device_id = ?1")?;
        let rows = st
            .query_map([device], |r| Ok((r.get::<_, String>(0)?, LinkRow { peer_profile_id: r.get(1)?, last_synced_hash: r.get(2)? })))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().collect())
    }

    /// Record that `profile` and the peer's `peer_profile` agree on `hash`.
    fn set_link(&self, device: &str, profile: &str, peer_profile: &str, hash: &str) -> Result<()> {
        let rev = self.touch_local(profile, hash)?;
        let c = state_conn(self)?;
        c.execute(
            "INSERT INTO sync_state (device_id, profile_id, rev, content_hash, last_synced_hash, peer_profile_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)
             ON CONFLICT(device_id, profile_id) DO UPDATE SET rev = ?3, content_hash = ?4, last_synced_hash = ?4,
               peer_profile_id = ?5, updated_at = ?6",
            params![device, profile, rev, hash, peer_profile, self.d.clock.now_iso()],
        )?;
        Ok(())
    }

    // --- manifest ------------------------------------------------------------------------------

    /// My profiles as `device` sees them (D9 manifest exchange).
    pub async fn sync_manifest(&self, device: &str) -> Result<PeerManifest> {
        self.prune_dead().await;
        let links = self.links(device)?;
        let mut out = Vec::new();
        let trashed: Vec<cm_core::Profile> = self.d.profiles.list_trashed().await?.into_iter().map(|t| t.profile).collect();
        let live = self.d.profiles.list().await?;
        let all = live.into_iter().map(|p| (p, false)).chain(trashed.into_iter().map(|p| (p, true)));
        for (p, trashed) in all {
            let closed = self.is_closed(&p.id).await;
            let (hash, size) = if closed {
                let (h, s) = self.sync_content_hash(&p.id).await?;
                (Some(h), s)
            } else {
                (None, 0)
            };
            let rev = match &hash {
                Some(h) => self.touch_local(&p.id, h)? as u64,
                None => 0,
            };
            out.push(ManifestEntry {
                link: links.get(&p.id).and_then(|l| {
                    l.peer_profile_id.clone().map(|id| LinkInfo { peer_profile_id: id, last_synced_hash: l.last_synced_hash.clone() })
                }),
                id: p.id,
                name: p.name,
                rev,
                content_hash: hash,
                size,
                closed,
                trashed,
            });
        }
        let me = self.device_hub().map(|h| h.me());
        Ok(PeerManifest {
            device_id: me.as_ref().map(|m| m.id.clone()).unwrap_or_default(),
            name: me.map(|m| m.name).unwrap_or_default(),
            env: self.sync_env().await,
            profiles: out,
        })
    }

    // --- peer side: a profile leaves this machine (pull) ----------------------------------------

    /// Export a closed profile to `<data>/sync/out/<id>.zip` for `peer` (or reuse a still valid
    /// earlier export named by `resume`).
    async fn sync_export(&self, peer: &str, profile: &str, resume: Option<&str>) -> Result<Meta> {
        let store = self.out_store();
        store.gc(self.d.clock.now(), KEEP_SECS);
        let p = self.profile(profile).await?;
        if !self.is_closed(profile).await {
            bail!(refuse(409, "profile_running", format!("{} is running on this device; close it first", p.name)));
        }
        let (hash, _) = self.sync_content_hash(profile).await?;
        if let Some(m) = resume.and_then(|t| store.load(t))
            && m.peer == peer
            && m.profile_id == profile
            && m.content_hash == hash
            && fs::metadata(store.data_path(&m.id)).map(|x| x.len() == m.size).unwrap_or(false)
        {
            return Ok(m);
        }
        // An earlier export of the same profile for the same peer is stale now.
        for m in store.all().into_iter().filter(|m| m.peer == peer && m.profile_id == profile) {
            store.remove(&m.id);
        }
        let r = self.export_profile(profile).await?;
        let id = new_id()?;
        store.ensure_dir()?;
        let dest = store.data_path(&id);
        let src = PathBuf::from(&r.path);
        let d2 = dest.clone();
        let sha = tokio::task::spawn_blocking(move || -> Result<String> {
            if fs::rename(&src, &d2).is_err() {
                fs::copy(&src, &d2)?;
                let _ = fs::remove_file(&src);
            }
            sha256_file(&d2)
        })
        .await??;
        let meta = Meta {
            id,
            peer: peer.into(),
            profile_id: profile.into(),
            name: p.name,
            size: fs::metadata(&dest)?.len(),
            sha256: sha,
            chunk: chunk_size_from_env(),
            content_hash: hash,
            created_at: self.d.clock.now(),
            action: None,
            target_id: None,
            proxy: None,
        };
        store.save(&meta)?;
        Ok(meta)
    }

    async fn proxy_info_for(&self, profile: &str, with_password: bool) -> Result<Option<ProxyInfo>> {
        let p = self.profile(profile).await?;
        Ok(match p.proxy_id {
            Some(id) => self.d.proxies.get(id).await?.map(|x| proxy_info(&x, with_password)),
            None => None,
        })
    }

    fn gate(&self, theirs: &Env, mine: &Env) -> Result<()> {
        mine.check_compatible(theirs).map_err(|r| refuse(409, "sync_incompatible", format!("these two devices cannot sync: {r}")))
    }

    /// `POST /sync/v1/transfer/prepare`
    pub async fn sync_prepare(&self, peer: &str, req: PrepareRequest) -> Result<PrepareResponse> {
        let mine = self.sync_env().await;
        self.gate(&req.env, &mine)?;
        let m = self.sync_export(peer, &req.profile_id, req.resume.as_deref()).await?;
        let proxy = if req.include_proxy { self.proxy_info_for(&req.profile_id, req.include_proxy_password).await? } else { None };
        Ok(PrepareResponse {
            transfer_id: m.id,
            name: m.name,
            size: m.size,
            sha256: m.sha256,
            chunk: m.chunk,
            content_hash: m.content_hash,
            env: mine,
            proxy,
        })
    }

    fn out_meta(&self, peer: &str, tid: &str) -> Result<Meta> {
        self.out_store()
            .load(tid)
            .filter(|m| m.peer == peer)
            .ok_or_else(|| refuse(404, "no_transfer", "no such transfer (it may have expired); start the sync again"))
    }

    /// `GET /sync/v1/transfer/{id}/chunk/{n}` → bytes and their SHA-256.
    pub async fn sync_chunk(&self, peer: &str, tid: &str, index: u64) -> Result<(Vec<u8>, String)> {
        let m = self.out_meta(peer, tid)?;
        let path = self.out_store().data_path(tid);
        let bytes = tokio::task::spawn_blocking(move || read_chunk(&path, m.size, m.chunk, index)).await?.map_err(from_transfer)?;
        let sha = sha256_hex(&bytes);
        Ok((bytes, sha))
    }

    /// `POST /sync/v1/transfer/{id}/done`: the caller applied the profile; remember the link.
    pub async fn sync_done(&self, peer: &str, tid: &str, req: DoneRequest) -> Result<()> {
        let m = self.out_meta(peer, tid)?;
        if let Some(result) = req.result_profile_id.as_deref().filter(|r| cm_core::profile::valid_id(r))
            && self.profile(&m.profile_id).await.is_ok()
        {
            self.set_link(peer, &m.profile_id, result, &m.content_hash)?;
        }
        self.out_store().remove(tid);
        Ok(())
    }

    /// `POST …/cancel` on an export.
    pub fn sync_cancel_out(&self, peer: &str, tid: &str) -> Result<()> {
        self.out_meta(peer, tid)?;
        self.out_store().remove(tid);
        Ok(())
    }

    // --- peer side: a profile arrives here (push) -----------------------------------------------

    /// `POST /sync/v1/inbox/begin`
    pub async fn sync_begin(&self, peer: &str, req: BeginRequest) -> Result<BeginResponse> {
        let mine = self.sync_env().await;
        self.gate(&req.env, &mine)?;
        if req.action == Action::Skip {
            bail!(CmError::Invalid("nothing to do".into()));
        }
        if req.size == 0 || req.size > MAX_TOTAL_BYTES + 64 * 1024 * 1024 {
            bail!(refuse(413, "too_large", "the snapshot is empty or larger than this app accepts"));
        }
        if req.chunk == 0 || req.chunk > MAX_CHUNK || !is_hex_sha256(&req.sha256) || !cm_core::profile::valid_id(&req.profile_id) {
            bail!(CmError::Invalid("invalid transfer description".into()));
        }
        match req.action {
            Action::Replace => {
                let t = req.target_id.as_deref().context("replace needs the profile to replace")?;
                let p = self.profile_or_trashed(t).await?;
                if !self.is_closed(t).await {
                    bail!(refuse(409, "profile_running", format!("{} is running on this device; close it first", p.name)));
                }
            }
            Action::Create | Action::Copy => {}
            Action::Skip => unreachable!(),
        }
        let store = self.in_store();
        store.gc(self.d.clock.now(), KEEP_SECS);
        let reuse = req
            .transfer_id
            .as_deref()
            .and_then(|t| store.load(t))
            .filter(|m| m.peer == peer && m.sha256 == req.sha256 && m.size == req.size);
        let meta = match reuse {
            Some(mut m) => {
                m.action = Some(action_str(req.action).into());
                m.target_id = req.target_id.clone();
                m.proxy = req.proxy.as_ref().map(serde_json::to_value).transpose()?;
                m.name = req.name.clone();
                m.content_hash = req.content_hash.clone();
                m
            }
            None => {
                if let Some(t) = req.transfer_id.as_deref() {
                    store.remove(t);
                }
                Meta {
                    id: new_id()?,
                    peer: peer.into(),
                    profile_id: req.profile_id.clone(),
                    name: req.name.clone(),
                    size: req.size,
                    sha256: req.sha256.clone(),
                    chunk: req.chunk,
                    content_hash: req.content_hash.clone(),
                    created_at: self.d.clock.now(),
                    action: Some(action_str(req.action).into()),
                    target_id: req.target_id.clone(),
                    proxy: req.proxy.as_ref().map(serde_json::to_value).transpose()?,
                }
            }
        };
        store.save(&meta)?;
        let part = PartFile::open(&store.data_path(&meta.id), meta.size, &meta.sha256, meta.chunk).map_err(from_transfer)?;
        Ok(BeginResponse { transfer_id: meta.id, received: part.received() })
    }

    fn in_meta(&self, peer: &str, tid: &str) -> Result<Meta> {
        self.in_store()
            .load(tid)
            .filter(|m| m.peer == peer)
            .ok_or_else(|| refuse(404, "no_transfer", "no such transfer (it may have expired); start the sync again"))
    }

    /// `GET /sync/v1/inbox/{id}`
    pub async fn sync_inbox_status(&self, peer: &str, tid: &str) -> Result<BeginResponse> {
        let m = self.in_meta(peer, tid)?;
        let part = PartFile::open(&self.in_store().data_path(tid), m.size, &m.sha256, m.chunk).map_err(from_transfer)?;
        Ok(BeginResponse { transfer_id: m.id, received: part.received() })
    }

    /// `PUT /sync/v1/inbox/{id}/chunk/{n}?sha=…`: verify and append one chunk.
    pub async fn sync_put_chunk(&self, peer: &str, tid: &str, index: u64, bytes: Vec<u8>, sha: String) -> Result<ChunkAck> {
        let m = self.in_meta(peer, tid)?;
        let path = self.in_store().data_path(tid);
        let received = tokio::task::spawn_blocking(move || -> std::result::Result<u64, TransferError> {
            let mut part = PartFile::open(&path, m.size, &m.sha256, m.chunk)?;
            part.append(index * m.chunk as u64, &bytes, &sha)
        })
        .await?
        .map_err(from_transfer)?;
        Ok(ChunkAck { received })
    }

    /// `POST /sync/v1/inbox/{id}/commit`: check the whole file, apply it through the journal.
    pub async fn sync_commit(&self, peer: &str, peer_name: &str, tid: &str) -> Result<CommitResponse> {
        let m = self.in_meta(peer, tid)?;
        let store = self.in_store();
        let path = store.data_path(tid);
        let (p2, mm) = (path.clone(), m.clone());
        let checked = tokio::task::spawn_blocking(move || -> std::result::Result<(), TransferError> {
            PartFile::open(&p2, mm.size, &mm.sha256, mm.chunk)?.finish()
        })
        .await?;
        if let Err(e) = checked {
            if matches!(e, TransferError::WholeHash) {
                store.remove(tid); // the data is damaged: start over
            }
            return Err(from_transfer(e));
        }
        let action = parse_action(m.action.as_deref()).context("transfer has no action")?;
        let proxy = m.proxy.clone().and_then(|v| serde_json::from_value::<ProxyInfo>(v).ok());
        let r = self.sync_apply(&path, peer, peer_name, &m.profile_id, action, m.target_id.as_deref(), proxy).await?;
        store.remove(tid);
        Ok(r)
    }

    /// `POST …/cancel` on an inbox transfer.
    pub fn sync_cancel_in(&self, peer: &str, tid: &str) -> Result<()> {
        self.in_meta(peer, tid)?;
        self.in_store().remove(tid);
        Ok(())
    }

    // --- applying a received snapshot ------------------------------------------------------------

    /// Apply a verified snapshot zip. Create/Copy add a profile, Replace swaps the data of
    /// `target` through the phase 15 journal. The snapshot's own manifest is checked against this
    /// machine again (same OS, arch, Camoufox build).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn sync_apply(
        &self,
        zip: &Path,
        peer: &str,
        peer_name: &str,
        source_profile: &str,
        action: Action,
        target: Option<&str>,
        proxy: Option<ProxyInfo>,
    ) -> Result<CommitResponse> {
        let z = zip.to_path_buf();
        let manifest = tokio::task::spawn_blocking(move || read_manifest(&z)).await??;
        let theirs = Env { os: manifest.os.clone(), arch: manifest.arch.clone(), camoufox_build: manifest.camoufox_build.clone() };
        self.gate(&theirs, &self.sync_env().await)?;
        let imported = match action {
            Action::Create => self.import_snapshot(zip, ImportIdMode::KeepIfFree, None).await?,
            Action::Copy => self.import_snapshot(zip, ImportIdMode::New, Some(copy_name(&manifest.profile.name, peer_name))).await?,
            Action::Replace => {
                let t = target.context("replace needs the profile to replace")?;
                // An update for a trashed profile brings it back first; if the swap fails it goes back to the trash.
                let was_trashed = self.d.profiles.list_trashed().await?.into_iter().find(|x| x.profile.id == t);
                if was_trashed.is_some() {
                    self.d.profiles.restore(t).await?;
                }
                match self.replace_from_snapshot(zip, t, None).await {
                    Ok(r) => r,
                    Err(e) => {
                        if let Some(x) = was_trashed {
                            let _ = self.d.profiles.trash(t, &x.deleted_at).await;
                        }
                        return Err(e);
                    }
                }
            }
            Action::Skip => bail!(CmError::Invalid("nothing to apply".into())),
        };
        let id = imported.profile.id.clone();
        let mut warning = None;
        if let Some(info) = proxy
            && let Err(e) = self.sync_attach_proxy(&id, &info).await
        {
            warning = Some(format!("the profile was copied, but its proxy could not be set: {e:#}"));
        }
        let (hash, _) = self.sync_content_hash(&id).await?;
        if action != Action::Copy {
            self.set_link(peer, &id, source_profile, &hash)?;
        }
        Ok(CommitResponse { profile_id: id, name: imported.profile.name, kept_id: imported.kept_id, content_hash: hash, warning })
    }

    /// Use the library entry that matches `info` (scheme, host, port, user) or add one, and assign it.
    async fn sync_attach_proxy(&self, profile: &str, info: &ProxyInfo) -> Result<()> {
        let scheme = ProxyScheme::parse(&info.scheme).context("unknown proxy scheme")?;
        let existing =
            self.d.proxies.list().await?.into_iter().find(|p| {
                p.scheme == scheme && p.host.eq_ignore_ascii_case(&info.host) && p.port == info.port && p.username == info.username
            });
        let id = match existing {
            Some(p) => p.id,
            None => {
                let mut np = NewProxy {
                    name: info.name.clone(),
                    scheme,
                    host: info.host.clone(),
                    port: info.port,
                    username: info.username.clone(),
                    password: info.password.clone(),
                    ..Default::default()
                };
                match self.create_proxy(np.clone()).await {
                    Ok(v) => v.id,
                    Err(_) => {
                        np.name = format!("{} (synced)", info.name);
                        self.create_proxy(np).await?.id
                    }
                }
            }
        };
        self.assign(profile, Some(id)).await
    }

    // --- local side: plan ----------------------------------------------------------------------

    fn hub(&self) -> Result<&Arc<cm_sync::DeviceHub>> {
        self.device_hub().ok_or_else(|| refuse(503, "sync_unavailable", "device sync is not available on this device"))
    }

    /// Diff the two machines for `direction`: one row per source profile.
    pub async fn sync_plan(&self, device: &str, direction: Direction) -> Result<SyncPlan> {
        let hub = self.hub()?;
        let dev = hub.get(device).await?.ok_or_else(|| CmError::NotFound(format!("device {device}")))?;
        if dev.revoked_at.is_some() {
            bail!(CmError::Conflict("this device was revoked; pair again to sync".into()));
        }
        let peer = hub.client_with_timeout(device, Duration::from_secs(60)).await?;
        let remote: PeerManifest = peer.get_json("/sync/v1/profiles").await?;
        let local = self.sync_manifest(device).await?;
        let compat = local.env.check_compatible(&remote.env);
        let rows = match direction {
            Direction::Pull => build_rows(&remote.profiles, &local.profiles, &compat),
            Direction::Push => build_rows(&local.profiles, &remote.profiles, &compat),
        };
        Ok(SyncPlan {
            direction,
            peer: PlanPeer { id: dev.id, name: dev.name, env: remote.env },
            me: local.env,
            incompatible: compat.err(),
            rows,
        })
    }

    // --- local side: jobs ----------------------------------------------------------------------

    /// Start a sync job. Returns at once; poll [`Manager::sync_job`].
    pub async fn sync_start(self: &Arc<Self>, device: &str, req: StartRequest) -> Result<JobView> {
        if !req.acknowledged {
            bail!(CmError::Invalid("confirm first: the same login sessions will exist on both machines, and websites may notice".into()));
        }
        if req.items.is_empty() {
            bail!(CmError::Invalid("select at least one profile".into()));
        }
        let hub = self.hub()?.clone();
        let dev = hub.get(device).await?.ok_or_else(|| CmError::NotFound(format!("device {device}")))?;
        if self.sync.jobs.lock().unwrap().iter().any(|j| {
            let v = j.view.lock().unwrap();
            v.device_id == device && v.state == "running"
        }) {
            bail!(CmError::Conflict("a sync with this device is already running".into()));
        }
        let plan = self.sync_plan(device, req.direction).await?;
        let mut planned = Vec::new();
        let mut items = Vec::new();
        for it in &req.items {
            let row = plan.rows.iter().find(|r| r.source_id == it.profile_id);
            let mut view = ItemView {
                profile_id: it.profile_id.clone(),
                name: row.map(|r| r.name.clone()).unwrap_or_default(),
                action: it.action,
                state: "queued",
                bytes: 0,
                total: row.map(|r| r.size).unwrap_or(0),
                message: None,
                code: None,
                result_profile_id: None,
            };
            match row {
                None => {
                    view.state = "skipped";
                    view.code = Some("not_found".into());
                    view.message = Some("this profile no longer exists".into());
                }
                Some(r) if r.blocked.is_some() => {
                    let b = r.blocked.as_ref().unwrap();
                    view.state = "skipped";
                    view.code = Some(b.code.clone());
                    view.message = Some(b.detail.clone());
                }
                Some(_) if it.action == Action::Skip => {
                    view.state = "skipped";
                    view.code = Some("kept".into());
                    view.message = Some("kept as it is".into());
                }
                Some(r) if !r.actions.contains(&it.action) => {
                    view.state = "skipped";
                    view.code = Some("bad_action".into());
                    view.message = Some("that choice is not possible for this profile any more".into());
                }
                Some(r) => planned
                    .push((items.len(), Planned { source_id: r.source_id.clone(), action: it.action, target_id: r.target_id.clone() })),
            }
            items.push(view);
        }
        let job = Arc::new(Job {
            cancel: AtomicBool::new(false),
            view: Mutex::new(JobView {
                id: new_id()?,
                device_id: dev.id.clone(),
                device_name: dev.name.clone(),
                direction: req.direction,
                state: "running",
                cancelling: false,
                started_at: self.d.clock.now(),
                finished_at: None,
                items,
                done: 0,
                failed: 0,
                skipped: 0,
            }),
        });
        {
            let mut jobs = self.sync.jobs.lock().unwrap();
            jobs.push(job.clone());
            while jobs.len() > MAX_JOBS_KEPT {
                if let Some(pos) = jobs.iter().position(|j| j.view.lock().unwrap().state != "running") {
                    jobs.remove(pos);
                } else {
                    break;
                }
            }
        }
        let view = job.snapshot();
        let m = self.clone();
        tokio::spawn(async move { m.run_job(job, dev.name, req, planned).await });
        Ok(view)
    }

    pub fn sync_job(&self, id: &str) -> Option<JobView> {
        self.sync.jobs.lock().unwrap().iter().find(|j| j.view.lock().unwrap().id == id).map(|j| j.snapshot())
    }

    /// Newest first.
    pub fn sync_jobs(&self) -> Vec<JobView> {
        self.sync.jobs.lock().unwrap().iter().rev().map(|j| j.snapshot()).collect()
    }

    /// Ask a running job to stop after the current piece; the profile being applied stays as it was.
    pub fn sync_cancel(&self, id: &str) -> bool {
        match self.sync.jobs.lock().unwrap().iter().find(|j| j.view.lock().unwrap().id == id) {
            Some(j) if j.view.lock().unwrap().state == "running" => {
                j.cancel.store(true, Ordering::SeqCst);
                true
            }
            _ => false,
        }
    }

    async fn run_job(self: Arc<Self>, job: Arc<Job>, device_name: String, req: StartRequest, planned: Vec<(usize, Planned)>) {
        let (device, direction) = {
            let v = job.view.lock().unwrap();
            (v.device_id.clone(), v.direction)
        };
        let peer = match self.hub().map(Arc::clone) {
            Ok(h) => h.client_with_timeout(&device, CHUNK_TIMEOUT).await,
            Err(e) => Err(e),
        };
        for (idx, p) in planned {
            if job.cancel.load(Ordering::SeqCst) {
                job.item(idx, |i| {
                    i.state = "cancelled";
                    i.message = Some("cancelled before it started".into());
                });
                continue;
            }
            let peer = match &peer {
                Ok(c) => c,
                Err(e) => {
                    let msg = format!("{e:#}");
                    job.item(idx, |i| {
                        i.state = "failed";
                        i.message = Some(msg);
                        i.code = Some("unreachable".into());
                    });
                    continue;
                }
            };
            let res = match direction {
                Direction::Pull => self.pull_item(&job, idx, &device, &device_name, peer, &p, &req).await,
                Direction::Push => self.push_item(&job, idx, &device, peer, &p, &req).await,
            };
            match res {
                Ok(done) => job.item(idx, |i| {
                    i.state = "done";
                    i.bytes = i.total;
                    i.result_profile_id = Some(done.profile_id);
                    i.message = done.warning;
                }),
                Err(e) => {
                    let cancelled = e.chain().any(|c| matches!(c.downcast_ref::<TransferError>(), Some(TransferError::Cancelled)));
                    let code = e.chain().find_map(|c| c.downcast_ref::<PeerError>().map(|p| p.code().to_owned()));
                    let msg = format!("{e:#}");
                    job.item(idx, |i| {
                        i.state = if cancelled { "cancelled" } else { "failed" };
                        i.code = code.or_else(|| Some(if cancelled { "cancelled".into() } else { "failed".into() }));
                        i.message = Some(if cancelled { "cancelled; the profile on the destination was not changed".into() } else { msg });
                    });
                }
            }
        }
        let mut v = job.view.lock().unwrap();
        v.state = if job.cancel.load(Ordering::SeqCst) { "cancelled" } else { "done" };
        v.finished_at = Some(self.d.clock.now());
    }

    /// Pull one profile: prepare on the peer, fetch chunks (resuming), verify, apply here.
    #[allow(clippy::too_many_arguments)]
    async fn pull_item(
        &self,
        job: &Arc<Job>,
        idx: usize,
        device: &str,
        device_name: &str,
        peer: &PeerClient,
        p: &Planned,
        req: &StartRequest,
    ) -> Result<CommitResponse> {
        job.set(idx, "preparing");
        let store = self.in_store();
        store.gc(self.d.clock.now(), KEEP_SECS);
        let earlier = store.all().into_iter().find(|m| m.peer == device && m.profile_id == p.source_id);
        let me = self.sync_env().await;
        let prep: PrepareResponse = peer
            .post_json(
                "/sync/v1/transfer/prepare",
                &PrepareRequest {
                    profile_id: p.source_id.clone(),
                    env: me.clone(),
                    resume: earlier.as_ref().map(|m| m.id.clone()),
                    include_proxy: req.include_proxy,
                    include_proxy_password: req.include_proxy && req.include_proxy_password,
                },
            )
            .await?;
        if let Some(e) = earlier.filter(|m| m.id != prep.transfer_id) {
            store.remove(&e.id); // the peer exported anew: the old partial file is useless
        }
        self.gate(&prep.env, &me)?;
        if !cm_sync::transfer::valid_transfer_id(&prep.transfer_id)
            || prep.chunk == 0
            || prep.chunk > MAX_CHUNK
            || !is_hex_sha256(&prep.sha256)
            || prep.size > MAX_TOTAL_BYTES + 64 * 1024 * 1024
        {
            bail!(PeerError::Protocol { host: peer.host().into(), detail: "invalid transfer description".into() });
        }
        let meta = Meta {
            id: prep.transfer_id.clone(),
            peer: device.into(),
            profile_id: p.source_id.clone(),
            name: prep.name.clone(),
            size: prep.size,
            sha256: prep.sha256.clone(),
            chunk: prep.chunk,
            content_hash: prep.content_hash.clone(),
            created_at: self.d.clock.now(),
            action: Some(action_str(p.action).into()),
            target_id: p.target_id.clone(),
            proxy: prep.proxy.as_ref().map(serde_json::to_value).transpose()?,
        };
        let keep_created = store.load(&meta.id).map(|m| m.created_at).unwrap_or(meta.created_at);
        store.save(&Meta { created_at: keep_created, ..meta.clone() })?;
        let mut part = PartFile::open(&store.data_path(&meta.id), meta.size, &meta.sha256, meta.chunk)?;
        job.item(idx, |i| {
            i.state = "transferring";
            i.total = meta.size;
            i.bytes = part.received();
        });
        let src = PullSource { peer: peer.clone(), tid: meta.id.clone() };
        let j2 = job.clone();
        let progress = move |n: u64| j2.item(idx, |i| i.bytes = n);
        let transferred = pull_file(&src, &mut part, &job.cancel, &progress).await;
        if let Err(e) = transferred {
            if matches!(e, TransferError::Cancelled) {
                store.remove(&meta.id);
                let _ = peer.send(cm_sync::Method::POST, &format!("/sync/v1/transfer/{}/cancel", meta.id), Vec::new(), None).await;
            }
            return Err(e.into());
        }
        job.set(idx, "verifying");
        let part_path = part.path().to_path_buf();
        let finished = tokio::task::spawn_blocking(move || part.finish()).await?;
        if let Err(e) = finished {
            if matches!(e, TransferError::WholeHash) {
                store.remove(&meta.id);
            }
            return Err(e.into());
        }
        job.set(idx, "applying");
        let proxy = prep.proxy.clone();
        let done = self.sync_apply(&part_path, device, device_name, &p.source_id, p.action, p.target_id.as_deref(), proxy).await?;
        store.remove(&meta.id);
        let result = (p.action != Action::Copy).then(|| done.profile_id.clone());
        let ack: Result<serde_json::Value, PeerError> =
            peer.post_json(&format!("/sync/v1/transfer/{}/done", meta.id), &DoneRequest { result_profile_id: result }).await;
        if let Err(e) = ack {
            eprintln!("sync: could not tell {device_name} that {} was copied: {e}", p.source_id);
        }
        Ok(done)
    }

    /// Push one profile: export here, `begin` on the peer, send chunks (resuming), `commit`.
    async fn push_item(
        &self,
        job: &Arc<Job>,
        idx: usize,
        device: &str,
        peer: &PeerClient,
        p: &Planned,
        req: &StartRequest,
    ) -> Result<CommitResponse> {
        job.set(idx, "preparing");
        let store = self.out_store();
        let earlier = store.all().into_iter().find(|m| m.peer == device && m.profile_id == p.source_id).map(|m| m.id);
        let meta = self.sync_export(device, &p.source_id, earlier.as_deref()).await?;
        let proxy = if req.include_proxy { self.proxy_info_for(&p.source_id, req.include_proxy_password).await? } else { None };
        let me = self.sync_env().await;
        let begin: BeginResponse = peer
            .post_json(
                "/sync/v1/inbox/begin",
                &BeginRequest {
                    transfer_id: Some(meta.id.clone()),
                    profile_id: p.source_id.clone(),
                    name: meta.name.clone(),
                    size: meta.size,
                    sha256: meta.sha256.clone(),
                    chunk: meta.chunk,
                    content_hash: meta.content_hash.clone(),
                    env: me,
                    action: p.action,
                    target_id: p.target_id.clone(),
                    proxy,
                },
            )
            .await?;
        job.item(idx, |i| {
            i.state = "transferring";
            i.total = meta.size;
            i.bytes = begin.received;
        });
        let sink = PushSink { peer: peer.clone(), tid: begin.transfer_id.clone() };
        let j2 = job.clone();
        let progress = move |n: u64| j2.item(idx, |i| i.bytes = n);
        let sent = push_file(&sink, &store.data_path(&meta.id), meta.size, meta.chunk, &job.cancel, &progress).await;
        if let Err(e) = sent {
            if matches!(e, TransferError::Cancelled) {
                let _ = peer.send(cm_sync::Method::POST, &format!("/sync/v1/inbox/{}/cancel", begin.transfer_id), Vec::new(), None).await;
                store.remove(&meta.id);
            }
            return Err(e.into());
        }
        job.set(idx, "applying");
        let slow = self.hub()?.client_with_timeout(device, APPLY_TIMEOUT).await?;
        let done: CommitResponse = slow.post_json(&format!("/sync/v1/inbox/{}/commit", begin.transfer_id), &serde_json::json!({})).await?;
        if p.action != Action::Copy {
            self.set_link(device, &p.source_id, &done.profile_id, &meta.content_hash)?;
        }
        store.remove(&meta.id);
        Ok(done)
    }
}

// ---------------------------------------------------------------------------------------------
// Chunk endpoints over the signed peer client
// ---------------------------------------------------------------------------------------------

struct PullSource {
    peer: PeerClient,
    tid: String,
}

#[async_trait::async_trait]
impl ChunkSource for PullSource {
    async fn chunk(&self, index: u64) -> Result<(Vec<u8>, String), PeerError> {
        let r = self.peer.send(cm_sync::Method::GET, &format!("/sync/v1/transfer/{}/chunk/{index}", self.tid), Vec::new(), None).await?;
        let sha = r.headers().get(CHUNK_SHA_HEADER).and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
        let bytes = r.bytes().await.map_err(|e| PeerError::Protocol { host: self.peer.host().into(), detail: e.to_string() })?;
        Ok((bytes.to_vec(), sha))
    }
}

struct PushSink {
    peer: PeerClient,
    tid: String,
}

#[async_trait::async_trait]
impl ChunkSink for PushSink {
    async fn put(&self, index: u64, bytes: Vec<u8>, sha256: &str) -> Result<u64, PeerError> {
        let r = self
            .peer
            .send(
                cm_sync::Method::PUT,
                &format!("/sync/v1/inbox/{}/chunk/{index}?sha={sha256}", self.tid),
                bytes,
                Some("application/octet-stream"),
            )
            .await?;
        let ack: ChunkAck = r.json().await.map_err(|e| PeerError::Protocol { host: self.peer.host().into(), detail: e.to_string() })?;
        Ok(ack.received)
    }

    async fn received(&self) -> Result<u64, PeerError> {
        let r: BeginResponse = self.peer.get_json(&format!("/sync/v1/inbox/{}", self.tid)).await?;
        Ok(r.received)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::AtomicU32;
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("cm-syncsvc-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn content_hash_follows_content_not_names_or_mtimes() {
        let d = tmp("hash");
        fs::write(d.join("cookies.sqlite"), b"abc").unwrap();
        fs::create_dir_all(d.join("cache2")).unwrap();
        fs::write(d.join("cache2/x"), b"cache").unwrap();
        let cache: HashCache = Default::default();
        let rec = serde_json::json!({ "os": "mac", "machine": { "a": 1 } });
        let (h1, size) = compute_hash(&d, &rec, &cache).unwrap();
        assert_eq!(size, 3, "caches are not part of the profile");
        assert_eq!(compute_hash(&d, &rec, &cache).unwrap().0, h1, "stable, and served from the cache");
        // touching a cache dir changes nothing
        fs::write(d.join("cache2/y"), b"more").unwrap();
        assert_eq!(compute_hash(&d, &rec, &Default::default()).unwrap().0, h1);
        // a changed file, a new file, or a changed record each change the hash
        fs::write(d.join("cookies.sqlite"), b"abd").unwrap();
        let h2 = compute_hash(&d, &rec, &cache).unwrap().0;
        assert_ne!(h1, h2);
        fs::write(d.join("extra"), b"1").unwrap();
        let h3 = compute_hash(&d, &rec, &cache).unwrap().0;
        assert_ne!(h2, h3);
        let rec2 = serde_json::json!({ "os": "mac", "machine": { "a": 2 } });
        assert_ne!(compute_hash(&d, &rec2, &cache).unwrap().0, h3);
        // same bytes elsewhere: same hash
        let e = tmp("hash2");
        fs::write(e.join("cookies.sqlite"), b"abd").unwrap();
        fs::write(e.join("extra"), b"1").unwrap();
        assert_eq!(compute_hash(&e, &rec, &Default::default()).unwrap().0, h3);
        let _ = fs::remove_dir_all(&d);
        let _ = fs::remove_dir_all(&e);
    }

    #[test]
    fn actions_roundtrip() {
        for a in [Action::Create, Action::Replace, Action::Copy] {
            assert_eq!(parse_action(Some(action_str(a))), Some(a));
        }
        assert_eq!(parse_action(Some("skip")), None);
    }

    // --- two in-process apps exchanging a profile through the peer-side methods -------------------

    use crate::Config;
    use cm_core::{
        Engine, EngineStatus, LaunchOptions, MachineSummary, NewProfile, Os, Profile, Proxy, ProxyCheck, ProxyChecker, Session, SystemClock,
    };
    use cm_store::*;
    use cm_sync::Status;
    use serde_json::{Value, json};
    use std::sync::atomic::AtomicBool;

    struct TestEngine {
        root: PathBuf,
        build: Option<String>,
        running: AtomicBool,
    }

    #[async_trait::async_trait]
    impl Engine for TestEngine {
        fn name(&self) -> &'static str {
            "Test"
        }
        fn ws_protocol(&self) -> &'static str {
            "Playwright"
        }
        async fn status(&self) -> EngineStatus {
            EngineStatus { ready: true, hint: None, version: self.build.clone() }
        }
        async fn generate(&self, os: Os) -> Result<Value> {
            Ok(json!({ "os": os.code() }))
        }
        fn summarize(&self, _m: &Value) -> MachineSummary {
            MachineSummary::default()
        }
        async fn launch(&self, _p: &Profile, _o: &LaunchOptions) -> Result<Session> {
            Ok(Session { ws_url: "ws://127.0.0.1:1/x".into() })
        }
        async fn stop(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn alive(&self, _id: &str) -> Option<bool> {
            Some(self.running.load(Ordering::SeqCst))
        }
        async fn hold_closed(&self, _id: &str) -> Result<Box<dyn std::any::Any + Send>> {
            Ok(Box::new(()))
        }
        fn data_dir(&self, id: &str) -> PathBuf {
            self.root.join("profiles").join(id).join("user-data")
        }
        async fn delete_data(&self, _id: &str) -> Result<()> {
            Ok(())
        }
    }

    struct NoCheck;
    #[async_trait::async_trait]
    impl ProxyChecker for NoCheck {
        async fn check(&self, _p: &Proxy) -> Result<ProxyCheck> {
            bail!("no network in tests")
        }
    }

    struct App {
        root: PathBuf,
        m: Manager,
        engine: Arc<TestEngine>,
    }

    fn app(tag: &str, build: Option<&str>) -> App {
        let root = tmp(tag);
        let db = Db::open(&root).unwrap();
        let engine = Arc::new(TestEngine { root: root.clone(), build: build.map(str::to_owned), running: AtomicBool::new(false) });
        let deps = crate::Deps {
            engine: engine.clone(),
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
            api_keys: Arc::new(SqliteApiKeys::new(db.clone())),
            clock: Arc::new(SystemClock),
        };
        let m = Manager::new(deps, Config { proxy_base_port: 45998, data_dir: root.clone(), db_path: db.path().map(Path::to_path_buf) });
        App { root, m, engine }
    }

    fn write_data(dir: &Path, cookies: &str) {
        fs::create_dir_all(dir.join("storage")).unwrap();
        fs::write(dir.join("cookies.sqlite"), cookies).unwrap();
        // incompressible, so the zip spans several 1 KiB chunks
        let mut x = 0x9E37_79B9u32;
        let noise: Vec<u8> = (0..6000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        fs::write(dir.join("storage/ls"), noise).unwrap();
    }

    fn cookies(a: &App, id: &str) -> String {
        fs::read_to_string(a.engine.data_dir(id).join("cookies.sqlite")).unwrap()
    }

    fn prep_req(profile: &str, env: Env) -> PrepareRequest {
        PrepareRequest { profile_id: profile.into(), env, resume: None, include_proxy: false, include_proxy_password: false }
    }

    /// Move a profile A -> B the way the HTTP routes would (A serves, B receives).
    async fn transfer(a: &App, b: &App, profile: &str, action: Action, target: Option<&str>) -> Result<CommitResponse> {
        let prep = a.m.sync_prepare("devB", prep_req(profile, b.m.sync_env().await)).await?;
        let begin =
            b.m.sync_begin(
                "devA",
                BeginRequest {
                    transfer_id: None,
                    profile_id: profile.into(),
                    name: prep.name.clone(),
                    size: prep.size,
                    sha256: prep.sha256.clone(),
                    chunk: 1024,
                    content_hash: prep.content_hash.clone(),
                    env: prep.env.clone(),
                    action,
                    target_id: target.map(str::to_owned),
                    proxy: None,
                },
            )
            .await?;
        let zip = fs::read(a.m.out_store().data_path(&prep.transfer_id)).unwrap();
        for (i, c) in zip.chunks(1024).enumerate() {
            b.m.sync_put_chunk("devA", &begin.transfer_id, i as u64, c.to_vec(), sha256_hex(c)).await?;
        }
        let done = b.m.sync_commit("devA", "Mac A", &begin.transfer_id).await?;
        a.m.sync_done(
            "devB",
            &prep.transfer_id,
            DoneRequest { result_profile_id: (action != Action::Copy).then(|| done.profile_id.clone()) },
        )
        .await?;
        Ok(done)
    }

    async fn rows(a: &App, b: &App) -> Vec<Row> {
        let (ma, mb) = (a.m.sync_manifest("devB").await.unwrap(), b.m.sync_manifest("devA").await.unwrap());
        build_rows(&ma.profiles, &mb.profiles, &ma.env.check_compatible(&mb.env))
    }

    fn code(e: &anyhow::Error) -> &'static str {
        e.downcast_ref::<SyncError>().map(|s| s.code).unwrap_or("-")
    }

    #[tokio::test]
    async fn profile_travels_updates_and_conflicts() {
        let (a, b) = (app("svc-a", Some("100")), app("svc-b", Some("100")));
        let v = a.m.create_profile(NewProfile { name: "Shop".into(), os: "mac".into() }).await.unwrap();
        write_data(&a.engine.data_dir(&v.id), "v1");

        // 1. new on B
        let r = rows(&a, &b).await;
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].status, Status::MissingOnTarget);
        let done = transfer(&a, &b, &v.id, Action::Create, None).await.unwrap();
        assert_eq!(done.profile_id, v.id, "free id is kept");
        assert_eq!(cookies(&b, &v.id), "v1");
        assert_eq!(b.m.profile(&v.id).await.unwrap().machine, a.m.profile(&v.id).await.unwrap().machine, "same pinned fingerprint");
        assert_eq!(rows(&a, &b).await[0].status, Status::Identical);
        assert!(a.m.out_store().all().is_empty() && b.m.in_store().all().is_empty(), "transfer files are cleaned up");

        // 2. only A changed -> Replace updates B and keeps its local name
        write_data(&a.engine.data_dir(&v.id), "v2");
        let local = b.m.profile(&v.id).await.unwrap();
        b.m.d.profiles.update(&Profile { name: "Shop on B".into(), ..local }).await.unwrap();
        let r = rows(&a, &b).await;
        assert_eq!(r[0].status, Status::SourceChanged);
        assert_eq!(r[0].default_action, Action::Replace);
        transfer(&a, &b, &v.id, Action::Replace, r[0].target_id.as_deref()).await.unwrap();
        assert_eq!(cookies(&b, &v.id), "v2");
        assert_eq!(b.m.profile(&v.id).await.unwrap().name, "Shop on B");
        assert_eq!(rows(&a, &b).await[0].status, Status::Identical);

        // 3. only B changed
        write_data(&b.engine.data_dir(&v.id), "b3");
        assert_eq!(rows(&a, &b).await[0].status, Status::TargetChanged);

        // 4. both changed -> conflict; "save as a copy" gives B a second profile and touches nothing else
        write_data(&a.engine.data_dir(&v.id), "a4");
        assert_eq!(rows(&a, &b).await[0].status, Status::Conflict);
        let copy = transfer(&a, &b, &v.id, Action::Copy, None).await.unwrap();
        assert_ne!(copy.profile_id, v.id);
        assert!(copy.name.contains("(from Mac A)"), "{}", copy.name);
        assert_eq!(cookies(&b, &copy.profile_id), "a4");
        assert_eq!(cookies(&b, &v.id), "b3", "my version is untouched");
        assert_eq!(rows(&a, &b).await[0].status, Status::Conflict, "a copy resolves nothing");
        let _ = (fs::remove_dir_all(&a.root), fs::remove_dir_all(&b.root));
    }

    #[tokio::test]
    async fn trashed_profile_is_restored_by_an_incoming_update_and_never_offered() {
        let (a, b) = (app("svc-ta", Some("100")), app("svc-tb", Some("100")));
        let v = a.m.create_profile(NewProfile { name: "Shop".into(), os: "mac".into() }).await.unwrap();
        write_data(&a.engine.data_dir(&v.id), "v1");
        transfer(&a, &b, &v.id, Action::Create, None).await.unwrap();

        // B trashes it: it is not offered to A, and B keeps a link row for A
        b.m.delete_profile(&v.id).await.unwrap();
        let mb = b.m.sync_manifest("devA").await.unwrap();
        assert!(mb.profiles.iter().all(|e| e.id != v.id || e.trashed), "a trashed profile is marked, never a source");
        let back = build_rows(&mb.profiles, &a.m.sync_manifest("devB").await.unwrap().profiles, &Ok(()));
        assert!(back.is_empty(), "the trashed profile is not offered to the peer: {back:?}");
        assert!(b.m.sync_prepare("devA", prep_req(&v.id, a.m.sync_env().await)).await.is_err(), "nor exported");

        // A changes it: for B this is an update of its trashed profile, not a new one
        write_data(&a.engine.data_dir(&v.id), "v2");
        let r = rows(&a, &b).await;
        assert_eq!((r[0].status, r[0].target_id.as_deref()), (Status::SourceChanged, Some(v.id.as_str())), "{r:?}");
        transfer(&a, &b, &v.id, Action::Replace, r[0].target_id.as_deref()).await.unwrap();
        assert_eq!(b.m.d.profiles.list().await.unwrap().len(), 1, "no duplicate");
        assert!(b.m.trashed_profiles().await.unwrap().profiles.is_empty(), "restored");
        assert_eq!(cookies(&b, &v.id), "v2");
        assert_eq!(rows(&a, &b).await[0].status, Status::Identical);

        // a trashed profile whose peer never synced it (same id, unlinked) is also found, not duplicated
        let w = a.m.create_profile(NewProfile { name: "Other".into(), os: "mac".into() }).await.unwrap();
        write_data(&a.engine.data_dir(&w.id), "w1");
        transfer(&a, &b, &w.id, Action::Create, None).await.unwrap();
        b.m.delete_profile(&w.id).await.unwrap();
        let r = rows(&a, &b).await;
        let row = r.iter().find(|x| x.source_id == w.id).unwrap();
        assert_eq!(row.target_id.as_deref(), Some(w.id.as_str()));

        // purge forgets the sync state of the profile
        b.m.purge_profile(&w.id).await.unwrap();
        let n: i64 =
            state_conn(&b.m).unwrap().query_row("SELECT COUNT(*) FROM sync_state WHERE profile_id = ?1", [&w.id], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "sync_state rows are removed with the purged profile");
        let _ = (fs::remove_dir_all(&a.root), fs::remove_dir_all(&b.root));
    }

    #[tokio::test]
    async fn gate_running_resume_and_damage() {
        let (a, b) = (app("svc-ga", Some("100")), app("svc-gb", Some("101")));
        let v = a.m.create_profile(NewProfile { name: "Shop".into(), os: "mac".into() }).await.unwrap();
        write_data(&a.engine.data_dir(&v.id), "v1");
        // different Camoufox build: refused at prepare, and the rows say why
        let e = transfer(&a, &b, &v.id, Action::Create, None).await.unwrap_err();
        assert_eq!(code(&e), "sync_incompatible", "{e:#}");
        assert_eq!(rows(&a, &b).await[0].blocked.as_ref().unwrap().code, "incompatible");

        let b = app("svc-gc", Some("100"));
        // a running profile is refused with a clear reason
        a.engine.running.store(true, Ordering::SeqCst);
        let e = transfer(&a, &b, &v.id, Action::Create, None).await.unwrap_err();
        assert_eq!(code(&e), "profile_running");
        assert!(format!("{e:#}").contains("close it first"));
        assert_eq!(rows(&a, &b).await[0].blocked.as_ref().unwrap().code, "source_running");
        a.engine.running.store(false, Ordering::SeqCst);

        // B has the profile linked with "OLD" data; damaged transfers must not touch it
        transfer(&a, &b, &v.id, Action::Create, None).await.unwrap();
        write_data(&b.engine.data_dir(&v.id), "OLD");
        write_data(&a.engine.data_dir(&v.id), "NEW");
        let prep = a.m.sync_prepare("devB", prep_req(&v.id, b.m.sync_env().await)).await.unwrap();
        let zip = fs::read(a.m.out_store().data_path(&prep.transfer_id)).unwrap();
        let begin_req = |sha: &str, tid: Option<String>| BeginRequest {
            transfer_id: tid,
            profile_id: v.id.clone(),
            name: prep.name.clone(),
            size: prep.size,
            sha256: sha.into(),
            chunk: 1024,
            content_hash: prep.content_hash.clone(),
            env: prep.env.clone(),
            action: Action::Replace,
            target_id: Some(v.id.clone()),
            proxy: None,
        };
        // wrong chunk checksum
        let begin = b.m.sync_begin("devA", begin_req(&prep.sha256, None)).await.unwrap();
        let e = b.m.sync_put_chunk("devA", &begin.transfer_id, 0, zip[..1024].to_vec(), sha256_hex(b"x")).await.unwrap_err();
        assert_eq!(code(&e), "bad_chunk");
        // an out-of-order chunk reports the confirmed offset
        let e =
            b.m.sync_put_chunk("devA", &begin.transfer_id, 2, zip[2048..3072].to_vec(), sha256_hex(&zip[2048..3072])).await.unwrap_err();
        assert_eq!((code(&e), e.downcast_ref::<SyncError>().unwrap().received), ("bad_offset", Some(0)));
        // another device cannot use my transfer id
        let e = b.m.sync_put_chunk("devEvil", &begin.transfer_id, 0, vec![], String::new()).await.unwrap_err();
        assert_eq!(code(&e), "no_transfer");

        // resume: two chunks arrive, the link drops, begin again with the same id continues at 2048
        for (i, c) in zip.chunks(1024).enumerate().take(2) {
            b.m.sync_put_chunk("devA", &begin.transfer_id, i as u64, c.to_vec(), sha256_hex(c)).await.unwrap();
        }
        let again = b.m.sync_begin("devA", begin_req(&prep.sha256, Some(begin.transfer_id.clone()))).await.unwrap();
        assert_eq!((again.transfer_id.as_str(), again.received), (begin.transfer_id.as_str(), 2048));
        assert_eq!(b.m.sync_inbox_status("devA", &begin.transfer_id).await.unwrap().received, 2048);

        // whole-file hash mismatch: every chunk verifies on its own but the declared file hash is wrong
        let bad = b.m.sync_begin("devA", begin_req(&sha256_hex(b"other file"), None)).await.unwrap();
        for (i, c) in zip.chunks(1024).enumerate() {
            b.m.sync_put_chunk("devA", &bad.transfer_id, i as u64, c.to_vec(), sha256_hex(c)).await.unwrap();
        }
        let e = b.m.sync_commit("devA", "Mac A", &bad.transfer_id).await.unwrap_err();
        assert_eq!(code(&e), "bad_file");
        assert!(b.m.in_store().load(&bad.transfer_id).is_none(), "a damaged file is discarded");
        assert_eq!(cookies(&b, &v.id), "OLD", "the target's profile was never touched");

        // the resumed transfer finishes and applies
        for (i, c) in zip.chunks(1024).enumerate().skip(2) {
            b.m.sync_put_chunk("devA", &begin.transfer_id, i as u64, c.to_vec(), sha256_hex(c)).await.unwrap();
        }
        b.m.sync_commit("devA", "Mac A", &begin.transfer_id).await.unwrap();
        assert_eq!(cookies(&b, &v.id), "NEW");
        assert!(b.m.in_store().all().is_empty());
        let _ = (fs::remove_dir_all(&a.root), fs::remove_dir_all(&b.root));
    }
}
