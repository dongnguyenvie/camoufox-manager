//! Use-cases of the app, independent of the browser engine, storage and HTTP.
//!
//! [`Manager`] holds the trait objects ([`Deps`]) and the in-memory runtime state (running
//! browsers, active relays). Each area adds an `impl Manager` block in its own module:
//!
//! | module | area | phase |
//! |---|---|---|
//! | `profiles` | overview, create/delete, start/stop | 1, 4 |
//! | `proxies` | library, check, assign, relays | 1, 2 |
//! | `cookies` | import/export | 5 |
//! | `transfer` | zip export/import, clone, bulk create | 6 |
//! | `org` | tags, groups, saved views, bulk actions | 7 |
//! | `templates` | templates + extensions | 9 |
//! | `scheduler` | scheduled launches | 10 |
//! | `sheets` | xlsx/csv import/export | 11 |
//! | `settings` | language, engine, API key | 1, 12 |
//! | `lease` | agent lease on a profile | 8 |
//! | `journal` | crash-safe replace of a profile's data | 15 |
//! | `doctor` | local diagnostics (read-only probes) | 19 |
//! | `notify` | bell notifications + optional macOS banner | 19 |

pub mod cookies;
pub mod doctor;
pub mod journal;
pub mod lease;
pub mod notify;
pub mod org;
pub mod profiles;
pub mod proxies;
pub mod scheduler;
pub mod settings;
pub mod sheets;
pub mod templates;
pub mod transfer;

use cm_core::{
    ApiKeyRepo, Clock, Engine, ExtensionRepo, GroupRepo, ProfileRepo, ProxyChecker, ProxyId, ProxyRepo, SavedViewRepo, ScheduleRepo,
    SettingsRepo, TagRepo, TemplateRepo,
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

/// Default first port of the local relays (D7: `CM_PROXY_BASE_PORT`).
pub const DEFAULT_PROXY_BASE_PORT: u16 = 31000;

/// Implementations wired by the app binary.
pub struct Deps {
    pub engine: Arc<dyn Engine>,
    pub profiles: Arc<dyn ProfileRepo>,
    pub proxies: Arc<dyn ProxyRepo>,
    pub checker: Arc<dyn ProxyChecker>,
    pub tags: Arc<dyn TagRepo>,
    pub groups: Arc<dyn GroupRepo>,
    pub views: Arc<dyn SavedViewRepo>,
    pub templates: Arc<dyn TemplateRepo>,
    pub extensions: Arc<dyn ExtensionRepo>,
    pub schedules: Arc<dyn ScheduleRepo>,
    pub settings: Arc<dyn SettingsRepo>,
    pub api_keys: Arc<dyn ApiKeyRepo>,
    pub clock: Arc<dyn Clock>,
}

#[derive(Clone, Debug)]
pub struct Config {
    /// Relay of proxy `id` listens on `proxy_base_port + id - 1`.
    pub proxy_base_port: u16,
    /// Shown on the settings page.
    pub data_dir: PathBuf,
    pub db_path: Option<PathBuf>,
}

/// A browser this app started.
struct Running {
    ws: String,
    proxy: Option<ProxyId>,
    port: Option<u16>,
    started_at: u64,
}

pub struct Manager {
    d: Deps,
    cfg: Config,
    running: Mutex<HashMap<String, Running>>,
    /// Agent leases by profile id (phase 8).
    leases: std::sync::Mutex<HashMap<String, lease::Lease>>,
    /// Scheduler runtime state (phase 10).
    sched: scheduler::SchedState,
    /// Active relays by proxy id.
    relays: Mutex<HashMap<ProxyId, (u16, JoinHandle<()>)>>,
    /// Set once the snapshot apply journal has been recovered (phase 15).
    journal_ready: tokio::sync::OnceCell<usize>,
    /// Read-only diagnostics probes (phase 19, attached by the binary).
    probes: std::sync::OnceLock<Arc<dyn doctor::Probes>>,
    /// Notification store (phase 19, attached by the binary).
    notifier: std::sync::OnceLock<notify::Notifier>,
}

impl Manager {
    pub fn new(deps: Deps, cfg: Config) -> Self {
        Self {
            d: deps,
            cfg,
            running: Mutex::new(HashMap::new()),
            leases: Default::default(),
            sched: Default::default(),
            relays: Mutex::new(HashMap::new()),
            journal_ready: Default::default(),
            probes: Default::default(),
            notifier: Default::default(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn deps(&self) -> &Deps {
        &self.d
    }
}

#[cfg(test)]
mod tests;
