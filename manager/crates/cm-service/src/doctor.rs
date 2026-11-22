//! Doctor (phase 19): local diagnostics with a plain-language fix for every problem.
//!
//! Every check yields `ok | warn | fail`, a `detail` (what was found) and, when not ok, a `fix`
//! code + English `hint` (the UI translates `fix` codes, falls back to the hint). Checks only look:
//! the one thing they start is a throw-away loopback relay, and `tailscale status --json` is only
//! read. **Nothing is ever sent anywhere** (no telemetry); the markdown report holds no proxy
//! passwords, API keys or cookie values and shortens the home directory to `~`.
//!
//! Checks that need other crates (python helper files, the database, process list) go through
//! [`Probes`], implemented by the app binary; tests use fakes.

use crate::Manager;
use cm_core::{ProxyScheme, Result};
use serde::Serialize;
use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    process::Command,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Stable id, also the i18n key suffix of the title (`doctor.check.<id>`).
    pub id: &'static str,
    pub status: Status,
    /// What was found.
    pub detail: String,
    /// Fix code (i18n key suffix `doctor.fix.<code>`) when not ok.
    pub fix: Option<&'static str>,
    /// English fix hint when not ok.
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub ok: usize,
    pub warn: usize,
    pub fail: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// No check failed (warnings are fine).
    pub ok: bool,
    pub summary: Summary,
    pub checks: Vec<Check>,
    pub generated_at: String,
    pub version: &'static str,
    pub platform: String,
    /// Copyable report, markdown, no secrets.
    pub markdown: String,
}

#[derive(Clone, Debug, Default)]
pub struct DoctorOptions {
    /// Port the UI/API answers on (from the request); `None` skips that check.
    pub ui_port: Option<u16>,
}

// ---- probes (implemented by the binary) ----

#[derive(Clone, Debug, Default)]
pub struct HelperProbe {
    pub missing: Vec<String>,
    pub outdated: Vec<String>,
    pub total: usize,
}

#[derive(Clone, Debug, Default)]
pub struct DbProbe {
    /// Rows of `PRAGMA quick_check` (`["ok"]` when healthy).
    pub integrity: Vec<String>,
    pub write_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct OrphanProbe {
    pub profile: String,
    pub pids: Vec<u32>,
    pub stale_lock: bool,
}

/// Read-only looks at things `cm-service` has no access to. Every method may be slow-ish (disk,
/// process list): the doctor calls them on a blocking thread.
pub trait Probes: Send + Sync {
    /// `None` = this engine has no helper files.
    fn helper(&self) -> Option<HelperProbe> {
        None
    }
    fn db(&self) -> Option<DbProbe> {
        None
    }
    fn orphans(&self) -> Vec<OrphanProbe> {
        vec![]
    }
}

// ---- building blocks ----

fn check(id: &'static str, status: Status, detail: impl Into<String>) -> Check {
    Check { id, status, detail: detail.into(), fix: None, hint: None }
}

fn with_fix(mut c: Check, fix: &'static str, hint: &str) -> Check {
    c.fix = Some(fix);
    c.hint = Some(hint.to_string());
    c
}

/// English title of a check (UI has its own translations under `doctor.check.<id>`).
pub fn title(id: &str) -> &'static str {
    match id {
        "camoufox.package" => "Camoufox package",
        "camoufox.browser" => "Camoufox browser",
        "helper.files" => "Helper files",
        "data.dir" => "Data folder",
        "db.writable" => "Database writable",
        "db.integrity" => "Database integrity",
        "port.ui" => "UI port",
        "port.proxy" => "Proxy relay ports",
        "relay.failclosed" => "Proxy relay fails closed",
        "processes.orphans" => "Leftover processes",
        "journals" => "Interrupted imports",
        "tailscale" => "Tailscale",
        _ => "Check",
    }
}

/// Plain `fn` so tests can call it with a temp dir.
pub fn check_data_dir(dir: &Path) -> Check {
    // Unique per call: two checks in one process (or tests sharing a dir) must not delete each other's probe.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let probe = dir.join(format!(".doctor-probe-{}-{n}", std::process::id()));
    let r = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&probe, b"ok")).and_then(|_| std::fs::remove_file(&probe));
    match r {
        Ok(()) => check("data.dir", Status::Ok, format!("{} is writable", dir.display())),
        Err(e) => with_fix(
            check("data.dir", Status::Fail, format!("cannot write in {}: {e}", dir.display())),
            "data_dir",
            "The data folder is not writable. Fix its permissions (for example `chmod -R u+rwX` on it) or free disk space, then run the doctor again.",
        ),
    }
}

