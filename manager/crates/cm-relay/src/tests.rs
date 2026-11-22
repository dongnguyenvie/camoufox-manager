//! End-to-end relay tests with in-process fake upstreams (no real network).

use crate::{Dest, Upstream, https, run, run_with_tls};
use cm_core::ProxyScheme;
use rustls::{
    RootCertStore, ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};

const CERT_PEM: &[u8] = include_bytes!("testdata/cert.pem");
const KEY_PEM: &[u8] = include_bytes!("testdata/key.pem");

fn up(scheme: ProxyScheme, port: u16, creds: Option<(&str, &str)>) -> Upstream {
    Upstream { scheme, host: "127.0.0.1".into(), port, username: creds.map(|c| c.0.to_string()), password: creds.map(|c| c.1.to_string()) }
}

async fn start_relay(up: Upstream, tls: Option<Arc<rustls::ClientConfig>>) -> u16 {
    let l = crate::bind(0).await.unwrap();
    let port = l.local_addr().unwrap().port();
    match tls {
        Some(t) => tokio::spawn(run_with_tls(l, up, t, false)),
        None => tokio::spawn(run(l, up)),
    };
    port
}

async fn dead_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port()
}

/// Browser does `CONNECT authority`, then (if answered 200) sends "ping". Returns everything received.
async fn browser_connect(relay: u16, authority: &str) -> Vec<u8> {
    let mut c = TcpStream::connect(("127.0.0.1", relay)).await.unwrap();
    c.write_all(format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes()).await.unwrap();
    // The relay may close before we write (fail-closed); ignore write errors.
    let _ = c.write_all(b"ping").await;
    let mut out = Vec::new();
    timeout(Duration::from_secs(10), c.read_to_end(&mut out)).await.expect("relay must close the connection").ok();
    out
}

async fn browser_get(relay: u16, url_and_host: (&str, &str)) -> Vec<u8> {
    let mut c = TcpStream::connect(("127.0.0.1", relay)).await.unwrap();
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nProxy-Authorization: Basic YnJvd3NlcjpzZWNyZXQ=\r\nProxy-Connection: keep-alive\r\n\r\n",
        url_and_host.0, url_and_host.1
    );
    c.write_all(req.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    timeout(Duration::from_secs(10), c.read_to_end(&mut out)).await.expect("relay must close the connection").ok();
    out
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).to_string()
}

// ---------- fake SOCKS5 ----------

#[derive(Debug, Default)]
struct Seen {
    dest: Option<Dest>,
    auth: Option<(String, String)>,
    data: Vec<u8>,
}

async fn read_n(s: &mut TcpStream, n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    s.read_exact(&mut b).await.unwrap();
    b
}

/// One-shot SOCKS5 server. `want` = required (user, pass); `None` = no auth.
async fn fake_socks5(want: Option<(&'static str, &'static str)>) -> (u16, JoinHandle<Seen>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let h = tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let mut seen = Seen::default();
        let hello = read_n(&mut s, 2).await;
        assert_eq!(hello[0], 5);
        let methods = read_n(&mut s, hello[1] as usize).await;
        let pick = match want {
            Some(_) if methods.contains(&2) => 2,
            None if methods.contains(&0) => 0,
            _ => 0xFF,
        };
        s.write_all(&[5, pick]).await.unwrap();
        if pick == 0xFF {
            return seen;
        }
        if pick == 2 {
            let v = read_n(&mut s, 2).await;
            let u = String::from_utf8(read_n(&mut s, v[1] as usize).await).unwrap();
            let pl = read_n(&mut s, 1).await[0];
            let p = String::from_utf8(read_n(&mut s, pl as usize).await).unwrap();
            let ok = Some((u.as_str(), p.as_str())) == want;
            seen.auth = Some((u, p));
            s.write_all(&[1, if ok { 0 } else { 1 }]).await.unwrap();
            if !ok {
                return seen;
            }
        }
        let r = read_n(&mut s, 4).await;
        assert_eq!(&r[..3], &[5, 1, 0]);
        seen.dest = Some(match r[3] {
            1 => {
                let a = read_n(&mut s, 6).await;
                Dest::V4([a[0], a[1], a[2], a[3]].into(), u16::from_be_bytes([a[4], a[5]]))
            }
            4 => {
                let a = read_n(&mut s, 18).await;
                let mut ip = [0u8; 16];
                ip.copy_from_slice(&a[..16]);
                Dest::V6(ip.into(), u16::from_be_bytes([a[16], a[17]]))
            }
            3 => {
                let n = read_n(&mut s, 1).await[0] as usize;
                let a = read_n(&mut s, n + 2).await;
                Dest::Domain(String::from_utf8(a[..n].to_vec()).unwrap(), u16::from_be_bytes([a[n], a[n + 1]]))
            }
            _ => panic!("bad atyp"),
        });
        s.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await.unwrap();
        let mut buf = vec![0u8; 4096];
        let n = s.read(&mut buf).await.unwrap();
        seen.data = buf[..n].to_vec();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\npong").await.unwrap();
        seen
    });
    (port, h)
}

