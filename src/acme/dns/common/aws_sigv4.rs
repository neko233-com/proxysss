use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub struct SignedRequest {
    pub uri: String,
    pub authorization: String,
    pub amz_date: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
}

pub struct AwsSigV4 {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    service: String,
}

impl AwsSigV4 {
    pub fn new(
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

    pub fn sign(
        &self,
        method: &str,
        host: &str,
        canonical_uri: &str,
        query: &str,
        payload: &[u8],
        content_type: Option<&str>,
    ) -> Result<SignedRequest> {
        let amz_date = amz_datetime();
        let date_stamp = &amz_date[..8];
        let payload_hash = hex_sha256(payload);

        let mut canonical_headers = format!("host:{host}\nx-amz-date:{amz_date}\n");
        let mut signed_headers = "host;x-amz-date".to_string();
        if let Some(content_type) = content_type {
            canonical_headers.push_str(&format!("content-type:{content_type}\n"));
            signed_headers = "content-type;host;x-amz-date".to_string();
        }

        let canonical_request = format!(
            "{method}\n{canonical_uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let credential_scope =
            format!("{date_stamp}/{}/{}/aws4_request", self.region, self.service);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
            hex_sha256(canonical_request.as_bytes())
        );

        let signing_key = derive_signing_key(
            &self.secret_access_key,
            date_stamp,
            &self.region,
            &self.service,
        )?;
        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|error| anyhow!("failed to initialize aws signer: {error}"))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex_encode(mac.finalize().into_bytes());

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers, signature
        );

        Ok(SignedRequest {
            uri: if query.is_empty() {
                canonical_uri.to_string()
            } else {
                format!("{canonical_uri}?{query}")
            },
            authorization,
            amz_date,
            content_type: content_type.map(str::to_string),
        })
    }
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Result<Vec<u8>> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    Ok(hmac_sha256(&k_service, b"aws4_request"))
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

fn amz_datetime() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seconds_per_day = 86_400;
    let days = now / seconds_per_day;
    let day_seconds = now % seconds_per_day;
    let mut year = 1970 + (days / 365);
    let mut remaining_days = days % 365;
    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }
    let mut month = 1;
    while remaining_days >= days_in_month(year, month) {
        remaining_days -= days_in_month(year, month);
        month += 1;
    }
    let day = remaining_days + 1;
    let hour = day_seconds / 3600;
    let minute = (day_seconds % 3600) / 60;
    let second = day_seconds % 60;
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

fn days_in_year(year: u64) -> u64 {
    if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) {
        366
    } else {
        365
    }
}

fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if days_in_year(year) == 366 => 29,
        _ => 28,
    }
}

#[allow(dead_code)]
pub fn route53_txt_value(value: &str) -> String {
    format!("\"{value}\"")
}

pub fn split_route53_txt_value(value: &str) -> Vec<String> {
    if value.len() <= 255 {
        return vec![format!("\"{value}\"")];
    }
    value
        .as_bytes()
        .chunks(255)
        .map(|chunk| format!("\"{}\"", String::from_utf8_lossy(chunk)))
        .collect()
}
