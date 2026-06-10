use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;

use crate::acme::dns::common::AzureTokenProvider;
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, split_fqdn};

const API_VERSION: &str = "2018-05-01";

pub struct AzureDns {
    client: Client,
    token_provider: AzureTokenProvider,
    subscription_id: String,
    resource_group: String,
}

impl AzureDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            token_provider: AzureTokenProvider::new(
                credential(credentials, &["tenant_id", "AZURE_TENANT_ID"])?,
                credential(credentials, &["client_id", "AZURE_CLIENT_ID"])?,
                credential(credentials, &["client_secret", "AZURE_CLIENT_SECRET"])?,
            ),
            subscription_id: credential(
                credentials,
                &["subscription_id", "AZURE_SUBSCRIPTION_ID"],
            )?,
            resource_group: credential(credentials, &["resource_group", "AZURE_RESOURCE_GROUP"])?,
        })
    }

    fn record_set_url(&self, zone_name: &str, relative_name: &str) -> String {
        format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.Network/dnsZones/{}/TXT/{}?api-version={}",
            self.subscription_id, self.resource_group, zone_name, relative_name, API_VERSION
        )
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (zone_name, host) = split_fqdn(fqdn);
        let relative_name = if host.is_empty() {
            "_acme-challenge".to_string()
        } else {
            host
        };

        let token = self.token_provider.access_token().await?;
        let body = serde_json::json!({
            "properties": {
                "TTL": 300,
                "TXTRecords": [
                    { "value": split_azure_txt_value(value) }
                ]
            }
        });

        let response = self
            .client
            .put(self.record_set_url(&zone_name, &relative_name))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .context("azure dns upsert txt request failed")?
            .error_for_status()
            .context("azure dns upsert txt returned error status")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("azure dns upsert txt failed ({status}): {text}"));
        }

        Ok(DnsRecordHandle {
            provider: "azure".to_string(),
            record_id: relative_name.clone(),
            zone: zone_name,
            name: relative_name,
        })
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let token = self.token_provider.access_token().await?;
        let response = self
            .client
            .delete(self.record_set_url(&handle.zone, &handle.record_id))
            .bearer_auth(token)
            .send()
            .await
            .context("azure dns delete txt request failed")?;

        if response.status().is_success() || response.status().as_u16() == 404 {
            return Ok(());
        }

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(anyhow!("azure dns delete txt failed ({status}): {text}"))
    }
}

fn split_azure_txt_value(value: &str) -> Vec<String> {
    if value.len() <= 255 {
        return vec![value.to_string()];
    }
    value
        .as_bytes()
        .chunks(255)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}
