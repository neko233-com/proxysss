//! nginx `ssl_stapling` parity: fetch OCSP responses from the certificate's AIA
//! responder and staple the DER `BasicOCSPResponse` onto the rustls CertifiedKey.
//! Pure Rust: hand-rolled ASN.1 subset + `sha1` + existing reqwest/rustls stack.
//! `ocsp_stapling_file` remains an operator override; auto fetch refreshes at
//! nextUpdate/2 (clamped) and republishes TLS material without a restart.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use sha1::{Digest, Sha1};

const OID_AIA: &str = "1.3.6.1.5.5.7.1.1";
const OID_OCSP: &str = "1.3.6.1.5.5.7.48.1";
const OID_SHA1: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
const OID_BASIC_OCSP: &str = "1.3.6.1.5.5.7.48.1.1";

pub struct OcspStaple {
    pub der: Vec<u8>,
    pub next_update: Option<SystemTime>,
}

/// Minimal DER cursor: tag/length/value walking over the ASN.1 subset OCSP needs.
struct Der<'a> {
    buf: &'a [u8],
}

impl<'a> Der<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
    fn peek_tag(&self) -> Option<u8> {
        self.buf.first().copied()
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.buf.len() < n {
            bail!("truncated DER");
        }
        let (head, rest) = self.buf.split_at(n);
        self.buf = rest;
        Ok(head)
    }
    /// Whole TLV bytes plus the content slice.
    fn tlv(&mut self) -> Result<(u8, &'a [u8], &'a [u8])> {
        let start = self.buf;
        let tag = self.take(1)?[0];
        let first = self.take(1)?[0];
        let len = if first & 0x80 == 0 {
            first as usize
        } else {
            let n = (first & 0x7f) as usize;
            if n == 0 || n > 4 {
                bail!("unsupported DER length");
            }
            let bytes = self.take(n)?;
            let mut len = 0usize;
            for b in bytes {
                len = (len << 8) | *b as usize;
            }
            len
        };
        let content = self.take(len)?;
        let whole = &start[..start.len() - self.buf.len()];
        Ok((tag, content, whole))
    }
    fn expect(&mut self, tag: u8) -> Result<&'a [u8]> {
        let (t, content, _) = self.tlv()?;
        if t != tag {
            bail!("expected DER tag {tag:#x}, got {t:#x}");
        }
        Ok(content)
    }
    fn optional(&mut self, tag: u8) -> Result<Option<&'a [u8]>> {
        if self.peek_tag() == Some(tag) {
            Ok(Some(self.expect(tag)?))
        } else {
            Ok(None)
        }
    }
}

fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len <= 0xff {
        vec![0x81, len as u8]
    } else {
        vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
    }
}

fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&der_len(content.len()));
    out.extend_from_slice(content);
    out
}

fn seq(parts: &[&[u8]]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(part);
    }
    der(0x30, &body)
}

fn oid(encoded: &[u8]) -> Vec<u8> {
    der(0x06, encoded)
}

fn octet(bytes: &[u8]) -> Vec<u8> {
    der(0x04, bytes)
}

#[cfg(test)]
fn integer_be(bytes: &[u8]) -> Vec<u8> {
    let mut trimmed = bytes;
    while trimmed.len() > 1 && trimmed[0] == 0 {
        trimmed = &trimmed[1..];
    }
    if trimmed.first().copied().unwrap_or(0) & 0x80 != 0 {
        let mut padded = vec![0u8];
        padded.extend_from_slice(trimmed);
        return der(0x02, &padded);
    }
    der(0x02, trimmed)
}