pub fn check_db(p: Option<&DbProbe>) -> Vec<Check> {
    let Some(p) = p else { return vec![] };
    let writable = match &p.write_error {
        None => check("db.writable", Status::Ok, "the database accepts writes"),
        Some(e) => with_fix(
            check("db.writable", Status::Fail, format!("cannot write: {e}")),
            "db_writable",
            "Another program may hold the database or the file is read-only. Close other copies of the app and check the file permissions.",
        ),
    };
    let intact = p.integrity.len() == 1 && p.integrity[0] == "ok";
    let integrity = if intact {
        check("db.integrity", Status::Ok, "quick_check: ok")
    } else {
        let first: Vec<&str> = p.integrity.iter().take(3).map(String::as_str).collect();
        with_fix(
            check("db.integrity", Status::Fail, format!("quick_check reported: {}", first.join("; "))),
            "db_repair",
            "The database file is damaged. Quit the app and keep a copy of camoufox-manager.db before doing anything else; \
             restore the newest backup if you have one. Do not delete the file: your profiles are listed in it.",
        )
    };
    vec![writable, integrity]
}

pub fn check_helper(p: Option<&HelperProbe>) -> Option<Check> {
    let p = p?;
    Some(if p.missing.is_empty() && p.outdated.is_empty() {
        check("helper.files", Status::Ok, format!("all {} helper files present and current", p.total))
    } else if !p.missing.is_empty() {
        with_fix(
            check("helper.files", Status::Fail, format!("missing: {}", p.missing.join(", "))),
            "restart_app",
            "Restart the app: it rewrites the helper files at startup. If they keep disappearing, check that the data folder is writable.",
        )
    } else {
        with_fix(
            check("helper.files", Status::Warn, format!("out of date: {}", p.outdated.join(", "))),
            "restart_app",
            "Restart the app: it rewrites the helper files at startup. If they keep disappearing, check that the data folder is writable.",
        )
    })
}

pub fn check_orphans(list: &[OrphanProbe]) -> Check {
    if list.is_empty() {
        return check("processes.orphans", Status::Ok, "no leftover helper or browser processes");
    }
    let parts: Vec<String> = list
        .iter()
        .map(|o| {
            let mut s = format!("profile {}", o.profile);
            if !o.pids.is_empty() {
                s.push_str(&format!(": process {}", o.pids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")));
            }
            if o.stale_lock {
                s.push_str(" (stale lock)");
            }
            s
        })
        .collect();
    with_fix(
        check("processes.orphans", Status::Warn, parts.join("; ")),
        "orphans",
        "A previous run left these behind. They are cleaned up automatically when the app starts, so restart the app to clean them now. Nothing was stopped by the doctor.",
    )
}

/// Rows of `snapshot_journal` (in the app database) that are not `complete` / `aborted`, as
/// `<profile id>: <state>`. Looks only: a missing database file means no journals.
pub fn unresolved_journals(db: &Path) -> Vec<String> {
    if !db.exists() {
        return vec![];
    }
    match crate::journal::Journal::open(db).and_then(|j| j.unresolved(None)) {
        Ok(rows) => rows.iter().map(|e| format!("{}: {}", e.profile_id, e.state.as_str())).collect(),
        Err(e) => vec![format!("cannot read snapshot_journal: {e:#}")],
    }
}

pub fn check_journals(list: &[String]) -> Check {
    if list.is_empty() {
        return check("journals", Status::Ok, "no interrupted imports");
    }
    with_fix(
        check("journals", Status::Warn, list.join("; ")),
        "journals",
        "An import or sync apply was interrupted. Restart the app so it can finish or roll back by itself; do not delete those files by hand.",
    )
}

/// Pure part of the Tailscale check: `tailscale status --json` text -> check. Informational: a
/// missing Tailscale is fine (only device sync needs it); nothing is ever changed.
pub fn tailscale_from_status(json: &str) -> Check {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return with_fix(
            check("tailscale", Status::Warn, "`tailscale status --json` returned something unreadable"),
            "tailscale",
            "Open the Tailscale app and make sure it is running. Only device sync needs it.",
        );
    };
    let state = v["BackendState"].as_str().unwrap_or("unknown");
    // The tailnet name (often an e-mail address) is deliberately left out of the report.
    match state {
        "Running" => check("tailscale", Status::Ok, "running"),
        other => with_fix(
            check("tailscale", Status::Warn, format!("installed but not connected (state: {other})")),
            "tailscale",
            "Open the Tailscale app, sign in and turn it on. Only device sync needs it; everything else works without.",
        ),
    }
}

fn tailscale_binary() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|d| d.join("tailscale"))
        .chain([PathBuf::from("/Applications/Tailscale.app/Contents/MacOS/Tailscale")])
        .find(|p| p.is_file())
}

