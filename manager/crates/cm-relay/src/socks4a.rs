//! SOCKS4a client handshake. Hostnames go to the proxy (IP 0.0.0.1 + name), so DNS is remote.
//! SOCKS4 cannot carry IPv6: such destinations are refused (fail closed, no local fallback).

use crate::target::Dest;
use anyhow::{Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) async fn connect<S: AsyncRead + AsyncWrite + Unpin>(s: &mut S, user_id: Option<&str>, dest: &Dest) -> Result<()> {
    let user = user_id.unwrap_or_default();
    if user.as_bytes().contains(&0) {
        bail!("SOCKS4 user id contains NUL");
    }
    let (ip, port, host) = match dest {
        Dest::V4(ip, p) => (ip.octets(), *p, None),
        Dest::V6(..) => bail!("SOCKS4/4a cannot reach IPv6 destinations"),
        Dest::Domain(h, p) => {
            if h.as_bytes().contains(&0) {
                bail!("hostname contains NUL");
            }
            ([0, 0, 0, 1], *p, Some(h))
        }
    };
    let mut req = vec![4, 1];
    req.extend_from_slice(&port.to_be_bytes());
    req.extend_from_slice(&ip);
    req.extend_from_slice(user.as_bytes());
    req.push(0);
    if let Some(h) = host {
        req.extend_from_slice(h.as_bytes());
        req.push(0);
    }
    s.write_all(&req).await?;
    let mut r = [0u8; 8];
    s.read_exact(&mut r).await?;
    match r[1] {
        0x5A => Ok(()),
        0x5B => bail!("SOCKS4 request rejected or failed"),
        0x5C | 0x5D => bail!("SOCKS4 identd check failed (user id rejected)"),
        c => bail!("bad SOCKS4 reply code {c}"),
    }
}
