use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

pub fn percent_encode_aliyun(value: &str) -> String {
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

pub fn aliyun_signed_get_url(
    endpoint: &str,
    access_key_id: &str,
    access_key_secret: &str,
    params: &BTreeMap<String, String>,
) -> Result<String> {
    let mut signed = params.clone();
    signed.insert("Format".to_string(), "JSON".to_string());
    signed.insert("Version".to_string(), "2015-01-09".to_string());
    signed.insert("AccessKeyId".to_string(), access_key_id.to_string());
    signed.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    signed.insert("Timestamp".to_string(), utc_timestamp());
    signed.insert("SignatureVersion".to_string(), "1.0".to_string());
    signed.insert(
        "SignatureNonce".to_string(),
        uuid::Uuid::new_v4().to_string(),
    );

    let canonicalized = signed
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encode_aliyun(key),
                percent_encode_aliyun(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");

    let string_to_sign = format!(
        "GET&{}&{}",
        percent_encode_aliyun("/"),
        percent_encode_aliyun(&canonicalized)
    );

    let mut mac = HmacSha1::new_from_slice(format!("{access_key_secret}&").as_bytes())
        .map_err(|error| anyhow!("failed to initialize aliyun signer: {error}"))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    signed.insert("Signature".to_string(), signature);

    let query = signed
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encode_aliyun(key),
                percent_encode_aliyun(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    Ok(format!("{endpoint}/?{query}"))
}

fn utc_timestamp() -> String {
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
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
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
