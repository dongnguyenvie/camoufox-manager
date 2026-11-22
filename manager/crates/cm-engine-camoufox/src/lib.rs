//! Camoufox engine (Firefox anti-detect, https://github.com/daijro/camoufox), all local.
//!
//! - On disk: `<root>/profiles/<id>/user-data/` (persistent Firefox profile: cookies, logins,
//!   history), `launch.json` (pinned machine handed to the helper) and `launch.log`.
//!   Profile records themselves live in the database (`ProfileRepo`).
//! - The pinned machine is generated once by Camoufox's generator (`helper generate`).
//! - Launch: `helper launch` (Playwright persistent context + `Browser.bind`) prints a WebSocket
//!   endpoint speaking the **Playwright protocol** — `playwright.firefox.connect(ws)` (D5).
//! - Optional install: without python/camoufox the app still runs and [`Engine::status`]
//!   says why (`manager/camoufox/setup.sh`).
//!
//! Modules: `helper` (embedded python), `machine` (pinned machine → summary, phase 4),
//! `process` (spawn/stop/alive, phase 8), `lock` (per-profile lock + orphan cleanup, phase 8).

mod closed;
mod cookies;
mod diag;
mod extensions;
mod helper;
mod lock;
mod machine;
mod places;
mod process;
mod version;

pub use diag::{HelperState, Orphan, find_orphans, helper_state};

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use cm_core::{
    CmError, CrashInfo, Engine, EngineHint, EngineStatus, EngineVersions, InstallInfo, LaunchOptions, MachineSummary, Os, Profile, Session,
    cookies::{Cookie, CookieFormat, CookieImportMode},
    profile::valid_id,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Mutex};

pub const SETUP_SCRIPT: &str = "manager/camoufox/setup.sh";

pub struct Camoufox {
    root: PathBuf,
    python: PathBuf,
    helper: PathBuf,
    headless: bool,
    procs: process::Procs,
    /// Last `helper check` result (python takes ~0.5s, so it is cached).
    check: Mutex<Option<(Instant, std::result::Result<(), EngineHint>)>>,
    /// Package/browser versions (python probe, cached like `check`; phase 12).
    versions: Mutex<Option<(Instant, Option<EngineVersions>)>>,
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn check_id(id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err(CmError::Invalid("invalid profile id".into()).into());
    }
    Ok(())
}

impl Camoufox {
    /// `root` = `<data_dir>/camoufox`. Never fails when Camoufox is not installed.
    pub fn new(root: PathBuf, python: PathBuf, headless: bool) -> Self {
        let _ = fs::create_dir_all(root.join("profiles"));
        let helper = helper::install(&root);
        // Stop browsers/helpers a crashed run left behind (matched by our lock files and the
        // profile directory in their command line — never by process name).
        let procs = process::Procs::default();
        for id in lock::recover_orphans(&root.join("profiles")) {
            println!("Stopped leftovers of a previous run in profile {id}");
            procs.mark_crashed(&id, "the app stopped unexpectedly while this profile was open; leftovers were cleaned up");
        }
        Self { root, python, helper, headless, procs, check: Mutex::new(None), versions: Mutex::new(None) }
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.root.join("profiles").join(id)
    }

    /// Python + camoufox package + browser installed? `Err` carries the hint for the UI.
    async fn installed(&self) -> std::result::Result<(), EngineHint> {
        if !self.python.is_file() {
            return Err(EngineHint { code: "not_installed", detail: format!("python not found at {}", self.python.display()) });
        }
        let mut c = self.check.lock().await;
        if let Some((at, r)) = &*c {
            // Installed: remember for long; not installed: re-check soon so a fresh install is picked up.
            let ttl = if r.is_ok() { 300 } else { 15 };
            if at.elapsed() < Duration::from_secs(ttl) {
                return r.clone();
            }
        }
        let r = match helper::run(&self.python, &self.helper, &["check"], Duration::from_secs(30)).await {
            Ok(v) if v["ok"].as_bool() == Some(true) => Ok(()),
            Ok(v) => Err(EngineHint {
                code: if v["code"] == "browser_missing" { "browser_missing" } else { "not_installed" },
                detail: v["error"].as_str().unwrap_or("Camoufox is not fully installed").to_string(),
            }),
            Err(e) => Err(EngineHint { code: "not_installed", detail: format!("{e:#}") }),
        };
        *c = Some((Instant::now(), r.clone()));
        r
    }

