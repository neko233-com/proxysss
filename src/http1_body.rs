//! Decode an HTTP/1 response body before handing it to an HTTP/1 or HTTP/2
//! encoder. Input stays bounded by the caller's read buffer; data is sliced,
//! never accumulated to a complete chunk or SSE event.

use anyhow::{anyhow, bail, Result};
use bytes::{Buf, Bytes};
#[cfg(test)]
use http::header;
use http::{HeaderName, HeaderValue, Method, StatusCode};

const LINE_LIMIT: usize = 16 * 1024;
const TRAILER_LIMIT: usize = 64 * 1024;

/// Shared allocation-free framing validation for raw response fast paths.
pub(super) fn response_framing<'a>(
    headers: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<(Option<u64>, bool)> {
    let mut length = None;
    let mut chunked = false;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked
                || !std::str::from_utf8(value)?
                    .trim()
                    .eq_ignore_ascii_case("chunked")
            {
                bail!("unsupported upstream transfer encoding");
            }
            chunked = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            for field in std::str::from_utf8(value)?.split(',') {
                let field = field.trim();
                if field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()) {
                    bail!("invalid upstream content length");
                }
                let parsed = field.parse::<u64>()?;
                if length.is_some_and(|previous| previous != parsed) {
                    bail!("conflicting upstream content lengths");
                }
                length = Some(parsed);
            }
        }
    }
    if chunked && length.is_some() {
        bail!("ambiguous upstream transfer encoding and content length");
    }
    Ok((length, chunked))
}

enum State {
    CloseDelimited,
    Fixed(u64),
    Size,
    Data(u64),
    DataCrLf(usize),
    Trailers,
    Done,
}

pub(super) struct ResponseBodyDecoder {
    state: State,
    line: Vec<u8>,
    trailer_bytes: usize,
}

impl ResponseBodyDecoder {
    pub(super) fn new(
        method: &Method,
        status: StatusCode,
        headers: &[(HeaderName, HeaderValue)],
    ) -> Result<Self> {
        let mut decoder = Self {
            state: State::CloseDelimited,
            line: Vec::new(),
            trailer_bytes: 0,
        };
        if method == Method::HEAD
            || status.is_informational()
            || status == StatusCode::NO_CONTENT
            || status == StatusCode::NOT_MODIFIED
        {
            decoder.state = State::Done;
            return Ok(decoder);
        }
        let (length, chunked) = response_framing(
            headers
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_bytes())),
        )?;
        decoder.state = if chunked {
            State::Size
        } else {
            match length {
                Some(0) => State::Done,
                Some(length) => State::Fixed(length),
                None => State::CloseDelimited,
            }
        };
        Ok(decoder)
    }

    pub(super) fn is_done(&self) -> bool {
        matches!(self.state, State::Done)
    }

    pub(super) fn finish_eof(&mut self) -> Result<()> {
        if !matches!(self.state, State::Done | State::CloseDelimited) {
            bail!("upstream closed before the response body was complete");
        }
        self.state = State::Done;
        Ok(())
    }

    pub(super) fn next_data(&mut self, input: &mut Bytes) -> Result<Option<Bytes>> {
        while !input.is_empty() {
            match self.state {
                State::CloseDelimited => return Ok(Some(input.split_to(input.len()))),
                State::Fixed(remaining) | State::Data(remaining) => {
                    let count = remaining.min(input.len() as u64) as usize;
                    let rest = remaining - count as u64;
                    self.state = match self.state {
                        State::Fixed(_) if rest == 0 => State::Done,
                        State::Fixed(_) => State::Fixed(rest),
                        _ if rest == 0 => State::DataCrLf(0),
                        _ => State::Data(rest),
                    };
                    return Ok(Some(input.split_to(count)));
                }
                State::Size | State::Trailers => {
                    let newline = input.iter().position(|byte| *byte == b'\n');
                    let count = newline.map_or(input.len(), |index| index + 1);
                    if self.line.len().saturating_add(count) > LINE_LIMIT {
                        bail!("upstream chunk framing line exceeded 16KiB");
                    }
                    if matches!(self.state, State::Trailers) {
                        self.trailer_bytes += count;
                        if self.trailer_bytes > TRAILER_LIMIT {
                            bail!("upstream chunk trailers exceeded 64KiB");
                        }
                    }
                    self.line.extend_from_slice(&input[..count]);
                    input.advance(count);
                    if newline.is_none() {
                        return Ok(None);
                    }
                    if !self.line.ends_with(b"\r\n") {
                        bail!("invalid upstream chunk framing line terminator");
                    }
                    let line = &self.line[..self.line.len() - 2];
                    self.state = if matches!(self.state, State::Size) {
                        let size = parse_size(line)?;
                        if size == 0 {
                            State::Trailers
                        } else {
                            State::Data(size)
                        }
                    } else if line.is_empty() {
                        State::Done
                    } else {
                        let colon = line
                            .iter()
                            .position(|byte| *byte == b':')
                            .ok_or_else(|| anyhow!("invalid upstream chunk trailer"))?;
                        HeaderName::from_bytes(&line[..colon])?;
                        HeaderValue::from_bytes(&line[colon + 1..])?;
                        // Trailer fields are not merged into response headers
                        // or emitted as SSE data. Their storage remains bounded.
                        State::Trailers
                    };
                    self.line.clear();
                }
                State::DataCrLf(seen) => {
                    if input[0] != b"\r\n"[seen] {
                        bail!("invalid upstream chunk body terminator");
                    }
                    input.advance(1);
                    self.state = if seen == 1 {
                        State::Size
                    } else {
                        State::DataCrLf(1)
                    };
                }
                State::Done => return Ok(None),
            }
        }
        Ok(None)
    }
}

