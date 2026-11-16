//! SOCKS5 client handshake (RFC 1928, RFC 1929 user/pass). Hostnames are sent as-is (ATYP 3):
//! DNS is always resolved by the proxy, never locally.

use crate::target::Dest;
use anyhow::{Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

fn reply_text(code: u8) -> &'static str {
    match code {
        1 => "general failure",
        2 => "connection not allowed",
        3 => "network unreachable",
        4 => "host unreachable",
        5 => "connection refused",
        6 => "TTL expired",
        7 => "command not supported",
        8 => "address type not supported",
        _ => "unknown error",
    }
}

/// Run the handshake on an already-connected stream; on `Ok` the stream is a tunnel to `dest`.
pub(crate) async fn connect<S: AsyncRead + AsyncWrite + Unpin>(s: &mut S, creds: Option<(&str, &str)>, dest: &Dest) -> Result<()> {
    s.write_all(if creds.is_some() { &[5, 2, 0, 2][..] } else { &[5, 1, 0][..] }).await?;
    let mut sel = [0u8; 2];
    s.read_exact(&mut sel).await?;
    if sel[0] != 5 {
        bail!("upstream is not a SOCKS5 server");
    }
    match sel[1] {
        0 => {}
        2 => {
            let Some((u, p)) = creds else { bail!("SOCKS5 server demands credentials, none configured") };
            if u.len() > 255 || p.len() > 255 {
                bail!("SOCKS5 credentials longer than 255 bytes");
            }
            let mut m = vec![1, u.len() as u8];
            m.extend_from_slice(u.as_bytes());
            m.push(p.len() as u8);
            m.extend_from_slice(p.as_bytes());
            s.write_all(&m).await?;
            let mut r = [0u8; 2];
            s.read_exact(&mut r).await?;
            if r[1] != 0 {
                bail!("SOCKS5 authentication rejected");
            }
        }
        0xFF => bail!("SOCKS5 server accepts none of our auth methods"),
        m => bail!("SOCKS5 server chose unsupported auth method {m}"),
    }

    let mut req = vec![5, 1, 0];
    let port = match dest {
        Dest::V4(ip, p) => {
            req.push(1);
            req.extend_from_slice(&ip.octets());
            *p
        }
        Dest::V6(ip, p) => {
            req.push(4);
            req.extend_from_slice(&ip.octets());
            *p
        }
        Dest::Domain(h, p) => {
            if h.is_empty() || h.len() > 255 {
                bail!("destination hostname length not allowed by SOCKS5");
            }
            req.push(3);
            req.push(h.len() as u8);
            req.extend_from_slice(h.as_bytes());
            *p
        }
    };
    req.extend_from_slice(&port.to_be_bytes());
    s.write_all(&req).await?;

    let mut h = [0u8; 4];
    s.read_exact(&mut h).await?;
    if h[0] != 5 {
        bail!("bad SOCKS5 reply");
    }
    if h[1] != 0 {
        bail!("SOCKS5 connect failed: {}", reply_text(h[1]));
    }
    let skip = match h[3] {
        1 => 4,
        4 => 16,
        3 => s.read_u8().await? as usize,
        _ => bail!("bad SOCKS5 bound address type"),
    };
    let mut rest = vec![0u8; skip + 2];
    s.read_exact(&mut rest).await?;
    Ok(())
}
