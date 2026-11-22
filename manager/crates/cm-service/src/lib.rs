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
//! | `sync` | device sync engine: manifest, diff, chunked transfer, apply | 17 |
//! | `doctor` | local diagnostics (read-only probes) | 19 |
//! | `notify` | bell notifications + optional macOS banner | 19 |
//!
//! Device sync (phase 16): the app binary builds a [`cm_sync::DeviceHub`] and hands it over with
//! [`Manager::set_device_hub`]; routes and the sync engine reach it through [`Manager::device_hub`]
//! (`None` when it could not be created, e.g. the key file is unreadable: the rest of the app works).

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
pub mod sync;
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

/// (proxy id, filters local network)
type RelayKey = (ProxyId, bool);

/// A browser this app started.
struct Running {
    ws: String,
    proxy: Option<ProxyId>,
    port: Option<u16>,
    started_at: u64,
    /// Whether its relay filters local-network targets (profile setting, default on).
    block_local: bool,
}

pub struct Manager {
    d: Deps,
    cfg: Config,
    running: Mutex<HashMap<String, Running>>,
    /// Agent leases by profile id (phase 8).
    leases: std::sync::Mutex<HashMap<String, lease::Lease>>,
    /// Scheduler runtime state (phase 10).
    sched: scheduler::SchedState,
    /// Active relays by (proxy id, filters local network). Blocking relays take the proxy's
    /// fixed port; the non-blocking variant (profile opted out) gets an ephemeral one.
    relays: Mutex<HashMap<RelayKey, (u16, JoinHandle<()>)>>,
    /// Set once the snapshot apply journal has been recovered (phase 15).
    journal_ready: tokio::sync::OnceCell<usize>,
    /// Read-only diagnostics probes (phase 19, attached by the binary).
    probes: std::sync::OnceLock<Arc<dyn doctor::Probes>>,
    /// Notification store (phase 19, attached by the binary).
    notifier: std::sync::OnceLock<notify::Notifier>,
    /// Device sync (phase 16), set once by the app binary.
    device_hub: std::sync::OnceLock<Arc<cm_sync::DeviceHub>>,
    /// Device sync engine state: jobs and the content-hash cache (phase 17).
    sync: sync::SyncState,
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
            device_hub: Default::default(),
            sync: Default::default(),
        }
    }

    /// Install the device hub (phase 16). Only the first call wins.
    pub fn set_device_hub(&self, hub: Arc<cm_sync::DeviceHub>) {
        let _ = self.device_hub.set(hub);
    }

    /// The device hub, when device sync is available in this process.
    pub fn device_hub(&self) -> Option<&Arc<cm_sync::DeviceHub>> {
        self.device_hub.get()
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
