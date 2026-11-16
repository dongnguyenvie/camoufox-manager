//! Where the app keeps its data: `~/Library/Application Support/camoufox-manager/`
//! (database, Camoufox venv, helper, profile user-data).

use std::path::PathBuf;

fn app_support() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/")).join("Library/Application Support")
}

pub fn data_dir() -> PathBuf {
    app_support().join("camoufox-manager")
}
