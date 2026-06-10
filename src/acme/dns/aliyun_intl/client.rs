use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::common::aliyun_signed_get_url;
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, split_fqdn};

const ENDPOINT: &str = "https://alidns.ap-southeast-1.aliyuncs.com";

pub struct AliyunIntlDns {
    client: Client,
    access_key_id: String,
    access_key_secret: String,
}

impl AliyunIntlDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            access_key_id: credential(
                credentials,
                &[
                    "access_key_id",
                    "AccessKeyId",
                    "Ali_Key",
                    "ALIBABA_CLOUD_ACCESS_KEY_ID",
                ],
            )?,
            access_key_secret: credential(
                credentials,
                &[
                    "access_key_secret",
                    "AccessKeySecret",
                    "Ali_Secret",
                    "ALIBABA_CLOUD_ACCESS_KEY_SECRET",
                ],
            )?,
        })
    }

    async fn call(&self, action: &str, params: BTreeMap<String, String>) -> Result<AliyunResponse> {
        let mut query = params;
        query.insert("Action".to_string(), action.to_string());
        let url = aliyun_signed_get_url(
            ENDPOINT,
            &self.access_key_id,
            &self.access_key_secret,
            &query,
        )?;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("aliyun_intl {action} request failed"))?
            .error_for_status()
            .with_context(|| format!("aliyun_intl {action} returned error status"))?
            .json::<AliyunResponse>()
            .await
            .with_context(|| format!("aliyun_intl {action} response decode failed"))?;
        if let Some(code) = &response.code {
            if code != "DomainRecordDuplicate" {
                return Err(anyhow!(
                    "aliyun_intl {action} failed: {} ({})",
                    response.message.as_deref().unwrap_or("unknown error"),
                    code
                ));
            }
        }
        Ok(response)
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (domain_name, rr) = split_fqdn(fqdn);
        let rr = if rr.is_empty() {
            "_acme-challenge".to_string()
        } else {
            rr
        };

        self.delete_existing_txt(&domain_name, &rr).await?;

        let mut params = BTreeMap::new();
        params.insert("DomainName".to_string(), domain_name.clone());
        params.insert("RR".to_string(), rr.clone());
        params.insert("Type".to_string(), "TXT".to_string());
        params.insert("Value".to_string(), value.to_string());
        params.insert("TTL".to_string(), "600".to_string());

        let response = self.call("AddDomainRecord", params).await?;
        let record_id = response
            .record_id
            .ok_or_else(|| anyhow!("aliyun_intl AddDomainRecord returned empty RecordId"))?;

        Ok(DnsRecordHandle {
            provider: "aliyun_intl".to_string(),
            record_id,
            zone: domain_name,
            name: rr,
        })
    }

    async fn delete_existing_txt(&self, domain_name: &str, rr: &str) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("DomainName".to_string(), domain_name.to_string());
        params.insert("RRKeyWord".to_string(), rr.to_string());
        params.insert("TypeKeyWord".to_string(), "TXT".to_string());

        let response = self.call("DescribeDomainRecords", params).await?;
        let records = response
            .domain_records
            .map(|wrapper| wrapper.record)
            .unwrap_or_default();
        for record in records {
            self.delete_txt_record(&DnsRecordHandle {
                provider: "aliyun_intl".to_string(),
                record_id: record.record_id,
                zone: domain_name.to_string(),
                name: rr.to_string(),
            })
            .await?;
        }
        Ok(())
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("RecordId".to_string(), handle.record_id.clone());
        self.call("DeleteDomainRecord", params).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct AliyunResponse {
    #[serde(rename = "RecordId")]
    record_id: Option<String>,
    #[serde(rename = "DomainRecords")]
    domain_records: Option<AliyunDomainRecords>,
    #[serde(rename = "Code")]
    code: Option<String>,
    #[serde(rename = "Message")]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AliyunDomainRecords {
    #[serde(rename = "Record")]
    record: Vec<AliyunRecord>,
}

#[derive(Debug, Deserialize)]
struct AliyunRecord {
    #[serde(rename = "RecordId")]
    record_id: String,
}
