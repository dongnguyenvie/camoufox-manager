//! Per-profile lock file `<profile dir>/app.lock` (phase 8).
//!
//! The file holds an exclusive OS file lock (`File::try_lock`) for as long as this process runs the
//! profile, so two app instances (or any two processes) never open one profile: the OS drops the
//! lock when the owner dies, even on `kill -9`. The JSON body is for humans and for recovery:
//!
//! ```json
//! {"v":1,"ownerPid":123,"ownerStart":"1790000000","helperPid":456,
//!  "helperStart":"…","startedAt":1790000000,"heartbeat":1790000005}
//! ```
//!
//! `heartbeat` is rewritten every few seconds while the browser runs. A body left behind with
//! nobody holding the lock means the owner crashed. On Windows the lock is taken on the sidecar
//! `app.lock.guard` (byte-range locks are mandatory there and would hide the body from readers).
//!
//! Orphan cleanup matches processes by **command line containing this profile's directory**
//! (`<root>/profiles/<id>/`: helper `--user-data`, Firefox `-profile`), never by name, so an
//! unrelated Firefox is never touched.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub const LOCK_FILE: &str = "app.lock";
#[cfg(windows)]
const GUARD_FILE: &str = "app.lock.guard";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LockInfo {
    pub v: u32,
    pub owner_pid: u32,
    pub owner_start: String,
    pub helper_pid: Option<u32>,
    pub helper_start: Option<String>,
    pub started_at: u64,
    pub heartbeat: u64,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Start time of a process (seconds since the epoch, as text), `None` when it does not exist.
pub fn proc_start(pid: u32) -> Option<String> {
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[p]), true, ProcessRefreshKind::nothing());
    sys.process(p).map(|p| p.start_time().to_string())
}

/// A held lock. Dropping it releases the file lock and clears the body.
pub struct Lock {
    file: File,
    /// The file that carries the OS lock when it is not `file` itself (Windows sidecar).
    guard: Option<File>,
    info: LockInfo,
}

pub enum Acquire {
    Got(Lock),
    /// Another live process holds it (its info when readable).
    Busy(Option<LockInfo>),
}

fn open(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)
}

#[cfg(unix)]
fn read_at(f: &File, buf: &mut [u8]) -> std::io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(f, buf, 0)
}
#[cfg(windows)]
fn read_at(f: &File, buf: &mut [u8]) -> std::io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(f, buf, 0)
}

#[cfg(unix)]
fn write_at(f: &File, buf: &[u8]) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(f, buf, 0)
}
#[cfg(windows)]
fn write_at(f: &File, buf: &[u8]) -> std::io::Result<()> {
    let mut off = 0u64;
    let mut rest = buf;
    while !rest.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_write(f, rest, off)?;
        if n == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        off += n as u64;
        rest = &rest[n..];
    }
    Ok(())
}

fn read_info(f: &File) -> Option<LockInfo> {
    let mut buf = vec![0u8; 4096];
    let n = read_at(f, &mut buf).ok()?;
    serde_json::from_slice(&buf[..n]).ok()
}

/// Is the profile's lock free right now? Probing locks and releases at once; the body is never
/// touched. `None` when there is no lock file.
pub fn lock_free(dir: &Path) -> Option<bool> {
    if !dir.join(LOCK_FILE).is_file() {
        return None;
    }
    #[cfg(windows)]
    let path = dir.join(GUARD_FILE);
    #[cfg(unix)]
    let path = dir.join(LOCK_FILE);
    let f = match OpenOptions::new().read(true).write(true).open(&path) {
        Ok(f) => f,
        #[cfg(windows)]
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(true),
        Err(_) => return None,
    };
    if f.try_lock().is_ok() {
        let _ = f.unlock();
        Some(true)
    } else {
        Some(false)
    }
}

impl Lock {
    /// Take the profile lock. When it is free but a body is left (crashed owner), the stale
    /// info is returned so the caller can clean up what that run left behind.
    pub fn acquire(dir: &Path) -> std::io::Result<(Acquire, Option<LockInfo>)> {
        fs::create_dir_all(dir)?;
        let f = open(&dir.join(LOCK_FILE))?;
        #[cfg(windows)]
        let guard = Some(open(&dir.join(GUARD_FILE))?);
        #[cfg(unix)]
        let guard: Option<File> = None;
        if guard.as_ref().unwrap_or(&f).try_lock().is_err() {
            return Ok((Acquire::Busy(read_info(&f)), None));
        }
        let stale = read_info(&f).filter(|i| i.owner_pid != 0);
        let info = LockInfo {
            v: 1,
            owner_pid: std::process::id(),
            owner_start: proc_start(std::process::id()).unwrap_or_default(),
            started_at: now(),
            heartbeat: now(),
            ..Default::default()
        };
        let lock = Lock { file: f, guard, info };
        lock.write()?;
        Ok((Acquire::Got(lock), stale))
    }

    fn write(&self) -> std::io::Result<()> {
        let body = serde_json::to_vec(&self.info).unwrap_or_default();
        self.file.set_len(0)?;
        write_at(&self.file, &body)
    }

    pub fn set_helper(&mut self, pid: Option<u32>) {
        self.info.helper_pid = pid;
        self.info.helper_start = pid.and_then(proc_start);
        let _ = self.write();
    }

    pub fn heartbeat(&mut self) {
        self.info.heartbeat = now();
        let _ = self.write();
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
        let _ = self.file.flush();
        let _ = self.guard.as_ref().unwrap_or(&self.file).unlock();
    }
}

