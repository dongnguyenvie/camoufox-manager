//! Local fail-closed proxy relay.
//!
//! The browser is pointed at a plain HTTP proxy on `127.0.0.1:P` with no auth; this crate
//! forwards to the real upstream and holds its credentials, so neither the engine nor the UI
//! ever sees them. Any upstream error closes the browser's connection — there is never a
//! direct path to the network (no real-IP leak).
//!
//! - `http`: upstream HTTP proxy (phase 1). Phase 3 adds `https`, `socks5`, `socks4a` modules
//!   and dispatches on [`Upstream::scheme`] in [`handle`].
//! - `check`: [`IpApiChecker`], the [`ProxyChecker`](cm_core::ProxyChecker) (phase 2 owns it).

mod check;
mod http;

pub use check::IpApiChecker;

use anyhow::{Context, Result, bail};
use cm_core::{Proxy, ProxyScheme};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

/// Where the relay forwards to (credentials included — never log it, use [`Upstream::label`]).
#[derive(Clone, Debug)]
pub struct Upstream {
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
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
    let up = Arc::new(up);
    loop {
        let Ok((client, _)) = listener.accept().await else { continue };
        let up = up.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(client, &up).await {
                eprintln!("[relay {}] {e:#}", up.label());
            }
        });
    }
}

/// One browser connection. Unsupported schemes close it (fail-closed).
async fn handle(client: TcpStream, up: &Upstream) -> Result<()> {
    match up.scheme {
        ProxyScheme::Http => http::handle(client, up).await,
        other => bail!("upstream scheme {} not supported yet", other.as_str()),
    }
}
