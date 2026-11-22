//! Settings (phases 1, 12): UI language, headless default, engine status + versions, optional
//! Camoufox install/update with a pollable log, data dir, named API keys.
//!
//! API keys look like `cmk_<id>_<secret>`: `id` (16 hex) is public and finds the row, `secret`
//! (48 hex, 192 random bits) is shown once at creation. Only `sha256(salt || secret)` is stored;
//! secrets are random and long, so a salted fast hash is enough (no password-guessing risk).

use crate::Manager;
use anyhow::{Result, bail};
use cm_core::{
    ApiKeyInfo, ApiKeyRecord, CmError, EngineHint, EngineVersions,
    settings::{HEADLESS, LANGUAGE, LANGUAGES, REQUIRE_API_KEY},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub name: &'static str,
    pub ready: bool,
    pub hint: Option<EngineHint>,
    /// One-line summary (`0.5.9 · browser 158.0.1-beta.41`).
    pub version: Option<String>,
    pub versions: Option<EngineVersions>,
    pub install_running: bool,
    /// Paths/version for manual install steps (`None` when the engine cannot install).
    #[serde(flatten)]
    pub install: Option<cm_core::InstallInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// `en` | `vi`
    pub language: String,
    pub data_dir: String,
    pub db_path: Option<String>,
    pub proxy_base_port: u16,
    /// Port this request reached; filled in by the API layer (the service does not know it).
    pub ui_port: Option<u16>,
    /// Default for launches (`false` when never set).
    pub headless: bool,
    /// Non-UI clients must send a valid `X-API-Key`.
    pub require_api_key: bool,
    pub engine: EngineInfo,
}

/// Partial update: only present fields change.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub language: Option<String>,
    pub headless: Option<bool>,
    pub require_api_key: Option<bool>,
}

/// A freshly created key: the only time the secret is visible.
#[derive(Serialize)]
pub struct CreatedKey {
    #[serde(flatten)]
    pub info: ApiKeyInfo,
    pub key: String,
}

/// Why a presented key was refused (all map to 401 `invalid_api_key`).
#[derive(Debug, PartialEq)]
pub enum KeyError {
    Malformed,
    Unknown,
    Revoked,
}

/// `n` random bytes as hex, from the OS (macOS only target, so `/dev/urandom` is always there).
fn random_hex(n: usize) -> Result<String> {
    let mut buf = vec![0u8; n];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(hex(&buf))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hash(salt: &str, secret: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(secret.as_bytes());
    hex(&h.finalize())
}

/// Comparison that does not stop at the first differing byte.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ---- engine install job (one at a time, process-wide) ----

const MAX_LOG_LINES: usize = 4000;

#[derive(Default)]
struct Job {
    running: bool,
    /// Absolute index of `lines[0]`, so pollers keep their position after old lines are dropped.
    base: usize,
    lines: Vec<String>,
    /// `Some(Ok)` / `Some(Err(message))` once the last run finished.
    result: Option<std::result::Result<(), String>>,
}

fn job() -> &'static Mutex<Job> {
    static JOB: OnceLock<Mutex<Job>> = OnceLock::new();
    JOB.get_or_init(Default::default)
}

/// An engine install is running right now (also exposed on the overview).
pub(crate) fn install_running() -> bool {
    job().lock().unwrap_or_else(|e| e.into_inner()).running
}