#[tokio::test]
async fn socks5_no_auth_sends_hostname_not_ip() {
    let (port, seen) = fake_socks5(None).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    // Unresolvable name: succeeds only if the relay never resolves locally.
    let out = browser_connect(relay, "no-such-host.invalid:443").await;
    let out = text(&out);
    assert!(out.starts_with("HTTP/1.1 200 Connection Established\r\n\r\n"), "{out}");
    assert!(out.ends_with("pong"));
    let seen = seen.await.unwrap();
    assert_eq!(seen.dest, Some(Dest::Domain("no-such-host.invalid".into(), 443)));
    assert_eq!(seen.data, b"ping");
    assert!(seen.auth.is_none());
}

#[tokio::test]
async fn socks5h_user_pass_rfc1929() {
    let (port, seen) = fake_socks5(Some(("us@er", "p:w d"))).await;
    let relay = start_relay(up(ProxyScheme::Socks5h, port, Some(("us@er", "p:w d"))), None).await;
    let out = text(&browser_connect(relay, "example.com:8443").await);
    assert!(out.ends_with("pong"), "{out}");
    let seen = seen.await.unwrap();
    assert_eq!(seen.auth, Some(("us@er".into(), "p:w d".into())));
    assert_eq!(seen.dest, Some(Dest::Domain("example.com".into(), 8443)));
}

#[tokio::test]
async fn socks5_ipv6_and_ipv4_destinations() {
    let (port, seen) = fake_socks5(None).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    let out = text(&browser_connect(relay, "[2001:db8::7]:443").await);
    assert!(out.ends_with("pong"), "{out}");
    assert_eq!(seen.await.unwrap().dest, Some(Dest::V6("2001:db8::7".parse().unwrap(), 443)));

    let (port, seen) = fake_socks5(None).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    browser_connect(relay, "203.0.113.9:80").await;
    assert_eq!(seen.await.unwrap().dest, Some(Dest::V4("203.0.113.9".parse().unwrap(), 80)));
}

#[tokio::test]
async fn socks5_plain_http_request_becomes_origin_form() {
    let (port, seen) = fake_socks5(None).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    let out = text(&browser_get(relay, ("http://example.com:8080/a?b=1", "example.com:8080")).await);
    assert!(out.ends_with("pong"), "{out}");
    let seen = seen.await.unwrap();
    assert_eq!(seen.dest, Some(Dest::Domain("example.com".into(), 8080)));
    let head = text(&seen.data);
    assert!(head.starts_with("GET /a?b=1 HTTP/1.1\r\n"), "{head}");
    assert!(!head.to_ascii_lowercase().contains("proxy-"), "browser proxy headers must not leak upstream: {head}");
    assert!(head.contains("Connection: close\r\n"));
}

#[tokio::test]
async fn socks5_bad_auth_fails_closed() {
    let (port, seen) = fake_socks5(Some(("good", "good"))).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, Some(("good", "WRONG"))), None).await;
    let out = browser_connect(relay, "example.com:443").await;
    assert!(out.is_empty(), "nothing may reach the browser: {}", text(&out));
    let seen = seen.await.unwrap();
    assert!(seen.dest.is_none(), "no connect request after failed auth");
}

#[tokio::test]
async fn socks5_server_wants_auth_but_none_configured_fails_closed() {
    let (port, _seen) = fake_socks5(Some(("u", "p"))).await;
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());
}

#[tokio::test]
async fn socks5_upstream_down_fails_closed() {
    let relay = start_relay(up(ProxyScheme::Socks5, dead_port().await, None), None).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());
    assert!(browser_get(relay, ("http://example.com/", "example.com")).await.is_empty());
}

#[tokio::test]
async fn socks5_connect_refused_reply_fails_closed() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        read_n(&mut s, 3).await;
        s.write_all(&[5, 0]).await.unwrap();
        read_n(&mut s, 5).await;
        s.write_all(&[5, 5, 0, 1, 0, 0, 0, 0, 0, 0]).await.unwrap(); // connection refused
    });
    let relay = start_relay(up(ProxyScheme::Socks5, port, None), None).await;
    assert!(browser_connect(relay, "1.2.3.4:80").await.is_empty());
}

// ---------- fake SOCKS4a ----------

