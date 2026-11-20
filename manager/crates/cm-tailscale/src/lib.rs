//! The user's own Tailscale, seen from Camoufox Manager (plan CM-1, D9, phase 16).
//!
//! - **Probe** ([`Tailscale::status`]): runs `tailscale status --json` (read-only, with a timeout)
//!   and turns it into a small [`Status`]: running or not, this node's MagicDNS name + 100.x IPs,
//!   whether HTTPS certificates are enabled for the tailnet, the logged-in user.
//! - **Serve helper** ([`ServePlan`]): builds the exact
//!   `tailscale serve --bg --https=<p> http://127.0.0.1:<sync_port>` command so the UI can show it,
//!   and runs it only through [`Tailscale::serve`], which needs a [`Confirmed`] token that the API
//!   layer creates only when the user pressed "confirm". Nothing else here changes Tailscale config.
//!   `tailscale funnel` (public internet) is never built or run.
//! - [`Fake`] for tests.
//!
//! Tailscale missing, stopped or logged out is a normal state, not an error: the feature turns off
//! with a hint and the rest of the app is unaffected.

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

/// Default HTTPS port used with `tailscale serve` (443 is often taken by something else).
pub const DEFAULT_HTTPS_PORT: u16 = 8443;
/// How long a `tailscale` CLI call may take before we give up.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(4);

/// Coarse state, used by the UI to pick a translated hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// No `tailscale` binary found.
    NotInstalled,
    /// Binary found but the daemon / app is not running (or did not answer in time).
    NotRunning,
    /// Installed and running, but not logged in (or the machine awaits admin approval).
    NeedsLogin,
    /// Logged in but switched off ("Disconnected").
    Stopped,
    Starting,
    Running,
    /// Anything we could not classify (detail in [`Status::detail`]).
    Error,
}

/// What the probe found. IPs and names only, never keys.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    /// Short Tailscale version (`1.104.1`).
    pub version: Option<String>,
    /// MagicDNS name of this machine without the trailing dot (`laptop.tail1234.ts.net`).
    pub dns_name: Option<String>,
    pub host_name: Option<String>,
    /// Tailscale IPs of this machine (100.x first, then fd7a:…).
    pub ips: Vec<String>,
    /// Tailnet name (`tail1234.ts.net`).
    pub tailnet: Option<String>,
    /// Login of the user owning this node (`someone@example.com`).
    pub login: Option<String>,
    /// HTTPS certificates are enabled for this tailnet (`tailscale serve --https` can work).
    pub https_certs: bool,
    pub magic_dns: bool,
    /// Health warnings reported by Tailscale.
    pub health: Vec<String>,
    /// Extra detail for `Error` / `NotRunning` (stderr line, timeout …).
    pub detail: Option<String>,
}

impl Status {
    pub fn with_state(state: State, detail: Option<String>) -> Status {
        Status {
            state,
            version: None,
            dns_name: None,
            host_name: None,
            ips: Vec::new(),
            tailnet: None,
            login: None,
            https_certs: false,
            magic_dns: false,
            health: Vec::new(),
            detail,
        }
    }

    pub fn running(&self) -> bool {
        self.state == State::Running
    }

    /// First 100.x address.
    pub fn ipv4(&self) -> Option<&str> {
        self.ips.iter().map(String::as_str).find(|ip| ip.starts_with("100."))
    }

    /// One plain-English sentence for the state (the UI translates by `state` instead).
    pub fn hint(&self) -> &'static str {
        match self.state {
            State::NotInstalled => "Tailscale is not installed. Install it from tailscale.com and log in to sync devices.",
            State::NotRunning => "Tailscale is installed but not running. Open the Tailscale app.",
            State::NeedsLogin => "Tailscale needs you to log in (or the admin to approve this machine).",
            State::Stopped => "Tailscale is switched off. Connect it in the Tailscale app.",
            State::Starting => "Tailscale is starting…",
            State::Running => "Tailscale is running.",
            State::Error => "Tailscale reported an error.",
        }
    }
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).filter(|x| !x.is_empty()).map(str::to_owned)
}