fn sha1(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

struct CertParts {
    serial_der: Vec<u8>,
    subject_der: Vec<u8>,
    spki_bits: Vec<u8>,
    extensions: Vec<(String, Vec<u8>)>,
}

fn oid_string(content: &[u8]) -> String {
    if content.is_empty() {
        return String::new();
    }
    let mut out = format!("{}", content[0] / 40);
    out.push('.');
    out.push_str(&(content[0] % 40).to_string());
    let mut value = 0u64;
    for b in &content[1..] {
        value = (value << 7) | (b & 0x7f) as u64;
        if b & 0x80 == 0 {
            out.push('.');
            out.push_str(&value.to_string());
            value = 0;
        }
    }
    out
}

/// Walk TBSCertificate for the fields OCSP CertID and AIA need.
fn cert_parts(der_bytes: &[u8]) -> Result<CertParts> {
    let mut cert = Der::new(der_bytes);
    let cert_body = cert.expect(0x30)?;
    let mut cert_body = Der::new(cert_body);
    let tbs = cert_body.expect(0x30)?;
    let mut tbs = Der::new(tbs);
    tbs.optional(0xa0)?; // version
    let (_, _, serial_der) = tbs.tlv()?; // serialNumber
    tbs.expect(0x30)?; // signature algorithm
    tbs.expect(0x30)?; // issuer
    tbs.expect(0x30)?; // validity
    let (_, _, subject_der) = tbs.tlv()?; // subject
    let spki = tbs.expect(0x30)?;
    let mut spki = Der::new(spki);
    spki.expect(0x30)?; // algorithm
    let bits = spki.expect(0x03)?; // subjectPublicKey BIT STRING
    if bits.is_empty() {
        bail!("empty subjectPublicKey");
    }
    let spki_bits = bits[1..].to_vec(); // skip unused-bits octet
    let mut extensions = Vec::new();
    if let Some(ext_wrap) = tbs.optional(0xa3)? {
        let mut ext_seq = Der::new(ext_wrap);
        let exts = ext_seq.expect(0x30)?;
        let mut exts = Der::new(exts);
        while exts.peek_tag().is_some() {
            let ext = exts.expect(0x30)?;
            let mut ext = Der::new(ext);
            let oid_content = ext.expect(0x06)?;
            ext.optional(0x01)?; // critical BOOLEAN
            let value = ext.expect(0x04)?;
            extensions.push((oid_string(oid_content), value.to_vec()));
        }
    }
    Ok(CertParts {
        serial_der: serial_der.to_vec(),
        subject_der: subject_der.to_vec(),
        spki_bits,
        extensions,
    })
}

fn aia_ocsp_url(parts: &CertParts) -> Result<String> {
    let (_, value) = parts
        .extensions
        .iter()
        .find(|(oid, _)| oid == OID_AIA)
        .ok_or_else(|| anyhow!("certificate has no authorityInfoAccess extension"))?;
    let mut seq = Der::new(value);
    let descs = seq.expect(0x30)?;
    let mut descs = Der::new(descs);
    while descs.peek_tag().is_some() {
        let desc = descs.expect(0x30)?;
        let mut desc = Der::new(desc);
        let method = oid_string(desc.expect(0x06)?);
        let (tag, location, _) = desc.tlv()?;
        if method == OID_OCSP && tag == 0x86 {
            return String::from_utf8(location.to_vec()).context("OCSP URI not UTF-8");
        }
    }
    bail!("no OCSP accessMethod in authorityInfoAccess")
}

fn build_request(leaf: &CertParts, issuer: &CertParts) -> Vec<u8> {
    let cert_id = seq(&[
        &seq(&[&oid(OID_SHA1), &der(0x05, &[])]),
        &octet(&sha1(&issuer.subject_der)),
        &octet(&sha1(&issuer.spki_bits)),
        &leaf.serial_der,
    ]);
    let request = seq(&[&cert_id]);
    let request_list = der(0x30, &request);
    let tbs = seq(&[&request_list]);
    seq(&[&tbs])
}

fn parse_generalized_time(content: &[u8]) -> Result<SystemTime> {
    let text = std::str::from_utf8(content).context("GeneralizedTime not UTF-8")?;
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 14 {
        bail!("short GeneralizedTime: {text}");
    }
    let num = |r: std::ops::Range<usize>| -> Result<i64> {
        digits[r]
            .parse::<i64>()
            .context("GeneralizedTime field")
    };
    let (y, m, d) = (num(0..4)?, num(4..6)?, num(6..8)?);
    let (hh, mm, ss) = (num(8..10)?, num(10..12)?, num(12..14)?);
    // Howard Hinnant days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + hh * 3600 + mm * 60 + ss;
    if secs < 0 {
        bail!("GeneralizedTime before epoch");
    }
    Ok(UNIX_EPOCH + Duration::from_secs(secs as u64))
}

/// Validate an OCSPResponse and return the staple DER plus nextUpdate.
pub fn parse_response(bytes: &[u8]) -> Result<(Vec<u8>, Option<SystemTime>)> {
    let mut resp = Der::new(bytes);
    let body = resp.expect(0x30)?;
    let mut body = Der::new(body);
    let status = body.expect(0x0a)?;
    if status != [0u8] {
        bail!("OCSP responseStatus {}", status.first().copied().unwrap_or(255));
    }
    let wrap = body
        .optional(0xa0)?
        .ok_or_else(|| anyhow!("OCSP response has no responseBytes"))?;
    let mut wrap = Der::new(wrap);
    let rb = wrap.expect(0x30)?;
    let mut rb = Der::new(rb);
    let response_type = oid_string(rb.expect(0x06)?);
    if response_type != OID_BASIC_OCSP {
        bail!("unsupported OCSP responseType {response_type}");
    }
    let staple = rb.expect(0x04)?.to_vec();
    let next_update = parse_next_update(&staple).ok();
    Ok((staple, next_update))
}

fn parse_next_update(staple: &[u8]) -> Result<SystemTime> {
    let mut basic = Der::new(staple);
    let basic_body = basic.expect(0x30)?;
    let mut basic_body = Der::new(basic_body);
    let tbs = basic_body.expect(0x30)?;
    let mut tbs = Der::new(tbs);
    tbs.optional(0xa0)?; // version
    let tag = tbs.peek_tag().ok_or_else(|| anyhow!("empty tbsResponseData"))?;
    if tag != 0xa1 && tag != 0xa2 {
        bail!("unexpected responderID tag {tag:#x}");
    }
    tbs.tlv()?; // responderID
    tbs.expect(0x18)?; // producedAt
    let responses = tbs.expect(0x30)?;
    let mut responses = Der::new(responses);
    let single = responses.expect(0x30)?;
    let mut single = Der::new(single);
    single.expect(0x30)?; // certID
    let status_tag = single.peek_tag().ok_or_else(|| anyhow!("missing certStatus"))?;
    if !matches!(status_tag, 0x80 | 0xa1 | 0x82) {
        bail!("unexpected certStatus tag {status_tag:#x}");
    }
    single.tlv()?; // certStatus
    single.expect(0x18)?; // thisUpdate
    let next = single
        .optional(0xa0)?
        .ok_or_else(|| anyhow!("no nextUpdate"))?;
    let mut next = Der::new(next);
    parse_generalized_time(next.expect(0x18)?)
}

/// Fetch a fresh OCSP staple for a leaf/issuer DER pair from the AIA responder.
pub async fn fetch(leaf_der: &[u8], issuer_der: &[u8]) -> Result<OcspStaple> {
    let leaf = cert_parts(leaf_der).context("leaf certificate")?;
    let issuer = cert_parts(issuer_der).context("issuer certificate")?;
    let url = aia_ocsp_url(&leaf)?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        bail!("unsupported OCSP URI scheme: {url}");
    }
    let request = build_request(&leaf, &issuer);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("ocsp http client")?;
    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/ocsp-request")
        .header(reqwest::header::ACCEPT, "application/ocsp-response")
        .body(request)
        .send()
        .await
        .with_context(|| format!("OCSP fetch {url}"))?;
    if !response.status().is_success() {
        bail!("OCSP responder returned {}", response.status());
    }
    let bytes = response.bytes().await.context("OCSP response body")?;
    let (der, next_update) = parse_response(&bytes)?;
    Ok(OcspStaple { der, next_update })
}

