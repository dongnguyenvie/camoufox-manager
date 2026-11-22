//! Where the app keeps its data: `~/Library/Application Support/camoufox-manager/`
//! (database, Camoufox venv, helper, profile user-data).

use std::path::PathBuf;

#[cfg(windows)]
fn app_support() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("USERPROFILE").map(|h| format!(r"{h}\AppData\Roaming")))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\"))
}

#[cfg(not(windows))]
fn app_support() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/")).join("Library/Application Support")
}

pub fn data_dir() -> PathBuf {
    app_support().join("camoufox-manager")
}
