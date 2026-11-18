//! Per-profile lock file `<profile dir>/app.lock` (phase 8).
//!
//! The file holds an exclusive `flock` for as long as this process runs the profile, so two app
//! instances (or any two processes) never open one profile: the kernel drops the lock when the
//! owner dies, even on `kill -9`. The JSON body is for humans and for recovery:
//!
//! ```json
//! {"v":1,"ownerPid":123,"ownerStart":"Sat Nov 22 18:00:00 2026","helperPid":456,
//!  "helperStart":"…","startedAt":1790000000,"heartbeat":1790000005}
//! ```
//!
//! `heartbeat` is rewritten every few seconds while the browser runs. A body left behind with
//! nobody holding the flock means the owner crashed.
//!
//! Orphan cleanup matches processes by **command line containing this profile's directory**
//! (`<root>/profiles/<id>/`: helper `--user-data`, Firefox `-profile`), never by name, so an
//! unrelated Firefox is never touched.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::{fd::AsRawFd, unix::fs::FileExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const LOCK_FILE: &str = "app.lock";

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

/// Start time string of a process (`ps -o lstart=`), `None` when it does not exist.
pub fn proc_start(pid: u32) -> Option<String> {
    let out = Command::new("/bin/ps").args(["-o", "lstart=", "-p", &pid.to_string()]).stderr(Stdio::null()).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !s.is_empty()).then_some(s)
}

/// A held lock. Dropping it releases the flock and clears the body.
pub struct Lock {
    file: File,
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

fn try_flock(f: &File) -> bool {
    // SAFETY: plain flock(2) on a valid fd.
    unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
}

fn read_info(f: &File) -> Option<LockInfo> {
    let mut buf = vec![0u8; 4096];
    let n = f.read_at(&mut buf, 0).ok()?;
    serde_json::from_slice(&buf[..n]).ok()
}

impl Lock {
    /// Take the profile lock. When it is free but a body is left (crashed owner), the stale
    /// info is returned so the caller can clean up what that run left behind.
    pub fn acquire(dir: &Path) -> std::io::Result<(Acquire, Option<LockInfo>)> {
        fs::create_dir_all(dir)?;
        let path = dir.join(LOCK_FILE);
        let f = open(&path)?;
        if !try_flock(&f) {
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
        let lock = Lock { file: f, info };
        lock.write()?;
        Ok((Acquire::Got(lock), stale))
    }

    fn write(&self) -> std::io::Result<()> {
        let body = serde_json::to_vec(&self.info).unwrap_or_default();
        self.file.set_len(0)?;
        self.file.write_all_at(&body, 0)
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
        // SAFETY: unlock our own fd (also released when the file closes).
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// One line of `ps`: pid, process group, full command.
pub struct ProcLine {
    pub pid: u32,
    pub pgid: u32,
    pub cmd: String,
}

pub fn ps_all() -> Vec<ProcLine> {
    let Ok(out) = Command::new("/bin/ps").args(["-axww", "-o", "pid=,pgid=,command="]).stderr(Stdio::null()).output() else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.trim_start().splitn(3, char::is_whitespace);
            let pid = it.next()?.parse().ok()?;
            let pgid = it.next()?.trim().parse().ok()?;
            Some(ProcLine { pid, pgid, cmd: it.next()?.trim().to_string() })
        })
        .collect()
}

/// Processes whose command line mentions `<dir>/` (the profile directory).
pub fn procs_of(dir: &Path, all: &[ProcLine]) -> Vec<(u32, u32)> {
    let needle = format!("{}/", dir.display());
    let me = std::process::id();
    all.iter().filter(|p| p.pid != me && p.cmd.contains(&needle)).map(|p| (p.pid, p.pgid)).collect()
}

fn signal(pid: i64, sig: i32) {
    // SAFETY: kill(2); negative pid = process group.
    unsafe { libc::kill(pid as libc::pid_t, sig) };
}

/// Stop the matched processes: SIGTERM their groups/pids, wait, then SIGKILL what is left.
/// Returns the pids that were running.
pub fn kill_procs(found: &[(u32, u32)]) -> Vec<u32> {
    if found.is_empty() {
        return vec![];
    }
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
    for _ in 0..40 {
        if found.iter().all(|&(pid, _)| proc_start(pid).is_none()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    send(libc::SIGKILL);
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
        match Lock::acquire(&d).unwrap().0 {
            Acquire::Busy(Some(i)) => assert_eq!(i.owner_pid, std::process::id()),
            _ => panic!("must be busy"),
        }
        drop(a);
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
}
