//! Browser processes: spawn the helper's `launch`, wait for the ws endpoint, stop gracefully,
//! notice when the browser went away. Phase 8: per-profile lock (see `lock`), heartbeat,
//! `crashed` status for browsers that died without being asked to stop.

use crate::lock::{self, Acquire, Lock};
use anyhow::{Context, Result, bail};
use cm_core::{CmError, engine::CrashInfo};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
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
    /// Windows: the Job Object the helper (and so Firefox) lives in; closing it kills them all.
    job: Option<job::Job>,
    /// Held while the browser runs; released (flock + body) when the `Proc` is dropped.
    /// Profile directory (`<root>/profiles/<id>`).
    dir: PathBuf,
    _lock: Arc<StdMutex<Lock>>,
    heartbeat: tokio::task::JoinHandle<()>,
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.heartbeat.abort();
    }
}

#[derive(Default)]
pub struct Procs {
    map: Mutex<HashMap<String, Proc>>,
    /// Browsers that died on their own and were not acknowledged yet (cleared by the next start).
    crashes: StdMutex<HashMap<String, CrashInfo>>,
}

/// How a helper that exited by itself is reported.
fn exit_detail(st: &std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        match (st.code(), st.signal()) {
            (Some(c), _) => format!("browser process exited with code {c}"),
            (_, Some(s)) => format!("browser process killed by signal {s}"),
            _ => "browser process ended".into(),
        }
    }
    #[cfg(not(unix))]
    match st.code() {
        Some(c) => format!("browser process exited with code {c}"),
        None => "browser process ended".into(),
    }
}

#[cfg(windows)]
mod job {
    use std::ffi::c_void;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
        },
    };

    /// A Job Object with kill-on-close: every process in it dies when the handle closes.
    pub struct Job(HANDLE);
    // SAFETY: a job handle is a plain kernel handle, usable from any thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> Option<Job> {
            // SAFETY: plain Win32 calls; the struct is zeroed then only LimitFlags is set.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    CloseHandle(h);
                    return None;
                }
                Some(Job(h))
            }
        }

        pub fn assign(&self, process: HANDLE) -> bool {
            // SAFETY: both handles are valid for the duration of the call.
            unsafe { AssignProcessToJobObject(self.0, process) != 0 }
        }

        pub fn kill(&self) {
            // SAFETY: valid job handle.
            unsafe { TerminateJobObject(self.0, 1) };
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: we own the handle; kill-on-close takes the remaining processes down.
            unsafe { CloseHandle(self.0) };
        }
    }
}
#[cfg(not(windows))]
mod job {
    pub struct Job;
}

