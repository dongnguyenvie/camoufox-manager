//! HTTP-style forwarding: HTTP and HTTPS upstream proxies (CONNECT + absolute-URI, auth
//! injected), plus the browser-facing half of the SOCKS tunnels (200 reply / origin-form).
//!
//! Per connection: the head is read from the browser, its proxy auth is dropped, the upstream
//! `Proxy-Authorization` added, then both sides are piped. Any failure closes the connection
//! (fail-closed); there is no direct path to the network.

use crate::{Upstream, target::Head};
use anyhow::Result;
use base64::Engine;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, copy_bidirectional};

fn auth_header(up: &Upstream) -> Option<String> {
    let user = up.username.as_deref()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{}", up.password.as_deref().unwrap_or_default()));
    Some(format!("Proxy-Authorization: Basic {b64}\r\n"))
}

/// Head for an HTTP(S) upstream proxy: same request line, browser auth replaced by ours.
fn proxied_head(head: &Head, up: &Upstream) -> String {
    let mut out = format!("{} {} {}\r\n", head.method, head.target, head.version);
    for l in head.lines() {
        let lower = l.to_ascii_lowercase();
        // Drop the client's auth; for plain HTTP also drop keep-alive so every request carries auth.
        if lower.starts_with("proxy-authorization:")
            || (!head.is_connect() && (lower.starts_with("proxy-connection:") || lower.starts_with("connection:")))
        {
            continue;
        }
        out.push_str(&l);
        out.push_str("\r\n");
    }
    if let Some(h) = auth_header(up) {
        out.push_str(&h);
    }
    if !head.is_connect() {
        out.push_str("Connection: close\r\n");
    }
    out.push_str("\r\n");
    out
}

/// Head for a plain request sent through a SOCKS tunnel: origin-form, no proxy headers.
fn origin_head(head: &Head, path: &str) -> String {
    let mut out = format!("{} {} {}\r\n", head.method, path, head.version);
    for l in head.lines() {
        let lower = l.to_ascii_lowercase();
        if lower.starts_with("proxy-authorization:") || lower.starts_with("proxy-connection:") || lower.starts_with("connection:") {
            continue;
        }
        out.push_str(&l);
        out.push_str("\r\n");
    }
    out.push_str("Connection: close\r\n\r\n");
    out
}

/// Pipe the browser through an HTTP(S) upstream proxy.
pub(crate) async fn forward<C, U>(client: &mut C, upstream: &mut U, head: &Head, up: &Upstream) -> Result<()>
where
    C: AsyncRead + AsyncWrite + Unpin,
    U: AsyncRead + AsyncWrite + Unpin,
{
    upstream.write_all(proxied_head(head, up).as_bytes()).await?;
    upstream.write_all(&head.extra).await?;
    upstream.flush().await?;
    copy_bidirectional(client, upstream).await?;
    Ok(())
}

/// Pipe the browser through an established SOCKS tunnel to the request's destination.
pub(crate) async fn tunnel<C, U>(client: &mut C, upstream: &mut U, head: &Head, path: &str) -> Result<()>
where
    C: AsyncRead + AsyncWrite + Unpin,
    U: AsyncRead + AsyncWrite + Unpin,
{
    if head.is_connect() {
        client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    } else {
        upstream.write_all(origin_head(head, path).as_bytes()).await?;
    }
    upstream.write_all(&head.extra).await?;
    upstream.flush().await?;
    copy_bidirectional(client, upstream).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::ProxyScheme;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    /// Fake upstream: records the head it receives and answers 200.
    #[tokio::test]
    async fn injects_auth_and_strips_client_auth() {
        let up_l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up_port = up_l.local_addr().unwrap().port();
        let seen = tokio::spawn(async move {
            let (mut s, _) = up_l.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = s.read(&mut buf).await.unwrap();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await.unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });

        let up = Upstream {
            scheme: ProxyScheme::Http,
            host: "127.0.0.1".into(),
            port: up_port,
            username: Some("u".into()),
            password: Some("p".into()),
        };
        let relay = crate::bind(0).await.unwrap();
        let relay_port = relay.local_addr().unwrap().port();
        tokio::spawn(crate::run(relay, up));

        let mut c = TcpStream::connect(("127.0.0.1", relay_port)).await.unwrap();
        c.write_all(
            b"GET http://example.com/ HTTP/1.1\r\nHost: example.com\r\nProxy-Authorization: Basic xxx\r\nConnection: keep-alive\r\n\r\n",
        )
        .await
        .unwrap();
        let mut resp = String::new();
        c.read_to_string(&mut resp).await.unwrap();
        assert!(resp.ends_with("ok"));

        let head = seen.await.unwrap();
        assert!(head.contains("Proxy-Authorization: Basic dTpw\r\n"));
        assert!(!head.contains("Basic xxx"));
        assert!(head.contains("Connection: close\r\n"));
    }

    #[tokio::test]
    async fn fail_closed_when_upstream_down() {
        // Grab a free port, then close it so nothing listens there.
        let dead = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port();
        let up = Upstream { scheme: ProxyScheme::Http, host: "127.0.0.1".into(), port: dead, username: None, password: None };
        let relay = crate::bind(0).await.unwrap();
        let relay_port = relay.local_addr().unwrap().port();
        tokio::spawn(crate::run(relay, up));
        let mut c = TcpStream::connect(("127.0.0.1", relay_port)).await.unwrap();
        c.write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n").await.unwrap();
        let mut resp = Vec::new();
        c.read_to_end(&mut resp).await.unwrap();
        assert!(resp.is_empty(), "connection closed, nothing forwarded");
    }
}
