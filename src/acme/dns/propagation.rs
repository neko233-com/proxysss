//! Check all DNS-01 values before asking the CA to validate an order.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

fn contains_txt(payload: &Value, expected: &str) -> bool {
    payload.get("Status").and_then(Value::as_i64) == Some(0)
        && payload
            .get("Answer")
            .and_then(Value::as_array)
            .is_some_and(|answers| {
                answers.iter().any(|entry| {
                    entry.get("type").and_then(Value::as_i64) == Some(16)
                        && entry
                            .get("data")
                            .and_then(Value::as_str)
                            .is_some_and(|value| value.trim_matches('"') == expected)
                })
            })
}

pub async fn wait_for_txt(records: &[(String, String)]) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to initialize DNS propagation checker")?;
    // Some DNS plans enforce a 600-second TXT TTL; allow cached prior
    // challenge values to expire before declaring the new order failed.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(900);
    let mut reported_wait = false;
    loop {
        let mut ready = true;
        for (name, value) in records {
            let mut visible = false;
            for endpoint in [
                "https://dns.alidns.com/resolve",
                "https://cloudflare-dns.com/dns-query",
            ] {
                let result = client
                    .get(endpoint)
                    .query(&[("name", name.trim_end_matches('.')), ("type", "TXT")])
                    .header("Accept", "application/dns-json")
                    .send()
                    .await;
                if let Ok(mut response) = result {
                    if response.status().is_success() {
                        let mut bytes = Vec::new();
                        let mut complete = true;
                        loop {
                            match response.chunk().await {
                                Ok(Some(chunk)) if bytes.len() + chunk.len() <= 65536 => {
                                    bytes.extend_from_slice(&chunk);
                                }
                                Ok(None) => break,
                                _ => {
                                    complete = false;
                                    break;
                                }
                            }
                        }
                        if complete {
                            if let Ok(json) = serde_json::from_slice::<Value>(&bytes) {
                                if contains_txt(&json, value) {
                                    visible = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !visible {
                if !reported_wait {
                    tracing::info!(record = %name, "waiting for DNS-01 TXT to become visible in public DNS");
                    reported_wait = true;
                }
                ready = false;
                break;
            }
        }
        if ready {
            tracing::info!(
                records = records.len(),
                "all DNS-01 TXT values are visible in public DNS"
            );
            // Give other public resolver caches a short additional interval.
            tokio::time::sleep(Duration::from_secs(10)).await;
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "DNS-01 TXT records did not become visible in public DNS within 900 seconds"
            ));
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_exact_txt_without_accepting_other_types_or_case_changes() {
        let response = json!({"Status": 0, "Answer": [
            {"type": 1, "data": "AbC"}, {"type": 16, "data": "\"AbC\""}
        ]});
        assert!(contains_txt(&response, "AbC"));
        assert!(!contains_txt(&response, "abc"));
        assert!(!contains_txt(&json!({"Status": 3}), "AbC"));
    }
}