/// Parse `tailscale status --json`.
pub fn parse_status(json: &[u8]) -> Status {
    let v: Value = match serde_json::from_slice(json) {
        Ok(v) => v,
        Err(e) => return Status::with_state(State::Error, Some(format!("unreadable tailscale status: {e}"))),
    };
    let state = match v.get("BackendState").and_then(Value::as_str).unwrap_or("") {
        "Running" => State::Running,
        "Starting" => State::Starting,
        "Stopped" => State::Stopped,
        "NeedsLogin" | "NeedsMachineAuth" | "NoState" => State::NeedsLogin,
        other => return Status::with_state(State::Error, Some(format!("unknown BackendState {other:?}"))),
    };
    let me = v.get("Self").cloned().unwrap_or(Value::Null);
    let mut ips: Vec<String> = me
        .get("TailscaleIPs")
        .or_else(|| v.get("TailscaleIPs"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    ips.sort_by_key(|ip| !ip.starts_with("100."));
    let tailnet = v.get("CurrentTailnet").and_then(|t| s(t, "MagicDNSSuffix")).or_else(|| s(&v, "MagicDNSSuffix"));
    let magic_dns = v.get("CurrentTailnet").and_then(|t| t.get("MagicDNSEnabled")).and_then(Value::as_bool).unwrap_or(tailnet.is_some());
    let login = me.get("UserID").and_then(|id| {
        let key = match id {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => return None,
        };
        v.get("User").and_then(|u| u.get(&key)).and_then(|u| s(u, "LoginName"))
    });
    let https_certs = v.get("CertDomains").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    let health = v
        .get("Health")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    Status {
        state,
        version: s(&v, "Version").map(|x| x.split('-').next().unwrap_or(&x).to_owned()),
        dns_name: s(&me, "DNSName").map(|d| d.trim_end_matches('.').to_owned()),
        host_name: s(&me, "HostName"),
        ips,
        tailnet,
        login,
        https_certs,
        magic_dns,
        health,
        detail: None,
    }
}

/// Does `tailscale serve status --json` already proxy `https://<any>:<https_port>/` to `target`?
pub fn serve_status_has(json: &[u8], https_port: u16, target: &str) -> bool {
    let Ok(v) = serde_json::from_slice::<Value>(json) else { return false };
    let Some(web) = v.get("Web").and_then(Value::as_object) else { return false };
    web.iter().any(|(hostport, cfg)| {
        hostport.rsplit_once(':').is_some_and(|(_, p)| p == https_port.to_string())
            && cfg
                .get("Handlers")
                .and_then(|h| h.get("/"))
                .and_then(|h| h.get("Proxy"))
                .and_then(Value::as_str)
                .is_some_and(|p| p.trim_end_matches('/') == target)
    })
}

/// The one Tailscale config change this app can make, spelled out before it is made.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServePlan {
    pub https_port: u16,
    pub local_port: u16,
    /// `http://127.0.0.1:<local_port>`
    pub target: String,
    /// Arguments after the binary name.
    #[serde(skip)]
    pub args: Vec<String>,
    /// Exactly what will run, as shown to the user.
    pub command: String,
    /// How to undo it (shown, never run by the app).
    pub undo: String,
}

impl ServePlan {
    pub fn new(https_port: u16, local_port: u16) -> ServePlan {
        let target = format!("http://127.0.0.1:{local_port}");
        let args = vec!["serve".to_owned(), "--bg".to_owned(), format!("--https={https_port}"), target.clone()];
        let command = format!("tailscale {}", args.join(" "));
        ServePlan { https_port, local_port, target, args, command, undo: format!("tailscale serve --https={https_port} off") }
    }
}

/// Proof that the user confirmed a [`ServePlan`] in the app. Only the API endpoint behind the
/// "confirm" button creates it ([`Confirmed::by_user`]); nothing runs `serve` without one.
pub struct Confirmed(());

impl Confirmed {
    /// Call only from the explicit confirm action of the UI.
    pub fn by_user() -> Confirmed {
        Confirmed(())
    }
}

#[async_trait]
pub trait Tailscale: Send + Sync {
    /// Read-only probe (`tailscale status --json`). Never fails: problems are a [`State`].
    async fn status(&self) -> Status;
    /// Read-only: is `plan` already in place (`tailscale serve status --json`)?
    async fn serving(&self, plan: &ServePlan) -> bool;
    /// Run `plan`. Changes the user's Tailscale config; requires [`Confirmed`].
    async fn serve(&self, plan: &ServePlan, confirmed: Confirmed) -> Result<String>;
}

/// Where the CLI usually lives on macOS (a GUI app does not inherit the shell PATH).
const CANDIDATES: &[&str] = &[
    "/usr/local/bin/tailscale",
    "/opt/homebrew/bin/tailscale",
    "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
    "/usr/bin/tailscale",
];

/// Find the `tailscale` binary: `CM_TAILSCALE_BIN`, then `PATH`, then the usual places.
pub fn find_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CM_TAILSCALE_BIN") {
        return Some(PathBuf::from(p)).filter(|p| p.is_file());
    }
    let from_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join("tailscale"))
        .find(|p| p.is_file());
    from_path.or_else(|| CANDIDATES.iter().map(PathBuf::from).find(|p| p.is_file()))
}

