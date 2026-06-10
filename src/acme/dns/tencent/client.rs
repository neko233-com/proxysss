use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, split_fqdn};

const API_BASE: &str = "https://dnsapi.cn";

pub struct TencentDns {
    client: Client,
    login_token: String,
}

impl TencentDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        if let Ok(token) = credential(credentials, &["login_token", "DP_Login_Token"]) {
            return Ok(Self {
                client: Client::new(),
                login_token: token,
            });
        }

        let api_token = credential(credentials, &["api_token", "DP_Id", "SecretId"])?;
        let api_secret = credential(credentials, &["api_secret", "DP_Key", "SecretKey"])?;
        let digest = md5::compute(format!("{api_token},{api_secret}"));
        Ok(Self {
            client: Client::new(),
            login_token: format!("{api_token},{digest:x}"),
        })
    }

    async fn call(
        &self,
        action: &str,
        mut params: BTreeMap<String, String>,
    ) -> Result<TencentResponse> {
        params.insert("login_token".to_string(), self.login_token.clone());
        params.insert("format".to_string(), "json".to_string());
        params.insert("lang".to_string(), "en".to_string());

        let response = self
            .client
            .post(format!("{API_BASE}/{action}"))
            .form(&params)
            .send()
            .await
            .with_context(|| format!("tencent {action} request failed"))?
            .error_for_status()
            .with_context(|| format!("tencent {action} returned error status"))?
            .json::<TencentResponse>()
            .await
            .with_context(|| format!("tencent {action} response decode failed"))?;

        if response.status.code != "1" {
            return Err(anyhow!(
                "tencent {action} failed: {} ({})",
                response.status.message,
                response.status.code
            ));
        }

        Ok(response)
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (domain, sub_domain) = split_fqdn(fqdn);
        let sub_domain = if sub_domain.is_empty() {
            "_acme-challenge".to_string()
        } else {
            sub_domain
        };

        self.delete_existing_txt(&domain, &sub_domain).await?;

        let mut params = BTreeMap::new();
        params.insert("domain".to_string(), domain.clone());
        params.insert("sub_domain".to_string(), sub_domain.clone());
        params.insert("record_type".to_string(), "TXT".to_string());
        params.insert("record_line".to_string(), "默认".to_string());
        params.insert("value".to_string(), value.to_string());

        let response = self.call("Record.Create", params).await?;
        let record_id = response
            .record
            .and_then(|record| record.id)
            .ok_or_else(|| anyhow!("tencent Record.Create returned empty record id"))?;

        Ok(DnsRecordHandle {
            provider: "tencent".to_string(),
            record_id: record_id.to_string(),
            zone: domain,
            name: sub_domain,
        })
    }

    async fn delete_existing_txt(&self, domain: &str, sub_domain: &str) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("domain".to_string(), domain.to_string());
        params.insert("subdomain".to_string(), sub_domain.to_string());
        params.insert("record_type".to_string(), "TXT".to_string());

        let response = self.call("Record.List", params).await?;
        for record in response.records.unwrap_or_default() {
            self.delete_txt_record(&DnsRecordHandle {
                provider: "tencent".to_string(),
                record_id: record.id.to_string(),
                zone: domain.to_string(),
                name: sub_domain.to_string(),
            })
            .await?;
        }
        Ok(())
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("domain".to_string(), handle.zone.clone());
        params.insert("record_id".to_string(), handle.record_id.clone());
        self.call("Record.Remove", params).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct TencentResponse {
    status: TencentStatus,
    record: Option<TencentCreatedRecord>,
    records: Option<Vec<TencentRecord>>,
}

#[derive(Debug, Deserialize)]
struct TencentStatus {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct TencentCreatedRecord {
    id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TencentRecord {
    id: u64,
}
