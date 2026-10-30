use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::acme::dns::common::VolcengineSigV4;
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, optional_credential};

const HOST: &str = "dns.volcengineapi.com";
const VERSION: &str = "2018-08-01";

pub struct VolcengineDns {
    client: Client,
    signer: VolcengineSigV4,
    base_url: String,
}

impl VolcengineDns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        let region = optional_credential(credentials, &["region"])
            .unwrap_or_else(|| "cn-beijing".to_string());
        Ok(Self {
            client: Client::new(),
            base_url: format!("https://{HOST}"),
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
        let url = format!("{}/?{query}", self.base_url);
        let body = self
            .client
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
            .with_context(|| format!("volcengine {action} get response decode failed"))?;
        check_api_error(action, &body)?;
        Ok(body)
    }

    async fn signed_post(&self, action: &str, body: &serde_json::Value) -> Result<String> {
        let query = format!("Action={action}&Version={VERSION}");
        let payload = serde_json::to_vec(body).context("volcengine request body encode failed")?;
        let signed = self.signer.sign_post(HOST, &query, &payload)?;
        let url = format!("{}/?{query}", self.base_url);
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
        let body = request
            .body(payload)
            .send()
            .await
            .with_context(|| format!("volcengine {action} post request failed"))?
            .error_for_status()
            .with_context(|| format!("volcengine {action} post returned error status"))?
            .text()
            .await
            .with_context(|| format!("volcengine {action} post response decode failed"))?;
        check_api_error(action, &body)?;
        Ok(body)
    }

    async fn resolve_zone(&self, fqdn: &str) -> Result<(u64, String)> {
        let labels: Vec<&str> = fqdn.trim_end_matches('.').split('.').collect();
        if labels.len() < 3 || labels.iter().any(|label| label.is_empty()) {
            return Err(anyhow!("invalid DNS challenge name"));
        }
        for index in 1..labels.len() - 1 {
            let candidate = labels[index..].join(".");
            let body = self
                .signed_get(
                    "ListZones",
                    &[
                        ("Key", candidate.clone()),
                        ("SearchMode", "exact".to_string()),
                    ],
                )
                .await?;
            let parsed: VolcengineResponse<ListZonesResult> = serde_json::from_str(&body)
                .context("volcengine ListZones response decode failed")?;
            let zones = parsed
                .result
                .ok_or_else(|| anyhow!("volcengine ListZones returned no result"))?
                .zones;
            if let Some(zone) = zones
                .into_iter()
                .find(|zone| zone.name.eq_ignore_ascii_case(&candidate))
            {
                return Ok((zone.zid, labels[..index].join(".")));
            }
        }
        Err(anyhow!("volcengine DNS zone not found for {fqdn}"))
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (zid, host) = self.resolve_zone(fqdn).await?;
        // The apex and wildcard authorizations can share this name. Keep
        // other TXT values and remove only the record created by this order.
        let body = serde_json::json!({
            "ZID": zid,
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
            record_id,
            zone: zid.to_string(),
            name: host,
        })
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let body = serde_json::json!({
            "RecordID": handle.record_id,
        });
        self.signed_post("DeleteRecord", &body).await?;
        Ok(())
    }
}

