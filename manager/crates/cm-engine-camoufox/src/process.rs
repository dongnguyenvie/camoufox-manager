//! Browser processes: spawn the helper's `launch`, wait for the ws endpoint, stop gracefully,
//! notice when the user closed the window. Phase 8 owns this module (lock files, orphan
//! cleanup, crash status).

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::Mutex,
};

struct Proc {
    child: Child,
    stdin: Option<ChildStdin>,
    /// The helper runs in its own process group (pgid = pid) so Firefox children die with it.
    pgid: Option<u32>,
}

#[derive(Default)]
pub struct Procs {
    map: Mutex<HashMap<String, Proc>>,
}

pub fn kill_group(pgid: Option<u32>, sig: &str) {
    if let Some(g) = pgid {
        let _ = std::process::Command::new("/bin/kill").args([sig, &format!("-{g}")]).stderr(Stdio::null()).status();
    }
}

/// Last lines of the launch log, for a readable error.
fn log_tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(6)..].join("\n")
}

impl Procs {
    /// Spawn `cmd` (stdout = JSON lines, stderr → `log_path`), wait for `{"ws": …}`.
    pub async fn spawn(&self, id: &str, mut cmd: Command, log_path: &Path) -> Result<String> {
        let log = fs::File::create(log_path)?;
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(log).process_group(0);
        let mut child = cmd.spawn().context("cannot start the Camoufox helper")?;
        let pgid = child.id();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().context("helper has no stdout")?;

        let mut lines = BufReader::new(stdout).lines();
        let waited = tokio::time::timeout(Duration::from_secs(120), async {
            while let Some(line) = lines.next_line().await? {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if let Some(ws) = v["ws"].as_str() {
                        return Ok(ws.to_string());
                    }
                    if let Some(e) = v["error"].as_str() {
                        bail!("{e}");
                    }
                }
            }
            bail!("helper exited before the browser opened")
        })
        .await;
        let ws = match waited {
            Ok(Ok(ws)) => ws,
            other => {
                kill_group(pgid, "-KILL");
                let _ = child.wait().await;
                let why = match other {
                    Ok(Err(e)) => format!("{e:#}"),
                    _ => "browser did not open within 120s".into(),
                };
                bail!("Camoufox failed to start: {why}\n{}", log_tail(log_path));
            }
        };
        // Keep draining stdout so the helper never blocks on a full pipe.
        tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
        self.map.lock().await.insert(id.to_string(), Proc { child, stdin, pgid });
        Ok(ws)
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        let Some(mut p) = self.map.lock().await.remove(id) else { return Ok(()) };
        // Graceful: the helper gets "stop" → closes the context (Firefox writes cookies/session) → exits.
        if let Some(mut stdin) = p.stdin.take() {
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }
        if tokio::time::timeout(Duration::from_secs(20), p.child.wait()).await.is_ok() {
            return Ok(());
        }
        kill_group(p.pgid, "-TERM");
        if tokio::time::timeout(Duration::from_secs(5), p.child.wait()).await.is_err() {
            kill_group(p.pgid, "-KILL");
            let _ = p.child.wait().await;
        }
        Ok(())
    }

    /// `Some(true)` running, `Some(false)` not running (exited processes are reaped here).
    pub async fn alive(&self, id: &str) -> Option<bool> {
        let mut map = self.map.lock().await;
        let Some(p) = map.get_mut(id) else { return Some(false) };
        match p.child.try_wait() {
            Ok(None) => Some(true),
            _ => {
                // User closed the window (or crash): clean up leftover children too.
                kill_group(p.pgid, "-KILL");
                map.remove(id);
                Some(false)
            }
        }
    }
}
