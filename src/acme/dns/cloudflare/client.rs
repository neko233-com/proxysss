use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, split_fqdn};

const API_BASE: &str = "https://api.cloudflare.com/client/v4";

pub struct CloudflareDns {
    client: Client,
    auth_header: String,
}

impl CloudflareDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        let auth_header = if let Ok(token) = credential(credentials, &["api_token", "CF_Token"]) {
            format!("Bearer {token}")
        } else {
            let api_key = credential(credentials, &["api_key", "CF_Key"])?;
            let email = credential(credentials, &["email", "CF_Email"])?;
            format!("X-Auth-Email: {email}; X-Auth-Key: {api_key}")
        };

        Ok(Self {
            client: Client::new(),
            auth_header,
        })
    }

    fn apply_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.auth_header.starts_with("Bearer ") {
            request.header("Authorization", &self.auth_header)
        } else {
            let mut email = String::new();
            let mut key = String::new();
            for part in self.auth_header.split(';') {
                let part = part.trim();
                if let Some(value) = part.strip_prefix("X-Auth-Email: ") {
                    email = value.to_string();
                } else if let Some(value) = part.strip_prefix("X-Auth-Key: ") {
                    key = value.to_string();
                }
            }
            request
                .header("X-Auth-Email", email)
                .header("X-Auth-Key", key)
        }
    }

    async fn zone_id(&self, zone_name: &str) -> Result<String> {
        let response = self
            .apply_auth(
                self.client
                    .get(format!("{API_BASE}/zones"))
                    .query(&[("name", zone_name), ("status", "active")]),
            )
            .send()
            .await
            .context("cloudflare list zones request failed")?
            .error_for_status()
            .context("cloudflare list zones returned error status")?
            .json::<CloudflareResponse<Vec<CloudflareZone>>>()
            .await
            .context("cloudflare list zones response decode failed")?;

        if !response.success {
            return Err(anyhow!(
                "cloudflare list zones failed: {}",
                response.errors_message()
            ));
        }

        response
            .result
            .and_then(|zones| zones.into_iter().next())
            .map(|zone| zone.id)
            .ok_or_else(|| {
                anyhow!(
                    "cloudflare zone not found for {zone_name}; ensure the token can manage that zone"
                )
            })
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (zone_name, host) = split_fqdn(fqdn);
        let zone_id = self.zone_id(&zone_name).await?;
        let record_name = if host.is_empty() {
            fqdn.trim_end_matches('.').to_string()
        } else {
            format!("{host}.{zone_name}")
        };

        self.delete_existing_txt(&zone_id, &record_name).await?;

        let body = serde_json::json!({
            "type": "TXT",
            "name": record_name,
            "content": value,
            "ttl": 120,
        });

        let response = self
            .apply_auth(
                self.client
                    .post(format!("{API_BASE}/zones/{zone_id}/dns_records"))
                    .json(&body),
            )
            .send()
            .await
            .context("cloudflare create txt record request failed")?
            .error_for_status()
            .context("cloudflare create txt record returned error status")?
            .json::<CloudflareResponse<CloudflareRecord>>()
            .await
            .context("cloudflare create txt record response decode failed")?;

        if !response.success {
            return Err(anyhow!(
                "cloudflare create txt record failed: {}",
                response.errors_message()
            ));
        }

        let record = response
            .result
            .ok_or_else(|| anyhow!("cloudflare create txt record returned empty result"))?;

        Ok(DnsRecordHandle {
            provider: "cloudflare".to_string(),
            record_id: record.id,
            zone: zone_id,
            name: record_name,
        })
    }

    async fn delete_existing_txt(&self, zone_id: &str, record_name: &str) -> Result<()> {
        let response = self
            .apply_auth(
                self.client
                    .get(format!("{API_BASE}/zones/{zone_id}/dns_records"))
                    .query(&[("type", "TXT"), ("name", record_name)]),
            )
            .send()
            .await
            .context("cloudflare list txt records request failed")?
            .error_for_status()
            .context("cloudflare list txt records returned error status")?
            .json::<CloudflareResponse<Vec<CloudflareRecord>>>()
            .await
            .context("cloudflare list txt records response decode failed")?;

        if !response.success {
            return Err(anyhow!(
                "cloudflare list txt records failed: {}",
                response.errors_message()
            ));
        }

        if let Some(records) = response.result {
            for record in records {
                self.delete_txt_record(&DnsRecordHandle {
                    provider: "cloudflare".to_string(),
                    record_id: record.id,
                    zone: zone_id.to_string(),
                    name: record_name.to_string(),
                })
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let response = self
            .apply_auth(self.client.delete(format!(
                "{API_BASE}/zones/{}/dns_records/{}",
                handle.zone, handle.record_id
            )))
            .send()
            .await
            .context("cloudflare delete txt record request failed")?
            .error_for_status()
            .context("cloudflare delete txt record returned error status")?
            .json::<CloudflareResponse<CloudflareRecord>>()
            .await
            .context("cloudflare delete txt record response decode failed")?;

        if !response.success {
            return Err(anyhow!(
                "cloudflare delete txt record failed: {}",
                response.errors_message()
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    result: Option<T>,
    errors: Vec<CloudflareError>,
}

impl<T> CloudflareResponse<T> {
    fn errors_message(&self) -> String {
        if self.errors.is_empty() {
            "unknown cloudflare error".to_string()
        } else {
            self.errors
                .iter()
                .map(|error| error.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        }
    }
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareZone {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareRecord {
    id: String,
}
