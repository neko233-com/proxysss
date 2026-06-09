use std::collections::BTreeMap;

use crate::config::{domain_matches_pattern, normalize_stream_listen, StreamRouteConfig};

/// Extract TLS SNI hostname from a TLS ClientHello record (ssl_preread style).
pub fn parse_tls_client_hello_sni(payload: &[u8]) -> Option<String> {
    if payload.len() < 5 || payload[0] != 0x16 {
        return None;
    }
    let record_len = u16::from_be_bytes([payload[3], payload[4]]) as usize;
    if payload.len() < 5 + record_len {
        return None;
    }
    let handshake = &payload[5..5 + record_len];
    if handshake.is_empty() || handshake[0] != 0x01 {
        return None;
    }
    if handshake.len() < 4 {
        return None;
    }
    let hs_len = u32::from_be_bytes([0, handshake[1], handshake[2], handshake[3]]) as usize;
    if handshake.len() < 4 + hs_len {
        return None;
    }
    let mut offset = 4 + 2 + 32;
    if offset >= handshake.len() {
        return None;
    }
    let session_id_len = handshake[offset] as usize;
    offset += 1 + session_id_len;
    if offset + 2 > handshake.len() {
        return None;
    }
    let cipher_len = u16::from_be_bytes([handshake[offset], handshake[offset + 1]]) as usize;
    offset += 2 + cipher_len;
    if offset >= handshake.len() {
        return None;
    }
    let comp_len = handshake[offset] as usize;
    offset += 1 + comp_len;
    if offset + 2 > handshake.len() {
        return None;
    }
    let ext_total = u16::from_be_bytes([handshake[offset], handshake[offset + 1]]) as usize;
    offset += 2;
    let ext_end = offset.saturating_add(ext_total);
    if ext_end > handshake.len() {
        return None;
    }
    while offset + 4 <= ext_end {
        let ext_type = u16::from_be_bytes([handshake[offset], handshake[offset + 1]]);
        let ext_len = u16::from_be_bytes([handshake[offset + 2], handshake[offset + 3]]) as usize;
        offset += 4;
        if offset + ext_len > ext_end {
            break;
        }
        if ext_type == 0 {
            return parse_sni_extension(&handshake[offset..offset + ext_len]);
        }
        offset += ext_len;
    }
    None
}

fn parse_sni_extension(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    if data.len() < 2 + list_len {
        return None;
    }
    let mut offset = 2;
    let list_end = 2 + list_len;
    while offset + 3 <= list_end {
        let name_type = data[offset];
        let name_len = u16::from_be_bytes([data[offset + 1], data[offset + 2]]) as usize;
        offset += 3;
        if offset + name_len > list_end {
            break;
        }
        if name_type == 0 {
            return String::from_utf8(data[offset..offset + name_len].to_vec()).ok();
        }
        offset += name_len;
    }
    None
}

pub struct StreamRouteTable {
    pub by_bind: BTreeMap<String, Vec<StreamRouteConfig>>,
}

impl StreamRouteTable {
    pub fn from_config(routes: &[StreamRouteConfig]) -> Self {
        let mut by_bind = BTreeMap::<String, Vec<StreamRouteConfig>>::new();
        for route in routes {
            let bind = normalize_stream_listen(&route.listen);
            by_bind.entry(bind).or_default().push(route.clone());
        }
        Self { by_bind }
    }

    pub fn routes_for_bind(&self, bind: &str) -> Option<&[StreamRouteConfig]> {
        self.by_bind.get(bind).map(Vec::as_slice)
    }

    pub fn resolve_upstream(
        &self,
        bind: &str,
        sni: Option<&str>,
        default_upstream: &str,
    ) -> Option<ResolvedStreamRoute<'_>> {
        let routes = self.by_bind.get(bind)?;
        if routes.is_empty() {
            return None;
        }
        if let Some(sni_host) = sni {
            for route in routes {
                if route
                    .domains
                    .iter()
                    .any(|domain| domain_matches_pattern(sni_host, domain))
                {
                    return Some(ResolvedStreamRoute {
                        route,
                        upstream: route.upstream.as_str(),
                        protocol: route.protocol.as_str(),
                    });
                }
            }
        }
        if routes.len() == 1 {
            return Some(ResolvedStreamRoute {
                route: &routes[0],
                upstream: routes[0].upstream.as_str(),
                protocol: routes[0].protocol.as_str(),
            });
        }
        let _ = default_upstream;
        None
    }
}

pub struct ResolvedStreamRoute<'a> {
    pub route: &'a StreamRouteConfig,
    pub upstream: &'a str,
    pub protocol: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_tls_client_hello_with_sni(hostname: &str) -> Vec<u8> {
        let host = hostname.as_bytes();
        let sni_list_len = (3 + host.len()) as u16;
        let sni_ext_len = 2 + sni_list_len;
        let mut sni_ext = Vec::new();
        sni_ext.extend_from_slice(&0u16.to_be_bytes());
        sni_ext.extend_from_slice(&sni_ext_len.to_be_bytes());
        sni_ext.extend_from_slice(&sni_list_len.to_be_bytes());
        sni_ext.push(0);
        sni_ext.extend_from_slice(&(host.len() as u16).to_be_bytes());
        sni_ext.extend_from_slice(host);

        let mut handshake = Vec::new();
        handshake.extend_from_slice(&[0x03, 0x03]);
        handshake.extend_from_slice(&[0_u8; 32]);
        handshake.push(0);
        handshake.extend_from_slice(&2u16.to_be_bytes());
        handshake.extend_from_slice(&[0xc0, 0x2c]);
        handshake.push(1);
        handshake.push(0);
        handshake.extend_from_slice(&(sni_ext.len() as u16).to_be_bytes());
        handshake.extend_from_slice(&sni_ext);

        let hs_len = handshake.len();
        let mut record = vec![0x16, 0x03, 0x01];
        record.extend_from_slice(&((hs_len + 4) as u16).to_be_bytes());
        record.push(0x01);
        record.extend_from_slice(&(hs_len as u32).to_be_bytes()[1..]);
        record.extend_from_slice(&handshake);
        record
    }

    #[test]
    fn parse_tls_sni_from_sample_client_hello() {
        let hello = build_tls_client_hello_with_sni("redis.example.com");
        let parsed = parse_tls_client_hello_sni(&hello).expect("sni");
        assert_eq!(parsed, "redis.example.com");
    }

    #[test]
    fn stream_route_table_matches_sni_domain() {
        let routes = vec![StreamRouteConfig {
            name: "redis".to_string(),
            domains: vec!["redis.example.com".to_string()],
            listen: "6379".to_string(),
            upstream: "127.0.0.1:6379".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "redis".to_string(),
            tls_mode: Default::default(),
            access_control: Default::default(),
        }];
        let table = StreamRouteTable::from_config(&routes);
        let resolved = table
            .resolve_upstream("0.0.0.0:6379", Some("redis.example.com"), "fallback:6379")
            .expect("match");
        assert_eq!(resolved.upstream, "127.0.0.1:6379");
        assert_eq!(resolved.protocol, "redis");
    }
}