fn check_api_error(action: &str, body: &str) -> Result<()> {
    let response: serde_json::Value = serde_json::from_str(body)
        .with_context(|| format!("volcengine {action} returned invalid JSON"))?;
    if !response
        .get("ResponseMetadata")
        .is_some_and(|value| value.is_object())
    {
        return Err(anyhow!("volcengine {action} returned no response metadata"));
    }
    if let Some(error) = response.pointer("/ResponseMetadata/Error") {
        let code = error
            .get("Code")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(anyhow!("volcengine {action} failed: {code}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_errors_are_not_mistaken_for_success() {
        assert!(check_api_error(
            "DeleteRecord",
            r#"{"ResponseMetadata":{"Error":{"Code":"AccessDenied"}}}"#
        )
        .is_err());
        assert!(check_api_error("DeleteRecord", r#"{"ResponseMetadata":{},"Result":{}}"#).is_ok());
        assert!(check_api_error("DeleteRecord", "not json").is_err());
        assert!(check_api_error("DeleteRecord", "{}").is_err());
    }

    #[tokio::test]
    async fn shared_challenge_name_preserves_each_record_until_its_own_cleanup() -> Result<()> {
        use http_body_util::{BodyExt, Full};
        use hyper::service::service_fn;
        use hyper::{Request, Response};
        use hyper_util::rt::TokioIo;
        use std::sync::{Arc, Mutex};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let events = Arc::new(Mutex::new(Vec::<(String, serde_json::Value)>::new()));
        let captured = events.clone();
        let server = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let events = captured.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request: Request<hyper::body::Incoming>| {
                        let events = events.clone();
                        async move {
                            let query = request.uri().query().unwrap_or_default().to_string();
                            let body = request.into_body().collect().await.unwrap().to_bytes();
                            let body: serde_json::Value =
                                serde_json::from_slice(&body).unwrap_or_default();
                            let mut events = events.lock().unwrap();
                            events.push((query.clone(), body));
                            let result = if query.contains("Action=ListZones") {
                                serde_json::json!({"Zones":[{"ZID":123,"ZoneName":"example.com"}]})
                            } else if query.contains("Action=CreateRecord") {
                                let id = events
                                    .iter()
                                    .filter(|(q, _)| q.contains("Action=CreateRecord"))
                                    .count();
                                serde_json::json!({"RecordID":id.to_string()})
                            } else {
                                serde_json::json!({})
                            };
                            let body = serde_json::to_vec(
                                &serde_json::json!({"ResponseMetadata":{},"Result":result}),
                            )
                            .unwrap();
                            Ok::<_, std::convert::Infallible>(Response::new(Full::new(
                                bytes::Bytes::from(body),
                            )))
                        }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        let mut credentials = BTreeMap::new();
        credentials.insert("access_key_id".into(), "test".into());
        credentials.insert("secret_access_key".into(), "test".into());
        let mut provider = VolcengineDns::new(&credentials)?;
        provider.base_url = format!("http://{address}");
        let first = provider
            .upsert_txt_record("_acme-challenge.example.com", "root-value")
            .await?;
        let second = provider
            .upsert_txt_record("_acme-challenge.example.com", "wildcard-value")
            .await?;
        provider.delete_txt_record(&first).await?;
        provider.delete_txt_record(&second).await?;
        let nested = provider
            .upsert_txt_record("_acme-challenge.sub.example.com", "nested-value")
            .await?;
        provider.delete_txt_record(&nested).await?;
        server.abort();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 10);
        assert!(!events
            .iter()
            .any(|(query, _)| query.contains("ListRecords")));
        assert!(events[0].0.contains("Action=ListZones"));
        assert!(events[0].0.contains("Key=example.com"));
        assert!(events[0].0.contains("SearchMode=exact"));
        assert_eq!(events[1].1["Host"], "_acme-challenge");
        assert_eq!(events[1].1["Value"], "root-value");
        assert_eq!(events[3].1["Value"], "wildcard-value");
        assert_eq!(events[4].1["RecordID"], "1");
        assert_eq!(events[5].1["RecordID"], "2");
        assert!(events[4].1.get("ZID").is_none());
        assert!(events[6].0.contains("Key=sub.example.com"));
        assert!(events[7].0.contains("Key=example.com"));
        assert_eq!(events[8].1["Host"], "_acme-challenge.sub");
        assert_eq!(events[9].1["RecordID"], "3");
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
struct ListZonesResult {
    #[serde(rename = "Zones", default)]
    zones: Vec<VolcengineDomain>,
}

#[derive(Debug, Deserialize)]
struct VolcengineDomain {
    #[serde(rename = "ZID")]
    zid: u64,
    #[serde(rename = "ZoneName")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct CreateRecordResult {
    #[serde(rename = "RecordID")]
    record_id: Option<String>,
}
