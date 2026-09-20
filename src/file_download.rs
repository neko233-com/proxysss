//! Shared bounded downloads for writable file services. Callers authorize and
//! confine the path before entering this module; no file contents are cached.
use super::{file_io::FileVersion, static_ranges, GatewayHttpResponse};
use anyhow::Result;
use bytes::Bytes;
use http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use std::fs::Metadata;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct Validators {
    etag: String,
    modified: Option<SystemTime>,
}

impl Validators {
    pub(super) fn new(metadata: &Metadata) -> Self {
        let modified = metadata.modified().ok();
        let mut hash = DefaultHasher::new();
        FileVersion::new(metadata).hash(&mut hash);
        let etag = format!("W/\"{:x}-{:x}\"", metadata.len(), hash.finish());
        Self { etag, modified }
    }

    fn unmodified_since(&self, value: &HeaderValue) -> Option<bool> {
        value
            .to_str()
            .ok()
            .and_then(|v| httpdate::parse_http_date(v).ok())
            .zip(self.modified)
            .map(|(date, time)| {
                time.duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    <= date
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
            })
    }

    pub(super) fn status(&self, headers: &HeaderMap) -> Option<StatusCode> {
        // Metadata ETags are weak: only '*' satisfies If-Match strong comparison.
        let failed = if let Some(value) = headers.get(header::IF_MATCH) {
            !value.to_str().is_ok_and(|v| v.trim() == "*")
        } else {
            headers
                .get(header::IF_UNMODIFIED_SINCE)
                .and_then(|v| self.unmodified_since(v))
                == Some(false)
        };
        if failed {
            return Some(StatusCode::PRECONDITION_FAILED);
        }
        let not_modified = if let Some(value) = headers.get(header::IF_NONE_MATCH) {
            value.to_str().is_ok_and(|v| {
                v.split(',').any(|v| {
                    v.trim() == "*"
                        || v.trim().trim_start_matches("W/") == self.etag.trim_start_matches("W/")
                })
            })
        } else {
            headers
                .get(header::IF_MODIFIED_SINCE)
                .and_then(|v| self.unmodified_since(v))
                == Some(true)
        };
        not_modified.then_some(StatusCode::NOT_MODIFIED)
    }

    pub(super) fn allows_range(&self, headers: &HeaderMap) -> bool {
        headers.get(header::IF_RANGE).is_none_or(|value| {
            value
                .to_str()
                .ok()
                .and_then(|v| httpdate::parse_http_date(v).ok())
                .zip(self.modified)
                .is_some_and(|(date, time)| {
                    httpdate::fmt_http_date(date) == httpdate::fmt_http_date(time)
                        && time.elapsed().is_ok_and(|age| age.as_secs() >= 60)
                })
        })
    }

    pub(super) fn append(&self, response: &mut GatewayHttpResponse) -> Result<()> {
        response
            .headers
            .push((header::ETAG, HeaderValue::from_str(&self.etag)?));
        if let Some(modified) = self.modified {
            response.headers.push((
                header::LAST_MODIFIED,
                HeaderValue::from_str(&httpdate::fmt_http_date(modified))?,
            ));
        }
        Ok(())
    }
}

