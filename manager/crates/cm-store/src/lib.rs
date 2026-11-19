//! SQLite storage: one file `<data_dir>/camoufox-manager.db` (D2).
//!
//! - [`Db`]: shared connection + migration runner ([`migrate`]).
//! - One module per area, each implementing its cm-core repo trait on a small struct that
//!   holds a [`Db`]: `profiles`, `proxies`, `settings` (phase 1), `org` (7), `templates` (9),
//!   `schedules` (10). Stubs return `CmError::NotImplemented` until their phase lands.
//! - [`legacy`]: one-time import of the pre-SQLite files (`state.json`, `proxies.txt`,
//!   `camoufox/profiles/*/profile.json`).
//!
//! rusqlite is synchronous; queries are tiny and local, so repos run them inline under a mutex.

pub mod legacy;
pub mod migrate;
pub mod org;
pub mod profiles;
pub mod proxies;
pub mod schedules;
pub mod settings;
pub mod templates;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub use org::{SqliteGroups, SqliteSavedViews, SqliteTags};
pub use profiles::SqliteProfiles;
pub use proxies::SqliteProxies;
pub use schedules::SqliteSchedules;
pub use settings::{SqliteApiKeys, SqliteSettings};
pub use templates::{SqliteExtensions, SqliteTemplates};

/// File name of the database inside the data dir.
pub const DB_FILE: &str = "camoufox-manager.db";

/// Shared SQLite connection.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl Db {
    /// Open (or create) `<data_dir>/camoufox-manager.db` and run pending migrations.
    pub fn open(data_dir: &Path) -> Result<Db> {
        std::fs::create_dir_all(data_dir).with_context(|| format!("cannot create {}", data_dir.display()))?;
        let path = data_dir.join(DB_FILE);
        let conn = Connection::open(&path).with_context(|| format!("cannot open {}", path.display()))?;
        Db::init(conn, Some(path))
    }

    /// In-memory database (tests).
    pub fn memory() -> Result<Db> {
        Db::init(Connection::open_in_memory()?, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Db> {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")?;
        let db = Db { conn: Arc::new(Mutex::new(conn)), path };
        db.with(migrate::run)?;
        Ok(db)
    }

    /// Path of the database file (`None` for in-memory).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Run `f` with the connection locked.
    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut c = self.conn.lock().map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        f(&mut c)
    }
}
