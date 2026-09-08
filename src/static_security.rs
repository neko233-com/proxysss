//! Generic static-origin authorization. Viewer identity remains an edge/plugin concern.
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use http::{HeaderMap, Uri};
use sha2::Sha256;

use crate::config::StaticSiteConfig;

pub const ORIGIN_TOKEN_HEADER: &str = "x-proxysss-origin-token";
type HmacSha256 = Hmac<Sha256>;

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn requires_authorization(site: &StaticSiteConfig) -> bool {
    token_enabled(site)
        || (site.security.enabled
            && site.security.allowed_peers_enabled
            && !site.security.allowed_peers.is_empty())
        || signed_url_enabled(site)
}

pub fn token_enabled(site: &StaticSiteConfig) -> bool {
    site.security.enabled
        && site.security.origin_token_enabled
        && !site.security.origin_token.is_empty()
}

pub fn signed_url_enabled(site: &StaticSiteConfig) -> bool {
    site.security.enabled && site.security.signed_url.enabled
}

pub fn site_peer_allowed(site: &StaticSiteConfig, peer: IpAddr) -> bool {
    !site.security.enabled
        || !site.security.allowed_peers_enabled
        || peer_allowed(peer, &site.security.allowed_peers)
}

pub fn fast_lane_eligible(site: &StaticSiteConfig) -> bool {
    !requires_authorization(site) && site.cache_control.is_empty() && !site.rate_limit.enabled
}

fn parse_peer(rule: &str) -> Option<(IpAddr, u8)> {
    let (address, prefix) = match rule.split_once('/') {
        Some((address, prefix)) => (address, Some(prefix.parse::<u8>().ok()?)),
        None => (rule, None),
    };
    let address = address.parse::<IpAddr>().ok()?;
    let bits = if address.is_ipv4() { 32 } else { 128 };
    let prefix = prefix.unwrap_or(bits);
    (prefix <= bits).then_some((address, prefix))
}

pub fn peer_allowed(peer: IpAddr, rules: &[String]) -> bool {
    let peer = match peer {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(peer),
        _ => peer,
    };
    rules.is_empty()
        || rules.iter().any(|rule| match (peer, parse_peer(rule)) {
            (IpAddr::V4(ip), Some((IpAddr::V4(base), prefix))) => {
                let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
                u32::from(ip) & mask == u32::from(base) & mask
            }
            (IpAddr::V6(ip), Some((IpAddr::V6(base), prefix))) => {
                let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
                u128::from(ip) & mask == u128::from(base) & mask
            }
            _ => false,
        })
}

pub fn validate(site: &StaticSiteConfig) -> Result<()> {
    let security = &site.security;
    if token_enabled(site)
        && (security.origin_token.len() < 32
            || http::HeaderValue::from_str(&security.origin_token).is_err())
    {
        bail!("static origin_token must be a valid header value of at least 32 bytes");
    }
    if security
        .allowed_peers
        .iter()
        .any(|rule| parse_peer(rule).is_none())
    {
        bail!("static allowed_peers must contain valid IP addresses or CIDRs");
    }
    if signed_url_enabled(site) {
        if security.signed_url.secret.len() < 32 {
            bail!("static signed_url.secret must contain at least 32 bytes");
        }
        if security.signed_url.max_ttl_secs == 0 || security.signed_url.max_ttl_secs > 86400 {
            bail!("static signed_url.max_ttl_secs must be between 1 and 86400");
        }
        if site.autoindex {
            bail!("static signed_url cannot enable autoindex: directory authorization must not mint child URLs");
        }
    }
    if !(1..=10000).contains(&site.autoindex_max_entries) {
        bail!("static autoindex_max_entries must be between 1 and 10000");
    }
    if http::HeaderValue::from_str(&site.cache_control).is_err() {
        bail!("static cache_control must be a valid header value");
    }
    Ok(())
}

fn mac(site: &StaticSiteConfig, path: &str, expires: u64) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(site.security.signed_url.secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(b"proxysss-static-v1\n");
    mac.update(site.name.as_bytes());
    mac.update(b"\n");
    mac.update(path.as_bytes());
    mac.update(b"\n");
    mac.update(expires.to_string().as_bytes());
    mac
}