async fn fake_socks4a(accept: bool) -> (u16, JoinHandle<(Vec<u8>, String, Option<String>, Vec<u8>)>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let h = tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let hdr = read_n(&mut s, 8).await; // ver, cmd, port(2), ip(4)
        assert_eq!(&hdr[..2], &[4, 1]);
        async fn cstr(s: &mut TcpStream) -> String {
            let mut v = Vec::new();
            loop {
                let b = s.read_u8().await.unwrap();
                if b == 0 {
                    return String::from_utf8(v).unwrap();
                }
                v.push(b);
            }
        }
        let user = cstr(&mut s).await;
        let host = (hdr[4..7] == [0, 0, 0] && hdr[7] != 0).then_some(());
        let host = match host {
            Some(()) => Some(cstr(&mut s).await),
            None => None,
        };
        s.write_all(&[0, if accept { 0x5A } else { 0x5B }, 0, 0, 0, 0, 0, 0]).await.unwrap();
        let mut data = Vec::new();
        if accept {
            let mut buf = vec![0u8; 4096];
            let n = s.read(&mut buf).await.unwrap();
            data = buf[..n].to_vec();
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\npong").await.unwrap();
        }
        (hdr, user, host, data)
    });
    (port, h)
}

#[tokio::test]
async fn socks4a_sends_hostname_with_remote_dns() {
    let (port, seen) = fake_socks4a(true).await;
    let relay = start_relay(up(ProxyScheme::Socks4a, port, Some(("uid", "ignored"))), None).await;
    let out = text(&browser_connect(relay, "no-such-host.invalid:443").await);
    assert!(out.starts_with("HTTP/1.1 200 Connection Established\r\n\r\n") && out.ends_with("pong"), "{out}");
    let (hdr, user, host, data) = seen.await.unwrap();
    assert_eq!(u16::from_be_bytes([hdr[2], hdr[3]]), 443);
    assert_eq!(&hdr[4..8], &[0, 0, 0, 1]);
    assert_eq!(user, "uid");
    assert_eq!(host.as_deref(), Some("no-such-host.invalid"));
    assert_eq!(data, b"ping");
}

#[tokio::test]
async fn socks4a_ipv4_literal_uses_plain_socks4_form() {
    let (port, seen) = fake_socks4a(true).await;
    let relay = start_relay(up(ProxyScheme::Socks4a, port, None), None).await;
    browser_connect(relay, "203.0.113.9:80").await;
    let (hdr, user, host, _) = seen.await.unwrap();
    assert_eq!(&hdr[4..8], &[203, 0, 113, 9]);
    assert!((user.is_empty(), host.is_none()) == (true, true));
}

#[tokio::test]
async fn socks4a_rejects_ipv6_destination_without_contacting_upstream() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let relay = start_relay(up(ProxyScheme::Socks4a, port, None), None).await;
    assert!(browser_connect(relay, "[2001:db8::1]:443").await.is_empty());
    // The relay refuses before dialing, so nothing is waiting in the accept queue handshake.
    let r = timeout(Duration::from_millis(200), l.accept()).await;
    if let Ok(Ok((mut s, _))) = r {
        let mut b = Vec::new();
        s.read_to_end(&mut b).await.ok();
        assert!(b.is_empty(), "no SOCKS bytes may be sent for an IPv6 destination");
    }
}

#[tokio::test]
async fn socks4a_rejected_and_down_fail_closed() {
    let (port, _seen) = fake_socks4a(false).await;
    let relay = start_relay(up(ProxyScheme::Socks4a, port, None), None).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());

    let relay = start_relay(up(ProxyScheme::Socks4a, dead_port().await, None), None).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());
}

// ---------- fake HTTPS proxy (self-signed, injected root) ----------

fn client_tls_trusting_fake() -> Arc<rustls::ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from_pem_slice(CERT_PEM).unwrap()).unwrap();
    https::config_with_roots(roots)
}

/// One-shot TLS proxy; returns the head it received.
async fn fake_tls_proxy() -> (u16, JoinHandle<String>) {
    let certs = vec![CertificateDer::from_pem_slice(CERT_PEM).unwrap()];
    let key = PrivateKeyDer::from_pem_slice(KEY_PEM).unwrap();
    let cfg = ServerConfig::builder_with_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let h = tokio::spawn(async move {
        let (tcp, _) = l.accept().await.unwrap();
        let Ok(mut s) = acceptor.accept(tcp).await else { return String::new() };
        let mut buf = vec![0u8; 4096];
        let n = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await.unwrap();
        let _ = s.shutdown().await;
        String::from_utf8_lossy(&buf[..n]).to_string()
    });
    (port, h)
}

