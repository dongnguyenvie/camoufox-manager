//! Camoufox Manager: local UI to manage Camoufox profiles, give each its own proxy, start/stop.
//!
//! - Listens on 127.0.0.1 only.
//! - Proxy credentials stay in this process; the engine and the UI only see 127.0.0.1:port.
//!
//! This binary only wires implementations together (D3):
//! `cm-store` (SQLite repos) + `cm-engine-camoufox` (Engine) + `cm-relay` (checker)
//! → `cm-service::Manager` → `cm-api::router`.
//!
//! Modes: no argument = desktop window (`desktop`), `serve` = headless HTTP server,
//! other subcommands = CLI (`cli`, phase 12).
//!
//! Environment: `CM_PORT` (UI/API port, default 7878), `CM_PROXY_BASE_PORT` (first relay port,
//! default 31000), `CAMOUFOX_PYTHON`, `CAMOUFOX_HEADLESS=1`, `PROXY_FILE` (legacy import only).
//! Device sync (phase 16): `CM_SYNC_PORT` (sync listener on 127.0.0.1, default 7879),
//! `CM_SYNC_HTTPS_PORT` (`tailscale serve --https` port, default 8443), `CM_SYNC_ADVERTISE`
//! (`host:port` advertised as plain HTTP instead of the tailnet; tests only), `CM_TAILSCALE_BIN`.

mod cli;
mod desktop;
mod diag;
mod paths;

use anyhow::{Context, Result};
use cm_engine_camoufox::Camoufox;
use cm_service::{Config, DEFAULT_PROXY_BASE_PORT, Deps, Manager};
use cm_store::*;
use std::{path::PathBuf, sync::Arc};

pub const DEFAULT_PORT: u16 = 7878;

fn env_port(key: &str) -> Result<Option<u16>> {
    match std::env::var(key) {
        Ok(v) => Ok(Some(v.trim().parse().with_context(|| format!("{key}={v} is not a port"))?)),
        Err(_) => Ok(None),
    }
}

/// The browser engine. Camoufox is optional: without it the app still runs and the UI says
/// "not installed" (see camoufox/setup.sh).
fn build_engine(data_dir: &std::path::Path) -> Arc<Camoufox> {
    let root = data_dir.join("camoufox");
    let python = std::env::var("CAMOUFOX_PYTHON").map(PathBuf::from).unwrap_or_else(|_| root.join(if cfg!(windows) { "venv\Scripts\python.exe" } else { "venv/bin/python" }));
    let headless = matches!(std::env::var("CAMOUFOX_HEADLESS").as_deref(), Ok("1" | "true"));
    Arc::new(Camoufox::new(root, python, headless))
}