/// Hard-stop the helper's whole tree: unix signals its process group, Windows terminates the job.
/// `graceful` (SIGTERM) is a no-op on Windows, where the stdin `stop` line is the graceful path.
pub fn kill_tree(pgid: Option<u32>, job: Option<&job::Job>, graceful: bool) {
    #[cfg(unix)]
    {
        let _ = job;
        if let Some(g) = pgid {
            let sig = if graceful { "-TERM" } else { "-KILL" };
            let _ = std::process::Command::new("/bin/kill").args([sig, &format!("-{g}")]).stderr(Stdio::null()).status();
        }
    }
    #[cfg(windows)]
    {
        let _ = pgid;
        if !graceful && let Some(j) = job {
            j.kill();
        }
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
    pub async fn spawn(&self, id: &str, profile_dir: &Path, mut cmd: Command, log_path: &Path) -> Result<String> {
        // Lock first: a profile open in another app instance (or process) is refused.
        let (acq, stale) = Lock::acquire(profile_dir).context("cannot create the profile lock")?;
        let mut lock = match acq {
            Acquire::Got(l) => l,
            Acquire::Busy(info) => {
                let who = info.map(|i| format!(" (pid {})", i.owner_pid)).unwrap_or_default();
                bail!(CmError::Conflict(format!("profile is already open in another Camoufox Manager instance{who}")));
            }
        };
        if stale.is_some() {
            // A crashed run left a lock behind: stop whatever it left running in this profile.
            lock::kill_procs(&lock::procs_of(profile_dir, &lock::ps_all()));
        }
        self.crashes.lock().unwrap().remove(id);
        let log = fs::File::create(log_path)?;
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(log);
        #[cfg(unix)]
        cmd.process_group(0);
        #[cfg(windows)]
        cmd.creation_flags(0x0000_0200); // CREATE_NEW_PROCESS_GROUP
        let mut child = cmd.spawn().context("cannot start the Camoufox helper")?;
        #[cfg(windows)]
        let job = {
            let j = job::Job::new();
            if let (Some(j), Some(h)) = (&j, child.raw_handle()) {
                j.assign(h as _);
            }
            j
        };
        #[cfg(not(windows))]
        let job: Option<job::Job> = None;
        let pgid = child.id();
        lock.set_helper(pgid);
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
                kill_tree(pgid, job.as_ref(), false);
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
        let lock = Arc::new(StdMutex::new(lock));
        let hb = lock.clone();
        let heartbeat = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                hb.lock().unwrap().heartbeat();
            }
        });
        self.map
            .lock()
            .await
            .insert(id.to_string(), Proc { child, stdin, pgid, job, dir: profile_dir.to_path_buf(), _lock: lock, heartbeat });
        Ok(ws)
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        let Some(mut p) = self.map.lock().await.remove(id) else {
            self.crashes.lock().unwrap().remove(id);
            return Ok(());
        };
        // Graceful: the helper gets "stop" → closes the context (Firefox writes cookies/session) → exits.
        if let Some(mut stdin) = p.stdin.take() {
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }
        if tokio::time::timeout(Duration::from_secs(20), p.child.wait()).await.is_ok() {
            return Ok(());
        }
        kill_tree(p.pgid, p.job.as_ref(), true);
        if tokio::time::timeout(Duration::from_secs(5), p.child.wait()).await.is_err() {
            kill_tree(p.pgid, p.job.as_ref(), false);
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
            res => {
                // The helper is gone without us asking. Exit 0 + Firefox shut down cleanly = the
                // user closed the last window. Anything else is a crash: helper signal/error, a
                // Firefox left running, or Firefox's `.startup-incomplete` marker still on disk
                // (Firefox removes it on a clean shutdown only), e.g. the browser was killed.
                kill_tree(p.pgid, p.job.as_ref(), false);
                let mut why = match &res {
                    Ok(Some(st)) if st.success() => None,
                    Ok(Some(st)) => Some(exit_detail(st)),
                    Ok(None) => Some("browser process ended".into()),
                    Err(e) => Some(format!("cannot read the browser state: {e}")),
                };
                let mut left = lock::procs_of(&p.dir, &lock::ps_all());
                for _ in 0..15 {
                    if left.is_empty() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await; // Firefox finishing a clean exit
                    left = lock::procs_of(&p.dir, &lock::ps_all());
                }
                if !left.is_empty() {
                    lock::kill_procs(&left);
                    why.get_or_insert_with(|| "the browser helper died and left Firefox running".into());
                } else if why.is_none() && p.dir.join("user-data/.startup-incomplete").exists() {
                    why = Some("the browser exited without shutting down cleanly".into());
                }
                if let Some(detail) = why {
                    self.crashes.lock().unwrap().insert(id.to_string(), CrashInfo { at: lock::now(), detail });
                }
                map.remove(id);
                Some(false)
            }
        }
    }

    pub fn crashed(&self, id: &str) -> Option<CrashInfo> {
        self.crashes.lock().unwrap().get(id).cloned()
    }

    /// Profiles a crashed run left behind (found at startup) show as crashed too.
    pub fn mark_crashed(&self, id: &str, detail: &str) {
        self.crashes.lock().unwrap().insert(id.to_string(), CrashInfo { at: lock::now(), detail: detail.into() });
    }
}