pub(crate) async fn response(
    target: &Path,
    method: &Method,
    headers: &HeaderMap,
    content_type: &str,
    upstream: &str,
) -> Result<GatewayHttpResponse> {
    if method != Method::GET && method != Method::HEAD {
        let mut response = GatewayHttpResponse::error(
            StatusCode::METHOD_NOT_ALLOWED,
            "file downloads require GET or HEAD",
        );
        response
            .headers
            .push((header::ALLOW, HeaderValue::from_static("GET, HEAD")));
        return Ok(response);
    }
    let metadata = match tokio::fs::metadata(target).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "file not found",
            ))
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Ok(GatewayHttpResponse::error(
            StatusCode::FORBIDDEN,
            "only regular files can be downloaded",
        ));
    }
    let validators = Validators::new(&metadata);
    let mut response = if let Some(status) = validators.status(headers) {
        GatewayHttpResponse::bytes(status, content_type, Bytes::new(), upstream)
    } else if method == Method::HEAD || metadata.len() == 0 && !headers.contains_key(header::RANGE)
    {
        let mut response =
            GatewayHttpResponse::bytes(StatusCode::OK, content_type, Bytes::new(), upstream);
        response.headers.push((
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&metadata.len().to_string())?,
        ));
        response
    } else {
        let range = if validators.allows_range(headers) {
            static_ranges::parse(headers, metadata.len())
        } else {
            static_ranges::RangeDecision::NoRange
        };
        match range {
            static_ranges::RangeDecision::Unsatisfiable => {
                let mut response = GatewayHttpResponse::error(
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    "range not satisfiable",
                );
                response.headers.push((
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes */{}", metadata.len()))?,
                ));
                response
            }
            static_ranges::RangeDecision::Ranges(ranges) => {
                static_ranges::response_typed(target, &metadata, ranges, content_type, true).await?
            }
            static_ranges::RangeDecision::NoRange if metadata.len() == 0 => {
                let mut response = GatewayHttpResponse::bytes(
                    StatusCode::OK,
                    content_type,
                    Bytes::new(),
                    upstream,
                );
                response
                    .headers
                    .push((header::CONTENT_LENGTH, HeaderValue::from_static("0")));
                response
            }
            static_ranges::RangeDecision::NoRange => {
                static_ranges::response_typed(
                    target,
                    &metadata,
                    vec![static_ranges::ByteRange {
                        start: 0,
                        end: metadata.len() - 1,
                    }],
                    content_type,
                    false,
                )
                .await?
            }
        }
    };
    response.upstream = upstream.to_string();
    if !response
        .headers
        .iter()
        .any(|(name, _)| name == header::ACCEPT_RANGES)
    {
        response
            .headers
            .push((header::ACCEPT_RANGES, HeaderValue::from_static("bytes")));
    }
    validators.append(&mut response)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn file_download_range_head_and_conditions_share_exact_semantics() {
        let fixture = TestDir::new("file-download");
        let path = fixture.join("asset.bin");
        tokio::fs::write(&path, b"0123456789").await.unwrap();
        let empty = HeaderMap::new();
        let full = response(
            &path,
            &Method::GET,
            &empty,
            "application/octet-stream",
            "test",
        )
        .await
        .unwrap();
        assert_eq!(full.status, StatusCode::OK);
        assert_eq!(full.body.as_ref(), b"0123456789");
        let etag = full
            .headers
            .iter()
            .find(|(name, _)| name == header::ETAG)
            .unwrap()
            .1
            .clone();
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, "bytes=8-9,0-1".parse().unwrap());
        let mut partial = response(&path, &Method::GET, &headers, "application/custom", "test")
            .await
            .unwrap();
        assert_eq!(partial.status, StatusCode::PARTIAL_CONTENT);
        let body = partial
            .stream_body
            .take()
            .unwrap()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("Content-Type: application/custom"));
        assert!(text.contains("\r\n89\r\n"));
        assert!(text.contains("\r\n01\r\n"));
        let head = response(&path, &Method::HEAD, &headers, "application/custom", "test")
            .await
            .unwrap();
        assert_eq!(head.status, StatusCode::OK);
        assert!(head.body.is_empty() && head.stream_body.is_none());
        assert!(head
            .headers
            .iter()
            .any(|(name, value)| name == header::CONTENT_LENGTH && value == "10"));
        headers.insert(header::IF_NONE_MATCH, etag.clone());
        assert_eq!(
            response(&path, &Method::GET, &headers, "application/custom", "test")
                .await
                .unwrap()
                .status,
            StatusCode::NOT_MODIFIED
        );
        headers.insert(header::IF_MATCH, etag.clone());
        assert_eq!(
            response(&path, &Method::GET, &headers, "application/custom", "test")
                .await
                .unwrap()
                .status,
            StatusCode::PRECONDITION_FAILED
        );
        headers.remove(header::IF_MATCH);
        headers.remove(header::IF_NONE_MATCH);
        headers.insert(header::IF_RANGE, etag);
        assert_eq!(
            response(&path, &Method::GET, &headers, "application/custom", "test")
                .await
                .unwrap()
                .status,
            StatusCode::OK
        );
        headers.remove(header::IF_RANGE);
        headers.insert(header::RANGE, "bytes=100-".parse().unwrap());
        assert_eq!(
            response(&path, &Method::GET, &headers, "application/custom", "test")
                .await
                .unwrap()
                .status,
            StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            response(&path, &Method::POST, &empty, "application/custom", "test")
                .await
                .unwrap()
                .status,
            StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn file_download_large_full_response_is_pull_driven_and_fails_on_truncation() {
        let fixture = TestDir::new("file-download-stream");
        let path = fixture.join("large.bin");
        let file = tokio::fs::File::create(&path).await.unwrap();
        file.set_len(8 * 1024 * 1024).await.unwrap();
        drop(file);
        let mut result = response(
            &path,
            &Method::GET,
            &HeaderMap::new(),
            "application/octet-stream",
            "test",
        )
        .await
        .unwrap();
        assert!(result.body.is_empty());
        let mut body = result.stream_body.take().unwrap();
        let frame = body.frame().await.unwrap().unwrap();
        assert_eq!(frame.into_data().unwrap().len(), 64 * 1024);
        tokio::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .await
            .unwrap()
            .set_len(0)
            .await
            .unwrap();
        assert!(body.frame().await.unwrap().is_err());
        drop(body);
    }
}
