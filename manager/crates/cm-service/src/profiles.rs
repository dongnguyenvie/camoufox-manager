//! Profiles: overview (list for the UI), create/delete, start/stop, running state.
//! Phase 4 extends this module (edit settings, pinned machine, OS change).

use crate::{Manager, Running, proxies::ProxyView};
use anyhow::{Context, Result, bail};
use cm_core::{CmError, EngineHint, LaunchOptions, MachineSummary, NewProfile, Os, Profile, ProfileSettings, ProxyCheck, ProxyId};
use serde::Serialize;
use std::{collections::HashMap, fs, io::Read, path::Path, path::PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningView {
    pub ws: String,
    pub port: Option<u16>,
    pub started_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfo {
    pub dir: String,
    pub exists: bool,
    pub size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub os: Os,
    pub notes: Option<String>,
    pub created_at: String,
    /// Last start, or creation time if never started.
    pub last_activity: Option<String>,
    /// Timezone override; `None` + `timezone_by_ip` = follows the proxy.
    pub timezone: Option<String>,
    pub timezone_by_ip: bool,
    pub settings: ProfileSettings,
    /// Pinned machine (read-only).
    pub fp: MachineSummary,
    pub assigned: Option<ProxyView>,
    pub running: Option<RunningView>,
    pub path: PathInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub provider: &'static str,
    /// Engine can launch; false + `provider_hint` when not installed (Camoufox is optional).
    pub provider_available: bool,
    pub provider_hint: Option<EngineHint>,
    /// Protocol of the ws URL returned by start (`Playwright`).
    pub ws_protocol: &'static str,
    pub proxy_count: usize,
    pub api_error: Option<String>,
    pub profiles: Vec<ProfileView>,
}

#[derive(Serialize)]
pub struct StartResult {
    pub ws: String,
    pub port: Option<u16>,
    pub geo: Option<ProxyCheck>,
    pub already: bool,
}

fn dir_size(p: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(p) else { return 0 };
    rd.flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// 24 random hex chars (from /dev/urandom, no extra crate).
pub(crate) fn new_id() -> Result<String> {
    let mut b = [0u8; 12];
    fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).context("cannot read /dev/urandom")?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

impl Manager {
    fn path_info(&self, id: &str) -> PathInfo {
        let d = self.d.engine.data_dir(id);
        let exists = d.is_dir();
        PathInfo { dir: d.display().to_string(), exists, size_bytes: if exists { dir_size(&d) } else { 0 } }
    }

    pub(crate) async fn profile(&self, id: &str) -> Result<Profile> {
        self.d.profiles.get(id).await?.ok_or_else(|| CmError::NotFound(format!("profile {id}")).into())
    }

    /// Forget browsers the engine reports as gone (user closed the window, crash).
    async fn prune_dead(&self) {
        let ids: Vec<String> = self.running.lock().await.keys().cloned().collect();
        for id in ids {
            if self.d.engine.alive(&id).await == Some(false) {
                let proxy = self.running.lock().await.remove(&id).and_then(|r| r.proxy);
                if let Some(px) = proxy {
                    self.release_relay(px).await;
                }
                println!("{id} exited on its own");
            }
        }
    }

    pub async fn overview(&self) -> Overview {
        self.prune_dead().await;
        let status = self.d.engine.status().await;
        let (profiles, proxies, api_error) = match (self.d.profiles.list().await, self.d.proxies.list().await) {
            (Ok(p), Ok(x)) => (p, x, None),
            (p, x) => {
                let e = p.as_ref().err().or(x.as_ref().err()).map(|e| format!("{e:#}"));
                (p.unwrap_or_default(), x.unwrap_or_default(), e)
            }
        };
        let by_id: HashMap<ProxyId, ProxyView> = proxies.iter().map(|p| (p.id, self.proxy_view(p))).collect();
        let running = self.running.lock().await;
        let views = profiles
            .into_iter()
            .map(|p| ProfileView {
                assigned: p.proxy_id.and_then(|i| by_id.get(&i).cloned()),
                running: running.get(&p.id).map(|r| RunningView { ws: r.ws.clone(), port: r.port, started_at: r.started_at }),
                path: self.path_info(&p.id),
                fp: self.d.engine.summarize(&p.machine),
                timezone: p.settings.timezone.clone(),
                timezone_by_ip: p.settings.timezone.is_none(),
                last_activity: p.last_activity.clone().or_else(|| Some(p.created_at.clone())),
                id: p.id,
                name: p.name,
                os: p.os,
                notes: p.notes,
                created_at: p.created_at,
                settings: p.settings,
            })
            .collect();
        Overview {
            provider: self.d.engine.name(),
            provider_available: status.ready,
            provider_hint: status.hint,
            ws_protocol: self.d.engine.ws_protocol(),
            proxy_count: proxies.len(),
            api_error,
            profiles: views,
        }
    }

    pub async fn start(&self, id: &str) -> Result<StartResult> {
        self.prune_dead().await;
        if let Some(r) = self.running.lock().await.get(id) {
            return Ok(StartResult { ws: r.ws.clone(), port: r.port, geo: None, already: true });
        }
        let profile = self.profile(id).await?;
        // Fail fast (before the slow proxy check) when the engine cannot launch anyway.
        let status = self.d.engine.status().await;
        if !status.ready {
            let why = status.hint.map(|h| h.detail).unwrap_or_default();
            bail!("{} is not ready: {why}", self.d.engine.name());
        }
        let mut opts = LaunchOptions::default();
        let (mut port, mut geo) = (None, None);
        if let Some(pid) = profile.proxy_id {
            // Relay up and geo known BEFORE the browser opens: dead proxy → no browser (fail-closed).
            let proxy = self.proxy(pid).await?;
            let p = self.ensure_relay(&proxy).await?;
            let g = match self.check_proxy(pid, false).await {
                Ok(g) => g,
                Err(e) => {
                    self.release_relay(pid).await;
                    return Err(e.context("proxy check failed, browser not opened"));
                }
            };
            // D4: timezone always set together with geolocation.
            opts.proxy = Some(("127.0.0.1".into(), p));
            opts.timezone = Some(profile.settings.timezone.clone().unwrap_or_else(|| g.timezone.clone()));
            opts.geolocation = Some((g.lat, g.lon));
            opts.public_ip = Some(g.ip.clone());
            port = Some(p);
            geo = Some(g);
        } else if let Some(tz) = &profile.settings.timezone {
            opts.timezone = Some(tz.clone());
        }
        let session = match self.d.engine.launch(&profile, &opts).await {
            Ok(s) => s,
            Err(e) => {
                if let Some(pid) = profile.proxy_id {
                    self.release_relay(pid).await;
                }
                return Err(e);
            }
        };
        let now = self.d.clock.now();
        self.running
            .lock()
            .await
            .insert(id.to_string(), Running { ws: session.ws_url.clone(), proxy: profile.proxy_id, port, started_at: now });
        let _ = self.d.profiles.touch(id, &cm_core::clock::iso(now)).await;
        Ok(StartResult { ws: session.ws_url, port, geo, already: false })
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        self.d.engine.stop(id).await?;
        let proxy = self.running.lock().await.remove(id).and_then(|r| r.proxy);
        if let Some(px) = proxy {
            self.release_relay(px).await;
        }
        Ok(())
    }

    /// Stop every running profile so the engine closes browsers cleanly (app exit).
    pub async fn stop_all(&self) {
        let ids: Vec<String> = self.running.lock().await.keys().cloned().collect();
        for id in ids {
            let _ = self.stop(&id).await;
            println!("stopped {id}");
        }
    }

    pub async fn create_profile(&self, req: NewProfile) -> Result<ProfileView> {
        let name = req.name.trim();
        if name.is_empty() {
            bail!(CmError::Invalid("profile name must not be empty".into()));
        }
        let Some(os) = Os::parse(&req.os) else { bail!(CmError::Invalid("os must be win, mac or lin".into())) };
        let machine = self.d.engine.generate(os).await?;
        let now = self.d.clock.now_iso();
        let p = Profile {
            id: new_id()?,
            name: name.to_string(),
            os,
            notes: None,
            created_at: now.clone(),
            updated_at: now,
            last_activity: None,
            proxy_id: None,
            machine,
            settings: ProfileSettings::default(),
        };
        self.d.profiles.insert(&p).await?;
        let id = p.id.clone();
        self.overview().await.profiles.into_iter().find(|v| v.id == id).context("created profile vanished")
    }

    /// Delete: stop if running, remove the record, then the data on disk.
    pub async fn delete_profile(&self, id: &str) -> Result<()> {
        self.profile(id).await?;
        if self.running.lock().await.contains_key(id) {
            self.stop(id).await?;
        }
        self.d.profiles.delete(id).await?;
        self.d.engine.delete_data(id).await
    }

    /// Data directory of the profile; error if it does not exist yet.
    pub async fn profile_dir(&self, id: &str) -> Result<PathBuf> {
        self.profile(id).await?;
        let dir = self.d.engine.data_dir(id);
        if !dir.is_dir() {
            bail!(CmError::NotFound("profile data on disk (never started)".into()));
        }
        Ok(dir)
    }
}
