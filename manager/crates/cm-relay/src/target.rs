//! Browser-side request parsing: where does the browser want to go?

use anyhow::{Result, anyhow, bail};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Final destination handed to an upstream. Hostnames stay hostnames (remote DNS).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dest {
    Domain(String, u16),
    V4(Ipv4Addr, u16),
    V6(Ipv6Addr, u16),
}

impl Dest {
    pub fn new(host: &str, port: u16) -> Dest {
        let h = host.trim_start_matches('[').trim_end_matches(']');
        if let Ok(ip) = h.parse::<Ipv4Addr>() {
            Dest::V4(ip, port)
        } else if let Ok(ip) = h.parse::<Ipv6Addr>() {
            Dest::V6(ip, port)
        } else {
            Dest::Domain(h.to_string(), port)
        }
    }

    /// `host:port` / `[v6]:port` (CONNECT authority form).
    pub fn parse_authority(s: &str, default_port: Option<u16>) -> Result<Dest> {
        let (host, port) = if let Some(r) = s.strip_prefix('[') {
            let (h, after) = r.split_once(']').ok_or_else(|| anyhow!("bad authority"))?;
            (h, after.strip_prefix(':'))
        } else {
            match s.rsplit_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (s, None),
            }
        };
        let port = match (port, default_port) {
            (Some(p), _) => p.parse::<u16>().map_err(|_| anyhow!("bad port"))?,
            (None, Some(d)) => d,
            (None, None) => bail!("missing port"),
        };
        if host.is_empty() {
            bail!("missing host");
        }
        Ok(Dest::new(host, port))
    }
}

/// A parsed browser request head.
pub struct Head {
    pub raw: Vec<u8>,
    /// Bytes read past the head (start of the body / tunnelled data).
    pub extra: Vec<u8>,
    pub method: String,
    pub target: String,
    pub version: String,
}

impl Head {
    pub fn is_connect(&self) -> bool {
        self.method.eq_ignore_ascii_case("CONNECT")
    }

    /// Header lines (without the request line).
    pub fn lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.raw).split("\r\n").skip(1).filter(|l| !l.is_empty()).map(str::to_string).collect()
    }

    /// Destination and origin-form path of the request.
    pub fn dest(&self) -> Result<(Dest, String)> {
        if self.is_connect() {
            return Ok((Dest::parse_authority(&self.target, None)?, String::new()));
        }
        let (default_port, rest) = if let Some(r) = strip_ci(&self.target, "http://") {
            (80, r)
        } else if let Some(r) = strip_ci(&self.target, "https://") {
            (443, r)
        } else {
            bail!("not an absolute-URI request");
        };
        let (auth, path) = match rest.find(['/', '?']) {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let auth = auth.rsplit_once('@').map_or(auth, |(_, h)| h);
        let path = if path.starts_with('?') { format!("/{path}") } else { path.to_string() };
        Ok((Dest::parse_authority(auth, Some(default_port))?, path))
    }
}

fn strip_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    (s.len() >= prefix.len() && s.is_char_boundary(prefix.len()) && s[..prefix.len()].eq_ignore_ascii_case(prefix))
        .then(|| &s[prefix.len()..])
}

/// Read up to the end of the request head. `Ok(None)` = peer closed before sending anything.
pub async fn read_head<R: AsyncRead + Unpin>(r: &mut R) -> Result<Option<Head>> {
    let mut buf = Vec::with_capacity(4096);
    let head_end = loop {
        let mut chunk = [0u8; 4096];
        let n = r.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 64 * 1024 {
            bail!("request head too large");
        }
    };
    let extra = buf.split_off(head_end);
    let text = String::from_utf8_lossy(&buf).to_string();
    let mut parts = text.lines().next().unwrap_or_default().split_whitespace();
    let (m, t, v) = (parts.next(), parts.next(), parts.next());
    let (Some(m), Some(t)) = (m, t) else { bail!("malformed request line") };
    Ok(Some(Head { method: m.into(), target: t.into(), version: v.unwrap_or("HTTP/1.1").into(), raw: buf, extra }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(line: &str) -> Head {
        let mut p = line.split_whitespace();
        Head { raw: vec![], extra: vec![], method: p.next().unwrap().into(), target: p.next().unwrap().into(), version: "HTTP/1.1".into() }
    }

    #[test]
    fn connect_targets() {
        assert_eq!(head("CONNECT example.com:443 H").dest().unwrap().0, Dest::Domain("example.com".into(), 443));
        assert_eq!(head("CONNECT [2001:db8::1]:443 H").dest().unwrap().0, Dest::V6("2001:db8::1".parse().unwrap(), 443));
        assert_eq!(head("CONNECT 1.2.3.4:8443 H").dest().unwrap().0, Dest::V4("1.2.3.4".parse().unwrap(), 8443));
        assert!(head("CONNECT example.com H").dest().is_err());
    }

    #[test]
    fn absolute_uri() {
        let (d, p) = head("GET http://example.com/a?b=1 H").dest().unwrap();
        assert_eq!((d, p.as_str()), (Dest::Domain("example.com".into(), 80), "/a?b=1"));
        let (d, p) = head("GET http://[::1]:8080 H").dest().unwrap();
        assert_eq!((d, p.as_str()), (Dest::V6("::1".parse().unwrap(), 8080), "/"));
        assert!(head("GET /relative H").dest().is_err());
    }
}