/// The real CLI.
pub struct Cli {
    bin: Option<PathBuf>,
    timeout: Duration,
}

impl Cli {
    pub fn new(bin: Option<PathBuf>, timeout: Duration) -> Cli {
        Cli { bin, timeout }
    }

    /// Binary from [`find_binary`], [`DEFAULT_TIMEOUT`].
    pub fn detect() -> Cli {
        Cli::new(find_binary(), DEFAULT_TIMEOUT)
    }

    pub fn binary(&self) -> Option<&Path> {
        self.bin.as_deref()
    }

    /// Run `tailscale <args>`; `Err` carries a one-line reason. Killed on timeout.
    async fn run(&self, args: &[String]) -> std::result::Result<Vec<u8>, RunError> {
        debug_assert!(!args.iter().any(|a| a == "funnel"), "funnel is never used");
        let Some(bin) = &self.bin else { return Err(RunError::Missing) };
        let child = tokio::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { RunError::Missing } else { RunError::Failed(e.to_string()) })?;
        let out = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Err(_) => return Err(RunError::Timeout),
            Ok(Err(e)) => return Err(RunError::Failed(e.to_string())),
            Ok(Ok(o)) => o,
        };
        if out.status.success() {
            Ok(out.stdout)
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let line = err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("tailscale exited with an error");
            Err(RunError::Failed(line.chars().take(300).collect()))
        }
    }
}

enum RunError {
    Missing,
    Timeout,
    Failed(String),
}

#[async_trait]
impl Tailscale for Cli {
    async fn status(&self) -> Status {
        match self.run(&["status".into(), "--json".into()]).await {
            Ok(out) => parse_status(&out),
            Err(RunError::Missing) => Status::with_state(State::NotInstalled, None),
            Err(RunError::Timeout) => Status::with_state(State::NotRunning, Some("tailscale did not answer in time".into())),
            // `tailscale status` exits non-zero when the daemon is down or logged out; the JSON
            // variant still prints JSON in some versions, but the stderr line is what we have here.
            Err(RunError::Failed(line)) => {
                let l = line.to_lowercase();
                let state = if l.contains("logged out") || l.contains("needslogin") || l.contains("log in") {
                    State::NeedsLogin
                } else if l.contains("connect") || l.contains("daemon") || l.contains("not running") || l.contains("stopped") {
                    State::NotRunning
                } else {
                    State::Error
                };
                Status::with_state(state, Some(line))
            }
        }
    }

    async fn serving(&self, plan: &ServePlan) -> bool {
        match self.run(&["serve".into(), "status".into(), "--json".into()]).await {
            Ok(out) => serve_status_has(&out, plan.https_port, &plan.target),
            Err(_) => false,
        }
    }

    async fn serve(&self, plan: &ServePlan, _confirmed: Confirmed) -> Result<String> {
        if plan.args.first().map(String::as_str) != Some("serve") || plan.args.iter().any(|a| a == "funnel") {
            bail!("refusing to run an unexpected tailscale command");
        }
        match self.run(&plan.args).await {
            Ok(out) => Ok(String::from_utf8_lossy(&out).trim().to_owned()),
            Err(RunError::Missing) => bail!("Tailscale is not installed"),
            Err(RunError::Timeout) => bail!("`{}` did not finish in time", plan.command),
            Err(RunError::Failed(line)) => bail!("`{}` failed: {line}", plan.command),
        }
    }
}

