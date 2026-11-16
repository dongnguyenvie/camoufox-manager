//! Proxy check: ask ip-api.com through the proxy itself → exit IP, country, city, timezone,
//! coordinates, latency. Phase 2 owns this file (route it through the relay, more fields).

use anyhow::{Result, bail};
use async_trait::async_trait;
use cm_core::{Clock, Proxy, ProxyCheck, ProxyChecker, ProxyScheme, SystemClock};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default)]
pub struct IpApiChecker;

#[async_trait]
impl ProxyChecker for IpApiChecker {
    async fn check(&self, p: &Proxy) -> Result<ProxyCheck> {
        if p.scheme != ProxyScheme::Http {
            bail!("checking {} proxies is not supported yet", p.scheme.as_str());
        }
        let mut proxy = reqwest::Proxy::all(format!("http://{}", p.label()))?;
        if let Some(u) = &p.username {
            proxy = proxy.basic_auth(u, p.password.as_deref().unwrap_or_default());
        }
        let client = reqwest::Client::builder().proxy(proxy).timeout(Duration::from_secs(15)).build()?;
        let t0 = Instant::now();
        let g: serde_json::Value = client.get("http://ip-api.com/json").send().await?.json().await?;
        let latency = t0.elapsed().as_millis() as u64;
        if g["status"] != "success" {
            bail!("proxy returned no geo: {g}");
        }
        let s = |k: &str| g[k].as_str().unwrap_or_default().to_string();
        Ok(ProxyCheck {
            ip: s("query"),
            country: s("country"),
            country_code: s("countryCode"),
            city: s("city"),
            timezone: s("timezone"),
            lat: g["lat"].as_f64().unwrap_or_default(),
            lon: g["lon"].as_f64().unwrap_or_default(),
            checked_at: SystemClock.now(),
            latency_ms: Some(latency),
        })
    }
}