#[tokio::test]
async fn https_proxy_tls_with_injected_root_and_basic_auth() {
    let (port, seen) = fake_tls_proxy().await;
    let relay = start_relay(up(ProxyScheme::Https, port, Some(("u", "p"))), Some(client_tls_trusting_fake())).await;
    let out = text(&browser_get(relay, ("http://example.com/x", "example.com")).await);
    assert!(out.ends_with("ok"), "{out}");
    let head = seen.await.unwrap();
    assert!(head.starts_with("GET http://example.com/x HTTP/1.1\r\n"), "{head}");
    assert!(head.contains("Proxy-Authorization: Basic dTpw\r\n"), "{head}");
    assert!(!head.contains("YnJvd3Nlcjpzc2VjcmV0"));
}

#[tokio::test]
async fn https_proxy_connect_is_forwarded_over_tls() {
    let (port, seen) = fake_tls_proxy().await;
    let relay = start_relay(up(ProxyScheme::Https, port, Some(("u", "p"))), Some(client_tls_trusting_fake())).await;
    let out = text(&browser_connect(relay, "example.com:443").await);
    assert!(out.starts_with("HTTP/1.1 200 OK"), "{out}");
    let head = seen.await.unwrap();
    assert!(head.starts_with("CONNECT example.com:443 HTTP/1.1\r\n"), "{head}");
    assert!(head.contains("Proxy-Authorization: Basic dTpw\r\n"));
}

#[tokio::test]
async fn https_proxy_untrusted_cert_fails_closed() {
    // Default config = webpki roots only, so the self-signed fake must be rejected.
    let (port, seen) = fake_tls_proxy().await;
    let relay = start_relay(up(ProxyScheme::Https, port, Some(("u", "p"))), None).await;
    assert!(browser_get(relay, ("http://example.com/", "example.com")).await.is_empty());
    assert_eq!(seen.await.unwrap(), "", "no request may be sent over an unverified TLS session");
}

#[tokio::test]
async fn https_proxy_down_fails_closed() {
    let relay = start_relay(up(ProxyScheme::Https, dead_port().await, None), Some(client_tls_trusting_fake())).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());
}

#[tokio::test]
async fn https_upstream_that_speaks_plain_tcp_fails_closed() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut s, _) = l.accept().await.unwrap();
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await;
    });
    let relay = start_relay(up(ProxyScheme::Https, port, None), Some(client_tls_trusting_fake())).await;
    assert!(browser_connect(relay, "example.com:443").await.is_empty());
}

// ---------- block local network ----------

/// Upstream HTTP proxy on loopback (a LAN upstream must stay usable) that answers 200 and counts connections.
async fn counting_upstream() -> (u16, Arc<std::sync::atomic::AtomicUsize>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let n = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let n2 = n.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else { return };
            n2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf).await;
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").await;
            });
        }
    });
    (port, n)
}

async fn start_filtered(up: Upstream, block: bool) -> u16 {
    let l = crate::bind(0).await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(crate::run_filtered(l, up, block));
    port
}

#[tokio::test]
async fn block_local_refuses_local_targets_but_not_the_upstream() {
    let (up_port, hits) = counting_upstream().await;
    let relay = start_filtered(up(ProxyScheme::Http, up_port, None), true).await;
    for (url, host) in [
        ("http://127.0.0.1:8080/", "127.0.0.1"),
        ("http://localhost/x", "localhost"),
        ("http://192.168.1.1/", "192.168.1.1"),
        ("http://[::1]:3000/", "[::1]"),
        ("http://foo.localhost/", "foo.localhost"),
        ("http://2130706433/", "2130706433"),
    ] {
        let r = text(&browser_get(relay, (url, host)).await);
        assert!(r.starts_with("HTTP/1.1 403"), "{url}: {r}");
    }
    for a in ["127.0.0.1:22", "10.0.0.5:443", "[fd00::1]:443", "100.64.0.1:80", "169.254.169.254:80"] {
        assert!(text(&browser_connect(relay, a).await).starts_with("HTTP/1.1 403"), "{a}");
    }
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 0, "nothing reached the upstream");
    // A public target still goes through (the upstream itself is on 127.0.0.1).
    let r = text(&browser_get(relay, ("http://example.com/", "example.com")).await);
    assert!(r.ends_with("ok"), "{r}");
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn block_local_off_forwards_local_targets() {
    let (up_port, hits) = counting_upstream().await;
    let relay = start_filtered(up(ProxyScheme::Http, up_port, None), false).await;
    let r = text(&browser_get(relay, ("http://127.0.0.1:8080/", "127.0.0.1")).await);
    assert!(r.ends_with("ok"), "{r}");
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn debug_never_prints_credentials() {
    let u = up(ProxyScheme::Socks5, 1080, Some(("alice", "hunter2")));
    let d = format!("{u:?}");
    assert!(!d.contains("hunter2") && !d.contains("alice"), "{d}");
}
