//! Use-case tests with a fake engine/checker over the in-memory SQLite store.

use crate::{Config, Deps, Manager, settings::SettingsPatch};
use anyhow::{Result, bail};
use async_trait::async_trait;
use cm_core::{
    Engine, EngineStatus, LaunchOptions, MachineSummary, NewProfile, NewProxy, Os, Profile, Proxy, ProxyCheck, ProxyChecker, Session,
    SystemClock,
};
use cm_store::*;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct FakeEngine {
    launched: Mutex<Vec<LaunchOptions>>,
}

#[async_trait]
impl Engine for FakeEngine {
    fn name(&self) -> &'static str {
        "Fake"
    }
    fn ws_protocol(&self) -> &'static str {
        "Playwright"
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus { ready: true, hint: None, version: None }
    }
    async fn generate(&self, os: Os) -> Result<Value> {
        Ok(json!({ "os": os.code() }))
    }
    fn summarize(&self, _machine: &Value) -> MachineSummary {
        MachineSummary::default()
    }
    async fn launch(&self, _p: &Profile, opts: &LaunchOptions) -> Result<Session> {
        self.launched.lock().unwrap().push(opts.clone());
        Ok(Session { ws_url: "ws://127.0.0.1:1/x".into() })
    }
    async fn stop(&self, _id: &str) -> Result<()> {
        Ok(())
    }
    async fn alive(&self, _id: &str) -> Option<bool> {
        Some(true)
    }
    fn data_dir(&self, id: &str) -> PathBuf {
        std::env::temp_dir().join("cm-fake").join(id)
    }
    async fn delete_data(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}

struct FakeChecker {
    ok: bool,
}

#[async_trait]
impl ProxyChecker for FakeChecker {
    async fn check(&self, _p: &Proxy) -> Result<ProxyCheck> {
        if !self.ok {
            bail!("dead proxy");
        }
        Ok(ProxyCheck { ip: "9.9.9.9".into(), timezone: "America/Sao_Paulo".into(), lat: -23.5, lon: -46.6, ..Default::default() })
    }
}

fn manager(ok: bool, base_port: u16) -> (Manager, Arc<FakeEngine>) {
    let db = Db::memory().unwrap();
    let engine = Arc::new(FakeEngine::default());
    let deps = Deps {
        engine: engine.clone(),
        profiles: Arc::new(SqliteProfiles::new(db.clone())),
        proxies: Arc::new(SqliteProxies::new(db.clone())),
        checker: Arc::new(FakeChecker { ok }),
        tags: Arc::new(SqliteTags::new(db.clone())),
        groups: Arc::new(SqliteGroups::new(db.clone())),
        views: Arc::new(SqliteSavedViews::new(db.clone())),
        templates: Arc::new(SqliteTemplates::new(db.clone())),
        extensions: Arc::new(SqliteExtensions::new(db.clone())),
        schedules: Arc::new(SqliteSchedules::new(db.clone())),
        settings: Arc::new(SqliteSettings::new(db)),
        clock: Arc::new(SystemClock),
    };
    (Manager::new(deps, Config { proxy_base_port: base_port, data_dir: PathBuf::from("/tmp"), db_path: None }), engine)
}

async fn free_port() -> u16 {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port()
}

#[tokio::test]
async fn start_with_proxy_applies_geo_and_timezone() {
    let (m, engine) = manager(true, free_port().await);
    let p = m.create_profile(NewProfile { name: " Shop ".into(), os: "mac".into() }).await.unwrap();
    assert_eq!(p.name, "Shop");
    let px =
        m.deps().proxies.insert(&NewProxy { name: "a".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() }).await.unwrap();
    m.assign(&p.id, Some(px.id)).await.unwrap();

    let r = m.start(&p.id).await.unwrap();
    assert_eq!(r.port, Some(m.config().proxy_base_port));
    let opts = engine.launched.lock().unwrap()[0].clone();
    assert_eq!(opts.timezone.as_deref(), Some("America/Sao_Paulo"));
    assert_eq!(opts.geolocation, Some((-23.5, -46.6)));
    assert_eq!(opts.public_ip.as_deref(), Some("9.9.9.9"));
    assert!(m.assign(&p.id, None).await.is_err(), "cannot change proxy while running");

    let ov = m.overview().await;
    assert_eq!(ov.profiles[0].assigned.as_ref().unwrap().geo.as_ref().unwrap().ip, "9.9.9.9");
    assert!(ov.profiles[0].running.is_some());
    m.stop(&p.id).await.unwrap();
    assert!(m.relays.lock().await.is_empty(), "relay released");
}

#[tokio::test]
async fn dead_proxy_fails_closed() {
    let (m, engine) = manager(false, free_port().await);
    let p = m.create_profile(NewProfile { name: "x".into(), os: "win".into() }).await.unwrap();
    let px =
        m.deps().proxies.insert(&NewProxy { name: "a".into(), host: "127.0.0.1".into(), port: 9, ..Default::default() }).await.unwrap();
    m.assign(&p.id, Some(px.id)).await.unwrap();
    assert!(m.start(&p.id).await.is_err());
    assert!(engine.launched.lock().unwrap().is_empty(), "browser never launched");
    assert!(m.relays.lock().await.is_empty());
}

#[tokio::test]
async fn create_validates_and_delete_works() {
    let (m, _) = manager(true, free_port().await);
    assert!(m.create_profile(NewProfile { name: "  ".into(), os: "win".into() }).await.is_err());
    assert!(m.create_profile(NewProfile { name: "a".into(), os: "android".into() }).await.is_err());
    let p = m.create_profile(NewProfile { name: "a".into(), os: "lin".into() }).await.unwrap();
    m.delete_profile(&p.id).await.unwrap();
    assert!(m.overview().await.profiles.is_empty());
    assert!(m.delete_profile(&p.id).await.is_err());
}

#[tokio::test]
async fn language_setting() {
    let (m, _) = manager(true, free_port().await);
    assert_eq!(m.language().await, "en");
    m.update_settings(SettingsPatch { language: Some("vi".into()) }).await.unwrap();
    assert_eq!(m.settings().await.language, "vi");
    assert!(m.update_settings(SettingsPatch { language: Some("ru".into()) }).await.is_err());
}