pub fn sign(site: &StaticSiteConfig, path: &str, ttl_secs: u64, now: u64) -> Result<String> {
    validate(site)?;
    if !signed_url_enabled(site)
        || ttl_secs == 0
        || ttl_secs > site.security.signed_url.max_ttl_secs
    {
        bail!("signed URLs must be enabled and TTL must be within max_ttl_secs");
    }
    let uri: Uri = path.parse().context("invalid static URL path")?;
    let prefix = site.path_prefix.trim_end_matches('/');
    if !path.starts_with('/')
        || path.starts_with("//")
        || uri.scheme().is_some()
        || uri.authority().is_some()
        || uri.query().is_some()
        || path.contains('#')
        || !(uri.path() == prefix || uri.path().starts_with(&format!("{prefix}/")))
        || !crate::security::request_uri_is_safe(&uri)
    {
        bail!("path must be an encoded absolute path within the selected site, without query or fragment");
    }
    let expires = now.checked_add(ttl_secs).context("expiry overflow")?;
    let signature = URL_SAFE_NO_PAD.encode(mac(site, uri.path(), expires).finalize().into_bytes());
    Ok(format!(
        "{}?expires={expires}&signature={signature}",
        uri.path()
    ))
}

/// Constant-time verification of the token and signature; fail closed on duplicate parameters.
pub fn authorized(site: &StaticSiteConfig, uri: &Uri, headers: &HeaderMap, now: u64) -> bool {
    if token_enabled(site) {
        let mut values = headers.get_all(ORIGIN_TOKEN_HEADER).iter();
        let Some(token) = values.next() else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        // Compare fixed-size MACs rather than an early-exit byte/string comparison.
        let mut expected =
            HmacSha256::new_from_slice(site.security.origin_token.as_bytes()).unwrap();
        expected.update(site.security.origin_token.as_bytes());
        let mut supplied =
            HmacSha256::new_from_slice(site.security.origin_token.as_bytes()).unwrap();
        supplied.update(token.as_bytes());
        if expected
            .verify_slice(&supplied.finalize().into_bytes())
            .is_err()
        {
            return false;
        }
    }
    if !signed_url_enabled(site) {
        return true;
    }
    let Some(query) = uri.query() else {
        return false;
    };
    let mut expires = None;
    let mut signature = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("expires", value)) if expires.is_none() => expires = Some(value),
            Some(("signature", value)) if signature.is_none() => signature = Some(value),
            _ => return false,
        }
    }
    let (Some(expires), Some(signature)) = (expires, signature) else {
        return false;
    };
    let Ok(expiry) = expires.parse::<u64>() else {
        return false;
    };
    if expiry.to_string() != expires
        || expiry <= now
        || expiry - now > site.security.signed_url.max_ttl_secs
    {
        return false;
    }
    let Ok(signature) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    mac(site, uri.path(), expiry)
        .verify_slice(&signature)
        .is_ok()
}

pub fn encode_segment(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            result.push(char::from(byte));
        } else {
            use std::fmt::Write;
            let _ = write!(result, "%{byte:02X}");
        }
    }
    result
}

pub fn allowed_component(name: &str, hide_dotfiles: bool) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if hide_dotfiles && name.starts_with('.') {
        return false;
    }
    !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
}

