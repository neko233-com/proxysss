use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

/// Resolve explicit environment references only at provider construction time.
/// The configuration continues to contain the reference rather than the secret.
pub fn resolve_credentials(
    credentials: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    credentials
        .iter()
        .map(|(key, value)| {
            let Some(name) = value.strip_prefix("env:") else {
                return Ok((key.clone(), value.clone()));
            };
            if name.is_empty()
                || !name.chars().enumerate().all(|(index, c)| {
                    c == '_' || c.is_ascii_alphabetic() || (index > 0 && c.is_ascii_digit())
                })
            {
                return Err(anyhow!(
                    "invalid DNS credential environment variable name for {key}"
                ));
            }
            let secret = std::env::var(name).map_err(|_| {
                anyhow!("DNS credential environment variable {name} is unavailable")
            })?;
            if secret.trim().is_empty() {
                return Err(anyhow!(
                    "DNS credential environment variable {name} is empty"
                ));
            }
            Ok((key.clone(), secret))
        })
        .collect()
}

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
    fn credential_environment_references_are_explicit_and_checked() {
        let credentials = BTreeMap::from([
            ("region".to_owned(), "cn-beijing".to_owned()),
            ("key".to_owned(), "env:PATH".to_owned()),
        ]);
        let resolved = resolve_credentials(&credentials).unwrap();
        assert_eq!(resolved["region"], "cn-beijing");
        assert_eq!(resolved["key"], std::env::var("PATH").unwrap());
        assert_eq!(credentials["key"], "env:PATH");
        for reference in ["env:", "env:INVALID-NAME", "env:9INVALID"] {
            assert!(
                resolve_credentials(&BTreeMap::from([("key".into(), reference.into())])).is_err()
            );
        }
        assert!(resolve_credentials(&BTreeMap::from([(
            "key".into(),
            "env:PROXYSSS_TEST_MISSING_DNS_SECRET_832715".into()
        )]))
        .is_err());
    }

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