/// Refresh interval: half of the remaining validity, clamped to [5min, 24h].
pub fn refresh_interval(staple: &OcspStaple, now: SystemTime) -> Duration {
    let Some(next) = staple.next_update else {
        return Duration::from_secs(12 * 3600);
    };
    let remaining = next.duration_since(now).unwrap_or(Duration::ZERO);
    (remaining / 2).clamp(Duration::from_secs(300), Duration::from_secs(24 * 3600))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gt(secs: u64) -> Vec<u8> {
        // 2026-11-23T00:00:00Z style fixed fields derived from epoch seconds.
        let days = secs / 86400;
        let rem = secs % 86400;
        let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
        // civil_from_days (inverse of days_from_civil).
        let z = days as i64 + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        der(
            0x18,
            format!("{y:04}{m:02}{d:02}{hh:02}{mm:02}{ss:02}Z").as_bytes(),
        )
    }

    fn fixture_response(next: Option<Vec<u8>>) -> Vec<u8> {
        let cert_id = seq(&[
            &seq(&[&oid(OID_SHA1), &der(0x05, &[])]),
            &octet(&[1u8; 20]),
            &octet(&[2u8; 20]),
            &integer_be(&[7]),
        ]);
        let mut single_parts: Vec<Vec<u8>> = vec![
            cert_id,
            der(0x80, &[]), // good
            gt(1_763_000_000),
        ];
        if let Some(next) = next {
            single_parts.push(der(0xa0, &next));
        }
        let refs: Vec<&[u8]> = single_parts.iter().map(|v| v.as_slice()).collect();
        let single = seq(&refs);
        let responses = der(0x30, &single);
        let tbs = seq(&[&der(0xa1, &octet(b"responder")), &gt(1_762_990_000), &responses]);
        let staple = seq(&[&tbs, &seq(&[&oid(OID_SHA1), &der(0x05, &[])]), &octet(&[0u8; 64])]);
        let rb = seq(&[&oid(&[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01]), &octet(&staple)]);
        seq(&[&der(0x0a, &[0]), &der(0xa0, &rb)])
    }

    #[test]
    fn response_roundtrip_and_next_update() {
        let next_secs = 1_763_864_000u64;
        let bytes = fixture_response(Some(gt(next_secs)));
        let (staple, next) = parse_response(&bytes).unwrap();
        assert!(!staple.is_empty());
        assert_eq!(next.unwrap(), UNIX_EPOCH + Duration::from_secs(next_secs));
        let (staple2, next2) = parse_response(&fixture_response(None)).unwrap();
        assert!(!staple2.is_empty());
        assert!(next2.is_none());
    }

    #[test]
    fn unsuccessful_status_rejected() {
        let bytes = seq(&[&der(0x0a, &[1])]);
        assert!(parse_response(&bytes).is_err());
    }

    #[test]
    fn request_certid_uses_sha1_of_issuer_name_and_key() {
        let leaf = CertParts {
            serial_der: integer_be(&[0x01, 0x02]),
            subject_der: seq(&[]),
            spki_bits: vec![9u8; 32],
            extensions: vec![],
        };
        let issuer = CertParts {
            serial_der: integer_be(&[1]),
            subject_der: seq(&[&octet(b"issuer")]),
            spki_bits: vec![3u8; 65],
            extensions: vec![],
        };
        let req = build_request(&leaf, &issuer);
        let mut outer = Der::new(&req);
        let tbs = outer.expect(0x30).unwrap();
        let mut tbs = Der::new(tbs);
        let list = tbs.expect(0x30).unwrap();
        let mut list = Der::new(list);
        let request = list.expect(0x30).unwrap();
        let mut request = Der::new(request);
        let cert_id_tlv = request.expect(0x30).unwrap();
        let mut cert_id_outer = Der::new(cert_id_tlv);
        let cert_id = cert_id_outer.expect(0x30).unwrap();
        let mut cert_id = Der::new(cert_id);
        cert_id.expect(0x30).unwrap();
        assert_eq!(cert_id.expect(0x04).unwrap(), sha1(&issuer.subject_der));
        assert_eq!(cert_id.expect(0x04).unwrap(), sha1(&issuer.spki_bits));
        assert_eq!(cert_id.tlv().unwrap().2, integer_be(&[0x01, 0x02]));
    }

    #[test]
    fn aia_extraction_and_refresh_clamp() {
        let uri = b"http://ocsp.example.test";
        let desc = seq(&[&oid(&[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01]), &der(0x86, uri)]);
        let aia = der(0x30, &desc);
        let parts = CertParts {
            serial_der: integer_be(&[1]),
            subject_der: seq(&[]),
            spki_bits: vec![0],
            extensions: vec![(OID_AIA.to_string(), aia)],
        };
        assert_eq!(aia_ocsp_url(&parts).unwrap(), "http://ocsp.example.test");
        let staple = OcspStaple { der: vec![0], next_update: Some(UNIX_EPOCH + Duration::from_secs(9_999_400)) };
        let now = UNIX_EPOCH + Duration::from_secs(9_999_000);
        assert_eq!(refresh_interval(&staple, now), Duration::from_secs(300));
        let far = OcspStaple { der: vec![0], next_update: Some(UNIX_EPOCH + Duration::from_secs(20_000_000)) };
        assert_eq!(refresh_interval(&far, now), Duration::from_secs(24 * 3600));
        let none = OcspStaple { der: vec![0], next_update: None };
        assert_eq!(refresh_interval(&none, now), Duration::from_secs(12 * 3600));
    }

    /// Live responder smoke; run explicitly with `--ignored` when network allows.
    #[tokio::test]
    #[ignore]
    async fn live_fetch_from_public_responder() {
        let leaf_pem = std::fs::read("/tmp/ocsp-leaf.pem").unwrap();
        let issuer_pem = std::fs::read("/tmp/ocsp-issuer.pem").unwrap();
        let leaf = rustls_pemfile::certs(&mut leaf_pem.as_slice()).next().unwrap().unwrap();
        let issuer = rustls_pemfile::certs(&mut issuer_pem.as_slice()).next().unwrap().unwrap();
        let staple = fetch(leaf.as_ref(), issuer.as_ref()).await.unwrap();
        assert!(!staple.der.is_empty());
        assert!(staple.next_update.is_some());
    }
}