    /// Export/import on the profile's user-data (sqlite only: no python, no browser start).
    /// Returns (exported cookies, count).
    async fn cookie_op(&self, p: &Profile, import: Option<(Vec<Cookie>, CookieImportMode)>) -> Result<(Vec<Cookie>, usize)> {
        check_id(&p.id)?;
        if self.procs.alive(&p.id).await == Some(true) {
            bail!(CmError::Conflict("profile is running".into()));
        }
        let user_data = self.data_dir(&p.id);
        tokio::task::spawn_blocking(move || match import {
            Some((list, mode)) => cookies::import(&user_data, &list, mode).map(|n| (Vec::new(), n)),
            None => cookies::export(&user_data).map(|l| {
                let n = l.len();
                (l, n)
            }),
        })
        .await?
    }

    async fn ensure_installed(&self) -> Result<()> {
        self.installed().await.map_err(|h| anyhow!("Camoufox is not ready: {} — run {SETUP_SCRIPT}", h.detail))
    }
}

#[async_trait]
impl Engine for Camoufox {
    fn name(&self) -> &'static str {
        "Camoufox"
    }

    fn ws_protocol(&self) -> &'static str {
        "Playwright"
    }

    async fn status(&self) -> EngineStatus {
        let r = self.installed().await;
        let version = self.versions().await.as_ref().and_then(version::summary);
        EngineStatus { ready: r.is_ok(), hint: r.err(), version }
    }

    async fn versions(&self) -> Option<EngineVersions> {
        if !self.python.is_file() {
            return None;
        }
        let mut c = self.versions.lock().await;
        if let Some((at, v)) = &*c
            && at.elapsed() < Duration::from_secs(if v.is_some() { 300 } else { 15 })
        {
            return v.clone();
        }
        let v = version::detect(&self.python).await;
        *c = Some((Instant::now(), v.clone()));
        v
    }

    async fn install(&self, log: &(dyn Fn(String) + Send + Sync)) -> Result<()> {
        let r = version::install(&self.python, log).await;
        // Whatever happened, look again at the next status call.
        *self.check.lock().await = None;
        *self.versions.lock().await = None;
        r
    }

    fn install_info(&self) -> Option<InstallInfo> {
        Some(InstallInfo {
            uv_path: version::find_uv().map(|p| p.display().to_string()),
            venv_python: self.python.display().to_string(),
            camoufox_version: version::CAMOUFOX_VERSION.to_string(),
        })
    }

    async fn generate(&self, os: Os) -> Result<Value> {
        self.ensure_installed().await?;
        let v = helper::run(&self.python, &self.helper, &["generate", "--os", os.code()], Duration::from_secs(180)).await?;
        v.get("config").filter(|c| c.is_object()).cloned().context("helper generate returned no config")
    }

    fn summarize(&self, machine: &Value) -> MachineSummary {
        machine::summarize(machine)
    }

    async fn launch(&self, p: &Profile, opts: &LaunchOptions) -> Result<Session> {
        check_id(&p.id)?;
        self.ensure_installed().await?;
        if self.procs.alive(&p.id).await == Some(true) {
            bail!(CmError::Conflict("profile is already running".into()));
        }
        let dir = self.dir(&p.id);
        let user_data = dir.join("user-data");
        fs::create_dir_all(&user_data)?;
        let launch_file = dir.join("launch.json");
        write_atomic(&launch_file, &serde_json::to_vec(&json!({ "id": p.id, "config": p.machine }))?)?;

        // Firefox extensions assigned in the app: sideloaded into <user-data>/extensions before the
        // browser opens (see `extensions`).
        let (ud, xpis) = (user_data.clone(), opts.extensions.clone());
        let (installed, _) = tokio::task::spawn_blocking(move || extensions::sync(&ud, &xpis)).await??;

        // CM-3.4: bookmarks go into places.sqlite while the profile is closed. Never fatal: a locked or
        // odd database only means the bookmarks are not refreshed this time.
        let (ud, bms) = (user_data.clone(), opts.bookmarks.clone());
        if let Ok(Err(e)) = tokio::task::spawn_blocking(move || places::sync(&ud, &bms)).await {
            eprintln!("bookmarks not synced: {e}");
        }

        let mut cmd = Command::new(&self.python);
        cmd.arg(&self.helper).arg("launch").arg("--profile").arg(&launch_file).arg("--user-data").arg(&user_data);
        // Only ever the app's local relay 127.0.0.1:port — never upstream credentials.
        if let Some((host, port)) = &opts.proxy {
            cmd.arg("--proxy").arg(format!("http://{host}:{port}"));
        }
        if let Some(tz) = &opts.timezone {
            cmd.arg("--timezone").arg(tz);
        }
        // D4: geolocation only together with a timezone (the helper enforces it too).
        if let (Some((lat, lon)), Some(_)) = (opts.geolocation, &opts.timezone) {
            // `--geo=…` form: a negative latitude/longitude (south/west) starts with `-`; as a
            // separate argument argparse would take it for a new option.
            cmd.arg(format!("--geo={lat},{lon}"));
        }
        if let Some(ip) = &opts.public_ip {
            cmd.arg("--ip").arg(ip);
        }
        machine::launch_args(opts, &mut cmd);
        if !installed.is_empty() {
            cmd.arg("--extension-prefs");
        }
        if opts.headless.unwrap_or(self.headless) {
            cmd.arg("--headless");
        }
        let ws = self.procs.spawn(&p.id, &dir, cmd, &dir.join("launch.log")).await?;
        Ok(Session { ws_url: ws })
    }

    async fn stop(&self, id: &str) -> Result<()> {
        self.procs.stop(id).await
    }

    async fn alive(&self, id: &str) -> Option<bool> {
        self.procs.alive(id).await
    }

    async fn crashed(&self, id: &str) -> Option<CrashInfo> {
        self.procs.crashed(id)
    }

    fn data_dir(&self, id: &str) -> PathBuf {
        self.dir(id).join("user-data")
    }

    async fn browse(&self, ws_url: &str, urls: &[String], dwell_secs: u32) -> Result<()> {
        let urls_json = serde_json::to_string(urls)?;
        let limit = Duration::from_secs(u64::from(dwell_secs) * urls.len() as u64 + 90);
        let dwell = dwell_secs.to_string();
        let v = helper::run(&self.python, &self.helper, &["warm", "--ws", ws_url, "--urls", &urls_json, "--dwell", &dwell], limit).await?;
        if let Some(n) = v["failed"].as_array().filter(|a| !a.is_empty()) {
            eprintln!("warm: {} URL(s) could not be opened: {}", n.len(), v["failed"]);
        }
        Ok(())
    }

    async fn hold_closed(&self, id: &str) -> Result<Box<dyn std::any::Any + Send>> {
        check_id(id)?;
        if self.procs.alive(id).await == Some(true) {
            bail!(CmError::Conflict("profile is running; close it first".into()));
        }
        let dir = self.dir(id);
        let guard = tokio::task::spawn_blocking(move || closed::prove_closed(&dir)).await??;
        Ok(Box::new(guard))
    }

    async fn export_cookies(&self, p: &Profile, _format: CookieFormat) -> Result<Vec<Cookie>> {
        Ok(self.cookie_op(p, None).await?.0)
    }

    async fn import_cookies(&self, p: &Profile, cookies: &[Cookie], mode: CookieImportMode) -> Result<usize> {
        Ok(self.cookie_op(p, Some((cookies.to_vec(), mode))).await?.1)
    }

    async fn delete_data(&self, id: &str) -> Result<()> {
        check_id(id)?;
        if self.procs.alive(id).await == Some(true) {
            self.procs.stop(id).await?;
        }
        let dir = self.dir(id);
        if dir.exists() {
            fs::remove_dir_all(&dir).with_context(|| format!("cannot delete {}", dir.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_written_and_not_ready_without_python() {
        let root = std::env::temp_dir().join(format!("cm-engine-{}", std::process::id()));
        let e = Camoufox::new(root.clone(), root.join("no-python"), true);
        for f in ["__main__.py", "common.py", "generate.py", "launch.py"] {
            assert!(root.join("helper").join(f).is_file(), "{f}");
        }
        let st = tokio::runtime::Runtime::new().unwrap().block_on(e.status());
        assert!(!st.ready);
        assert_eq!(st.hint.unwrap().code, "not_installed");
        let _ = fs::remove_dir_all(&root);
    }
}
