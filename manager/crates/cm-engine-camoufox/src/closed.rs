//! Closed-profile proof (phase 15): the evidence a snapshot export / journalled import needs that
//! nobody is using a profile's user-data dir.
//!
//! Three independent signals, all of which must say "closed":
//!
//! 1. no process of this engine runs the profile (checked by the caller via `Procs::alive`),
//! 2. the per-profile `app.lock` flock ([`crate::lock`]) is acquirable. The returned
//!    [`ClosedGuard`] **keeps holding it**, so the profile cannot start (in this or another app
//!    instance) while a capture or an apply is in progress,
//! 3. (Windows: `parent.lock`, which Firefox holds open exclusively: it cannot be opened with
//!    share mode none while Firefox runs.) Firefox's own markers are absent: `user-data/lock` (a symlink `<ip>:+<pid>`) naming a live
//!    pid, and a held fcntl lock on `user-data/.parentlock`. A marker left by a dead process is
//!    stale and ignored, because Firefox itself reclaims it.

use crate::lock::{Acquire, Lock};
use cm_core::CmError;
use std::{fs, path::Path};

/// Keeps the profile closed while alive (holds `app.lock`).
pub struct ClosedGuard {
    _lock: Lock,
}

fn conflict(msg: impl Into<String>) -> anyhow::Error {
    CmError::Conflict(msg.into()).into()
}

/// Live-process markers Firefox leaves in a user-data dir (empty = none).
#[cfg(unix)]
pub fn firefox_markers(user_data: &Path) -> Vec<String> {
    use crate::lock::proc_start;
    let mut found = Vec::new();
    // `lock` -> "<ip>:+<pid>" (dangling symlink while Firefox runs).
    if let Ok(target) = fs::read_link(user_data.join("lock")) {
        let pid = target.to_string_lossy().rsplit('+').next().and_then(|p| p.parse::<u32>().ok());
        if pid.is_none_or(|p| proc_start(p).is_some()) {
            found.push("lock".to_string());
        }
    } else if user_data.join("lock").exists() {
        found.push("lock".to_string()); // a regular file named `lock`: not ours to judge
    }
    if parentlock_held(&user_data.join(".parentlock")) {
        found.push(".parentlock".to_string());
    }
    found
}

/// Does another process hold an fcntl lock on `path`?
#[cfg(unix)]
fn parentlock_held(path: &Path) -> bool {
    use std::os::fd::AsRawFd;
    let Ok(f) = fs::File::open(path) else { return false };
    // SAFETY: F_GETLK on a valid fd with a zeroed flock struct.
    unsafe {
        let mut fl: libc::flock = std::mem::zeroed();
        fl.l_type = libc::F_WRLCK as _;
        fl.l_whence = libc::SEEK_SET as _;
        libc::fcntl(f.as_raw_fd(), libc::F_GETLK, &mut fl) == 0 && i64::from(fl.l_type) != i64::from(libc::F_UNLCK)
    }
}

/// Live-process markers Firefox leaves in a user-data dir (empty = none). On Windows Firefox keeps
/// `parent.lock` open exclusively, so it is running iff the file cannot be opened with share mode
/// none (a missing file means closed).
#[cfg(windows)]
pub fn firefox_markers(user_data: &Path) -> Vec<String> {
    use std::os::windows::fs::OpenOptionsExt;
    let path = user_data.join("parent.lock");
    if !path.exists() {
        return vec![];
    }
    match fs::OpenOptions::new().read(true).share_mode(0).open(&path) {
        Ok(_) => vec![],
        Err(_) => vec!["parent.lock".to_string()],
    }
}

