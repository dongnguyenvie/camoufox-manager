//! Local-network target filter: what the browser may not reach through the relay when a profile
//! has "block local network" on. Only the browser's requested destination is checked, never the
//! upstream proxy (which may legitimately sit on a LAN address).
//!
//! Hostnames are not resolved here (direct mode, `run_direct`, also checks the resolved IPs): the relay never does local DNS (the upstream resolves them
//! remotely), so only `localhost` / `*.localhost` names and IP literals (including the legacy
//! `inet_aton` spellings such as `2130706433` or `0x7f.1`) are recognised.

use crate::Dest;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Loopback, unspecified/"this network", RFC1918, link-local and CGNAT (100.64/10).
pub fn is_local_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || o[0] == 0
        || (o[0] == 100 && (64..128).contains(&o[1]))
}

/// `::1`, `::`, `fc00::/7`, `fe80::/10`, and IPv4-mapped forms of the IPv4 ranges above.
pub fn is_local_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    if ip.is_loopback() || ip.is_unspecified() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    ip.to_ipv4_mapped().is_some_and(is_local_v4)
}

/// `localhost`, `*.localhost` (trailing dot and case ignored).
fn is_localhost_name(h: &str) -> bool {
    let h = h.trim_end_matches('.').to_ascii_lowercase();
    h == "localhost" || h.ends_with(".localhost")
}

/// `inet_aton` rules: 1-4 dot-separated numbers (decimal, `0x` hex, leading-0 octal); the last
/// one fills the remaining bytes. `None` when the name is not such a literal.
fn parse_legacy_v4(h: &str) -> Option<Ipv4Addr> {
    let parts: Vec<&str> = h.trim_end_matches('.').split('.').collect();
    if parts.len() > 4 {
        return None;
    }
    let mut nums = Vec::new();
    for p in &parts {
        let n = if let Some(x) = p.strip_prefix("0x").or_else(|| p.strip_prefix("0X")) {
            u64::from_str_radix(x, 16).ok()?
        } else if p.len() > 1 && p.starts_with('0') {
            u64::from_str_radix(&p[1..], 8).ok()?
        } else {
            p.parse::<u64>().ok()?
        };
        nums.push(n);
    }
    let (last, head) = nums.split_last()?;
    let mut v: u64 = 0;
    for (i, n) in head.iter().enumerate() {
        if *n > 255 {
            return None;
        }
        v |= n << (24 - 8 * i);
    }
    let room = 8 * (4 - head.len() as u32);
    if *last >= (1u64 << room) {
        return None;
    }
    Some(Ipv4Addr::from((v | last) as u32))
}

/// Is this destination on the local machine or network?
pub fn is_local(d: &Dest) -> bool {
    match d {
        Dest::V4(ip, _) => is_local_v4(*ip),
        Dest::V6(ip, _) => is_local_v6(*ip),
        Dest::Domain(h, _) => is_localhost_name(h) || parse_legacy_v4(h).is_some_and(is_local_v4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(host: &str) -> bool {
        is_local(&Dest::new(host, 80))
    }

    #[test]
    fn blocks_local_ranges() {
        for h in [
            "127.0.0.1",
            "127.9.9.9",
            "0.0.0.0",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.127.255.255",
            "::1",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "febf::1",
            "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
            "localhost",
            "LOCALHOST",
            "localhost.",
            "a.b.localhost",
            "2130706433",
            "0x7f.1",
            "017700000001",
            "10.1",
            "192.168.257",
        ] {
            assert!(local(h), "{h} must be blocked");
        }
    }

    #[test]
    fn allows_public_targets() {
        for h in [
            "8.8.8.8",
            "172.15.0.1",
            "172.32.0.1",
            "100.63.255.255",
            "100.128.0.1",
            "192.169.0.1",
            "2001:db8::1",
            "fec0::1",
            "::ffff:8.8.8.8",
            "example.com",
            "localhost.example.com",
            "notlocalhost",
            "1.2.3.4.5",
            "999.1.1.1",
        ] {
            assert!(!local(h), "{h} must be allowed");
        }
    }
}
