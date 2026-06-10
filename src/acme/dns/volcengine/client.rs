use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::common::VolcengineSigV4;
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, optional_credential, split_fqdn};

const HOST: &str = "dns.volcengineapi.com";
const VERSION: &str = "2018-08-01";

pub struct VolcengineDns {
    client: Client,
    signer: VolcengineSigV4,
}

impl VolcengineDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        let region = optional_credential(credentials, &["region"])
            .unwrap_or_else(|| "cn-beijing".to_string());
        Ok(Self {
            client: Client::new(),
            signer: VolcengineSigV4::new(
                credential(
                    credentials,
                    &["access_key_id", "VOLCENGINE_ACCESS_KEY", "VOLC_ACCESSKEY"],
                )?,
                credential(
                    credentials,
                    &[
                        "secret_access_key",
                        "VOLCENGINE_SECRET_KEY",
                        "VOLC_SECRETKEY",
                    ],
                )?,
                region,
            ),
        })
    }

    async fn signed_get(&self, action: &str, params: &[(&str, String)]) -> Result<String> {
        let mut query = format!("Action={action}&Version={VERSION}");
        for (key, value) in params {
            query.push('&');
            query.push_str(key);
            query.push('=');
            query.push_str(&urlencoding(value));
        }
        let signed = self.signer.sign_get(HOST, &query)?;
        let url = format!("https://{HOST}/?{query}");
        self.client
            .get(url)
            .header("Authorization", signed.authorization)
            .header("X-Date", signed.x_date)
            .header("Host", HOST)
            .send()
            .await
            .with_context(|| format!("volcengine {action} get request failed"))?
            .error_for_status()
            .with_context(|| format!("volcengine {action} get returned error status"))?
            .text()
            .await
            .with_context(|| format!("volcengine {action} get response decode failed"))
    }

    async fn signed_post(&self, action: &str, body: &serde_json::Value) -> Result<String> {
        let query = format!("Action={action}&Version={VERSION}");
        let payload = serde_json::to_vec(body).context("volcengine request body encode failed")?;
        let signed = self.signer.sign_post(HOST, &query, &payload)?;
        let url = format!("https://{HOST}/?{query}");
        let mut request = self
            .client
            .post(url)
            .header("Authorization", signed.authorization)
            .header("X-Date", signed.x_date)
            .header("Host", HOST);
        if let Some(content_sha256) = signed.content_sha256 {
            request = request.header("X-Content-Sha256", content_sha256);
        }
        if let Some(content_type) = signed.content_type {
            request = request.header("Content-Type", content_type);
        }
        request
            .body(payload)
            .send()
            .await
            .with_context(|| format!("volcengine {action} post request failed"))?
            .error_for_status()
            .with_context(|| format!("volcengine {action} post returned error status"))?
            .text()
            .await
            .with_context(|| format!("volcengine {action} post response decode failed"))
    }

    async fn resolve_zid(&self, domain: &str) -> Result<String> {
        let body = self
            .signed_get("ListDomain", &[("Domain", domain.to_string())])
            .await?;
        let parsed: VolcengineResponse<ListDomainResult> =
            serde_json::from_str(&body).context("volcengine ListDomain response decode failed")?;
        parsed
            .result
            .and_then(|result| result.domain_list.into_iter().next())
            .map(|domain| domain.zid.to_string())
            .ok_or_else(|| anyhow!("volcengine domain not found for {domain}"))
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (domain_name, host) = split_fqdn(fqdn);
        let host = if host.is_empty() {
            "_acme-challenge".to_string()
        } else {
            host
        };
        let zid = self.resolve_zid(&domain_name).await?;
        self.delete_existing_txt(&zid, &host).await?;

        let body = serde_json::json!({
            "ZID": zid.parse::<u64>().unwrap_or(0),
            "Host": host,
            "Type": "TXT",
            "Value": value,
            "TTL": 600,
            "Line": "default"
        });
        let response_text = self.signed_post("CreateRecord", &body).await?;
        let parsed: VolcengineResponse<CreateRecordResult> =
            serde_json::from_str(&response_text)
                .context("volcengine CreateRecord response decode failed")?;
        let record_id = parsed
            .result
            .and_then(|result| result.record_id)
            .ok_or_else(|| anyhow!("volcengine CreateRecord returned empty record id"))?;

        Ok(DnsRecordHandle {
            provider: "volcengine".to_string(),
            record_id: record_id.to_string(),
            zone: zid,
            name: host,
        })
    }

    async fn delete_existing_txt(&self, zid: &str, host: &str) -> Result<()> {
        let body = self
            .signed_get(
                "ListRecords",
                &[
                    ("ZID", zid.to_string()),
                    ("Host", host.to_string()),
                    ("Type", "TXT".to_string()),
                ],
            )
            .await?;
        let parsed: VolcengineResponse<ListRecordsResult> =
            serde_json::from_str(&body).context("volcengine ListRecords response decode failed")?;
        if let Some(records) = parsed.result.map(|result| result.record_list) {
            for record in records {
                self.delete_txt_record(&DnsRecordHandle {
                    provider: "volcengine".to_string(),
                    record_id: record.record_id.to_string(),
                    zone: zid.to_string(),
                    name: host.to_string(),
                })
                .await?;
            }
        }
        Ok(())
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let body = serde_json::json!({
            "RecordID": handle.record_id.parse::<u64>().unwrap_or(0),
            "ZID": handle.zone.parse::<u64>().unwrap_or(0),
        });
        self.signed_post("DeleteRecord", &body).await?;
        Ok(())
    }
}

fn urlencoding(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[derive(Debug, Deserialize)]
struct VolcengineResponse<T> {
    #[serde(rename = "Result")]
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct ListDomainResult {
    #[serde(rename = "DomainList", default)]
    domain_list: Vec<VolcengineDomain>,
}

#[derive(Debug, Deserialize)]
struct VolcengineDomain {
    #[serde(rename = "ZID")]
    zid: u64,
}

#[derive(Debug, Deserialize)]
struct CreateRecordResult {
    #[serde(rename = "RecordID")]
    record_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ListRecordsResult {
    #[serde(rename = "RecordList", default)]
    record_list: Vec<VolcengineRecord>,
}

#[derive(Debug, Deserialize)]
struct VolcengineRecord {
    #[serde(rename = "RecordID")]
    record_id: u64,
}
