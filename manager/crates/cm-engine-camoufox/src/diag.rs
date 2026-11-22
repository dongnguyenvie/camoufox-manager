//! Read-only diagnostics for the doctor (phase 19). Nothing here stops, starts or writes anything
//! in a profile: it only looks (helper files on disk, lock files, the process list).

use crate::{helper, lock};
use std::{
    fs,
    path::Path,
};

/// State of the embedded python helper under `<root>/helper/`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HelperState {
    /// Files that do not exist.
    pub missing: Vec<String>,
    /// Files whose content differs from the version embedded in this build.
    pub outdated: Vec<String>,
    pub total: usize,
}

pub fn helper_state(root: &Path) -> HelperState {
    let dir = root.join("helper");
    let mut st = HelperState { total: helper::FILES.len(), ..Default::default() };
    for (name, body) in helper::FILES {
        match fs::read_to_string(dir.join(name)) {
            Ok(s) if s == *body => {}
            Ok(_) => st.outdated.push((*name).to_string()),
            Err(_) => st.missing.push((*name).to_string()),
        }
    }
    st
}

/// A profile with leftovers nobody owns.
#[derive(Clone, Debug, PartialEq)]
pub struct Orphan {
    pub profile: String,
    /// Helper/browser processes still running with this profile's directory on their command line.
    pub pids: Vec<u32>,
    /// A lock body left behind by an owner that died (no processes needed).
    pub stale_lock: bool,
}

/// Leftover helper/browser processes and stale locks under `<root>/profiles/`. Same matching as the
/// startup cleanup (`lock::recover_orphans`: command line containing the profile directory, never
/// the process name), but report-only. A profile whose lock is held by a live owner (a browser this
/// app is running) is not an orphan.
pub fn find_orphans(root: &Path) -> Vec<Orphan> {
    let profiles = root.join("profiles");
    let Ok(rd) = fs::read_dir(&profiles) else { return vec![] };
    let all = lock::ps_all();
    let mut out = vec![];
    for e in rd.flatten() {
        let dir = e.path();
        let Some(id) = dir.file_name().and_then(|n| n.to_str()).map(str::to_string) else { continue };
        if !dir.is_dir() {
            continue;
        }
        let pids: Vec<u32> = lock::procs_of(&dir, &all).into_iter().map(|p| p.0).collect();
        let lock_path = dir.join(lock::LOCK_FILE);
        let has_lock_file = lock_path.is_file();
        if pids.is_empty() && !has_lock_file {
            continue;
        }
        if has_lock_file && lock::lock_free(&dir) == Some(false) {
            continue; // owned by a live process
        }
        let stale_lock =
            fs::read(&lock_path).ok().and_then(|b| serde_json::from_slice::<lock::LockInfo>(&b).ok()).is_some_and(|i| i.owner_pid != 0);
        if !pids.is_empty() || stale_lock {
            out.push(Orphan { profile: id, pids, stale_lock });
        }
    }
    out.sort_by(|a, b| a.profile.cmp(&b.profile));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("cm-diag-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn helper_state_reports_missing_and_outdated() {
        let root = tmp("helper");
        assert_eq!(helper_state(&root).missing.len(), helper::FILES.len());
        helper::install(&root);
        assert_eq!(helper_state(&root), HelperState { total: helper::FILES.len(), ..Default::default() });
        fs::write(root.join("helper/common.py"), "# edited").unwrap();
        fs::remove_file(root.join("helper/warm.py")).unwrap();
        let st = helper_state(&root);
        assert_eq!(st.outdated, ["common.py"]);
        assert_eq!(st.missing, ["warm.py"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn orphans_are_stale_locks_not_live_ones() {
        let root = tmp("orphans");
        let stale = root.join("profiles/aaaa");
        let live = root.join("profiles/bbbb");
        fs::create_dir_all(&stale).unwrap();
        fs::create_dir_all(&live).unwrap();
        let body = serde_json::to_vec(&lock::LockInfo { v: 1, owner_pid: 99999, ..Default::default() }).unwrap();
        fs::write(stale.join(lock::LOCK_FILE), &body).unwrap();
        let before = fs::read(stale.join(lock::LOCK_FILE)).unwrap();
        let lock::Acquire::Got(_held) = lock::Lock::acquire(&live).unwrap().0 else { panic!("free") };
        let found = find_orphans(&root);
        assert_eq!(found, vec![Orphan { profile: "aaaa".into(), pids: vec![], stale_lock: true }]);
        assert_eq!(fs::read(stale.join(lock::LOCK_FILE)).unwrap(), before, "report only: lock file untouched");
        let _ = fs::remove_dir_all(&root);
    }
}