async fn check_tailscale() -> Check {
    let Some(bin) = tailscale_binary() else {
        return check("tailscale", Status::Ok, "not installed (only needed for device sync)");
    };
    let run = Command::new(&bin).args(["status", "--json"]).stdin(std::process::Stdio::null()).kill_on_drop(true).output();
    match tokio::time::timeout(Duration::from_secs(6), run).await {
        Ok(Ok(out)) if !out.stdout.is_empty() => tailscale_from_status(&String::from_utf8_lossy(&out.stdout)),
        Ok(Ok(out)) => with_fix(
            check(
                "tailscale",
                Status::Warn,
                format!("`tailscale status` printed nothing: {}", String::from_utf8_lossy(&out.stderr).trim()),
            ),
            "tailscale",
            "Open the Tailscale app and make sure it is running. Only device sync needs it.",
        ),
        Ok(Err(e)) => check("tailscale", Status::Warn, format!("cannot run {}: {e}", bin.display())),
        Err(_) => check("tailscale", Status::Warn, "`tailscale status` did not answer within 6 seconds"),
    }
}

/// Start a loopback relay whose upstream is a closed port and confirm a browser request gets no
/// tunnel (fail-closed: no direct path to the network).
pub async fn check_relay_failclosed() -> Check {
    let broken = |detail: String| {
        with_fix(
            check("relay.failclosed", Status::Fail, detail),
            "relay",
            "Do not start browsers with a proxy until this passes. Restart the app; if it still fails, the relay is broken and your real IP could leak.",
        )
    };
    let dead = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await {
        Ok(l) => l.local_addr().map(|a| a.port()).unwrap_or(1),
        Err(e) => return broken(format!("cannot open a loopback port: {e}")),
    };
    let listener = match cm_relay::bind(0).await {
        Ok(l) => l,
        Err(e) => return broken(format!("cannot start the relay: {e:#}")),
    };
    let Ok(addr) = listener.local_addr() else { return broken("relay has no address".into()) };
    let up = cm_relay::Upstream { scheme: ProxyScheme::Http, host: "127.0.0.1".into(), port: dead, username: None, password: None };
    let task = tokio::spawn(cm_relay::run(listener, up));
    let verdict = async {
        let mut s = TcpStream::connect(addr).await.map_err(|e| format!("cannot reach the relay: {e}"))?;
        s.write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n").await.ok();
        let mut buf = [0u8; 256];
        match tokio::time::timeout(Duration::from_secs(8), s.read(&mut buf)).await {
            Err(_) => Err("the relay neither answered nor closed within 8 seconds".to_string()),
            Ok(Ok(n)) if buf[..n].starts_with(b"HTTP/1.1 200") || buf[..n].starts_with(b"HTTP/1.0 200") => {
                Err("the relay opened a tunnel although its upstream is down".to_string())
            }
            Ok(_) => Ok(()),
        }
    }
    .await;
    task.abort();
    match verdict {
        Ok(()) => {
            check("relay.failclosed", Status::Ok, "a request through a relay with a dead upstream was refused (no direct connection)")
        }
        Err(e) => broken(e),
    }
}

/// Is `port` free on loopback right now?
async fn port_free(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await.is_ok()
}

impl Manager {
    /// Attach the read-only probes (binary wiring). First call wins.
    pub fn set_probes(&self, p: std::sync::Arc<dyn Probes>) {
        let _ = self.probes.set(p);
    }

