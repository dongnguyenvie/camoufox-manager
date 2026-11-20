//! Source-address policy of the sync listener (and of IP literals in pairing URLs): loopback and
//! the Tailscale ranges only. Everything else, LAN and public internet included, is refused.
//!
//! - loopback `127.0.0.0/8`, `::1` (what `tailscale serve` connects from)
//! - Tailscale IPv4 CGNAT `100.64.0.0/10`
//! - Tailscale IPv6 ULA `fd7a:115c:a1e0::/48`
//!
//! IPv4-mapped IPv6 (`::ffff:a.b.c.d`) is judged by its IPv4 address.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Loopback,
    Tailnet,
    /// Anything else: refused.
    Other,
}

fn v4(ip: Ipv4Addr) -> Source {
    let o = ip.octets();
    if o[0] == 127 {
        Source::Loopback
    } else if o[0] == 100 && (64..=127).contains(&o[1]) {
        Source::Tailnet
    } else {
        Source::Other
    }
}

fn v6(ip: Ipv6Addr) -> Source {
    if let Some(m) = ip.to_ipv4_mapped() {
        return v4(m);
    }
    if ip.is_loopback() {
        return Source::Loopback;
    }
    let s = ip.segments();
    if s[0] == 0xfd7a && s[1] == 0x115c && s[2] == 0xa1e0 { Source::Tailnet } else { Source::Other }
}

pub fn classify(ip: IpAddr) -> Source {
    match ip {
        IpAddr::V4(a) => v4(a),
        IpAddr::V6(a) => v6(a),
    }
}

/// May a connection from `ip` talk to the sync listener?
pub fn allowed(ip: IpAddr) -> bool {
    classify(ip) != Source::Other
}

/// Plain-language refusal for the peer (and the log).
pub fn refusal(ip: IpAddr) -> String {
    format!(
        "refused: {ip} is not on your tailnet. Device sync only accepts Tailscale (100.64.0.0/10, fd7a:115c:a1e0::/48) and this machine."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> bool {
        allowed(s.parse().unwrap())
    }

    #[test]
    fn policy() {
        for good in [
            "127.0.0.1",
            "127.9.9.9",
            "::1",
            "100.64.0.1",
            "100.98.86.81",
            "100.127.255.255",
            "fd7a:115c:a1e0::1",
            "fd7a:115c:a1e0:ffff::1",
            "::ffff:127.0.0.1",
            "::ffff:100.100.1.2",
        ] {
            assert!(ok(good), "{good} should pass");
        }
        for bad in [
            "100.63.255.255",
            "100.128.0.0",
            "192.168.1.10",
            "10.0.0.1",
            "172.16.0.1",
            "8.8.8.8",
            "0.0.0.0",
            "fd7a:115c:a1e1::1",
            "fe80::1",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
            "::",
        ] {
            assert!(!ok(bad), "{bad} should be refused");
        }
        assert_eq!(classify("100.100.1.2".parse().unwrap()), Source::Tailnet);
        assert_eq!(classify("::1".parse().unwrap()), Source::Loopback);
    }
}
