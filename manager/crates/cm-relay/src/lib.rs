//! Local fail-closed proxy relay.
//!
//! The browser is pointed at a plain HTTP proxy on `127.0.0.1:P` with no auth (CONNECT and
//! absolute-URI requests); this crate forwards to the real upstream and holds its credentials,
//! so neither the engine nor the UI ever sees them. Any error closes the browser's connection —
//! there is never a direct path to the network (no real-IP leak).
//!
//! Upstream protocols, chosen by [`Upstream::scheme`]:
//! - `http`: HTTP proxy (credentials as `Proxy-Authorization: Basic`).
//! - `https`: HTTP proxy spoken over TLS (rustls, webpki roots).
//! - `socks5` / `socks5h`: SOCKS5, no-auth or user/pass; the destination hostname is sent to the
//!   proxy (remote DNS), IPv6 destinations supported.
//! - `socks4a` / `socks4`: SOCKS4a with remote DNS, no auth (username becomes the user id);
//!   IPv6 destinations are refused. Plain `socks4` is treated as 4a: it never resolves locally.
//!
//! [`run_direct`] is the no-upstream variant for profiles without a proxy: it connects straight to
//! the target (resolving names locally) and refuses local-network targets, resolved IPs included.
//!
//! Also: [`parse_proxy_url`] for `scheme://user:pass@host:port` strings, and `check`
//! ([`IpApiChecker`], phase 2).

mod check;
mod direct;
mod filter;
mod http;
mod https;
mod socks4a;
mod socks5;
mod target;
mod url;

pub use check::IpApiChecker;
pub use direct::run_direct;
pub use filter::{is_local, is_local_v4, is_local_v6};
pub use target::Dest;
pub use url::parse_proxy_url;

use anyhow::{Context, Result, anyhow, bail};
use cm_core::{Proxy, ProxyScheme};
use rustls::{ClientConfig, pki_types::ServerName};
use std::{fmt, sync::Arc, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    time::timeout,
};

/// Connect + handshake budget for the upstream side.
const SETUP_TIMEOUT: Duration = Duration::from_secs(20);

/// Where the relay forwards to (credentials included — never log it, use [`Upstream::label`]).
#[derive(Clone)]
pub struct Upstream {
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl fmt::Debug for Upstream {
    /// Never prints the password.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Upstream")
            .field("scheme", &self.scheme)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username.as_ref().map(|_| "<set>"))
            .field("password", &self.password.as_ref().map(|_| "<set>"))
            .finish()
    }
}

impl Upstream {
    /// `host:port`, safe to log.
    pub fn label(&self) -> String {
        if self.host.contains(':') { format!("[{}]:{}", self.host, self.port) } else { format!("{}:{}", self.host, self.port) }
    }
}

impl From<&Proxy> for Upstream {
    fn from(p: &Proxy) -> Self {
        Upstream { scheme: p.scheme, host: p.host.clone(), port: p.port, username: p.username.clone(), password: p.password.clone() }
    }
}

/// Bind first (so a busy port fails right away), then hand the listener to [`run`].
/// Port 0 picks a free port.
pub async fn bind(port: u16) -> Result<TcpListener> {
    TcpListener::bind(("127.0.0.1", port)).await.with_context(|| format!("cannot listen on 127.0.0.1:{port}"))
}

/// Accept loop; spawn it as a task and abort the task to stop the relay.
pub async fn run(listener: TcpListener, up: Upstream) {
    run_with_tls(listener, up, https::default_config(), false).await
}

/// [`run`] that, when `block_local` is set, refuses browser requests aimed at loopback, private,
/// link-local or CGNAT addresses and `localhost` names (answered `403`, nothing forwarded).
pub async fn run_filtered(listener: TcpListener, up: Upstream, block_local: bool) {
    run_with_tls(listener, up, https::default_config(), block_local).await
}

/// [`run`] with an explicit TLS client config for `https` upstreams (tests inject a root).
pub(crate) async fn run_with_tls(listener: TcpListener, up: Upstream, tls: Arc<ClientConfig>, block_local: bool) {
    let up = Arc::new(up);
    loop {
        let Ok((client, _)) = listener.accept().await else { continue };
        let up = up.clone();
        let tls = tls.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(client, &up, &tls, block_local).await {
                eprintln!("[relay {}] {e:#}", up.label());
            }
        });
    }
}

async fn connect_upstream(up: &Upstream) -> Result<TcpStream> {
    let s = timeout(SETUP_TIMEOUT, TcpStream::connect((up.host.as_str(), up.port)))
        .await
        .map_err(|_| anyhow!("upstream {} timed out", up.label()))?
        .with_context(|| format!("upstream {} unreachable", up.label()))?;
    let _ = s.set_nodelay(true);
    Ok(s)
}

/// One browser connection. Every error drops it (fail-closed).
async fn handle(mut client: TcpStream, up: &Upstream, tls: &Arc<ClientConfig>, block_local: bool) -> Result<()> {
    let Some(head) = target::read_head(&mut client).await? else { return Ok(()) };
    if block_local {
        // Only the browser's target is checked (never the upstream, which may be on a LAN).
        let (dest, _) = head.dest()?;
        if filter::is_local(&dest) {
            let _ = client.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
            bail!("blocked local-network target");
        }
    }
    match up.scheme {
        ProxyScheme::Http => {
            let mut s = connect_upstream(up).await?;
            http::forward(&mut client, &mut s, &head, up).await
        }
        ProxyScheme::Https => {
            let tcp = connect_upstream(up).await?;
            let name = ServerName::try_from(up.host.clone()).map_err(|_| anyhow!("invalid TLS name for upstream {}", up.label()))?;
            let mut s = timeout(SETUP_TIMEOUT, tokio_rustls::TlsConnector::from(tls.clone()).connect(name, tcp))
                .await
                .map_err(|_| anyhow!("TLS handshake with {} timed out", up.label()))?
                .with_context(|| format!("TLS handshake with {} failed", up.label()))?;
            http::forward(&mut client, &mut s, &head, up).await
        }
        ProxyScheme::Socks5 | ProxyScheme::Socks5h => {
            let (dest, path) = head.dest()?;
            let mut s = connect_upstream(up).await?;
            let creds = up.username.as_deref().map(|u| (u, up.password.as_deref().unwrap_or_default()));
            timeout(SETUP_TIMEOUT, socks5::connect(&mut s, creds, &dest)).await.map_err(|_| anyhow!("SOCKS5 handshake timed out"))??;
            http::tunnel(&mut client, &mut s, &head, &path).await
        }
        ProxyScheme::Socks4 | ProxyScheme::Socks4a => {
            let (dest, path) = head.dest()?;
            let mut s = connect_upstream(up).await?;
            timeout(SETUP_TIMEOUT, socks4a::connect(&mut s, up.username.as_deref(), &dest))
                .await
                .map_err(|_| anyhow!("SOCKS4a handshake timed out"))??;
            http::tunnel(&mut client, &mut s, &head, &path).await
        }
    }
}

#[cfg(test)]
mod tests;
