//! Direct mode: a local HTTP/CONNECT proxy with no upstream. It connects straight to the
//! browser's target, so a profile without a proxy can still have local-network blocking.
//!
//! Same filter as the proxy relays, plus one thing they cannot do: hostnames are resolved here
//! (there is no remote DNS to leak to), and a name that resolves to a loopback / private /
//! link-local / CGNAT address is refused too (DNS rebinding, `localtest.me`-style names). The
//! connection goes to the address that was checked, so the answer cannot change in between.

use crate::{Dest, SETUP_TIMEOUT, filter, http, target};
use anyhow::{Result, anyhow, bail};
use std::net::{IpAddr, SocketAddr};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, lookup_host},
    time::timeout,
};

/// Accept loop of the direct relay; spawn it as a task and abort the task to stop it.
pub async fn run_direct(listener: TcpListener) {
    loop {
        let Ok((client, _)) = listener.accept().await else { continue };
        tokio::spawn(async move {
            if let Err(e) = handle(client).await {
                eprintln!("[relay direct] {e:#}");
            }
        });
    }
}

async fn refuse(client: &mut TcpStream) -> Result<()> {
    let _ = client.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
    bail!("blocked local-network target")
}

/// Addresses of `dest` that may be connected to; `None` when the target must be refused.
async fn allowed_addrs(dest: &Dest) -> Result<Option<Vec<SocketAddr>>> {
    if filter::is_local(dest) {
        return Ok(None);
    }
    let addrs: Vec<SocketAddr> = match dest {
        Dest::V4(ip, port) => vec![SocketAddr::new((*ip).into(), *port)],
        Dest::V6(ip, port) => vec![SocketAddr::new((*ip).into(), *port)],
        Dest::Domain(host, port) => timeout(SETUP_TIMEOUT, lookup_host((host.as_str(), *port)))
            .await
            .map_err(|_| anyhow!("DNS lookup of {host} timed out"))??
            .collect(),
    };
    if addrs.is_empty() {
        bail!("no address for the target");
    }
    let local = |a: &SocketAddr| match a.ip() {
        IpAddr::V4(v) => filter::is_local_v4(v),
        IpAddr::V6(v) => filter::is_local_v6(v),
    };
    if addrs.iter().any(local) {
        return Ok(None);
    }
    Ok(Some(addrs))
}

async fn handle(mut client: TcpStream) -> Result<()> {
    let Some(head) = target::read_head(&mut client).await? else { return Ok(()) };
    let (dest, path) = head.dest()?;
    let Some(addrs) = allowed_addrs(&dest).await? else { return refuse(&mut client).await };
    let mut last = None;
    for a in addrs {
        match timeout(SETUP_TIMEOUT, TcpStream::connect(a)).await {
            Ok(Ok(mut s)) => {
                let _ = s.set_nodelay(true);
                return http::tunnel(&mut client, &mut s, &head, &path).await;
            }
            Ok(Err(e)) => last = Some(anyhow!(e)),
            Err(_) => last = Some(anyhow!("connect to {a} timed out")),
        }
    }
    Err(last.unwrap_or_else(|| anyhow!("no address to connect to")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn direct() -> u16 {
        let l = crate::bind(0).await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(run_direct(l));
        port
    }

    async fn ask(port: u16, req: &str) -> String {
        let mut c = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        c.write_all(req.as_bytes()).await.unwrap();
        let mut out = Vec::new();
        let _ = c.read_to_end(&mut out).await;
        String::from_utf8_lossy(&out).to_string()
    }

    #[tokio::test]
    async fn refuses_literal_and_resolving_local_targets() {
        let p = direct().await;
        for t in ["127.0.0.1:80", "localhost:80", "10.0.0.5:443", "[::1]:80"] {
            let r = ask(p, &format!("CONNECT {t} HTTP/1.1\r\nHost: {t}\r\n\r\n")).await;
            assert!(r.starts_with("HTTP/1.1 403"), "{t} -> {r:?}");
        }
        let r = ask(p, "GET http://127.0.0.1:1/ HTTP/1.1\r\nHost: x\r\n\r\n").await;
        assert!(r.starts_with("HTTP/1.1 403"));
    }

    #[tokio::test]
    async fn allowed_addrs_checks_the_literal_and_the_resolved_ips() {
        assert!(allowed_addrs(&Dest::new("localhost", 80)).await.unwrap().is_none());
        assert!(allowed_addrs(&Dest::new("8.8.8.8", 53)).await.unwrap().is_some());
        // `localhost.` style aliases that only DNS reveals are caught by the resolved-IP check.
        assert!(allowed_addrs(&Dest::Domain("ip6-localhost".into(), 80)).await.map_or(true, |a| a.is_none()));
    }
}