fn parse_size(line: &[u8]) -> Result<u64> {
    let hex = line.split(|byte| *byte == b';').next().unwrap_or_default();
    let hex = hex.trim_ascii_end();
    if hex.is_empty() || !hex.iter().all(u8::is_ascii_hexdigit) {
        bail!("invalid upstream chunk size");
    }
    Ok(u64::from_str_radix(std::str::from_utf8(hex)?, 16)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunked() -> ResponseBodyDecoder {
        ResponseBodyDecoder::new(
            &Method::GET,
            StatusCode::OK,
            &[(
                header::TRANSFER_ENCODING,
                HeaderValue::from_static("chunked"),
            )],
        )
        .unwrap()
    }

    #[test]
    fn chunk_decoder_handles_every_fragmentation_and_preserves_only_data() {
        let encoded = b"3;id=one\r\nabc\r\n2\r\nde\r\n0\r\nx-result: done\r\n\r\n";
        for width in 1..=encoded.len() {
            let mut decoder = chunked();
            let mut body = Vec::new();
            for part in encoded.chunks(width) {
                let mut input = Bytes::copy_from_slice(part);
                while let Some(data) = decoder.next_data(&mut input).unwrap() {
                    body.extend_from_slice(&data);
                }
            }
            assert!(decoder.is_done(), "fragment width {width}");
            assert_eq!(body, b"abcde");
        }
    }

    #[test]
    fn incomplete_and_oversized_chunk_framing_fails_without_body_buffering() {
        for data in [
            b"3\r\nab".as_slice(),
            b"1\r\na\r\n0\r\nx: y\r\n",
            b"0\r\n\r",
        ] {
            let mut decoder = chunked();
            let mut input = Bytes::copy_from_slice(data);
            while decoder.next_data(&mut input).unwrap().is_some() {}
            assert!(decoder.finish_eof().is_err());
        }
        for data in [b"1\na\r\n0\r\n\r\n".as_slice(), b"+1\r\na\r\n", b"1\r\naXX"] {
            let mut decoder = chunked();
            let mut input = Bytes::copy_from_slice(data);
            let result: Result<()> = (|| {
                while decoder.next_data(&mut input)?.is_some() {}
                Ok(())
            })();
            assert!(result.is_err());
        }
        let mut decoder = chunked();
        assert!(decoder
            .next_data(&mut Bytes::from(vec![b'0'; LINE_LIMIT + 1]))
            .is_err());
        let mut decoder = chunked();
        let mut input = Bytes::from_static(b"ffffffffffffffff\r\na");
        assert_eq!(
            decoder.next_data(&mut input).unwrap().unwrap(),
            b"a".as_slice()
        );
        assert!(
            decoder.line.capacity() < 128,
            "declared chunk size cannot reserve body memory"
        );
        let mut decoder = chunked();
        let mut trailers = b"0\r\n".to_vec();
        for _ in 0..16 {
            trailers.extend_from_slice(b"x: ");
            trailers.extend(std::iter::repeat_n(b'a', 4096));
            trailers.extend_from_slice(b"\r\n");
        }
        assert!(decoder.next_data(&mut Bytes::from(trailers)).is_err());
    }

    #[test]
    fn fixed_empty_and_close_delimited_bodies_obey_message_boundaries() {
        let mut decoder = ResponseBodyDecoder::new(
            &Method::GET,
            StatusCode::OK,
            &[(header::CONTENT_LENGTH, HeaderValue::from_static("3"))],
        )
        .unwrap();
        let mut input = Bytes::from_static(b"abcnext");
        assert_eq!(
            decoder.next_data(&mut input).unwrap().unwrap(),
            b"abc".as_slice()
        );
        assert!(decoder.is_done());
        assert_eq!(input, b"next".as_slice());
        for (method, status) in [
            (Method::HEAD, StatusCode::OK),
            (Method::GET, StatusCode::NO_CONTENT),
        ] {
            assert!(ResponseBodyDecoder::new(&method, status, &[])
                .unwrap()
                .is_done());
        }
        let mut decoder = ResponseBodyDecoder::new(&Method::GET, StatusCode::OK, &[]).unwrap();
        assert!(!decoder.is_done());
        decoder.finish_eof().unwrap();
        assert!(decoder.is_done());
        assert!(ResponseBodyDecoder::new(
            &Method::GET,
            StatusCode::OK,
            &[
                (
                    header::TRANSFER_ENCODING,
                    HeaderValue::from_static("chunked")
                ),
                (header::CONTENT_LENGTH, HeaderValue::from_static("3"))
            ]
        )
        .is_err());
    }
}
