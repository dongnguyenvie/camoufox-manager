//! Proxy check: ask ip-api.com through the proxy itself → exit IP, country, city, timezone,
//! coordinates, latency.
//!
//! The request never talks to the upstream directly: it goes through a short-lived relay on an
//! ephemeral loopback port, exactly like a browser would. So every upstream protocol the relay
//! supports is checked the same way, and credentials stay inside the relay.

use crate::{Upstream, bind, run};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use cm_core::{Clock, Proxy, ProxyCheck, ProxyChecker, SystemClock};
use std::time::{Duration, Instant};

const GEO_URL: &str = "http://ip-api.com/json?fields=status,message,country,countryCode,city,lat,lon,timezone,query";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Default)]
pub struct IpApiChecker;

#[async_trait]
impl ProxyChecker for IpApiChecker {
    async fn check(&self, p: &Proxy) -> Result<ProxyCheck> {
        check_via(Upstream::from(p), GEO_URL).await
    }
}

/// Aborts the relay task when the check ends (success, error or cancellation).
struct StopOnDrop(tokio::task::JoinHandle<()>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn check_via(up: Upstream, url: &str) -> Result<ProxyCheck> {
    let listener = bind(0).await?;
    let port = listener.local_addr()?.port();
    let _relay = StopOnDrop(tokio::spawn(run(listener, up)));

    let client = reqwest::Client::builder().proxy(reqwest::Proxy::all(format!("http://127.0.0.1:{port}"))?).timeout(TIMEOUT).build()?;
    let t0 = Instant::now();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("proxy unreachable, refused the connection or timed out ({})", e.without_url()))?;
    let latency = t0.elapsed().as_millis() as u64;
    let status = resp.status();
    if status.as_u16() == 407 {
        bail!("proxy rejected the credentials (407)");
    }
    if !status.is_success() {
        bail!("proxy answered HTTP {status}");
    }
    let g: serde_json::Value = resp.json().await.context("proxy returned something that is not the geo answer")?;
    if g["status"] != "success" {
        bail!("geo lookup failed: {}", g["message"].as_str().unwrap_or("no details"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::ProxyScheme;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    const BODY: &str = r#"{"status":"success","country":"Brazil","countryCode":"BR","city":"Sao Paulo","lat":-23.5,"lon":-46.6,"timezone":"America/Sao_Paulo","query":"9.9.9.9"}"#;

    fn up(port: u16, user: Option<&str>) -> Upstream {
        Upstream {
            scheme: ProxyScheme::Http,
            host: "127.0.0.1".into(),
            port,
            username: user.map(Into::into),
            password: user.map(|_| "pw".into()),
        }
    }

    /// Fake HTTP proxy answering every request with `reply` (after reading the head).
    async fn fake_proxy(reply: String) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let (mut s, _) = l.accept().await.unwrap();
                let reply = reply.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let _ = s.read(&mut buf).await;
                    let _ = s.write_all(reply.as_bytes()).await;
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn checks_through_the_relay() {
        let port = fake_proxy(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{BODY}", BODY.len())).await;
        let c = check_via(up(port, Some("u")), "http://geo.test/json").await.unwrap();
        assert_eq!((c.ip.as_str(), c.country_code.as_str(), c.timezone.as_str()), ("9.9.9.9", "BR", "America/Sao_Paulo"));
        assert!(c.latency_ms.is_some());
    }

    #[tokio::test]
    async fn bad_credentials_and_dead_proxy_are_errors() {
        let port = fake_proxy("HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n".into()).await;
        let e = check_via(up(port, Some("u")), "http://geo.test/json").await.unwrap_err();
        assert!(e.to_string().contains("407"), "{e}");

        let dead = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port();
        let e = check_via(up(dead, None), "http://geo.test/json").await.unwrap_err();
        assert!(e.to_string().contains("unreachable"), "{e}");
        assert!(!e.to_string().contains("pw"));
    }
}
