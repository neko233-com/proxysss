use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const SESSION_COOKIE: &str = "filecloud_session";

pub fn derive_secret(password: &str, root: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b":");
    hasher.update(root.as_bytes());
    hasher.finalize().into()
}

pub fn issue_session(secret: &[u8; 32], ttl_secs: u64) -> String {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + ttl_secs.max(300);
    let sig = sign_exp(secret, exp);
    format!("{exp}.{sig}")
}

pub fn verify_session(secret: &[u8; 32], token: &str) -> bool {
    let Some((exp_raw, sig)) = token.split_once('.') else {
        return false;
    };
    let Ok(exp) = exp_raw.parse::<u64>() else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if exp < now {
        return false;
    }
    let expected = sign_exp(secret, exp);
    constant_time_eq(sig.as_bytes(), expected.as_bytes())
}

fn sign_exp(secret: &[u8; 32], exp: u64) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(exp.to_string().as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn password_matches(configured: &str, provided: &str) -> bool {
    constant_time_eq(configured.as_bytes(), provided.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_roundtrip() {
        let secret = derive_secret("secret", "/data");
        let token = issue_session(&secret, 3600);
        assert!(verify_session(&secret, &token));
        assert!(!verify_session(&secret, "bad.token"));
    }
}
