use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;

use crate::acme::dns::common::aws_sigv4::{split_route53_txt_value, AwsSigV4};
use crate::acme::dns::types::DnsRecordHandle;
use crate::acme::dns::util::{credential, fqdn_with_trailing_dot, optional_credential, split_fqdn};

const ROUTE53_HOST: &str = "route53.amazonaws.com";
const ROUTE53_REGION: &str = "us-east-1";

pub struct AwsRoute53Dns {
    client: Client,
    signer: AwsSigV4,
    hosted_zone_id: Option<String>,
}

impl AwsRoute53Dns {
    pub fn new(credentials: &BTreeMap<String, String>) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            signer: AwsSigV4::new(
                credential(
                    credentials,
                    &["access_key_id", "AWS_ACCESS_KEY_ID", "AWSAccessKeyId"],
                )?,
                credential(
                    credentials,
                    &[
                        "secret_access_key",
                        "AWS_SECRET_ACCESS_KEY",
                        "AWSSecretAccessKey",
                    ],
                )?,
                optional_credential(credentials, &["region"])
                    .unwrap_or_else(|| ROUTE53_REGION.to_string()),
                "route53".to_string(),
            ),
            hosted_zone_id: optional_credential(
                credentials,
                &["hosted_zone_id", "AWSHostedZoneId"],
            ),
        })
    }

    async fn signed_get(&self, path: &str, query: &str) -> Result<String> {
        let signed = self
            .signer
            .sign("GET", ROUTE53_HOST, path, query, b"", None)?;
        let url = format!("https://{ROUTE53_HOST}{}", signed.uri);
        self.client
            .get(url)
            .header("Authorization", signed.authorization)
            .header("x-amz-date", signed.amz_date)
            .send()
            .await
            .context("aws route53 get request failed")?
            .error_for_status()
            .context("aws route53 get returned error status")?
            .text()
            .await
            .context("aws route53 get response decode failed")
    }

    async fn signed_post(&self, path: &str, body: &str) -> Result<String> {
        let signed = self.signer.sign(
            "POST",
            ROUTE53_HOST,
            path,
            "",
            body.as_bytes(),
            Some("application/xml"),
        )?;
        let url = format!("https://{ROUTE53_HOST}{path}");
        self.client
            .post(url)
            .header("Authorization", signed.authorization)
            .header("x-amz-date", signed.amz_date)
            .header("content-type", "application/xml")
            .body(body.to_string())
            .send()
            .await
            .context("aws route53 post request failed")?
            .error_for_status()
            .context("aws route53 post returned error status")?
            .text()
            .await
            .context("aws route53 post response decode failed")
    }

    async fn resolve_hosted_zone_id(&self, zone_name: &str) -> Result<String> {
        if let Some(zone_id) = &self.hosted_zone_id {
            return Ok(zone_id.clone());
        }

        let dns_name = fqdn_with_trailing_dot(zone_name);
        let query = format!("dnsname={}&maxitems=1", urlencoding(dns_name.as_str()));
        let body = self
            .signed_get("/2013-04-01/hostedzonesbyname", &query)
            .await?;
        let zone_id = extract_xml_tag(&body, "Id")
            .and_then(|value| value.strip_prefix("/hostedzone/").map(str::to_string))
            .ok_or_else(|| anyhow!("aws route53 hosted zone not found for {zone_name}"))?;
        Ok(zone_id)
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        let (zone_name, _host) = split_fqdn(fqdn);
        let zone_id = self.resolve_hosted_zone_id(&zone_name).await?;
        let record_name = fqdn_with_trailing_dot(fqdn);
        let relative_name = record_name
            .trim_end_matches('.')
            .strip_suffix(&format!(".{zone_name}"))
            .unwrap_or("_acme-challenge")
            .to_string();

        self.delete_existing_txt(&zone_id, &record_name).await?;

        let values = split_route53_txt_value(value);
        let mut resource_records = String::new();
        for quoted in values {
            resource_records.push_str(&format!(
                "<ResourceRecord><Value>{}</Value></ResourceRecord>",
                xml_escape(&quoted)
            ));
        }

        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ChangeResourceRecordSetsRequest xmlns=\"https://route53.amazonaws.com/doc/2013-04-01/\"><ChangeBatch><Changes><Change><Action>UPSERT</Action><ResourceRecordSet><Name>{name}</Name><Type>TXT</Type><TTL>300</TTL><ResourceRecords>{records}</ResourceRecords></ResourceRecordSet></Change></Changes></ChangeBatch></ChangeResourceRecordSetsRequest>",
            name = xml_escape(&record_name),
            records = resource_records
        );

        self.signed_post(&format!("/2013-04-01/hostedzone/{zone_id}/rrset/"), &xml)
            .await?;

        Ok(DnsRecordHandle {
            provider: "aws".to_string(),
            record_id: record_name.clone(),
            zone: zone_id,
            name: relative_name,
        })
    }

    async fn delete_existing_txt(&self, zone_id: &str, record_name: &str) -> Result<()> {
        let list_body = self
            .signed_get(
                &format!("/2013-04-01/hostedzone/{zone_id}/rrset/"),
                &format!("name={}&type=TXT&maxitems=1", urlencoding(record_name)),
            )
            .await?;

        if !list_body.contains("<Name>") {
            return Ok(());
        }

        let delete_xml = extract_rrset_delete_fragment(&list_body, record_name);
        if let Some(delete_xml) = delete_xml {
            self.signed_post(
                &format!("/2013-04-01/hostedzone/{zone_id}/rrset/"),
                &delete_xml,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        let record_name = if handle.record_id.contains('.') {
            handle.record_id.clone()
        } else {
            fqdn_with_trailing_dot(&format!("{}.{}", handle.name, handle.zone))
        };
        self.delete_existing_txt(&handle.zone, &record_name).await
    }
}

fn extract_rrset_delete_fragment(list_body: &str, record_name: &str) -> Option<String> {
    if !list_body.contains(record_name) {
        return None;
    }
    let resource_records = extract_xml_block(list_body, "ResourceRecords")?;
    let ttl = extract_xml_tag(list_body, "TTL").unwrap_or_else(|| "300".to_string());
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ChangeResourceRecordSetsRequest xmlns=\"https://route53.amazonaws.com/doc/2013-04-01/\"><ChangeBatch><Changes><Change><Action>DELETE</Action><ResourceRecordSet><Name>{name}</Name><Type>TXT</Type><TTL>{ttl}</TTL><ResourceRecords>{records}</ResourceRecords></ResourceRecordSet></Change></Changes></ChangeBatch></ChangeResourceRecordSetsRequest>",
        name = xml_escape(record_name),
        ttl = xml_escape(&ttl),
        records = resource_records
    ))
}

fn extract_xml_tag(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some(body[start..end].to_string())
}

fn extract_xml_block(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)?;
    let end = body.find(&close)? + close.len();
    Some(body[start..end].to_string())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