fn push_line(line: String) {
    let mut j = job().lock().unwrap_or_else(|e| e.into_inner());
    j.lines.push(line);
    if j.lines.len() > MAX_LOG_LINES {
        let drop = j.lines.len() - MAX_LOG_LINES;
        j.lines.drain(..drop);
        j.base += drop;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallLog {
    pub running: bool,
    /// Lines from the requested position on.
    pub lines: Vec<String>,
    /// Position to ask for next time.
    pub next: usize,
    /// `None` while running or never run, else whether the last run succeeded.
    pub ok: Option<bool>,
    pub error: Option<String>,
}

impl Manager {
    pub async fn language(&self) -> String {
        match self.d.settings.get(LANGUAGE).await {
            Ok(Some(v)) => v.as_str().filter(|l| LANGUAGES.contains(l)).unwrap_or("en").to_string(),
            _ => "en".into(),
        }
    }

    /// Headless default chosen in settings; `None` = never chosen (the engine's own default applies).
    pub async fn headless_default(&self) -> Option<bool> {
        self.d.settings.get(HEADLESS).await.ok().flatten().and_then(|v| v.as_bool())
    }

    pub async fn require_api_key(&self) -> bool {
        self.d.settings.get(REQUIRE_API_KEY).await.ok().flatten().and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub async fn settings(&self) -> SettingsView {
        let status = self.d.engine.status().await;
        let versions = self.d.engine.versions().await;
        SettingsView {
            language: self.language().await,
            data_dir: self.cfg.data_dir.display().to_string(),
            db_path: self.cfg.db_path.as_ref().map(|p| p.display().to_string()),
            proxy_base_port: self.cfg.proxy_base_port,
            ui_port: None,
            headless: self.headless_default().await.unwrap_or(false),
            require_api_key: self.require_api_key().await,
            engine: EngineInfo {
                name: self.d.engine.name(),
                ready: status.ready,
                hint: status.hint,
                version: status.version,
                versions,
                install_running: install_running(),
                install: self.d.engine.install_info(),
            },
        }
    }

    pub async fn update_settings(&self, patch: SettingsPatch) -> Result<SettingsView> {
        if let Some(lang) = patch.language {
            if !LANGUAGES.contains(&lang.as_str()) {
                bail!(CmError::Invalid(format!("unsupported language {lang}")));
            }
            self.d.settings.set(LANGUAGE, &json!(lang)).await?;
        }
        if let Some(h) = patch.headless {
            self.d.settings.set(HEADLESS, &json!(h)).await?;
        }
        if let Some(r) = patch.require_api_key {
            self.d.settings.set(REQUIRE_API_KEY, &json!(r)).await?;
        }
        Ok(self.settings().await)
    }

    // ---- engine install / update ----

    /// Start installing or updating the engine in the background. Errors when one is already running.
    pub fn start_engine_install(&self) -> Result<()> {
        {
            let mut j = job().lock().unwrap_or_else(|e| e.into_inner());
            if j.running {
                bail!(CmError::Conflict("an install is already running".into()));
            }
            j.running = true;
            j.result = None;
        }
        push_line(format!("== {} install / update", self.d.engine.name()));
        let engine = Arc::clone(&self.d.engine);
        tokio::spawn(async move {
            let r = engine.install(&push_line).await;
            let mut j = job().lock().unwrap_or_else(|e| e.into_inner());
            j.running = false;
            j.result = Some(r.map_err(|e| format!("{e:#}")));
        });
        Ok(())
    }

    /// Log lines from absolute position `from` (0 = everything still kept).
    pub fn engine_install_log(&self, from: usize) -> InstallLog {
        let j = job().lock().unwrap_or_else(|e| e.into_inner());
        let start = from.max(j.base);
        let lines = j.lines.get(start - j.base..).unwrap_or_default().to_vec();
        let (ok, error) = match (&j.result, j.running) {
            (Some(Ok(())), false) => (Some(true), None),
            (Some(Err(e)), false) => (Some(false), Some(e.clone())),
            _ => (None, None),
        };
        InstallLog { running: j.running, next: j.base + j.lines.len(), lines, ok, error }
    }

    // ---- API keys ----

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeyInfo>> {
        self.d.api_keys.list().await
    }

    pub async fn create_api_key(&self, name: &str) -> Result<CreatedKey> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            bail!(CmError::Invalid("key name must be 1-60 characters".into()));
        }
        let (id, secret, salt) = (random_hex(8)?, random_hex(24)?, random_hex(16)?);
        let info =
            ApiKeyInfo { id: id.clone(), name: name.to_string(), created_at: self.d.clock.now_iso(), last_used_at: None, revoked_at: None };
        self.d.api_keys.insert(&ApiKeyRecord { info: info.clone(), hash: hash(&salt, &secret), salt }).await?;
        Ok(CreatedKey { info, key: format!("cmk_{id}_{secret}") })
    }

    pub async fn revoke_api_key(&self, id: &str) -> Result<()> {
        if !self.d.api_keys.revoke(id, &self.d.clock.now_iso()).await? {
            bail!(CmError::NotFound("api key".into()));
        }
        Ok(())
    }

    /// Check a presented key; on success record its use and return it.
    pub async fn verify_api_key(&self, presented: &str) -> Result<ApiKeyInfo, KeyError> {
        let rest = presented.trim().strip_prefix("cmk_").ok_or(KeyError::Malformed)?;
        let (id, secret) = rest.split_once('_').ok_or(KeyError::Malformed)?;
        let rec = self.d.api_keys.get(id).await.ok().flatten().ok_or(KeyError::Unknown)?;
        if !same(&rec.hash, &hash(&rec.salt, secret)) {
            return Err(KeyError::Unknown);
        }
        if rec.info.revoked_at.is_some() {
            return Err(KeyError::Revoked);
        }
        let now = self.d.clock.now_iso();
        let _ = self.d.api_keys.touch(id, &now).await;
        Ok(ApiKeyInfo { last_used_at: Some(now), ..rec.info })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::manager;

    use super::*;

    #[tokio::test]
    async fn keys_create_verify_revoke() {
        let (m, _) = manager(true, 0);
        let k = m.create_api_key("  claude on laptop ").await.unwrap();
        assert_eq!(k.info.name, "claude on laptop");
        assert!(k.key.starts_with("cmk_"));
        assert!(m.create_api_key(" ").await.is_err());

        let info = m.verify_api_key(&k.key).await.unwrap();
        assert!(info.last_used_at.is_some());
        assert!(m.list_api_keys().await.unwrap()[0].last_used_at.is_some());
        // The secret is not stored anywhere in the listing.
        let secret = k.key.rsplit('_').next().unwrap();
        assert!(!serde_json::to_string(&m.list_api_keys().await.unwrap()).unwrap_or_default().contains(secret));

        assert_eq!(m.verify_api_key("cmk_nope_x").await, Err(KeyError::Unknown));
        assert_eq!(m.verify_api_key("garbage").await, Err(KeyError::Malformed));
        let wrong = format!("cmk_{}_{}", k.info.id, "0".repeat(48));
        assert_eq!(m.verify_api_key(&wrong).await, Err(KeyError::Unknown));

        m.revoke_api_key(&k.info.id).await.unwrap();
        assert_eq!(m.verify_api_key(&k.key).await, Err(KeyError::Revoked));
        assert!(m.revoke_api_key(&k.info.id).await.is_err());
    }

    #[tokio::test]
    async fn settings_roundtrip() {
        let (m, _) = manager(true, 0);
        assert!(!m.settings().await.headless);
        let v = m.update_settings(SettingsPatch { headless: Some(true), require_api_key: Some(true), ..Default::default() }).await.unwrap();
        assert!(v.headless && v.require_api_key && v.engine.ready);
        assert_eq!(m.headless_default().await, Some(true));
        assert!(m.update_settings(SettingsPatch { language: Some("xx".into()), ..Default::default() }).await.is_err());
    }

    #[test]
    fn constant_time_compare() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "ab"));
    }
}