    async fn check_engine(&self) -> Vec<Check> {
        let st = self.d.engine.status().await;
        let v = self.d.engine.versions().await.unwrap_or_default();
        let code = st.hint.as_ref().map(|h| h.code);
        let detail = st.hint.as_ref().map(|h| h.detail.clone()).unwrap_or_default();
        let install = "Open Settings, then Camoufox, then Install / Update. Or run manager/camoufox/setup.sh in a terminal. Camoufox is optional: the rest of the app works without it.";
        let package = match (&v.package, code) {
            (Some(p), _) => check("camoufox.package", Status::Ok, format!("version {p}")),
            (None, Some("not_installed")) => {
                with_fix(check("camoufox.package", Status::Fail, format!("not installed: {detail}")), "install_engine", install)
            }
            (None, _) if st.ready => check("camoufox.package", Status::Ok, "installed (version unknown)"),
            (None, _) => with_fix(check("camoufox.package", Status::Fail, format!("not ready: {detail}")), "install_engine", install),
        };
        let browser = match (&v.browser, code) {
            (Some(b), None) => check("camoufox.browser", Status::Ok, format!("build {b}")),
            (_, Some("browser_missing")) => with_fix(
                check("camoufox.browser", Status::Fail, "the package is installed but the browser build is not downloaded"),
                "fetch_browser",
                "Press Install / Update in Settings (or run `python -m camoufox fetch`) to download the browser build.",
            ),
            (_, Some("not_installed")) => check("camoufox.browser", Status::Warn, "unknown until the Camoufox package is installed"),
            (Some(b), Some(_)) => with_fix(
                check("camoufox.browser", Status::Fail, format!("build {b}, but the engine reports: {detail}")),
                "install_engine",
                install,
            ),
            (None, Some(_)) => with_fix(check("camoufox.browser", Status::Fail, detail), "install_engine", install),
            (None, None) => check("camoufox.browser", Status::Ok, "installed (build unknown)"),
        };
        vec![package, browser]
    }

    async fn check_ports(&self, opts: &DoctorOptions) -> Vec<Check> {
        let mut out = vec![];
        if let Some(port) = opts.ui_port {
            let reachable = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect((Ipv4Addr::LOCALHOST, port))).await;
            out.push(match reachable {
                Ok(Ok(_)) => check("port.ui", Status::Ok, format!("127.0.0.1:{port} is served by this app")),
                _ => with_fix(
                    check("port.ui", Status::Fail, format!("127.0.0.1:{port} does not answer")),
                    "ui_port",
                    "Start the app with another port (CM_PORT=<n>) and open that address.",
                ),
            });
        }
        let ours: Vec<u16> = self.relays.lock().await.values().map(|(p, _)| *p).collect();
        let ids: Vec<i64> = match self.d.proxies.list().await {
            Ok(l) => l.iter().map(|p| p.id).collect(),
            Err(_) => vec![],
        };
        let mut ports: Vec<(Option<i64>, u16)> = ids.iter().filter_map(|&i| self.relay_port(i).map(|p| (Some(i), p))).collect();
        if ports.is_empty() {
            ports.push((None, self.cfg.proxy_base_port));
        }
        let mut taken = vec![];
        for (id, port) in &ports {
            if ours.contains(port) || port_free(*port).await {
                continue;
            }
            taken.push(match id {
                Some(i) => format!("proxy #{i} -> port {port}"),
                None => format!("port {port}"),
            });
        }
        let base = self.cfg.proxy_base_port;
        out.push(if taken.is_empty() {
            check("port.proxy", Status::Ok, format!("{} relay port(s) from {base} are free or ours", ports.len()))
        } else {
            with_fix(
                check("port.proxy", Status::Fail, format!("in use by another program: {}", taken.join(", "))),
                "proxy_ports",
                "Quit the program that uses the port, or start the app with a different CM_PROXY_BASE_PORT (for example 41000).",
            )
        });
        out
    }

    /// Run every check. Never changes anything.
    pub async fn doctor(&self, opts: DoctorOptions) -> Result<Report> {
        let probes = self.probes.get().cloned();
        let mut checks = self.check_engine().await;

        let p = probes.clone();
        let (helper, db, orphans) = tokio::task::spawn_blocking(move || match p {
            Some(p) => (p.helper(), p.db(), p.orphans()),
            None => (None, None, vec![]),
        })
        .await?;
        checks.extend(check_helper(helper.as_ref()));
        let dir = self.cfg.data_dir.clone();
        let db_file = self.db_path();
        let (data, journals) = tokio::task::spawn_blocking(move || (check_data_dir(&dir), unresolved_journals(&db_file))).await?;
        checks.push(data);
        checks.extend(check_db(db.as_ref()));
        checks.extend(self.check_ports(&opts).await);
        checks.push(check_relay_failclosed().await);
        checks.push(check_orphans(&orphans));
        checks.push(check_journals(&journals));
        checks.push(check_tailscale().await);

        let home = std::env::var("HOME").ok().filter(|h| h.len() > 1);
        let scrub = |s: &str| match &home {
            Some(h) => s.replace(h.as_str(), "~"),
            None => s.to_string(),
        };
        for c in &mut checks {
            c.detail = scrub(&c.detail);
            c.hint = c.hint.as_deref().map(scrub);
        }
        let summary = Summary {
            ok: checks.iter().filter(|c| c.status == Status::Ok).count(),
            warn: checks.iter().filter(|c| c.status == Status::Warn).count(),
            fail: checks.iter().filter(|c| c.status == Status::Fail).count(),
        };
        let generated_at = self.d.clock.now_iso();
        let platform = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
        let markdown = markdown(&checks, &summary, &generated_at, &platform);
        Ok(Report { ok: summary.fail == 0, summary, checks, generated_at, version: env!("CARGO_PKG_VERSION"), platform, markdown })
    }
}

