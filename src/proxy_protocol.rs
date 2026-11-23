//! HAProxy PROXY protocol v1/v2 parsing plus nginx `set_real_ip_from` style
//! trusted-proxy client-IP recovery. Only a trusted immediate peer may supply
//! the real address; the origin otherwise keeps the TCP peer.
use std::net::IpAddr;

use http::HeaderMap;
use tokio::io::AsyncReadExt;

use crate::config::RealIpConfig;

const V2_SIG: &[u8; 12] = b"\x0d\x0a\x0d\x0a\x00\x0d\x0a\x51\x55\x49\x54\x0a";

/// Returns (bytes_consumed, real_client_ip). `None` IP means LOCAL/UNKNOWN:
/// the connection keeps its TCP peer address.
pub fn parse(buf: &[u8]) -> Option<(usize, Option<IpAddr>)> {
    if buf.starts_with(b"PROXY ") {
        let end = buf.windows(2).position(|w| w == b"\r\n")?;
        let line = std::str::from_utf8(&buf[..end]).ok()?;
        let consumed = end + 2;
        let mut parts = line.split(' ');
        let _ = parts.next()?; // PROXY
        let proto = parts.next()?;
        if proto == "UNKNOWN" {
            return Some((consumed, None));
        }
        let src = parts.next()?;
        let _dst = parts.next()?;
        let _sport = parts.next()?;
        let _dport = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        let ip = src.parse::<IpAddr>().ok()?;
        return Some((consumed, Some(ip)));
    }
    if buf.len() >= 16 && &buf[..12] == V2_SIG {
        let ver_cmd = buf[12];
        if ver_cmd >> 4 != 2 {
            return None;
        }
        let fam = buf[13];
        let len = u16::from_be_bytes([buf[14], buf[15]]) as usize;
        let total = 16 + len;
        if buf.len() < total {
            return None;
        }
        let addr = &buf[16..total];
        let ip = match fam {
            0x11 if addr.len() >= 12 => Some(IpAddr::from(
                <[u8; 4]>::try_from(&addr[0..4]).ok()?,
            )),
            0x21 if addr.len() >= 36 => Some(IpAddr::from(
                <[u8; 16]>::try_from(&addr[0..16]).ok()?,
            )),
            _ => None, // LOCAL / UNIX / unspecified: keep peer
        };
        // Command LOCAL (0x20) carries no meaningful address.
        if ver_cmd & 0x0f == 0x00 {
            return Some((total, None));
        }
        return Some((total, ip));
    }
    None
}

/// Peek (non-consuming) the PROXY header on a fresh connection; only consume it
/// when it parses. Returns the advertised client IP, or None to keep the peer.
pub async fn accept_strip(
    stream: &mut tokio::net::TcpStream,
    enabled: bool,
) -> std::io::Result<Option<IpAddr>> {
    if !enabled {
        return Ok(None);
    }
    let mut buf = [0u8; 256];
    let n = stream.peek(&mut buf).await?;
    if n == 0 {
        return Ok(None);
    }
    let peeked = &buf[..n];
    if peeked.starts_with(b"PROXY ") {
        if let Some((consumed, ip)) = parse(peeked) {
            let mut skip = vec![0u8; consumed];
            stream.read_exact(&mut skip).await?;
            return Ok(ip);
        }
        return Ok(None);
    }
    if n >= 16 && &peeked[..12] == V2_SIG {
        let len = u16::from_be_bytes([peeked[14], peeked[15]]) as usize;
        let total = 16 + len;
        if total > 4096 {
            return Ok(None);
        }
        let mut head = vec![0u8; total];
        let mut pn = stream.peek(&mut head).await?;
        let mut tries = 0;
        while pn < total && tries < 8 {
            stream.readable().await?;
            pn = stream.peek(&mut head).await?;
            if pn == 0 {
                break;
            }
            tries += 1;
        }
        if pn < total {
            return Ok(None);
        }
        if let Some((consumed, ip)) = parse(&head) {
            let mut skip = vec![0u8; consumed];
            stream.read_exact(&mut skip).await?;
            return Ok(ip);
        }
    }
    Ok(None)
}

/// Recover the client IP from a trusted proxy's forwarded header (left-most).
pub fn real_ip(peer: IpAddr, headers: &HeaderMap, cfg: &RealIpConfig) -> IpAddr {
    if !cfg.enabled || cfg.trusted.is_empty() {
        return peer;
    }
    if !crate::static_security::peer_allowed(peer, &cfg.trusted) {
        return peer;
    }
    let Some(value) = headers.get(cfg.header.as_str()) else {
        return peer;
    };
    let Ok(value) = value.to_str() else {
        return peer;
    };
    let mut chain: Vec<IpAddr> = value
        .split(',')
        .filter_map(|token| token.trim().parse::<IpAddr>().ok())
        .collect();
    if chain.is_empty() {
        return peer;
    }
    // nginx real_ip default: the last (right-most) address in the header.
    if !cfg.recursive {
        return chain.pop().unwrap();
    }
    // real_ip_recursive: right-most address not inside the trusted set.
    while let Some(ip) = chain.pop() {
        if !crate::static_security::peer_allowed(ip, &cfg.trusted) {
            return ip;
        }
    }
    peer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_tcp4_and_unknown() {
        let (n, ip) = parse(b"PROXY TCP4 1.2.3.4 5.6.7.8 1111 80\r\nGET /").unwrap();
        assert_eq!(ip.unwrap(), "1.2.3.4".parse::<IpAddr>().unwrap());
        assert_eq!(&b"PROXY TCP4 1.2.3.4 5.6.7.8 1111 80\r\nGET /"[n..], b"GET /");
        let (n, ip) = parse(b"PROXY UNKNOWN\r\nx").unwrap();
        assert!(ip.is_none());
        assert_eq!(n, 15);
        assert!(parse(b"GET / HTTP/1.1\r\n").is_none());
    }

    #[test]
    fn v2_tcp4_binary() {
        let mut buf = V2_SIG.to_vec();
        buf.push(0x21); // v2 PROXY
        buf.push(0x11); // TCP4
        buf.extend_from_slice(&12u16.to_be_bytes());
        buf.extend_from_slice(&[1, 2, 3, 4]);
        buf.extend_from_slice(&[5, 6, 7, 8]);
        buf.extend_from_slice(&[0x04, 0x52]);
        buf.extend_from_slice(&[0x00, 0x50]);
        buf.extend_from_slice(b"GET /");
        let (n, ip) = parse(&buf).unwrap();
        assert_eq!(ip.unwrap(), "1.2.3.4".parse::<IpAddr>().unwrap());
        assert_eq!(&buf[n..], b"GET /");
    }

    #[test]
    fn real_ip_only_from_trusted_peer() {
        let mut cfg = RealIpConfig {
            enabled: true,
            trusted: vec!["10.0.0.0/8".into()],
            header: "x-forwarded-for".into(),
            recursive: false,
        };
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "9.9.9.9, 10.0.0.1".parse().unwrap());
        // nginx default takes the right-most address.
        assert_eq!(
            real_ip("10.0.0.1".parse().unwrap(), &h, &cfg),
            "10.0.0.1".parse::<IpAddr>().unwrap()
        );
        cfg.recursive = true;
        assert_eq!(
            real_ip("10.0.0.1".parse().unwrap(), &h, &cfg),
            "9.9.9.9".parse::<IpAddr>().unwrap()
        );
        // Untrusted peer keeps its own address.
        assert_eq!(
            real_ip("8.8.8.8".parse().unwrap(), &h, &cfg),
            "8.8.8.8".parse::<IpAddr>().unwrap()
        );
    }
}