//! Proxies: library listing, on-demand check, assignment to profiles, local relays.
//! Phase 2 extends this module (CRUD, import, bulk/sequential assignment).

use crate::Manager;
use anyhow::{Result, bail};
use cm_core::{CmError, Proxy, ProxyCheck, ProxyId};
use serde::Serialize;

/// A cached check younger than this is reused at start; older → check again (rotating IPs).
const CHECK_TTL_SECS: u64 = 600;

/// Proxy as the UI sees it (no password).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyView {
    pub id: ProxyId,
    pub name: String,
    pub scheme: &'static str,
    /// `host:port`
    pub label: String,
    pub username: Option<String>,
    pub has_password: bool,
    /// Local relay port the browser uses for this proxy.
    pub port: Option<u16>,
    /// Last check, if any.
    pub geo: Option<ProxyCheck>,
}

impl Manager {
    /// Local relay port for proxy `id`: `base + id - 1`.
    pub(crate) fn relay_port(&self, id: ProxyId) -> Option<u16> {
        u16::try_from(id - 1).ok().and_then(|off| self.cfg.proxy_base_port.checked_add(off))
    }

    pub(crate) fn proxy_view(&self, p: &Proxy) -> ProxyView {
        ProxyView {
            id: p.id,
            name: p.name.clone(),
            scheme: p.scheme.as_str(),
            label: p.label(),
            username: p.username.clone(),
            has_password: p.has_password(),
            port: self.relay_port(p.id),
            geo: p.last_check.clone(),
        }
    }

    pub async fn proxies(&self) -> Result<Vec<ProxyView>> {
        Ok(self.d.proxies.list().await?.iter().map(|p| self.proxy_view(p)).collect())
    }

    pub(crate) async fn proxy(&self, id: ProxyId) -> Result<Proxy> {
        self.d.proxies.get(id).await?.ok_or_else(|| CmError::NotFound(format!("proxy #{id}")).into())
    }

    /// Check proxy `id` through itself. `fresh = false` reuses a recent stored check.
    pub async fn check_proxy(&self, id: ProxyId, fresh: bool) -> Result<ProxyCheck> {
        let p = self.proxy(id).await?;
        if !fresh
            && let Some(c) = &p.last_check
            && self.d.clock.now().saturating_sub(c.checked_at) < CHECK_TTL_SECS
        {
            return Ok(c.clone());
        }
        let c = self.d.checker.check(&p).await?;
        self.d.proxies.set_check(id, &c).await?;
        Ok(c)
    }

    /// Assign (`Some`) or clear (`None`) a profile's proxy. Applies at the next start.
    pub async fn assign(&self, profile: &str, proxy: Option<ProxyId>) -> Result<()> {
        if self.running.lock().await.contains_key(profile) {
            bail!(CmError::Conflict("stop the profile before changing its proxy".into()));
        }
        if let Some(id) = proxy {
            self.proxy(id).await?;
        }
        self.d.profiles.set_proxy(profile, proxy).await
    }

    /// Make sure the relay for proxy `id` is listening; returns its port.
    pub(crate) async fn ensure_relay(&self, p: &Proxy) -> Result<u16> {
        let mut relays = self.relays.lock().await;
        if let Some((port, h)) = relays.get(&p.id)
            && !h.is_finished()
        {
            return Ok(*port);
        }
        let Some(port) = self.relay_port(p.id) else { bail!("no local port left for proxy #{}", p.id) };
        let listener = cm_relay::bind(port).await?;
        relays.insert(p.id, (port, tokio::spawn(cm_relay::run(listener, p.into()))));
        Ok(port)
    }

    /// Stop the relay of proxy `id` unless a running profile still uses it.
    pub(crate) async fn release_relay(&self, id: ProxyId) {
        let in_use = self.running.lock().await.values().any(|r| r.proxy == Some(id));
        if !in_use && let Some((_, h)) = self.relays.lock().await.remove(&id) {
            h.abort();
        }
    }
}
