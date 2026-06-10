use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

pub fn credential(credentials: &BTreeMap<String, String>, aliases: &[&str]) -> Result<String> {
    for alias in aliases {
        if let Some(value) = credentials
            .get(*alias)
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(value.trim().to_string());
        }
    }
    Err(anyhow!(
        "missing DNS credential (expected one of: {})",
        aliases.join(", ")
    ))
}

pub fn optional_credential(
    credentials: &BTreeMap<String, String>,
    aliases: &[&str],
) -> Option<String> {
    credential(credentials, aliases).ok()
}

pub fn acme_challenge_fqdn(domain: &str) -> String {
    let normalized = domain.trim().trim_start_matches("*.").trim_end_matches('.');
    format!("_acme-challenge.{normalized}")
}

pub fn split_fqdn(fqdn: &str) -> (String, String) {
    let fqdn = fqdn.trim().trim_end_matches('.');
    let labels: Vec<&str> = fqdn.split('.').collect();
    if labels.len() < 2 {
        return (fqdn.to_string(), String::new());
    }
    let zone = labels[labels.len() - 2..].join(".");
    let host = labels[..labels.len() - 2].join(".");
    (zone, host)
}

pub fn fqdn_with_trailing_dot(name: &str) -> String {
    let name = name.trim().trim_end_matches('.');
    format!("{name}.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acme_challenge_fqdn_strips_wildcard_prefix() {
        assert_eq!(
            acme_challenge_fqdn("*.example.com"),
            "_acme-challenge.example.com"
        );
    }

    #[test]
    fn split_fqdn_extracts_zone_and_host() {
        assert_eq!(
            split_fqdn("_acme-challenge.example.com"),
            ("example.com".to_string(), "_acme-challenge".to_string())
        );
    }
}
