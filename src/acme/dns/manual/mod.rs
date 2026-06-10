use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::optional_credential;

const DEFAULT_TIMEOUT_SECS: u64 = 900;
const DEFAULT_POLL_INTERVAL_SECS: u64 = 15;

pub struct ManualDns {
    client: Client,
    timeout: Duration,
    poll_interval: Duration,
}

impl ManualDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        let timeout_secs = optional_credential(credentials, &["timeout_secs"])
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .max(60);
        let poll_interval_secs = optional_credential(credentials, &["poll_interval_secs"])
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECS)
            .max(5);
        Ok(Self {
            client: Client::new(),
            timeout: Duration::from_secs(timeout_secs),
            poll_interval: Duration::from_secs(poll_interval_secs),
        })
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        warn!(
            fqdn,
            txt = value,
            timeout_secs = self.timeout.as_secs(),
            "manual DNS-01: add this TXT record at your DNS provider; proxysss will poll public DNS until it appears"
        );
        info!("manual DNS-01\n  Name: {fqdn}\n  Type: TXT\n  Value: \"{value}\"");

        let deadline = Instant::now() + self.timeout;
        while Instant::now() < deadline {
            if txt_record_visible(&self.client, fqdn, value).await? {
                info!(fqdn, "manual DNS-01 TXT record detected");
                return Ok(DnsRecordHandle {
                    provider: "manual".to_string(),
                    zone: String::new(),
                    record_id: fqdn.to_string(),
                    name: fqdn.to_string(),
                });
            }
            tokio::time::sleep(self.poll_interval).await;
        }

        Err(anyhow!(
            "timed out waiting for manual DNS-01 TXT record {fqdn}; add the TXT record and retry"
        ))
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        info!(
            name = %handle.name,
            "manual DNS-01 complete; you may remove the TXT record from your DNS provider"
        );
        Ok(())
    }
}

async fn txt_record_visible(client: &Client, fqdn: &str, expected: &str) -> Result<bool> {
    let encoded = urlencoding(fqdn.trim_end_matches('.'));
    let url = format!("https://cloudflare-dns.com/dns-query?name={encoded}&type=TXT");
    let response = client
        .get(url)
        .header("Accept", "application/dns-json")
        .send()
        .await
        .context("manual dns-01 lookup request failed")?
        .error_for_status()
        .context("manual dns-01 lookup returned error status")?;
    let payload: DohResponse = response
        .json()
        .await
        .context("manual dns-01 lookup response decode failed")?;
    let needle = expected.trim_matches('"');
    Ok(payload
        .answer
        .into_iter()
        .any(|record| record.data.trim_matches('"').eq_ignore_ascii_case(needle)))
}

fn urlencoding(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct DohResponse {
    #[serde(default)]
    answer: Vec<DohAnswer>,
}

#[derive(Debug, Deserialize)]
struct DohAnswer {
    #[serde(default)]
    data: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_dns_accepts_empty_credentials() {
        ManualDns::new(&BTreeMap::new()).expect("manual dns provider");
    }
}
