//! Bounded byte-range parsing and pull-driven multipart file downloads.
use super::{static_content_type, ByteBufferPool, GatewayBody, GatewayHttpResponse, PooledBuffer};
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures::{stream, TryStreamExt};
use http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use http::{HeaderMap, HeaderValue, StatusCode};
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

const MAX_RANGES: usize = 16;
const CHUNK_BYTES: usize = 64 * 1024;

fn download_pool() -> &'static ByteBufferPool {
    static POOL: OnceLock<ByteBufferPool> = OnceLock::new();
    // At most 8 MiB of idle buffers, independent of file and connection counts.
    POOL.get_or_init(|| ByteBufferPool::new(128, CHUNK_BYTES))
}

struct Chunk(PooledBuffer, usize);

impl AsRef<[u8]> for Chunk {
    fn as_ref(&self) -> &[u8] {
        &self.0[..self.1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ByteRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum RangeDecision {
    NoRange,
    Ranges(Vec<ByteRange>),
    Unsatisfiable,
}

// Saturation preserves the meaning of arbitrarily large decimal offsets:
// an oversized end/suffix includes the file, an oversized start is unsatisfied.
fn decimal(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(value.bytes().fold(0_u64, |n, byte| {
        n.saturating_mul(10).saturating_add(u64::from(byte - b'0'))
    }))
}

/// Ignore unsupported/malformed or excessive ranges; reject only a valid set
/// with no satisfiable member. Coalesce overlap/adjacency to bound amplification.
pub(super) fn parse(headers: &HeaderMap, file_len: u64) -> RangeDecision {
    let mut values = headers.get_all(RANGE).iter();
    let Some(value) = values.next() else {
        return RangeDecision::NoRange;
    };
    // Range is not a list of independently repeated units.
    if values.next().is_some() {
        return RangeDecision::NoRange;
    }
    let Ok(value) = value.to_str() else {
        return RangeDecision::NoRange;
    };
    let Some((unit, spec)) = value.trim().split_once('=') else {
        return RangeDecision::NoRange;
    };
    if !unit.eq_ignore_ascii_case("bytes") {
        return RangeDecision::NoRange;
    }
    let mut ranges: Vec<(usize, ByteRange)> = Vec::new();
    let mut members = 0;
    for (index, member) in spec.split(',').enumerate() {
        if index >= MAX_RANGES * 2 {
            return RangeDecision::NoRange;
        }
        let member = member.trim();
        if member.is_empty() {
            continue;
        }
        members += 1;
        if members > MAX_RANGES {
            return RangeDecision::NoRange;
        }
        let Some((first, last)) = member.split_once('-') else {
            return RangeDecision::NoRange;
        };
        let range = if first.is_empty() {
            let Some(suffix) = decimal(last) else {
                return RangeDecision::NoRange;
            };
            if file_len == 0 || suffix == 0 {
                continue;
            }
            ByteRange {
                start: file_len.saturating_sub(suffix),
                end: file_len - 1,
            }
        } else {
            let Some(start) = decimal(first) else {
                return RangeDecision::NoRange;
            };
            let end = if last.is_empty() {
                u64::MAX
            } else if let Some(end) = decimal(last) {
                end
            } else {
                return RangeDecision::NoRange;
            };
            if start > end {
                return RangeDecision::NoRange;
            }
            if start >= file_len {
                continue;
            }
            ByteRange {
                start,
                end: end.min(file_len - 1),
            }
        };
        ranges.push((index, range));
    }
    if members == 0 {
        return RangeDecision::NoRange;
    }
    if ranges.is_empty() {
        return RangeDecision::Unsatisfiable;
    }
    ranges.sort_unstable_by_key(|(_, range)| range.start);
    let mut merged: Vec<(usize, ByteRange)> = Vec::with_capacity(ranges.len());
    for (index, range) in ranges {
        if let Some((first_index, previous)) = merged.last_mut() {
            if range.start <= previous.end.saturating_add(1) {
                previous.end = previous.end.max(range.end);
                *first_index = (*first_index).min(index);
                continue;
            }
        }
        merged.push((index, range));
    }
    merged.sort_unstable_by_key(|(index, _)| *index);
    RangeDecision::Ranges(merged.into_iter().map(|(_, range)| range).collect())
}

struct Part {
    range: ByteRange,
    header: Bytes,
}

struct Download {
    file: tokio::fs::File,
    parts: std::vec::IntoIter<Part>,
    remaining: u64,
    ending: Option<Bytes>,
}

/// One open file owns the whole response. Reads occur only as the HTTP body is
/// polled; cancellation drops the descriptor without an independent producer.
pub(super) async fn response(
    target: &Path,
    metadata: &std::fs::Metadata,
    ranges: Vec<ByteRange>,
) -> Result<GatewayHttpResponse> {
    response_typed(target, metadata, ranges, static_content_type(target), true).await
}

pub(super) async fn response_typed(
    target: &Path,
    metadata: &std::fs::Metadata,
    ranges: Vec<ByteRange>,
    content_type: &str,
    partial: bool,
) -> Result<GatewayHttpResponse> {
    let file_len = metadata.len();
    let mut response = GatewayHttpResponse::bytes(
        if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        },
        content_type,
        Bytes::new(),
        "proxysss://static-range",
    );
    let mut file = super::file_io::open_checked(target, metadata)
        .await
        .context("open static range file")?;
    let mut parts = Vec::with_capacity(ranges.len());
    let mut length = 0_u64;
    let ending;
    if ranges.len() == 1 {
        let range = ranges[0];
        length = range.end - range.start + 1;
        if partial {
            response.headers.push((
                CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes {}-{}/{file_len}", range.start, range.end))?,
            ));
        }
        if length <= CHUNK_BYTES as u64 {
            file.seek(SeekFrom::Start(range.start)).await?;
            let mut data = vec![0_u8; length as usize];
            file.read_exact(&mut data)
                .await
                .context("static range file truncated")?;
            response.body = Bytes::from(data);
        } else {
            parts.push(Part {
                range,
                header: Bytes::new(),
            });
        }
        ending = None;
    } else {
        let boundary = format!("proxysss-{}", uuid::Uuid::new_v4().simple());
        response.headers.retain(|(name, _)| name != CONTENT_TYPE);
        response.headers.push((
            CONTENT_TYPE,
            HeaderValue::from_str(&format!("multipart/byteranges; boundary={boundary}"))?,
        ));
        for (index, range) in ranges.into_iter().enumerate() {
            let prefix = if index == 0 { "" } else { "\r\n" };
            let header = Bytes::from(format!(
                "{prefix}--{boundary}\r\nContent-Type: {content_type}\r\nContent-Range: bytes {}-{}/{file_len}\r\n\r\n",
                range.start, range.end,
            ));
            length = length
                .checked_add(header.len() as u64)
                .and_then(|n| n.checked_add(range.end - range.start + 1))
                .ok_or_else(|| anyhow!("multipart range length overflow"))?;
            parts.push(Part { range, header });
        }
        let trailer = Bytes::from(format!("\r\n--{boundary}--\r\n"));
        length = length
            .checked_add(trailer.len() as u64)
            .ok_or_else(|| anyhow!("multipart range length overflow"))?;
        ending = Some(trailer);
    }
    if !parts.is_empty() {
        let state = Download {
            file,
            parts: parts.into_iter(),
            remaining: 0,
            ending,
        };
        let stream = stream::try_unfold(state, |mut state| async move {
            loop {
                if state.remaining != 0 {
                    let size = state.remaining.min(CHUNK_BYTES as u64) as usize;
                    let mut bytes = download_pool().acquire();
                    state
                        .file
                        .read_exact(&mut bytes[..size])
                        .await
                        .context("static range file truncated")?;
                    state.remaining -= size as u64;
                    return Ok::<_, anyhow::Error>(Some((
                        Bytes::from_owner(Chunk(bytes, size)),
                        state,
                    )));
                }
                if let Some(part) = state.parts.next() {
                    state.file.seek(SeekFrom::Start(part.range.start)).await?;
                    state.remaining = part.range.end - part.range.start + 1;
                    if !part.header.is_empty() {
                        return Ok(Some((part.header, state)));
                    }
                    continue;
                }
                return Ok(state.ending.take().map(|ending| (ending, state)));
            }
        });
        response.stream_body = Some(GatewayBody::Stream(
            StreamBody::new(stream.map_ok(Frame::data)).boxed_unsync(),
        ));
    }
    response
        .headers
        .push((ACCEPT_RANGES, HeaderValue::from_static("bytes")));
    response
        .headers
        .push((CONTENT_LENGTH, HeaderValue::from_str(&length.to_string())?));
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    fn decision(value: &str, len: u64) -> RangeDecision {
        let mut headers = HeaderMap::new();
        headers.insert(RANGE, value.parse().unwrap());
        parse(&headers, len)
    }

    #[test]
    fn ranges_bound_work_and_preserve_download_semantics() {
        let cases = [
            ("bytes=2-5", vec![(2, 5)]),
            ("bytes=7-", vec![(7, 9)]),
            ("bytes=-3", vec![(7, 9)]),
            ("BYTES=1-2", vec![(1, 2)]),
            ("bytes=9-999999999999999999999999999999", vec![(9, 9)]),
            ("bytes=-999999999999999999999999999999", vec![(0, 9)]),
            ("bytes=99-100,2-3,-2", vec![(2, 3), (8, 9)]),
            ("bytes=8-9,0-1,4-5", vec![(8, 9), (0, 1), (4, 5)]),
            ("bytes=8-9,0-2,2-4,5-8", vec![(0, 9)]),
            ("bytes=, 0-1, , 8-9,", vec![(0, 1), (8, 9)]),
        ];
        for (value, expected) in cases {
            assert_eq!(
                decision(value, 10),
                RangeDecision::Ranges(
                    expected
                        .into_iter()
                        .map(|(start, end)| ByteRange { start, end })
                        .collect()
                ),
                "{value}"
            );
        }
        for value in [
            "items=0-1",
            "bytes=",
            "bytes=2-1",
            "bytes=a-1",
            "bytes=+1-2",
            "bytes=0-1,broken",
            "bytes=1 -2",
        ] {
            assert_eq!(decision(value, 10), RangeDecision::NoRange, "{value}");
        }
        for value in [
            "bytes=99-100",
            "bytes=-0",
            "bytes=99999999999999999999999999-",
            "bytes=99-,-0",
        ] {
            assert_eq!(decision(value, 10), RangeDecision::Unsatisfiable, "{value}");
        }
        assert_eq!(decision("bytes=0-", 0), RangeDecision::Unsatisfiable);
        let many = format!("bytes={}", vec!["0-1"; MAX_RANGES + 1].join(","));
        assert_eq!(decision(&many, 10), RangeDecision::NoRange);
        let mut headers = HeaderMap::new();
        headers.append(RANGE, HeaderValue::from_static("bytes=0-1"));
        headers.append(RANGE, HeaderValue::from_static("bytes=8-9"));
        assert_eq!(parse(&headers, 10), RangeDecision::NoRange);
    }

    #[tokio::test]
    async fn sparse_multi_gigabyte_file_reads_only_requested_tail() {
        let root = TestDir::new("range-sparse");
        let path = root.join("large.bin");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        // Linux sparse files exercise >32-bit offsets without allocating 4 GiB.
        // Windows functional checks use a smaller file without FSCTL_SET_SPARSE.
        let size = if cfg!(target_os = "linux") {
            4_u64 * 1024 * 1024 * 1024
        } else {
            64 * 1024 * 1024
        };
        file.set_len(size).await.unwrap();
        file.seek(SeekFrom::Start(size - 4)).await.unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut file, b"tail")
            .await
            .unwrap();
        tokio::io::AsyncWriteExt::flush(&mut file).await.unwrap();
        drop(file);
        let result = response(
            &path,
            &std::fs::metadata(&path).unwrap(),
            vec![ByteRange {
                start: size - 4,
                end: size - 1,
            }],
        )
        .await
        .unwrap();
        assert_eq!(result.body.as_ref(), b"tail");
        assert!(result.stream_body.is_none());
    }

    #[tokio::test]
    async fn multipart_framing_and_length_match_selected_bytes() {
        let root = TestDir::new("range-multipart");
        let path = root.join("asset.bin");
        tokio::fs::write(&path, b"0123456789").await.unwrap();
        let mut result = response(
            &path,
            &std::fs::metadata(&path).unwrap(),
            vec![
                ByteRange { start: 8, end: 9 },
                ByteRange { start: 0, end: 1 },
            ],
        )
        .await
        .unwrap();
        assert!(!result.headers.iter().any(|(name, _)| name == CONTENT_RANGE));
        let content_type = result
            .headers
            .iter()
            .find(|(name, _)| name == CONTENT_TYPE)
            .unwrap()
            .1
            .to_str()
            .unwrap();
        let boundary = content_type
            .strip_prefix("multipart/byteranges; boundary=")
            .unwrap();
        let expected = format!("--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes 8-9/10\r\n\r\n89\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes 0-1/10\r\n\r\n01\r\n--{boundary}--\r\n");
        let length = result
            .headers
            .iter()
            .find(|(name, _)| name == CONTENT_LENGTH)
            .unwrap()
            .1
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let body = result
            .stream_body
            .take()
            .unwrap()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(body.as_ref(), expected.as_bytes());
        assert_eq!(length, body.len());
    }

    #[tokio::test]
    async fn truncation_is_an_error_instead_of_panic_or_successful_short_body() {
        let root = TestDir::new("range-truncated");
        let path = root.join("asset.bin");
        tokio::fs::write(&path, b"0123456789").await.unwrap();
        let old_metadata = std::fs::metadata(&path).unwrap();
        tokio::fs::write(&path, b"0123").await.unwrap();
        assert!(
            response(&path, &old_metadata, vec![ByteRange { start: 8, end: 9 }])
                .await
                .is_err()
        );
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(200_000)
            .unwrap();
        let result = response(
            &path,
            &std::fs::metadata(&path).unwrap(),
            vec![ByteRange {
                start: 0,
                end: 199_999,
            }],
        )
        .await
        .unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(4)
            .unwrap();
        assert!(result.stream_body.unwrap().collect().await.is_err());
    }

    #[tokio::test]
    async fn large_range_streams_bounded_frames_and_stops_at_exact_end() {
        let root = TestDir::new("range-stream");
        let path = root.join("asset.bin");
        let data: Vec<u8> = (0..300_000).map(|index| (index % 251) as u8).collect();
        tokio::fs::write(&path, &data).await.unwrap();
        let result = response(
            &path,
            &std::fs::metadata(&path).unwrap(),
            vec![ByteRange {
                start: 17,
                end: 200_019,
            }],
        )
        .await
        .unwrap();
        let mut body = result.stream_body.unwrap();
        let mut received = Vec::new();
        while let Some(frame) = body.frame().await {
            let bytes = frame.unwrap().into_data().unwrap();
            assert!(bytes.len() <= CHUNK_BYTES);
            received.extend_from_slice(&bytes);
        }
        assert_eq!(received, &data[17..=200_019]);
    }
}
