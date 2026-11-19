//! The embedded python helper (`python/helper/`): written to `<root>/helper/` and run as
//! `python <root>/helper <command> …` (the folder's `__main__.py`).

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

/// Every helper file. A new helper module must be added here.
const FILES: &[(&str, &str)] = &[
    ("__main__.py", include_str!("../python/helper/__main__.py")),
    ("common.py", include_str!("../python/helper/common.py")),
    ("generate.py", include_str!("../python/helper/generate.py")),
    ("launch.py", include_str!("../python/helper/launch.py")),
    ("cookies.py", include_str!("../python/helper/cookies.py")),
    ("extensions.py", include_str!("../python/helper/extensions.py")),
    ("warm.py", include_str!("../python/helper/warm.py")),
];

/// Write the helper into `<root>/helper/` (only files whose content changed) and return its path.
pub fn install(root: &Path) -> PathBuf {
    let dir = root.join("helper");
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("cannot create {}: {e}", dir.display());
    }
    for (name, body) in FILES {
        let path = dir.join(name);
        if fs::read_to_string(&path).ok().as_deref() != Some(*body)
            && let Err(e) = fs::write(&path, body)
        {
            eprintln!("cannot write {}: {e}", path.display());
        }
    }
    // Single-file helper of older versions.
    let _ = fs::remove_file(root.join("helper.py"));
    dir
}

/// Run a short helper command and return the last JSON line of stdout. `{"error": …}` → Err.
pub async fn run(python: &Path, helper: &Path, args: &[&str], timeout: Duration) -> Result<Value> {
    let out = tokio::time::timeout(timeout, Command::new(python).arg(helper).args(args).stdin(Stdio::null()).kill_on_drop(true).output())
        .await
        .map_err(|_| anyhow!("helper {} took longer than {}s", args[0], timeout.as_secs()))?
        .with_context(|| format!("cannot run {}", python.display()))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = stdout
        .lines()
        .rev()
        .find_map(|l| serde_json::from_str(l).ok())
        .with_context(|| format!("helper {} returned no JSON: {}", args[0], String::from_utf8_lossy(&out.stderr).trim()))?;
    if v["ok"].as_bool() != Some(false)
        && let Some(e) = v["error"].as_str()
    {
        bail!("Camoufox: {e}");
    }
    Ok(v)
}
