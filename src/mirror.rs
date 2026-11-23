//! nginx `mirror` parity: bounded, fire-and-forget shadow traffic. The mirror
//! request never blocks or alters the primary response; failures are dropped.
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, Method};
use tokio::sync::OnceCell;

static CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

async fn client() -> &'static reqwest::Client {
    CLIENT
        .get_or_init(|| async {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("mirror client")
        })
        .await
}

pub fn mirror_url(upstream: &str, path_and_query: &str) -> String {
    format!("{}{}", upstream.trim_end_matches('/'), path_and_query)
}

/// Hop-by-hop and framing headers must not be copied to the shadow request.
pub fn mirror_headers(headers: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (name, value) in headers {
        let n = name.as_str();
        if matches!(
            n,
            "host" | "connection" | "content-length" | "transfer-encoding" | "keep-alive" | "upgrade"
        ) {
            continue;
        }
        out.append(name.clone(), value.clone());
    }
    out
}

pub fn spawn(upstream: &str, method: &Method, path_and_query: &str, headers: &HeaderMap, body: Bytes) {
    if upstream.trim().is_empty() {
        return;
    }
    let url = mirror_url(upstream, path_and_query);
    let method = method.clone();
    let headers = mirror_headers(headers);
    tokio::spawn(async move {
        let client = client().await;
        let mut request = client.request(method, url).body(body);
        for (name, value) in &headers {
            request = request.header(name.clone(), value.clone());
        }
        let _ = request.send().await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_join_and_header_filter() {
        assert_eq!(mirror_url("http://127.0.0.1:9000/", "/a?b=1"), "http://127.0.0.1:9000/a?b=1");
        let mut h = HeaderMap::new();
        h.insert(http::header::HOST, "x".parse().unwrap());
        h.insert(http::header::CONTENT_LENGTH, "1".parse().unwrap());
        h.insert("x-keep", "yes".parse().unwrap());
        let out = mirror_headers(&h);
        assert!(out.get("host").is_none());
        assert!(out.get("content-length").is_none());
        assert_eq!(out.get("x-keep").unwrap(), "yes");
    }
}