use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub struct SignedRequest {
    #[allow(dead_code)]
    pub uri: String,
    pub authorization: String,
    pub x_date: String,
    pub content_type: Option<String>,
    pub content_sha256: Option<String>,
}

pub struct VolcengineSigV4 {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    service: String,
}

impl VolcengineSigV4 {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        Self::for_service(access_key_id, secret_access_key, region, "dns".to_string())
    }

    pub fn for_service(
        access_key_id: String,
        secret_access_key: String,
        region: String,
        service: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
            service,
        }
    }

    pub fn sign_get(&self, host: &str, query: &str) -> Result<SignedRequest> {
        self.sign("GET", host, query, b"", None)
    }

    pub fn sign_post(&self, host: &str, query: &str, body: &[u8]) -> Result<SignedRequest> {
        self.sign("POST", host, query, body, Some("application/json"))
    }

    fn sign(
        &self,
        method: &str,
        host: &str,
        query: &str,
        payload: &[u8],
        content_type: Option<&str>,
    ) -> Result<SignedRequest> {
        self.sign_at(method, host, query, payload, content_type, &x_datetime())
    }

    fn sign_at(
        &self,
        method: &str,
        host: &str,
        query: &str,
        payload: &[u8],
        content_type: Option<&str>,
        x_date: &str,
    ) -> Result<SignedRequest> {
        let date_stamp = &x_date[..8];
        let payload_hash = hex_sha256(payload);

        let (canonical_headers, signed_headers) = match (method, content_type) {
            ("POST", Some(content_type)) => (
                format!("content-type:{content_type}\nhost:{host}\nx-content-sha256:{payload_hash}\nx-date:{x_date}\n"),
                "content-type;host;x-content-sha256;x-date",
            ),
            ("POST", None) => (
                format!("host:{host}\nx-content-sha256:{payload_hash}\nx-date:{x_date}\n"),
                "host;x-content-sha256;x-date",
            ),
            _ => (format!("host:{host}\nx-date:{x_date}\n"), "host;x-date"),
        };
        let mut params: Vec<&str> = query.split('&').filter(|part| !part.is_empty()).collect();
        params.sort_unstable();
        let canonical_query = params.join("&");

        let canonical_request = format!(
            "{method}\n/\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let credential_scope = format!("{date_stamp}/{}/{}/request", self.region, self.service);
        let string_to_sign = format!(
            "HMAC-SHA256\n{x_date}\n{credential_scope}\n{}",
            hex_sha256(canonical_request.as_bytes())
        );

        let signing_key = derive_signing_key(
            &self.secret_access_key,
            date_stamp,
            &self.region,
            &self.service,
        )?;
        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|error| anyhow!("failed to initialize volcengine signer: {error}"))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex_encode(mac.finalize().into_bytes());

        let authorization = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers, signature
        );

        Ok(SignedRequest {
            uri: if query.is_empty() {
                "/".to_string()
            } else {
                format!("/?{query}")
            },
            authorization,
            x_date: x_date.to_string(),
            content_type: content_type.map(str::to_string),
            content_sha256: if method == "POST" {
                Some(payload_hash)
            } else {
                None
            },
        })
    }
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Result<Vec<u8>> {
    let k_date = hmac_sha256(secret.as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    Ok(hmac_sha256(&k_service, b"request"))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_sha256(data: &[u8]) -> String {
    hex_encode(Sha256::digest(data))
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn x_datetime() -> String {
    format_datetime(time::OffsetDateTime::now_utc())
}

fn format_datetime(now: time::OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_date_observes_leap_years_and_utc() {
        for (timestamp, expected) in [
            (0, "19700101T000000Z"),
            (1_709_251_200, "20240301T000000Z"),
            (1_798_761_600, "20270101T000000Z"),
        ] {
            assert_eq!(
                format_datetime(time::OffsetDateTime::from_unix_timestamp(timestamp).unwrap()),
                expected
            );
        }
    }

    #[test]
    fn signatures_use_sorted_query_and_canonical_headers() -> Result<()> {
        let signer =
            VolcengineSigV4::new("test-key".into(), "test-secret".into(), "cn-beijing".into());
        let get = signer.sign_at(
            "GET",
            "dns.volcengineapi.com",
            "Action=ListRecords&Version=2018-08-01&ZID=123&Host=x&Type=TXT",
            b"",
            None,
            "20261030T040000Z",
        )?;
        assert!(get.authorization.ends_with(
            "Signature=929639b2fd92a4eb82dbb92a9fdd40f76dbe2e4bdae878607f5f8fa6c43110be"
        ));
        let post = signer.sign_at(
            "POST",
            "dns.volcengineapi.com",
            "Action=CreateRecord&Version=2018-08-01",
            b"{}",
            Some("application/json"),
            "20261030T040000Z",
        )?;
        assert!(post.authorization.ends_with(
            "Signature=dc8d0ea8af32878a06c5ae52f3a32c0df21e5ca7a37d20cd9025e8496e7eb2bd"
        ));
        Ok(())
    }
}
