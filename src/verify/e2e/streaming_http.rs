use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::test_support::TestDir;
use crate::verify::harness::{base_gateway_yaml, reserve_port, spawn_gateway, write_config};

#[tokio::test]
async fn integration_e2e_chunked_sse_has_exact_body_and_finishes_on_keepalive() -> Result<()> {
    let root = TestDir::new("chunked-sse-framing");
    let upstream_port = reserve_port().await?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, upstream_port)).await?;
    let upstream = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                loop {
                    let mut request = Vec::new();
                    let mut buffer = [0; 1024];
                    while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        let read = stream.read(&mut buffer).await?;
                        if read == 0 || request.len() > 16384 {
                            return Ok::<_, std::io::Error>(());
                        }
                        request.extend_from_slice(&buffer[..read]);
                    }
                    if request.windows(6).any(|bytes| bytes == b"/gzip ") {
                        let mut encoder =
                            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                        std::io::Write::write_all(&mut encoder, b"data: encoded\n\n")?;
                        let encoded = encoder.finish()?;
                        let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Encoding: gzip\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n", encoded.len());
                        stream.write_all(head.as_bytes()).await?;
                        stream.write_all(&encoded).await?;
                        continue;
                    }
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\nTrailer: x-finished\r\n\r\n").await?;
                    // Split an SSE event across chunks and a chunk delimiter across
                    // writes: neither HTTP framing nor SSE boundaries are read boundaries.
                    stream.write_all(b"6;part=first\r\ndata: \r").await?;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    stream.write_all(b"\n7\r\nfirst\n\n\r\n").await?;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    stream
                        .write_all(b"e\r\ndata: second\n\n\r\n0\r\nx-finished: yes\r\n\r\n")
                        .await?;
                    // Keep serving the connection. End of HTTP body, not TCP EOF,
                    // must finish downstream consumption and allow pool reuse.
                }
            });
        }
    });
    let plain = reserve_port().await?;
    let tls = reserve_port().await?;
    let base = base_gateway_yaml(plain)
        .replace("tls_bind: ''", &format!("tls_bind: 127.0.0.1:{tls}"))
        + "  retries:\n    enabled: false\n  passive_health:\n    enabled: false\naffinity:\n  enabled: false\n";
    let yaml = format!("{base}\nservices:\n  ai_proxy:\n    enabled: true\n    routes:\n      - name: stream\n        provider: new-api\n        path_prefix: /v1\n        upstream: http://127.0.0.1:{upstream_port}\n  reverse_proxy:\n    routes:\n      - name: generic-stream\n        path_prefix: /proxy\n        strip_prefix: true\n        upstream: http://127.0.0.1:{upstream_port}\n");
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    for (h2, url) in [
        (false, format!("http://127.0.0.1:{plain}/v1/events")),
        (true, format!("https://127.0.0.1:{tls}/v1/events")),
        (false, format!("https://127.0.0.1:{tls}/v1/events")),
    ] {
        let builder = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(2));
        let client = if h2 {
            builder.http2_prior_knowledge()
        } else {
            builder.http1_only()
        }
        .build()?;
        let response = client
            .get(&url)
            .header("accept", "text/event-stream")
            .send()
            .await?;
        assert!(response.status().is_success(), "{url}");
        assert_eq!(
            response.bytes().await?.as_ref(),
            b"data: first\n\ndata: second\n\n",
            "{url}"
        );
        for prefix in ["/v1", "/proxy"] {
            let response = client
                .get(url.replace("/v1/events", &format!("{prefix}/gzip")))
                .header("accept", "text/event-stream")
                .send()
                .await?;
            assert_eq!(
                response
                    .headers()
                    .get("content-encoding")
                    .and_then(|value| value.to_str().ok()),
                Some("gzip"),
                "compressed SSE must retain its content coding: {url}"
            );
            let encoded = response.bytes().await?;
            let mut decoded = Vec::new();
            std::io::Read::read_to_end(
                &mut flate2::read::GzDecoder::new(encoded.as_ref()),
                &mut decoded,
            )?;
            assert_eq!(decoded, b"data: encoded\n\n");
        }
    }
    runner.abort();
    upstream.abort();
    Ok(())
}
