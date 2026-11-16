//! Upstream HTTP proxy (CONNECT + absolute-URI): the browser only sees 127.0.0.1:P, the
//! upstream credentials are injected here.
//!
//! Per connection: read the request head from the browser, drop its proxy auth, add the
//! upstream `Proxy-Authorization`, then pipe both ways. Upstream failure closes the connection
//! (fail-closed); there is no direct path to the network.

use crate::Upstream;
use anyhow::{Context, Result, bail};
use base64::Engine;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional},
    net::TcpStream,
};

fn auth_header(up: &Upstream) -> Option<String> {
    let user = up.username.as_deref()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{}", up.password.as_deref().unwrap_or_default()));
    Some(format!("Proxy-Authorization: Basic {b64}\r\n"))
}

pub(crate) async fn handle(mut client: TcpStream, up: &Upstream) -> Result<()> {
    // Read up to the end of the request head.
    let mut buf = Vec::with_capacity(4096);
    let head_end = loop {
        let mut chunk = [0u8; 4096];
        let n = client.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 64 * 1024 {
            bail!("request head too large");
        }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let is_connect = request_line.starts_with("CONNECT ");

    let mut new_head = format!("{request_line}\r\n");
    for l in lines.filter(|l| !l.is_empty()) {
        let lower = l.to_ascii_lowercase();
        // Drop the client's auth; for plain HTTP also drop keep-alive so every request carries auth.
        if lower.starts_with("proxy-authorization:")
            || (!is_connect && (lower.starts_with("proxy-connection:") || lower.starts_with("connection:")))
        {
            continue;
        }
        new_head.push_str(l);
        new_head.push_str("\r\n");
    }
    if let Some(h) = auth_header(up) {
        new_head.push_str(&h);
    }
    if !is_connect {
        new_head.push_str("Connection: close\r\n");
    }
    new_head.push_str("\r\n");

    let mut upstream =
        TcpStream::connect((up.host.as_str(), up.port)).await.with_context(|| format!("upstream {} unreachable", up.label()))?;
    upstream.write_all(new_head.as_bytes()).await?;
    upstream.write_all(&buf[head_end..]).await?;
    copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::ProxyScheme;
    use tokio::net::TcpListener;

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
