//! nginx `auth_basic` parity for generic HTTP routes (static / reverse / domain).
//! WebDAV/FileCloud keep their own Basic surface; this module serves the shared
//! route-level policy. Verification is constant-time and fails closed.
use base64::{engine::general_purpose::STANDARD, Engine};
use http::{HeaderMap, HeaderValue};

use crate::config::HttpBasicAuthConfig;

/// Returns the `WWW-Authenticate` challenge when the request must be rejected,
/// or `None` when access is granted (policy disabled or credentials match).
pub fn challenge(cfg: &HttpBasicAuthConfig, headers: &HeaderMap) -> Option<HeaderValue> {
    if !cfg.enabled {
        return None;
    }
    let challenge = HeaderValue::from_str(&format!(
        "Basic realm=\"{}\", charset=\"UTF-8\"",
        cfg.realm.replace('"', "")
    ))
    .ok()?;
    // Fail closed when enabled but misconfigured.
    if cfg.username.is_empty() || cfg.password.is_empty() {
        return Some(challenge);
    }
    let Some(value) = headers.get(http::header::AUTHORIZATION) else {
        return Some(challenge);
    };
    let Ok(value) = value.to_str() else {
        return Some(challenge);
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return Some(challenge);
    };
    let Ok(decoded) = STANDARD.decode(encoded.trim()) else {
        return Some(challenge);
    };
    let Ok(decoded) = String::from_utf8(decoded) else {
        return Some(challenge);
    };
    let Some((user, pass)) = decoded.split_once(':') else {
        return Some(challenge);
    };
    if constant_time_eq(user.as_bytes(), cfg.username.as_bytes())
        && constant_time_eq(pass.as_bytes(), cfg.password.as_bytes())
    {
        None
    } else {
        Some(challenge)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> HttpBasicAuthConfig {
        HttpBasicAuthConfig {
            enabled: true,
            username: "admin".into(),
            password: "s3cret".into(),
            realm: "proxysss".into(),
        }
    }

    fn auth(user: &str, pass: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        let token = STANDARD.encode(format!("{user}:{pass}"));
        h.insert(
            http::header::AUTHORIZATION,
            format!("Basic {token}").parse().unwrap(),
        );
        h
    }

    #[test]
    fn disabled_grants_and_missing_credentials_challenge() {
        let off = HttpBasicAuthConfig::default();
        assert!(challenge(&off, &HeaderMap::new()).is_none());
        let c = cfg();
        assert!(challenge(&c, &HeaderMap::new()).is_some());
        assert!(challenge(&c, &auth("admin", "wrong")).is_some());
        assert!(challenge(&c, &auth("other", "s3cret")).is_some());
        assert!(challenge(&c, &auth("admin", "s3cret")).is_none());
    }

    #[test]
    fn malformed_and_misconfigured_fail_closed() {
        let c = cfg();
        let mut h = HeaderMap::new();
        h.insert(http::header::AUTHORIZATION, "Bearer x".parse().unwrap());
        assert!(challenge(&c, &h).is_some());
        h.insert(http::header::AUTHORIZATION, "Basic !!!".parse().unwrap());
        assert!(challenge(&c, &h).is_some());
        let mut bad = cfg();
        bad.password = String::new();
        assert!(challenge(&bad, &auth("admin", "")).is_some());
    }
}