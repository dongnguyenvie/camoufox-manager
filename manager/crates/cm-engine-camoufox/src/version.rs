//! Camoufox version detection and the optional install/update (phase 12).
//!
//! - [`detect`]: asks the venv's python for the `camoufox` package version and the installed
//!   browser build (`camoufox.pkgman.installed_verstr`). Never starts a browser.
//! - [`install`]: the same steps as `manager/camoufox/setup.sh` (uv venv, pip install the pinned
//!   package, `camoufox fetch`), run from Rust with every output line handed to a log callback.
//!   Camoufox stays optional: nothing calls this unless the user presses "Install / Update".

use anyhow::{Context, Result, bail};
use cm_core::EngineVersions;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::mpsc,
};

/// Package version the app is tested with. Keep in sync with `CAMOUFOX_VERSION` in `manager/camoufox/setup.sh`.
pub const CAMOUFOX_VERSION: &str = "0.5.9";

const PROBE: &str = r#"
import json, importlib.metadata as m
out = {}
try:
    out["package"] = m.version("camoufox")
except Exception:
    pass
try:
    from camoufox.pkgman import installed_verstr
    out["browser"] = installed_verstr()
except Exception:
    pass
print(json.dumps(out))
"#;

/// Versions reported by the python at `python`; `None` when it cannot be run or knows neither.
pub(crate) async fn detect(python: &Path) -> Option<EngineVersions> {
    let run = Command::new(python).arg("-c").arg(PROBE).stdin(Stdio::null()).kill_on_drop(true).output();
    let out = tokio::time::timeout(Duration::from_secs(30), run).await.ok()?.ok()?;
    let v: Value = String::from_utf8_lossy(&out.stdout).lines().rev().find_map(|l| serde_json::from_str(l).ok())?;
    let get = |k: &str| v[k].as_str().filter(|s| !s.is_empty()).map(str::to_string);
    let versions = EngineVersions { package: get("package"), browser: get("browser") };
    (versions.package.is_some() || versions.browser.is_some()).then_some(versions)
}

/// One-line summary for `EngineStatus::version`: `0.5.9 · browser 158.0.1-beta.41`.
pub(crate) fn summary(v: &EngineVersions) -> Option<String> {
    match (&v.package, &v.browser) {
        (Some(p), Some(b)) => Some(format!("{p} · browser {b}")),
        (Some(p), None) => Some(p.clone()),
        (None, Some(b)) => Some(format!("browser {b}")),
        (None, None) => None,
    }
}

/// `uv` from PATH or the usual install folders (a GUI app has a short PATH).
fn find_uv() -> Option<PathBuf> {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    dirs.extend([home.join(".local/bin"), home.join(".cargo/bin")]);
    dirs.into_iter().map(|d| d.join("uv")).find(|p| p.is_file())
}

/// Forward every line (split on `\n` and `\r`, so progress bars become lines) to `tx`.
fn pump(mut r: impl AsyncRead + Unpin + Send + 'static, tx: mpsc::UnboundedSender<String>) {
    tokio::spawn(async move {
        let (mut buf, mut cur) = ([0u8; 4096], Vec::new());
        loop {
            match r.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    for &b in &buf[..n] {
                        if b == b'\n' || b == b'\r' {
                            if !cur.is_empty() {
                                let _ = tx.send(String::from_utf8_lossy(&cur).into_owned());
                                cur.clear();
                            }
                        } else {
                            cur.push(b);
                        }
                    }
                }
            }
        }
        if !cur.is_empty() {
            let _ = tx.send(String::from_utf8_lossy(&cur).into_owned());
        }
    });
}

async fn step(program: &Path, args: &[&str], path_env: &std::ffi::OsStr, log: &(dyn Fn(String) + Send + Sync)) -> Result<()> {
    log(format!("$ {} {}", program.display(), args.join(" ")));
    let mut child = Command::new(program)
        .args(args)
        .env("PATH", path_env)
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("cannot run {}", program.display()))?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    pump(child.stdout.take().context("no stdout")?, tx.clone());
    pump(child.stderr.take().context("no stderr")?, tx);
    while let Some(line) = rx.recv().await {
        log(line);
    }
    let status = child.wait().await?;
    if !status.success() {
        bail!("{} failed ({status})", program.file_name().and_then(|n| n.to_str()).unwrap_or("command"));
    }
    Ok(())
}

/// Install or update into the venv that `python` lives in (`<venv>/bin/python`).
pub(crate) async fn install(python: &Path, log: &(dyn Fn(String) + Send + Sync)) -> Result<()> {
    let venv = match (python.parent(), python.file_name().and_then(|n| n.to_str())) {
        (Some(bin), Some("python")) if bin.file_name().and_then(|n| n.to_str()) == Some("bin") => bin.parent().map(Path::to_path_buf),
        _ => None,
    }
    .with_context(|| format!("cannot install: {} is not <venv>/bin/python (CAMOUFOX_PYTHON points elsewhere)", python.display()))?;
    let Some(uv) = find_uv() else { bail!("uv is required: https://docs.astral.sh/uv/ (brew install uv)") };
    let mut path = vec![uv.parent().map(Path::to_path_buf).unwrap_or_default()];
    path.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
    let path_env = std::env::join_paths(path)?;
    let venv_s = venv.to_string_lossy().into_owned();
    let python_s = python.to_string_lossy().into_owned();

    if let Some(parent) = venv.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !python.is_file() {
        log("== create the Python environment".into());
        step(&uv, &["venv", "--python", "3.12", &venv_s], &path_env, log).await?;
    }
    log(format!("== install camoufox {CAMOUFOX_VERSION}"));
    step(&uv, &["pip", "install", "--python", &python_s, &format!("camoufox=={CAMOUFOX_VERSION}")], &path_env, log).await?;
    log("== download the Camoufox browser (large, can take several minutes)".into());
    step(python, &["-m", "camoufox", "fetch"], &path_env, log).await?;
    log("== done".into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_formats() {
        let v = EngineVersions { package: Some("0.5.9".into()), browser: Some("158.0.1-beta.41".into()) };
        assert_eq!(summary(&v).unwrap(), "0.5.9 · browser 158.0.1-beta.41");
        assert_eq!(summary(&EngineVersions::default()), None);
    }

    #[tokio::test]
    async fn detect_without_python_is_none() {
        assert!(detect(Path::new("/nonexistent/python")).await.is_none());
    }

    #[tokio::test]
    async fn install_refuses_foreign_python() {
        let err = install(Path::new("/usr/bin/python3"), &|_| {}).await.unwrap_err();
        assert!(format!("{err:#}").contains("not <venv>/bin/python"));
    }
}