fn markdown(checks: &[Check], s: &Summary, at: &str, platform: &str) -> String {
    let mut m = format!(
        "# Camoufox Manager doctor report\n\nGenerated {at} · app {} · {platform}\n\nResult: {} ok, {} warning, {} failed\n\n",
        env!("CARGO_PKG_VERSION"),
        s.ok,
        s.warn,
        s.fail
    );
    for c in checks {
        m.push_str(&format!("- **[{}] {}**: {}\n", c.status.label(), title(c.id), c.detail));
        if let Some(h) = &c.hint {
            m.push_str(&format!("  - Fix: {h}\n"));
        }
    }
    m.push_str(
        "\nPrivacy: this report was made on this computer and nothing was sent anywhere. It contains no proxy passwords, \
         API keys or cookie values.\n",
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::manager;
    use std::sync::Arc;

    struct Fake {
        helper: Option<HelperProbe>,
        db: Option<DbProbe>,
        orphans: Vec<OrphanProbe>,
    }

    impl Probes for Fake {
        fn helper(&self) -> Option<HelperProbe> {
            self.helper.clone()
        }
        fn db(&self) -> Option<DbProbe> {
            self.db.clone()
        }
        fn orphans(&self) -> Vec<OrphanProbe> {
            self.orphans.clone()
        }
    }

    fn healthy() -> Fake {
        Fake {
            helper: Some(HelperProbe { total: 7, ..Default::default() }),
            db: Some(DbProbe { integrity: vec!["ok".into()], write_error: None }),
            orphans: vec![],
        }
    }

    fn by_id<'a>(r: &'a Report, id: &str) -> &'a Check {
        r.checks.iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    async fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn healthy_setup_has_no_failures_and_fails_closed() {
        let (m, _) = manager(true, free_port().await);
        m.set_probes(Arc::new(healthy()));
        let r = m.doctor(DoctorOptions::default()).await.unwrap();
        assert!(r.ok, "{:#?}", r.checks.iter().filter(|c| c.status != Status::Ok).collect::<Vec<_>>());
        assert_eq!(by_id(&r, "relay.failclosed").status, Status::Ok);
        assert_eq!(by_id(&r, "db.integrity").status, Status::Ok);
        assert_eq!(by_id(&r, "helper.files").status, Status::Ok);
        assert!(r.markdown.contains("nothing was sent anywhere"));
    }

    #[tokio::test]
    async fn problems_get_status_and_a_plain_fix() {
        let (m, _) = manager(true, free_port().await);
        m.set_probes(Arc::new(Fake {
            helper: Some(HelperProbe { missing: vec!["launch.py".into()], total: 7, ..Default::default() }),
            db: Some(DbProbe { integrity: vec!["page 3 is damaged".into()], write_error: Some("database is locked".into()) }),
            orphans: vec![OrphanProbe { profile: "abc".into(), pids: vec![42], stale_lock: false }],
        }));
        let r = m.doctor(DoctorOptions::default()).await.unwrap();
        assert!(!r.ok);
        for (id, st) in [
            ("helper.files", Status::Fail),
            ("db.integrity", Status::Fail),
            ("db.writable", Status::Fail),
            ("processes.orphans", Status::Warn),
        ] {
            let c = by_id(&r, id);
            assert_eq!(c.status, st, "{id}");
            assert!(c.fix.is_some() && c.hint.as_deref().is_some_and(|h| h.len() > 20), "{id} needs a fix hint");
        }
        assert!(by_id(&r, "processes.orphans").detail.contains("process 42"));
        assert!(r.markdown.contains("[FAIL] Database integrity"));
        assert!(r.markdown.contains("Fix: "));
    }

    #[tokio::test]
    async fn engine_states_map_to_checks() {
        // The fake engine is ready with unknown versions: both ok.
        let (m, _) = manager(true, free_port().await);
        let r = m.doctor(DoctorOptions::default()).await.unwrap();
        assert_eq!(by_id(&r, "camoufox.package").status, Status::Ok);
        assert_eq!(by_id(&r, "camoufox.browser").status, Status::Ok);
    }

    #[tokio::test]
    async fn busy_proxy_port_fails_and_our_own_relay_does_not() {
        let blocker = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = blocker.local_addr().unwrap().port();
        let (m, _) = manager(true, port);
        let r = m.doctor(DoctorOptions::default()).await.unwrap();
        let c = by_id(&r, "port.proxy");
        assert_eq!(c.status, Status::Fail);
        assert!(c.detail.contains(&port.to_string()));
        assert_eq!(c.fix, Some("proxy_ports"));

        // A relay of ours on that port is fine.
        let (m, _) = manager(true, free_port().await);
        let px = m
            .deps()
            .proxies
            .insert(&cm_core::NewProxy { name: "a".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() })
            .await
            .unwrap();
        let p = m.proxy(px.id).await.unwrap();
        m.ensure_relay(&p, true).await.unwrap();
        let r = m.doctor(DoctorOptions::default()).await.unwrap();
        assert_eq!(by_id(&r, "port.proxy").status, Status::Ok);
    }

    #[tokio::test]
    async fn ui_port_check_follows_reachability() {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let (m, _) = manager(true, free_port().await);
        let r = m.doctor(DoctorOptions { ui_port: Some(port) }).await.unwrap();
        assert_eq!(by_id(&r, "port.ui").status, Status::Ok);
        drop(l);
        let r = m.doctor(DoctorOptions { ui_port: Some(port) }).await.unwrap();
        assert_eq!(by_id(&r, "port.ui").status, Status::Fail);
    }

    #[test]
    fn data_dir_check() {
        let dir = std::env::temp_dir().join(format!("cm-doctor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(check_data_dir(&dir).status, Status::Ok);
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "probe file removed");
        // A file where the folder should be: not writable.
        let file = dir.join("afile");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(check_data_dir(&file).status, Status::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn journals_check_reads_snapshot_journal() {
        let dir = std::env::temp_dir().join(format!("cm-doctor-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("camoufox-manager.db");
        // No database file: nothing to report, and the check does not create one.
        assert!(unresolved_journals(&db).is_empty());
        assert!(!db.exists());

        // Empty table: Ok.
        let j = crate::journal::Journal::open(&db).unwrap();
        assert!(unresolved_journals(&db).is_empty());
        assert_eq!(check_journals(&unresolved_journals(&db)).status, Status::Ok);

        // An unresolved row: Warn, listed by profile id and state.
        j.begin("j1", "aaa", &dir.join("staging"), &dir.join("backup"), "2026-01-01T00:00:00Z").unwrap();
        let list = unresolved_journals(&db);
        assert_eq!(list, vec!["aaa: preparing".to_string()]);
        assert_eq!(check_journals(&list).status, Status::Warn);

        // Closed rows (complete / aborted) do not count.
        j.set_state("j1", crate::journal::State::Complete, None, "2026-01-01T00:00:01Z").unwrap();
        j.begin("j2", "bbb", &dir.join("staging2"), &dir.join("backup2"), "2026-01-01T00:00:02Z").unwrap();
        j.set_state("j2", crate::journal::State::Aborted, None, "2026-01-01T00:00:03Z").unwrap();
        assert!(unresolved_journals(&db).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tailscale_states() {
        let running = tailscale_from_status(r#"{"BackendState":"Running","CurrentTailnet":{"Name":"me@example.com"}}"#);
        assert_eq!(running.status, Status::Ok);
        assert!(!running.detail.contains("example.com"));
        let stopped = tailscale_from_status(r#"{"BackendState":"NeedsLogin"}"#);
        assert_eq!(stopped.status, Status::Warn);
        assert!(stopped.detail.contains("NeedsLogin") && stopped.hint.is_some());
        assert_eq!(tailscale_from_status("not json").status, Status::Warn);
    }

    #[tokio::test]
    async fn relay_failclosed_check_passes_with_the_real_relay() {
        assert_eq!(check_relay_failclosed().await.status, Status::Ok);
    }
}
