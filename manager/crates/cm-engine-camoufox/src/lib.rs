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

mod helper;
mod lock;
mod machine;
mod process;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use cm_core::{
    CmError, CrashInfo, Engine, EngineHint, EngineStatus, LaunchOptions, MachineSummary, Os, Profile, Session,
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
        Self { root, python, helper, headless, procs, check: Mutex::new(None) }
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

    /// `helper cookies export|import` on the profile's user-data (python + stdlib sqlite only,
    /// no browser start). Returns (exported cookies, count).
    async fn cookie_helper(
        &self,
        p: &Profile,
        action: &str,
        cookies: Option<&[Cookie]>,
        mode: CookieImportMode,
    ) -> Result<(Vec<Cookie>, usize)> {
        check_id(&p.id)?;
        if self.procs.alive(&p.id).await == Some(true) {
            bail!(CmError::Conflict("profile is running".into()));
        }
        if !self.python.is_file() {
            bail!("python not found at {} — run {SETUP_SCRIPT}", self.python.display());
        }
        let dir = self.dir(&p.id);
        fs::create_dir_all(&dir)?;
        let file = dir.join(format!("cookies-{action}.json.tmp"));
        match cookies {
            Some(c) => fs::write(&file, serde_json::to_vec(c)?)?,
            None => {
                let _ = fs::remove_file(&file);
            }
        }
        let user_data = self.data_dir(&p.id);
        let (ud, f) = (user_data.to_string_lossy(), file.to_string_lossy());
        let mode = if mode == CookieImportMode::All { "all" } else { "domains" };
        let r = helper::run(
            &self.python,
            &self.helper,
            &["cookies", action, "--user-data", &ud, "--file", &f, "--mode", mode],
            Duration::from_secs(60),
        )
        .await;
        let exported = if action == "export" && r.is_ok() { fs::read(&file).ok() } else { None };
        let _ = fs::remove_file(&file); // holds cookie values: never leave it behind
        let count = r?["count"].as_u64().unwrap_or(0) as usize;
        let list = match exported {
            Some(b) => serde_json::from_slice(&b).context("helper cookies export wrote invalid JSON")?,
            None => Vec::new(),
        };
        Ok((list, count))
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
        EngineStatus { ready: r.is_ok(), hint: r.err(), version: None }
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
        for xpi in &opts.extensions {
            cmd.arg("--extension").arg(xpi);
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

    async fn export_cookies(&self, p: &Profile, _format: CookieFormat) -> Result<Vec<Cookie>> {
        let out = self.cookie_helper(p, "export", None, CookieImportMode::Domains).await?;
        Ok(out.0)
    }

    async fn import_cookies(&self, p: &Profile, cookies: &[Cookie], mode: CookieImportMode) -> Result<usize> {
        Ok(self.cookie_helper(p, "import", Some(cookies), mode).await?.1)
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
        for f in ["__main__.py", "common.py", "generate.py", "launch.py", "cookies.py", "extensions.py"] {
            assert!(root.join("helper").join(f).is_file(), "{f}");
        }
        let st = tokio::runtime::Runtime::new().unwrap().block_on(e.status());
        assert!(!st.ready);
        assert_eq!(st.hint.unwrap().code, "not_installed");
        let _ = fs::remove_dir_all(&root);
    }
}