/// One process: pid, process group (unix; 0 on Windows), full command line.
pub struct ProcLine {
    pub pid: u32,
    pub pgid: u32,
    pub cmd: String,
}

pub fn ps_all() -> Vec<ProcLine> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always));
    sys.processes()
        .iter()
        .map(|(pid, p)| {
            let pid = pid.as_u32();
            let cmd = p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" ");
            #[cfg(unix)]
            // SAFETY: getpgid(2) only reads a process attribute.
            let pgid = unsafe { libc::getpgid(pid as libc::pid_t) }.max(0) as u32;
            #[cfg(windows)]
            let pgid = 0;
            ProcLine { pid, pgid, cmd }
        })
        .collect()
}

/// Path text as compared against command lines (Windows: case-insensitive, backslashes).
fn norm(s: &str) -> String {
    if cfg!(windows) { s.replace('/', "\\").to_lowercase() } else { s.to_string() }
}

/// Processes whose command line mentions `<dir>/` (the profile directory).
pub fn procs_of(dir: &Path, all: &[ProcLine]) -> Vec<(u32, u32)> {
    let needle = norm(&format!("{}{}", dir.display(), std::path::MAIN_SEPARATOR));
    let me = std::process::id();
    all.iter().filter(|p| p.pid != me && norm(&p.cmd).contains(&needle)).map(|p| (p.pid, p.pgid)).collect()
}

#[cfg(unix)]
fn signal(pid: i64, sig: i32) {
    // SAFETY: kill(2); negative pid = process group.
    unsafe { libc::kill(pid as libc::pid_t, sig) };
}

fn wait_gone(found: &[(u32, u32)]) {
    for _ in 0..40 {
        if found.iter().all(|&(pid, _)| proc_start(pid).is_none()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Stop the matched processes (unix: SIGTERM their groups/pids, wait, then SIGKILL what is left;
/// Windows: terminate them). Returns the pids that were running.
pub fn kill_procs(found: &[(u32, u32)]) -> Vec<u32> {
    if found.is_empty() {
        return vec![];
    }
    #[cfg(unix)]
    {
        // SAFETY: getpgrp(2) has no failure mode.
        let my_pgid = unsafe { libc::getpgrp() } as u32;
        let send = |sig: i32| {
            for &(pid, pgid) in found {
                // The helper leads its own group (pgid == its pid): take Firefox's children along.
                if pgid > 1 && pgid != my_pgid {
                    signal(-(pgid as i64), sig);
                }
                signal(pid as i64, sig);
            }
        };
        send(libc::SIGTERM);
        wait_gone(found);
        send(libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let pids: Vec<Pid> = found.iter().map(|f| Pid::from_u32(f.0)).collect();
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, ProcessRefreshKind::nothing());
        for p in &pids {
            if let Some(proc_) = sys.process(*p) {
                proc_.kill();
            }
        }
        wait_gone(found);
    }
    found.iter().map(|f| f.0).collect()
}

/// App startup: stop browsers/helpers a crashed run left behind. Only profiles whose lock is
/// free (nobody alive owns them) are touched. Returns the ids that had leftovers
/// (a stale lock body or live processes).
pub fn recover_orphans(profiles_dir: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(profiles_dir) else { return vec![] };
    let all = ps_all();
    let mut recovered = vec![];
    for e in rd.flatten() {
        let dir: PathBuf = e.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(id) = dir.file_name().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        let found = procs_of(&dir, &all);
        if found.is_empty() && !dir.join(LOCK_FILE).is_file() {
            continue;
        }
        match Lock::acquire(&dir) {
            Ok((Acquire::Got(lock), stale)) => {
                let killed = kill_procs(&found);
                drop(lock);
                if !killed.is_empty() || stale.is_some() {
                    recovered.push(id);
                }
            }
            _ => continue, // owned by another live instance
        }
    }
    recovered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cm-lock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn second_holder_is_refused_until_release() {
        let d = tmp("busy");
        let (Acquire::Got(mut a), stale) = Lock::acquire(&d).unwrap() else { panic!("free") };
        assert!(stale.is_none());
        a.set_helper(Some(std::process::id()));
        assert_eq!(lock_free(&d), Some(false));
        match Lock::acquire(&d).unwrap().0 {
            Acquire::Busy(Some(i)) => assert_eq!(i.owner_pid, std::process::id()),
            _ => panic!("must be busy"),
        }
        drop(a);
        assert_eq!(lock_free(&d), Some(true));
        assert!(matches!(Lock::acquire(&d).unwrap().0, Acquire::Got(_)));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn stale_body_is_reported_and_unrelated_procs_are_not_matched() {
        let d = tmp("stale");
        let body = serde_json::to_vec(&LockInfo { v: 1, owner_pid: 99999, ..Default::default() }).unwrap();
        fs::write(d.join(LOCK_FILE), body).unwrap();
        let (a, stale) = Lock::acquire(&d).unwrap();
        assert!(matches!(a, Acquire::Got(_)));
        assert_eq!(stale.unwrap().owner_pid, 99999);
        // This test process's own command line does not contain the temp profile dir.
        assert!(procs_of(&d, &ps_all()).is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn own_process_is_found_by_start_time() {
        assert!(proc_start(std::process::id()).is_some());
        assert!(proc_start(999_999_999).is_none());
    }
}
