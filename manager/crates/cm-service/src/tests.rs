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
pub(crate) struct FakeEngine {
    pub(crate) launched: Mutex<Vec<LaunchOptions>>,
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

pub(crate) struct FakeChecker {
    ok: bool,
}

impl FakeChecker {
    pub(crate) fn new(ok: bool) -> Self {
        Self { ok }
    }
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

pub(crate) fn manager(ok: bool, base_port: u16) -> (Manager, Arc<FakeEngine>) {
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
        settings: Arc::new(SqliteSettings::new(db.clone())),
        api_keys: Arc::new(SqliteApiKeys::new(db)),
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
async fn delete_goes_to_trash_and_restores() {
    let (m, _) = manager(true, free_port().await);
    let p = m.create_profile(NewProfile { name: "a".into(), os: "lin".into() }).await.unwrap();
    assert_eq!(m.trash_days().await, 7);
    m.delete_profile(&p.id).await.unwrap();
    assert!(m.overview().await.profiles.is_empty());
    assert!(m.start(&p.id).await.is_err(), "a trashed profile cannot start");
    let t = m.trashed_profiles().await.unwrap();
    assert_eq!((t.days, t.profiles.len(), t.profiles[0].id.as_str()), (7, 1, p.id.as_str()));
    m.restore_profile(&p.id).await.unwrap();
    assert_eq!(m.overview().await.profiles.len(), 1);
    assert!(m.restore_profile(&p.id).await.is_err());
    // Delete again, then purge for good.
    m.delete_profile(&p.id).await.unwrap();
    m.purge_profile(&p.id).await.unwrap();
    assert!(m.trashed_profiles().await.unwrap().profiles.is_empty());
    assert!(m.purge_profile(&p.id).await.is_err());
}

#[tokio::test]
async fn zero_retention_deletes_at_once() {
    let (m, _) = manager(true, free_port().await);
    m.update_settings(SettingsPatch { trash_days: Some(0), ..Default::default() }).await.unwrap();
    let p = m.create_profile(NewProfile { name: "a".into(), os: "lin".into() }).await.unwrap();
    m.delete_profile(&p.id).await.unwrap();
    assert!(m.trashed_profiles().await.unwrap().profiles.is_empty());
    assert!(m.update_settings(SettingsPatch { trash_days: Some(9999), ..Default::default() }).await.is_err());
}

#[tokio::test]
async fn expired_trash_is_purged_and_empty_trash_clears_the_rest() {
    let (m, _) = manager(true, free_port().await);
    let old = m.create_profile(NewProfile { name: "old".into(), os: "lin".into() }).await.unwrap();
    let new = m.create_profile(NewProfile { name: "new".into(), os: "lin".into() }).await.unwrap();
    m.delete_profile(&new.id).await.unwrap();
    // Trashed 30 days ago (retention is 7).
    let ago = cm_core::clock::iso(cm_core::Clock::now(&SystemClock) - 30 * 86_400);
    m.d.profiles.trash(&old.id, &ago).await.unwrap();
    assert_eq!(m.purge_expired_trash().await.unwrap(), 1);
    let t = m.trashed_profiles().await.unwrap();
    assert_eq!((t.profiles.len(), t.profiles[0].id.as_str()), (1, new.id.as_str()));
    assert_eq!(m.empty_trash().await.unwrap(), 1);
    assert!(m.trashed_profiles().await.unwrap().profiles.is_empty());
}

#[tokio::test]
async fn language_setting() {
    let (m, _) = manager(true, free_port().await);
    assert_eq!(m.language().await, "en");
    m.update_settings(SettingsPatch { language: Some("vi".into()), ..Default::default() }).await.unwrap();
    assert_eq!(m.settings().await.language, "vi");
    assert!(m.update_settings(SettingsPatch { language: Some("ru".into()), ..Default::default() }).await.is_err());
}

#[tokio::test]
async fn proxy_import_is_all_or_nothing_and_skips_duplicates() {
    let (m, _) = manager(true, free_port().await);
    let r = m.import_proxies("a.example:1\nbroken line\nb.example:2").await.unwrap();
    assert_eq!(r.rejected.len(), 1);
    assert!(m.proxies().await.unwrap().is_empty(), "nothing saved");

    let r = m.import_proxies("a.example:1\nsocks5://u:p@b.example:2\nA.example:1").await.unwrap();
    assert_eq!((r.created.len(), r.skipped.clone()), (2, vec![3]));
    let r = m.import_proxies("a.example:1").await.unwrap();
    assert_eq!((r.created.len(), r.skipped.len()), (0, 1));
    let json = serde_json::to_string(&m.proxies().await.unwrap()).unwrap();
    assert!(!json.contains("\"p\"") && json.contains("\"hasPassword\":true"));
}

#[tokio::test]
async fn proxy_update_keeps_or_clears_password() {
    let (m, _) = manager(true, free_port().await);
    let v = m
        .create_proxy(NewProxy {
            host: "h.example".into(),
            port: 80,
            username: Some("u".into()),
            password: Some("pw".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(v.name, "h.example:80");
    let edit = || NewProxy { name: "x".into(), host: "h.example".into(), port: 81, username: Some("u".into()), ..Default::default() };
    assert!(m.update_proxy(v.id, edit(), false).await.unwrap().has_password);
    assert!(!m.update_proxy(v.id, edit(), true).await.unwrap().has_password);
    assert!(m.create_proxy(NewProxy { host: "bad host".into(), port: 1, ..Default::default() }).await.is_err());
    m.delete_proxy(v.id).await.unwrap();
    assert!(m.delete_proxy(v.id).await.is_err());
}

#[tokio::test]
async fn bulk_and_sequential_assignment() {
    let (m, _) = manager(true, free_port().await);
    let mut ids = vec![];
    for n in ["a", "b", "c"] {
        ids.push(m.create_profile(NewProfile { name: n.into(), os: "win".into() }).await.unwrap().id);
    }
    m.import_proxies("p1.example:1\np2.example:1\np3.example:1\np4.example:1").await.unwrap();

    let r = m.assign_many(Some(1), &[ids[0].clone(), "zz".into()]).await.unwrap();
    assert_eq!((r[0].ok, r[1].reason), (true, Some("not-found")));

    // profile c already uses proxy 2; it is outside the selection, so 2 is skipped for the others.
    m.assign(&ids[2], Some(2)).await.unwrap();
    let req = |dry| crate::proxies::SequenceRequest { profile_ids: ids[..2].to_vec(), proxy_ids: None, dry_run: dry };
    let dry = m.assign_sequence(req(true)).await.unwrap();
    assert_eq!(dry.pairs.iter().map(|p| p.proxy_id).collect::<Vec<_>>(), vec![Some(1), Some(3)]);
    assert_eq!(dry.skipped_proxies.len(), 1);
    assert!(dry.pairs.iter().all(|p| !p.applied));
    let done = m.assign_sequence(req(false)).await.unwrap();
    assert!(done.pairs.iter().all(|p| p.applied));
    let ov = m.overview().await;
    assert_eq!(ov.profiles[1].assigned.as_ref().unwrap().id, 3);

    // not enough proxies
    let many = crate::proxies::SequenceRequest { profile_ids: ids.clone(), proxy_ids: Some(vec![4]), dry_run: true };
    let r = m.assign_sequence(many).await.unwrap();
    assert_eq!(r.pairs[1].reason, Some("no-proxy"));
}

/// Loopback HTTP server answering every request with `status`; counts the requests.
async fn http_stub(status: &'static str) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/rotate?token=SECRET-TOKEN", l.local_addr().unwrap());
    let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let h = hits.clone();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            h.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf).await;
            let _ = s.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await;
        }
    });
    (url, hits)
}

#[tokio::test]
async fn rotate_ip_calls_link_then_rechecks() {
    let (m, _) = manager(true, free_port().await);
    let (url, hits) = http_stub("200 OK").await;
    let v =
        m.create_proxy(NewProxy { host: "h.example".into(), port: 80, rotate_url: Some(url.clone()), ..Default::default() }).await.unwrap();
    assert_eq!(v.rotate_url.as_deref(), Some(url.as_str()));
    let r = m.rotate_ip(v.id).await.unwrap();
    assert!(r.ok && r.error.is_none());
    assert_eq!(r.geo.unwrap().ip, "9.9.9.9");
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(m.proxy(v.id).await.unwrap().last_check.is_some());
    // no link -> invalid
    let plain = m.create_proxy(NewProxy { host: "g.example".into(), port: 80, ..Default::default() }).await.unwrap();
    assert!(m.rotate_ip(plain.id).await.is_err());
}

#[tokio::test]
async fn rotate_ip_failure_is_a_result_without_the_url() {
    let (m, _) = manager(true, free_port().await);
    let (url, _) = http_stub("500 Internal Server Error").await;
    let v = m.create_proxy(NewProxy { host: "h.example".into(), port: 80, rotate_url: Some(url), ..Default::default() }).await.unwrap();
    let r = m.rotate_ip(v.id).await.unwrap();
    assert!(!r.ok && r.geo.is_none());
    let err = r.error.unwrap();
    assert!(err.contains("500") && !err.contains("SECRET-TOKEN"), "{err}");
    // unreachable link: the connection error has no URL either
    let dead = format!("http://127.0.0.1:{}/r?token=SECRET-TOKEN", free_port().await);
    let d = m.create_proxy(NewProxy { host: "d.example".into(), port: 80, rotate_url: Some(dead), ..Default::default() }).await.unwrap();
    let r = m.rotate_ip(d.id).await.unwrap();
    assert!(!r.ok && !r.error.unwrap().contains("SECRET-TOKEN"));
}

#[tokio::test]
async fn rotate_on_start_runs_and_never_blocks_launch() {
    let (m, _) = manager(true, free_port().await);
    let (ok_url, hits) = http_stub("200 OK").await;
    let (bad_url, _) = http_stub("503 Service Unavailable").await;
    for (url, expect_hits) in [(ok_url, 1), (bad_url, 1)] {
        let px = m
            .create_proxy(NewProxy {
                host: "127.0.0.1".into(),
                port: 9,
                rotate_url: Some(url),
                rotate_on_start: true,
                ..Default::default()
            })
            .await
            .unwrap();
        let prof = m.create_profile(NewProfile { name: "p".into(), os: "win".into() }).await.unwrap().id;
        m.assign(&prof, Some(px.id)).await.unwrap();
        m.start(&prof).await.unwrap();
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), expect_hits);
        m.stop(&prof).await.unwrap();
    }
}