/// Canonicalize before caching/opening. The configured root is a trusted, operator-owned tree.
pub async fn confined_target(
    root: &std::path::Path,
    target: &std::path::Path,
    hide_dotfiles: bool,
) -> Option<std::path::PathBuf> {
    let root = tokio::fs::canonicalize(root).await.ok()?;
    let target = tokio::fs::canonicalize(target).await.ok()?;
    let relative = target.strip_prefix(&root).ok()?;
    // An innocuous symlink name must not disclose a hidden target in the same root.
    if relative.components().any(|part| {
        part.as_os_str()
            .to_str()
            .is_none_or(|name| !allowed_component(name, hide_dotfiles))
    }) {
        return None;
    }
    Some(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn site() -> StaticSiteConfig {
        let mut site = StaticSiteConfig {
            path_prefix: "/assets".into(),
            ..Default::default()
        };
        site.security.signed_url.enabled = true;
        site.security.signed_url.secret = "test-only-signing-secret-at-least-32-bytes".into();
        site
    }
    #[test]
    fn signed_urls_are_path_site_expiry_bound_and_reject_ambiguity() {
        let site = site();
        let url = sign(&site, "/assets/a%20b.bin", 60, 1000).unwrap();
        assert!(authorized(
            &site,
            &url.parse().unwrap(),
            &HeaderMap::new(),
            1001
        ));
        for invalid in [
            url.replace("a%20b", "other"),
            format!("{url}&expires=1060"),
            format!("{url}&x=1"),
            url.replace("expires=1060", "expires=01060"),
            url.replace("expires=1060", "expires=999999"),
            url.replace("signature=", "signature=A"),
        ] {
            assert!(
                !authorized(&site, &invalid.parse().unwrap(), &HeaderMap::new(), 1001),
                "{invalid}"
            );
        }
        assert!(!authorized(
            &site,
            &url.parse().unwrap(),
            &HeaderMap::new(),
            1060
        ));
        assert!(!authorized(
            &site,
            &url.parse().unwrap(),
            &HeaderMap::new(),
            0
        ));
        let mut other = site.clone();
        other.name = "other".into();
        assert!(!authorized(
            &other,
            &url.parse().unwrap(),
            &HeaderMap::new(),
            1001
        ));
        assert!(sign(&site, "/outside/a", 60, 1000).is_err());
        assert!(sign(&site, "/assets/a?x=1", 60, 1000).is_err());
    }
    #[test]
    fn origin_token_and_signed_url_are_both_required() {
        let mut site = site();
        site.security.origin_token = "origin-token-with-at-least-32-bytes".into();
        let uri = sign(&site, "/assets/a", 60, 1000).unwrap().parse().unwrap();
        let mut headers = HeaderMap::new();
        assert!(!authorized(&site, &uri, &headers, 1001));
        headers.insert(
            ORIGIN_TOKEN_HEADER,
            site.security.origin_token.parse().unwrap(),
        );
        assert!(authorized(&site, &uri, &headers, 1001));
        headers.append(ORIGIN_TOKEN_HEADER, "duplicate".parse().unwrap());
        assert!(!authorized(&site, &uri, &headers, 1001));
    }
    #[test]
    fn peer_rules_are_bounded_and_use_actual_peer() {
        for (rule, yes, no) in [
            ("192.0.2.0/24", "192.0.2.1", "192.0.3.1"),
            ("127.0.0.1", "127.0.0.1", "127.0.0.2"),
            ("2001:db8::/32", "2001:db8::1", "2001:db9::1"),
        ] {
            assert!(peer_allowed(yes.parse().unwrap(), &[rule.into()]));
            assert!(!peer_allowed(no.parse().unwrap(), &[rule.into()]));
        }
        assert!(peer_allowed(
            "::ffff:127.0.0.1".parse().unwrap(),
            &["127.0.0.1".into()]
        ));
        assert!(parse_peer("127.0.0.1/129").is_none());
        assert!(parse_peer("::1/129").is_none());
    }
    #[test]
    fn path_components_and_links_cannot_escape_or_inject() {
        for name in [".env", "..", "a\\b", "a:b", "a/../b", "a.", "a ", "a\0"] {
            assert!(!allowed_component(name, true), "{name}");
        }
        assert_eq!(encode_segment("a #?&中.txt"), "a%20%23%3F%26%E4%B8%AD.txt");
    }

    #[test]
    fn independent_switches_keep_rules_and_secrets_while_disabling_enforcement() {
        let mut site = StaticSiteConfig::default();
        site.security.origin_token = "test-only-origin-token-with-at-least-32-bytes".into();
        site.security.allowed_peers = vec!["192.0.2.0/24".into()];
        let uri = "/public/a".parse().unwrap();
        assert!(!authorized(&site, &uri, &HeaderMap::new(), 1));
        assert!(!site_peer_allowed(&site, "127.0.0.1".parse().unwrap()));
        site.security.origin_token_enabled = false;
        assert!(authorized(&site, &uri, &HeaderMap::new(), 1));
        assert!(!site_peer_allowed(&site, "127.0.0.1".parse().unwrap()));
        site.security.allowed_peers_enabled = false;
        assert!(site_peer_allowed(&site, "127.0.0.1".parse().unwrap()));
        site.security.signed_url.enabled = true;
        site.security.signed_url.secret = "test-only-signing-key-with-at-least-32-bytes".into();
        assert!(!authorized(&site, &uri, &HeaderMap::new(), 1));
        site.security.enabled = false;
        assert!(authorized(&site, &uri, &HeaderMap::new(), 1));
        assert!(!requires_authorization(&site));
        assert!(sign(&site, "/public/a", 30, 1).is_err());
        assert!(!site.security.origin_token.is_empty());
        assert_eq!(site.security.allowed_peers.len(), 1);
    }
}