/// Prove `<profile_dir>` closed and hold it closed. Errors are `CmError::Conflict`.
pub fn prove_closed(profile_dir: &Path) -> anyhow::Result<ClosedGuard> {
    let lock = match Lock::acquire(profile_dir)?.0 {
        Acquire::Got(l) => l,
        Acquire::Busy(info) => {
            let who = info.map(|i| format!(" (pid {})", i.owner_pid)).unwrap_or_default();
            return Err(conflict(format!("the profile is open{who}; close it first")));
        }
    };
    let markers = firefox_markers(&profile_dir.join("user-data"));
    if !markers.is_empty() {
        return Err(conflict(format!("the browser seems to be running (found {}); close it first", markers.join(", "))));
    }
    Ok(ClosedGuard { _lock: lock })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cm-closed-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("user-data")).unwrap();
        d
    }

    #[test]
    fn free_profile_is_held_and_blocks_a_second_holder() {
        let d = tmp("held");
        let g = prove_closed(&d).unwrap();
        // While the proof is held nobody else (an engine start, another proof) gets the lock.
        assert!(prove_closed(&d).is_err());
        assert!(matches!(Lock::acquire(&d).unwrap().0, Acquire::Busy(_)));
        drop(g);
        assert!(prove_closed(&d).is_ok());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn running_owner_means_not_closed() {
        let d = tmp("owner");
        let Acquire::Got(owner) = Lock::acquire(&d).unwrap().0 else { panic!("free") };
        let e = prove_closed(&d).err().expect("busy");
        assert!(matches!(e.downcast_ref::<CmError>(), Some(CmError::Conflict(_))));
        drop(owner);
        assert!(prove_closed(&d).is_ok());
        let _ = fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn firefox_lock_marker_with_live_pid_blocks_but_stale_does_not() {
        let d = tmp("marker");
        let ud = d.join("user-data");
        std::os::unix::fs::symlink(format!("127.0.0.1:+{}", std::process::id()), ud.join("lock")).unwrap();
        let e = prove_closed(&d).err().expect("marker with live pid");
        assert!(e.to_string().contains("lock"), "{e}");
        fs::remove_file(ud.join("lock")).unwrap();
        // pid far above any real one: stale
        std::os::unix::fs::symlink("127.0.0.1:+999999999", ud.join("lock")).unwrap();
        assert!(prove_closed(&d).is_ok(), "stale marker ignored");
        let _ = fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn parentlock_held_by_another_process_blocks() {
        let d = tmp("parent");
        let p = d.join("user-data/.parentlock");
        fs::write(&p, b"").unwrap();
        assert!(firefox_markers(&d.join("user-data")).is_empty(), "unlocked file is no marker");
        // fcntl locks are per process, so take it from a child process.
        let mut child = std::process::Command::new("/usr/bin/python3")
            .args([
                "-c",
                "import fcntl,sys,time\nf=open(sys.argv[1],'r+')\nfcntl.lockf(f,fcntl.LOCK_EX)\nprint('locked',flush=True)\ntime.sleep(30)",
            ])
            .arg(&p)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(child.stdout.take().unwrap()), &mut line).unwrap();
        assert!(line.starts_with("locked"));
        assert_eq!(firefox_markers(&d.join("user-data")), vec![".parentlock".to_string()]);
        assert!(prove_closed(&d).is_err());
        let _ = child.kill();
        let _ = child.wait();
        assert!(prove_closed(&d).is_ok());
        let _ = fs::remove_dir_all(&d);
    }

    #[cfg(windows)]
    #[test]
    fn parent_lock_held_exclusively_blocks() {
        use std::os::windows::fs::OpenOptionsExt;
        let d = tmp("parentlock");
        let p = d.join("user-data/parent.lock");
        assert!(firefox_markers(&d.join("user-data")).is_empty(), "missing file is no marker");
        fs::write(&p, b"").unwrap();
        assert!(firefox_markers(&d.join("user-data")).is_empty(), "unopened file is no marker");
        let held = fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&p).unwrap();
        assert_eq!(firefox_markers(&d.join("user-data")), vec!["parent.lock".to_string()]);
        let e = prove_closed(&d).err().expect("running");
        assert!(e.to_string().contains("parent.lock"), "{e}");
        drop(held);
        assert!(prove_closed(&d).is_ok());
        let _ = fs::remove_dir_all(&d);
    }
}
