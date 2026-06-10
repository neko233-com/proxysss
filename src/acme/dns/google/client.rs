use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::common::GoogleTokenProvider;
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, fqdn_with_trailing_dot, optional_credential};

const API_BASE: &str = "https://dns.googleapis.com/dns/v1";

pub struct GoogleCloudDns {
    client: Client,
    token_provider: GoogleTokenProvider,
    project_id: String,
    managed_zone: Option<String>,
}

impl GoogleCloudDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        let service_account_json = credential(
            credentials,
            &["service_account_json", "GOOGLE_SERVICE_ACCOUNT_JSON"],
        )?;
        Ok(Self {
            client: Client::new(),
            token_provider: GoogleTokenProvider::from_service_account_json(&service_account_json)?,
            project_id: optional_credential(credentials, &["project_id", "GOOGLE_PROJECT_ID"])
                .or_else(|| {
                    serde_json::from_str::<serde_json::Value>(&service_account_json)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("project_id")
                                .and_then(|item| item.as_str())
                                .map(str::to_string)
                        })
                })
                .ok_or_else(|| {
                    anyhow!("google dns requires project_id in credentials or service account json")
                })?,
            managed_zone: optional_credential(
                credentials,
                &["managed_zone", "GOOGLE_MANAGED_ZONE"],
            ),
        })
    }

    async fn access_token(&self) -> Result<String> {
        self.token_provider.access_token().await
    }

    async fn resolve_managed_zone(&self, zone_name: &str) -> Result<String> {
        if let Some(zone) = &self.managed_zone {
            return Ok(zone.clone());
        }

        let token = self.access_token().await?;
        let url = format!("{API_BASE}/projects/{}/managedZones", self.project_id);
        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .context("google dns list managed zones request failed")?
            .error_for_status()
            .context("google dns list managed zones returned error status")?
            .json::<ManagedZoneList>()
            .await
            .context("google dns list managed zones response decode failed")?;

        response
            .managed_zones
            .into_iter()
            .find(|zone| zone.dns_name.trim_end_matches('.') == zone_name)
            .map(|zone| zone.name)
            .ok_or_else(|| anyhow!("google cloud managed zone not found for {zone_name}"))
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let zone_name = fqdn
            .trim()
            .trim_end_matches('.')
            .split('.')
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(".");
        let managed_zone = self.resolve_managed_zone(&zone_name).await?;
        let record_name = fqdn_with_trailing_dot(fqdn);
        let token = self.access_token().await?;

        let existing = self
            .list_txt_record(&managed_zone, &record_name, &token)
            .await?;

        let mut change = serde_json::json!({ "additions": [build_txt_rrset(&record_name, value)] });
        if let Some(existing) = existing {
            change["deletions"] = serde_json::json!([existing]);
        }

        let url = format!(
            "{API_BASE}/projects/{}/managedZones/{}/changes",
            self.project_id, managed_zone
        );
        self.client
            .post(url)
            .bearer_auth(token)
            .json(&change)
            .send()
            .await
            .context("google dns create change request failed")?
            .error_for_status()
            .context("google dns create change returned error status")?;

        Ok(DnsRecordHandle {
            provider: "google".to_string(),
            record_id: record_name,
            zone: managed_zone,
            name: zone_name,
        })
    }

    async fn list_txt_record(
        &self,
        managed_zone: &str,
        record_name: &str,
        token: &str,
    ) -> Result<Option<serde_json::Value>> {
        let url = format!(
            "{API_BASE}/projects/{}/managedZones/{}/rrsets",
            self.project_id, managed_zone
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .query(&[("name", record_name), ("type", "TXT")])
            .send()
            .await
            .context("google dns list rrsets request failed")?
            .error_for_status()
            .context("google dns list rrsets returned error status")?
            .json::<ResourceRecordSetList>()
            .await
            .context("google dns list rrsets response decode failed")?;

        Ok(response.rrsets.into_iter().next().map(|record| {
            serde_json::json!({
                "kind": "dns#resourceRecordSet",
                "name": record.name,
                "type": record.r#type,
                "ttl": record.ttl,
                "rrdatas": record.rrdatas,
            })
        }))
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let token = self.access_token().await?;
        let existing = self
            .list_txt_record(&handle.zone, &handle.record_id, &token)
            .await?;
        let Some(existing) = existing else {
            return Ok(());
        };

        let change = serde_json::json!({ "deletions": [existing] });
        let url = format!(
            "{API_BASE}/projects/{}/managedZones/{}/changes",
            self.project_id, handle.zone
        );
        self.client
            .post(url)
            .bearer_auth(token)
            .json(&change)
            .send()
            .await
            .context("google dns delete change request failed")?
            .error_for_status()
            .context("google dns delete change returned error status")?;
        Ok(())
    }
}

fn build_txt_rrset(record_name: &str, value: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "dns#resourceRecordSet",
        "name": record_name,
        "type": "TXT",
        "ttl": 300,
        "rrdatas": [format!("\"{value}\"")],
    })
}

#[derive(Debug, Deserialize)]
struct ManagedZoneList {
    #[serde(default, rename = "managedZones")]
    managed_zones: Vec<ManagedZone>,
}

#[derive(Debug, Deserialize)]
struct ManagedZone {
    name: String,
    #[serde(rename = "dnsName")]
    dns_name: String,
}

#[derive(Debug, Deserialize)]
struct ResourceRecordSetList {
    #[serde(default, rename = "rrsets")]
    rrsets: Vec<ResourceRecordSet>,
}

#[derive(Debug, Deserialize)]
struct ResourceRecordSet {
    name: String,
    #[serde(rename = "type")]
    r#type: String,
    ttl: u64,
    rrdatas: Vec<String>,
}