/// Test double: a fixed status, records `serve` calls.
pub struct Fake {
    pub status: Mutex<Status>,
    pub serve_calls: Mutex<Vec<String>>,
    pub serve_result: Mutex<std::result::Result<String, String>>,
}

impl Fake {
    pub fn new(status: Status) -> Fake {
        Fake { status: Mutex::new(status), serve_calls: Mutex::new(Vec::new()), serve_result: Mutex::new(Ok(String::new())) }
    }

    /// A running node `test.tail0000.ts.net` / `100.100.1.2`, HTTPS certs on.
    pub fn running() -> Fake {
        Fake::new(parse_status(include_bytes!("../tests/fixtures/status-running.json")))
    }
}

#[async_trait]
impl Tailscale for Fake {
    async fn status(&self) -> Status {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    async fn serving(&self, plan: &ServePlan) -> bool {
        self.serve_calls.lock().unwrap_or_else(|e| e.into_inner()).contains(&plan.command)
    }

    async fn serve(&self, plan: &ServePlan, _confirmed: Confirmed) -> Result<String> {
        self.serve_calls.lock().unwrap_or_else(|e| e.into_inner()).push(plan.command.clone());
        self.serve_result.lock().unwrap_or_else(|e| e.into_inner()).clone().map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_running_fixture() {
        let s = parse_status(include_bytes!("../tests/fixtures/status-running.json"));
        assert_eq!(s.state, State::Running);
        assert_eq!(s.version.as_deref(), Some("1.104.1"));
        assert_eq!(s.dns_name.as_deref(), Some("test-mac.tail0000.ts.net"));
        assert_eq!(s.ips, vec!["100.100.1.2", "fd7a:115c:a1e0::1:2"]);
        assert_eq!(s.ipv4(), Some("100.100.1.2"));
        assert_eq!(s.tailnet.as_deref(), Some("tail0000.ts.net"));
        assert_eq!(s.login.as_deref(), Some("tester@example.com"));
        assert!(s.https_certs && s.magic_dns);
    }

    #[test]
    fn parses_no_certs_and_other_states() {
        let s = parse_status(include_bytes!("../tests/fixtures/status-no-certs.json"));
        assert_eq!(s.state, State::Running);
        assert!(!s.https_certs, "CertDomains null");
        assert_eq!(parse_status(include_bytes!("../tests/fixtures/status-needs-login.json")).state, State::NeedsLogin);
        assert_eq!(parse_status(br#"{"BackendState":"Stopped"}"#).state, State::Stopped);
        assert_eq!(parse_status(b"not json").state, State::Error);
    }

    #[test]
    fn serve_plan_is_exact_and_never_funnel() {
        let p = ServePlan::new(8443, 7879);
        assert_eq!(p.command, "tailscale serve --bg --https=8443 http://127.0.0.1:7879");
        assert_eq!(p.undo, "tailscale serve --https=8443 off");
        assert!(!p.command.contains("funnel"));
    }

    #[test]
    fn serve_status_detection() {
        let j = br#"{"TCP":{"8443":{"HTTPS":true}},"Web":{"test-mac.tail0000.ts.net:8443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:7879"}}}}}"#;
        assert!(serve_status_has(j, 8443, "http://127.0.0.1:7879"));
        assert!(!serve_status_has(j, 443, "http://127.0.0.1:7879"));
        assert!(!serve_status_has(j, 8443, "http://127.0.0.1:7880"));
        assert!(!serve_status_has(b"{}", 8443, "http://127.0.0.1:7879"));
    }

    #[tokio::test]
    async fn missing_binary_is_not_installed() {
        let cli = Cli::new(None, DEFAULT_TIMEOUT);
        assert_eq!(cli.status().await.state, State::NotInstalled);
        let p = ServePlan::new(8443, 7879);
        assert!(cli.serve(&p, Confirmed::by_user()).await.is_err());
    }

    #[tokio::test]
    async fn fake_records_serve() {
        let f = Fake::running();
        let p = ServePlan::new(8443, 7879);
        assert!(!f.serving(&p).await);
        f.serve(&p, Confirmed::by_user()).await.unwrap();
        assert!(f.serving(&p).await);
    }
}
