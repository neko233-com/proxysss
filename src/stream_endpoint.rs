//! Shared syntax for stream targets. Parsing never performs DNS or registry I/O.

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

// A timed-out libc/NSS lookup cannot be interrupted. Its permit therefore stays
// on the blocking job until it really exits, bounding abandoned resolver work.
static DNS_LOOKUPS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(64);

pub(crate) enum StreamEndpoint<'a> {
    Socket(SocketAddr),
    Host { name: &'a str, port: u16 },
}

impl<'a> StreamEndpoint<'a> {
    pub(crate) fn parse(value: &'a str) -> Result<Self> {
        if let Ok(address) = value.parse::<SocketAddr>() {
            if address.port() == 0 {
                bail!("stream upstream port must be in 1..=65535");
            }
            return Ok(Self::Socket(address));
        }
        let invalid = || {
            anyhow!("stream upstream must be host:port or [IPv6]:port, without a scheme, path or whitespace")
        };
        let (name, port_text) = value.rsplit_once(':').ok_or_else(invalid)?;
        let port = port_text
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .filter(|_| port_text.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(invalid)?;
        let host = name.strip_suffix('.').unwrap_or(name);
        if host.is_empty()
            || host.len() > 253
            || host.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            })
            || (host.contains('.') && host.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
        {
            return Err(invalid());
        }
        Ok(Self::Host { name, port })
    }

    /// Called only while establishing an association/probe, never per datagram.
    /// Resolution has a five-second deadline and a global in-flight bound.
    pub(crate) async fn resolve_first(self) -> Result<SocketAddr> {
        match self {
            Self::Socket(address) => Ok(address),
            Self::Host { name, port } => {
                let permit = DNS_LOOKUPS
                    .try_acquire()
                    .context("stream DNS resolver is busy")?;
                let name = name.to_owned();
                let lookup = tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    (name.as_str(), port)
                        .to_socket_addrs()
                        .context("failed resolving stream upstream")?
                        .next()
                        .ok_or_else(|| anyhow!("stream upstream resolved to no addresses"))
                });
                tokio::time::timeout(Duration::from_secs(5), lookup)
                    .await
                    .context("stream DNS resolution exceeded 5 seconds")?
                    .context("stream DNS resolver task failed")?
            }
        }
    }
}
