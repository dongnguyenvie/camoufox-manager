//! Proxy URL parser: `http|https|socks4|socks4a|socks5|socks5h://[user[:pass]@]host:port`.
//! Credentials are percent-decoded, IPv6 hosts are bracketed in the URL (stored bare).
//! Errors never echo the input (it may contain a password).

use crate::Upstream;
use anyhow::{Result, anyhow, bail};
use cm_core::ProxyScheme;

fn pct_decode(s: &str) -> Result<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = b.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
            out.push(h.ok_or_else(|| anyhow!("bad percent-escape in proxy credentials"))?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| anyhow!("proxy credentials are not valid UTF-8"))
}

/// Parse a proxy URL into an [`Upstream`]. The port is required.
pub fn parse_proxy_url(input: &str) -> Result<Upstream> {
    let input = input.trim();
    let (scheme, rest) = input.split_once("://").ok_or_else(|| anyhow!("proxy URL needs a scheme://"))?;
    let scheme = ProxyScheme::parse(scheme).ok_or_else(|| anyhow!("unsupported proxy scheme"))?;
    // Only the authority matters; ignore any path/query.
    let rest = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let (userinfo, hostport) = match rest.rfind('@') {
        Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
        None => (None, rest),
    };
    let (host, port) = if let Some(r) = hostport.strip_prefix('[') {
        let (h, after) = r.split_once(']').ok_or_else(|| anyhow!("unterminated [ in proxy host"))?;
        let p = after.strip_prefix(':').ok_or_else(|| anyhow!("proxy URL needs a port"))?;
        (h, p)
    } else {
        hostport.rsplit_once(':').ok_or_else(|| anyhow!("proxy URL needs a port"))?
    };
    if host.is_empty() {
        bail!("proxy URL has no host");
    }
    if !hostport.starts_with('[') && host.contains(':') {
        bail!("IPv6 proxy hosts must be in [brackets]");
    }
    let port: u16 = port.parse().ok().filter(|p| *p != 0).ok_or_else(|| anyhow!("bad proxy port"))?;
    let (username, password) = match userinfo {
        None => (None, None),
        Some(u) => match u.split_once(':') {
            Some((n, p)) => (Some(pct_decode(n)?), Some(pct_decode(p)?)),
            None => (Some(pct_decode(u)?), None),
        },
    };
    Ok(Upstream { scheme, host: host.to_string(), port, username: username.filter(|u| !u.is_empty()), password })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_url() {
        let u = parse_proxy_url("socks5h://us%40er:p%3Aw%2Fd@proxy.example.com:1080").unwrap();
        assert_eq!(u.scheme, ProxyScheme::Socks5h);
        assert_eq!((u.host.as_str(), u.port), ("proxy.example.com", 1080));
        assert_eq!(u.username.as_deref(), Some("us@er"));
        assert_eq!(u.password.as_deref(), Some("p:w/d"));
    }

    #[test]
    fn ipv6_and_no_auth() {
        let u = parse_proxy_url("http://[2001:db8::1]:8080/").unwrap();
        assert_eq!(u.host, "2001:db8::1");
        assert_eq!(u.port, 8080);
        assert!(u.username.is_none());
        assert_eq!(u.label(), "[2001:db8::1]:8080");
    }

    #[test]
    fn all_schemes() {
        for s in ["http", "https", "socks4", "socks4a", "socks5", "socks5h"] {
            assert_eq!(parse_proxy_url(&format!("{s}://h:1")).unwrap().scheme.as_str(), s);
        }
    }

    #[test]
    fn at_sign_in_password_and_user_only() {
        let u = parse_proxy_url("http://u:a@b@h:3128").unwrap();
        assert_eq!((u.username.as_deref(), u.password.as_deref(), u.host.as_str()), (Some("u"), Some("a@b"), "h"));
        let u = parse_proxy_url("socks4://uid@h:1080").unwrap();
        assert_eq!((u.username.as_deref(), u.password), (Some("uid"), None));
    }

    #[test]
    fn rejects_bad_input_without_echoing_secret() {
        for bad in ["h:1", "ftp://h:1", "http://h", "http://h:0", "http://h:99999", "http://:80", "http://u:p%zz@h:1", "http://::1:80"] {
            let e = parse_proxy_url(bad).unwrap_err().to_string();
            assert!(!e.contains("p%zz"), "{e}");
        }
    }
}