/// Open the data dir + database (migrating old layouts), then build the [`Manager`].
pub fn build_manager() -> Result<Arc<Manager>> {
    let data_dir = paths::data_dir();
    // .env in the app folder first, then the current directory.
    let _ = dotenvy::from_path(data_dir.join(".env"));
    let _ = dotenvy::dotenv();

    let db = Db::open(&data_dir)?;
    let proxy_file = std::env::var("PROXY_FILE").ok().map(PathBuf::from);
    match legacy::import(&db, &data_dir, proxy_file.as_deref()) {
        Ok(Some(r)) => {
            println!("Imported old data: {} profiles, {} proxies, {} assignments", r.profiles, r.proxies, r.assignments);
            r.renamed.iter().for_each(|f| println!("  kept original as {}", f.display()));
            r.warnings.iter().for_each(|w| eprintln!("  warning: {w}"));
        }
        Ok(None) => match legacy::import_proxy_file(&db, &data_dir) {
            Ok(Some(r)) => {
                println!("Imported {} new proxies from proxies.txt", r.proxies);
                r.renamed.iter().for_each(|f| println!("  kept original as {}", f.display()));
                r.warnings.iter().for_each(|w| eprintln!("  warning: {w}"));
            }
            Ok(None) => {}
            Err(e) => eprintln!("Import of proxies.txt failed: {e:#}"),
        },
        Err(e) => eprintln!("Import of old data failed (nothing changed): {e:#}"),
    }

    let deps = Deps {
        engine: build_engine(&data_dir),
        profiles: Arc::new(SqliteProfiles::new(db.clone())),
        proxies: Arc::new(SqliteProxies::new(db.clone())),
        checker: Arc::new(cm_relay::IpApiChecker),
        tags: Arc::new(SqliteTags::new(db.clone())),
        groups: Arc::new(SqliteGroups::new(db.clone())),
        views: Arc::new(SqliteSavedViews::new(db.clone())),
        templates: Arc::new(SqliteTemplates::new(db.clone())),
        extensions: Arc::new(SqliteExtensions::new(db.clone())),
        schedules: Arc::new(SqliteSchedules::new(db.clone())),
        settings: Arc::new(SqliteSettings::new(db.clone())),
        api_keys: Arc::new(SqliteApiKeys::new(db.clone())),
        clock: Arc::new(cm_core::SystemClock),
    };
    let cfg = Config {
        proxy_base_port: env_port("CM_PROXY_BASE_PORT")?.unwrap_or(DEFAULT_PROXY_BASE_PORT),
        db_path: db.path().map(PathBuf::from),
        data_dir: data_dir.clone(),
    };
    let engine_root = cfg.data_dir.join("camoufox");
    let manager = Arc::new(Manager::new(deps, cfg));
    // Phase 19: bell notifications + doctor probes.
    manager.attach_notifications(Arc::new(SqliteNotifications::new(db.clone())));
    manager.set_probes(Arc::new(diag::AppProbes { engine_root, db: db.clone() }));
    // Device sync is optional: without it (bad key file, …) the rest of the app runs normally.
    match build_device_hub(&data_dir, &db) {
        Ok(hub) => manager.set_device_hub(hub),
        Err(e) => eprintln!("Device sync disabled: {e:#}"),
    }
    Ok(manager)
}

/// Device identity + registry + Tailscale probe (phase 16). The listener starts in [`cm_api::sync::start`].
fn build_device_hub(data_dir: &std::path::Path, db: &Db) -> Result<Arc<cm_sync::DeviceHub>> {
    let identity = cm_sync::Identity::load_or_create(data_dir)?;
    let cfg = cm_sync::HubConfig {
        sync_port: env_port("CM_SYNC_PORT")?.unwrap_or(cm_sync::DEFAULT_SYNC_PORT),
        https_port: env_port("CM_SYNC_HTTPS_PORT")?.unwrap_or(cm_tailscale::DEFAULT_HTTPS_PORT),
        advertise: std::env::var("CM_SYNC_ADVERTISE").ok().filter(|s| !s.trim().is_empty()),
        device_name: cm_sync::local_device_name(),
    };
    let deps = cm_sync::HubDeps {
        identity: Arc::new(identity),
        repo: Arc::new(SqliteDevices::new(db.clone())),
        tailscale: Arc::new(cm_tailscale::Cli::detect()),
        settings: Arc::new(SqliteSettings::new(db.clone())),
        clock: Arc::new(cm_core::SystemClock),
    };
    Ok(Arc::new(cm_sync::DeviceHub::new(cfg, deps)))
}

/// Headless mode: `camoufox-manager serve` → http://127.0.0.1:$CM_PORT, Ctrl+C / SIGTERM to quit.
async fn serve_headless() -> Result<()> {
    let app = build_manager()?;
    if let Err(e) = app.recover_journals().await {
        eprintln!("Snapshot journal recovery failed: {e:#}");
    }
    app.clone().spawn_scheduler(); // phase 10: scheduled runs
    cm_api::sync::start(app.clone()).await; // phase 16: device sync listener
    let port = env_port("CM_PORT")?.unwrap_or(DEFAULT_PORT);
    let listener =
        tokio::net::TcpListener::bind(("127.0.0.1", port)).await.with_context(|| format!("cannot listen on 127.0.0.1:{port}"))?;
    println!("Camoufox Manager: http://127.0.0.1:{port}  (data: {})", app.config().data_dir.display());
    let app2 = app.clone();
    axum::serve(listener, cm_api::router(app))
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = async { match term.as_mut() { Some(t) => { t.recv().await; } None => std::future::pending().await } } => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            app2.stop_all().await;
        })
        .await?;
    Ok(())
}

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => desktop::run(),
        Some("serve") => tokio::runtime::Runtime::new()?.block_on(serve_headless()),
        Some(_) => cli::run(std::env::args().skip(1).collect()),
    }
}
