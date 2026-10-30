use super::*;
use crate::test_support::TestDir;
use bytes::Buf;
use http::header::ETAG;

#[test]
fn upstream_method_is_opt_in_and_rejects_noncanonical_or_tunnel_methods() {
    let mut route = RouteDecision::default();
    assert_eq!(
        route.upstream_http_method(&Method::HEAD).unwrap(),
        Method::HEAD
    );
    for method in [
        "GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "TRACE",
    ] {
        route.upstream_method = Some(method.into());
        route.validate_http_policy().unwrap();
        assert_eq!(
            route.upstream_http_method(&Method::HEAD).unwrap().as_str(),
            method
        );
    }
    for method in [
        "",
        "get",
        "GET ",
        " GET",
        "GET\r\nX: y",
        "GET\0",
        "CONNECT",
        "CUSTOM",
        "G/ET",
    ] {
        route.upstream_method = Some(method.into());
        assert!(
            route.validate_http_policy().is_err(),
            "invalid method accepted"
        );
    }
    for header in [
        "Content-Length",
        "Transfer-Encoding",
        "Connection",
        "Trailer",
        "Upgrade",
    ] {
        let route:RouteDecision=serde_json::from_value(serde_json::json!({"upstream":"http://127.0.0.1:9", "response_headers":{header:{"value":"fixture","append":true}}})).unwrap();
        assert!(route.validate_http_policy().is_err());
    }
}

#[test]
fn compression_override_keeps_streaming_ineligible_and_unbounded_entities() {
    let policy:ResponseCompressionConfig=serde_json::from_value(serde_json::json!({"enabled":true,"algorithms":["gzip"],"min_length":8,"content_types":["text/plain","text/event-stream"]})).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("1000000"));
    assert!(!should_stream_upstream_body(
        StatusCode::OK,
        &headers,
        "HTTP/1.1",
        None,
        &policy
    ));
    for (status, ctype, encoding) in [
        (StatusCode::PARTIAL_CONTENT, "text/plain", None),
        (StatusCode::OK, "application/octet-stream", None),
        (StatusCode::OK, "text/plain", Some("gzip")),
    ] {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(ctype));
        if let Some(value) = encoding {
            headers.insert(CONTENT_ENCODING, HeaderValue::from_static(value));
        } else {
            headers.remove(CONTENT_ENCODING);
        }
        assert!(should_stream_upstream_body(
            status, &headers, "HTTP/1.1", None, &policy
        ));
    }
    headers.remove(CONTENT_ENCODING);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    assert!(should_stream_upstream_body(
        StatusCode::OK,
        &headers,
        "HTTP/3",
        Some("cache-fixture"),
        &policy
    ));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    headers.remove(CONTENT_LENGTH);
    assert!(should_stream_upstream_body(
        StatusCode::OK,
        &headers,
        "HTTP/1.1",
        Some("cache-fixture"),
        &policy
    ));
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("4194305"));
    assert!(should_stream_upstream_body(
        StatusCode::OK,
        &headers,
        "HTTP/3",
        Some("cache-fixture"),
        &policy
    ));
    for length in [None, Some("4194305"), Some("7")] {
        let mut response = GatewayHttpResponse {
            status: StatusCode::OK,
            headers: vec![(CONTENT_TYPE, HeaderValue::from_static("text/plain"))],
            body: Bytes::new(),
            stream_body: None,
            upstream: "fixture".into(),
        };
        if let Some(value) = length {
            response.push_header(CONTENT_LENGTH, HeaderValue::from_static(value));
        }
        assert!(!apply_compression_vary(&mut response, &policy).unwrap());
        let has_vary = response.headers.iter().any(|(name, value)| {
            name == VARY && value.to_str().unwrap().contains("accept-encoding")
        });
        assert_eq!(
            has_vary,
            length != Some("7"),
            "unknown/large are Vary-only; known sub-minimum stays ineligible"
        );
    }
}

struct Task(tokio::task::JoinHandle<()>);
impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn route_compression_wire_gzip_identity_sse_range_and_encoded_cache_isolation() -> Result<()>
{
    use std::io::Read;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    const BODY: &[u8] =
        b"compression-fixture-content-compression-fixture-content-compression-fixture-content";
    let gzip = gzip_bytes(BODY)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let count = Arc::new(AtomicU64::new(0));
    let origin_count = count.clone();
    let _origin = Task(tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let gzip = gzip.clone();
            let count = origin_count.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    if stream.read_exact(&mut byte).await.is_err() {
                        return;
                    }
                    request.push(byte[0]);
                    if request.len() > 16384 {
                        return;
                    }
                }
                count.fetch_add(1, Ordering::Relaxed);
                let text = String::from_utf8_lossy(&request);
                let path = text.split_whitespace().nth(1).unwrap_or("/");
                if path == "/sse" || path == "/unknown" {
                    let content_type = if path == "/sse" {
                        "text/event-stream"
                    } else {
                        "text/html"
                    };
                    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nVary: Origin\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nD\r\ndata: first\n\n\r\n");
                    let _ = stream.write_all(head.as_bytes()).await;
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    let _ = stream.write_all(b"E\r\ndata: second\n\n\r\n").await;
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    let _ = stream.write_all(b"0\r\n\r\n").await;
                    return;
                }
                let range = path == "/range";
                let encoded = path.starts_with("/encoded")
                    && text.lines().any(|line| {
                        line.to_ascii_lowercase().starts_with("accept-encoding:")
                            && line.contains("gzip")
                    });
                let body = if range {
                    &BODY[..3]
                } else if encoded {
                    gzip.as_slice()
                } else {
                    BODY
                };
                let mut head=format!("HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nVary: Origin\r\nVary: X-Fixture\r\nConnection: close\r\n",if range {"206 Partial Content"}else{"200 OK"},body.len());
                if encoded {
                    head.push_str("Content-Encoding: gzip\r\n");
                }
                if range {
                    head.push_str(&format!("Content-Range: bytes 0-2/{}\r\n", BODY.len()));
                }
                head.push_str(if path == "/weak" {
                    "ETag: W/\"fixture\"\r\n"
                } else {
                    "ETag: \"fixture\"\r\n"
                });
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.write_all(body).await;
            });
        }
    }));
    let root = TestDir::new("compression-wire");
    let script = root.join("policy.ts");
    std::fs::write(
        &script,
        format!(
            r#"export default {{name:"compression-fixture",access(message){{const p=message.ctx.path;return {{upstream:"http://{address}",upstream_method:message.ctx.method==="HEAD"?"GET":null,compression:{{enabled:true,algorithms:["gzip"],min_length:8,content_types:["text/plain","text/html","text/event-stream"]}},cache:{{enabled:p.indexOf("/encoded")===0,behavior:"override",ttl_secs:30,vary_headers:p.indexOf("/encoded-vary")===0?["Accept-Encoding"]:[]}} }};}} }};"#
        ),
    )?;
    let mut config = GatewayConfig {
        root_dir: root.to_path_buf(),
        ..Default::default()
    };
    config.http.tls.cert_path = root.join("cert.pem");
    config.http.tls.key_path = root.join("key.pem");
    config.http.tls_bind.clear();
    config.http.h3_bind.clear();
    config.http.error_pages.enabled = false;
    config.script.enabled = true;
    config.script.entry = script;
    config.script.cwd = Some(root.to_path_buf());
    config.plugins.enabled = false;
    config.admin.enabled = false;
    config.monitoring.enabled = false;
    config.runtime.performance.enabled = false;
    config.logging.access_log = false;
    let gateway = Gateway::from_config(root.join("unused.yaml"), config).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = gateway.clone();
    let _proxy = Task(tokio::spawn(async move {
        loop {
            let (stream, peer) = listener.accept().await.unwrap();
            let gateway = server.clone();
            tokio::spawn(async move {
                gateway
                    .serve_plain_hyper_connection(stream, Bytes::new(), peer, 0)
                    .await;
            });
        }
    }));
    let mut metadata_errors = Vec::new();
    for h2 in [false, true] {
        let mut builder = reqwest::Client::builder().no_proxy().no_gzip();
        if h2 {
            builder = builder.http2_prior_knowledge();
        }
        let client = builder.build()?;
        for encoding in ["identity", "gzip"] {
            let response = client
                .head(format!("http://{address}/normal"))
                .header(ACCEPT_ENCODING, encoding)
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let actual_encoding = response
                .headers()
                .get(CONTENT_ENCODING)
                .map(|v| v.to_str().unwrap());
            let expected_encoding = (encoding == "gzip").then_some("gzip");
            if actual_encoding != expected_encoding {
                metadata_errors.push(format!(
                    "h2={h2} HEAD {encoding} Content-Encoding={actual_encoding:?}"
                ));
            }
            let actual_length = response
                .headers()
                .get(CONTENT_LENGTH)
                .map(|v| v.to_str().unwrap());
            let plaintext_length = BODY.len().to_string();
            let expected_length = (encoding == "identity").then_some(plaintext_length.as_str());
            if actual_length != expected_length {
                metadata_errors.push(format!(
                    "h2={h2} HEAD {encoding} Content-Length={actual_length:?}"
                ));
            }
            let expected_etag = if encoding == "gzip" {
                "W/\"fixture\""
            } else {
                "\"fixture\""
            };
            if response.headers().get(ETAG).unwrap() != expected_etag {
                metadata_errors.push(format!("h2={h2} HEAD {encoding} ETag mismatch"));
            }
            assert!(response
                .headers()
                .get_all(VARY)
                .iter()
                .any(|v| v.to_str().unwrap().contains("accept-encoding")));
            assert!(response.bytes().await?.is_empty());
        }
        for encoding in ["identity", "gzip"] {
            let response = client
                .get(format!("http://{address}/normal"))
                .header(ACCEPT_ENCODING, encoding)
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let tokens = response
                .headers()
                .get_all(VARY)
                .iter()
                .flat_map(|value| value.to_str().unwrap().split(','))
                .map(|token| token.trim().to_ascii_lowercase())
                .collect::<Vec<_>>();
            for token in ["origin", "x-fixture", "accept-encoding"] {
                assert!(tokens.iter().any(|value| value == token));
            }
            let is_gzip = response
                .headers()
                .get(CONTENT_ENCODING)
                .is_some_and(|value| value == "gzip");
            assert_eq!(is_gzip, encoding == "gzip");
            let expected_etag = if is_gzip {
                "W/\"fixture\""
            } else {
                "\"fixture\""
            };
            if response.headers().get(ETAG).unwrap() != expected_etag {
                metadata_errors.push(format!("h2={h2} GET {encoding} ETag mismatch"));
            }
            let bytes = response.bytes().await?;
            if is_gzip {
                let mut decoded = Vec::new();
                flate2::read::GzDecoder::new(bytes.as_ref()).read_to_end(&mut decoded)?;
                assert_eq!(decoded, BODY);
            } else {
                assert_eq!(bytes.as_ref(), BODY);
            }
        }
        let response = client
            .get(format!("http://{address}/weak"))
            .header(ACCEPT_ENCODING, "gzip")
            .send()
            .await?;
        assert_eq!(response.headers().get(ETAG).unwrap(), "W/\"fixture\"");
        response.bytes().await?;
        let response = client
            .get(format!("http://{address}/range"))
            .header(ACCEPT_ENCODING, "gzip")
            .header(RANGE, "bytes=0-2")
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(response.headers().get(ETAG).unwrap(), "\"fixture\"");
        assert!(response
            .headers()
            .get_all(VARY)
            .iter()
            .any(|value| value.to_str().unwrap().contains("accept-encoding")));
        assert_eq!(response.bytes().await?.as_ref(), &BODY[..3]);
        for path in ["/sse", "/unknown"] {
            let started = Instant::now();
            let mut response = tokio::time::timeout(
                Duration::from_secs(1),
                client
                    .get(format!("http://{address}{path}"))
                    .header(ACCEPT_ENCODING, "gzip")
                    .send(),
            )
            .await??;
            assert!(response.headers().get(CONTENT_ENCODING).is_none());
            if path == "/unknown" {
                assert!(response
                    .headers()
                    .get_all(VARY)
                    .iter()
                    .any(|v| v.to_str().unwrap().contains("accept-encoding")));
            }
            let first = tokio::time::timeout(Duration::from_secs(1), response.chunk())
                .await??
                .unwrap();
            assert_eq!(first.as_ref(), b"data: first\n\n");
            let second = tokio::time::timeout(Duration::from_secs(1), response.chunk())
                .await??
                .unwrap();
            assert_eq!(second.as_ref(), b"data: second\n\n");
            assert!(started.elapsed() < Duration::from_secs(2));
            drop(response);
        }
        let response = client
            .head(format!("http://{address}/unknown"))
            .header(ACCEPT_ENCODING, "gzip")
            .send()
            .await?;
        assert!(
            response.headers().get(CONTENT_ENCODING).is_none(),
            "unknown HEAD must not advertise an unperformed transform"
        );
        assert!(response
            .headers()
            .get_all(VARY)
            .iter()
            .any(|v| v.to_str().unwrap().contains("accept-encoding")));
        assert!(response.bytes().await?.is_empty());
        for vary in [false, true] {
            for gzip_first in [false, true] {
                let path = format!(
                    "/encoded{}-{}-{}",
                    if vary { "-vary" } else { "" },
                    h2,
                    gzip_first
                );
                for encoding in if gzip_first {
                    ["gzip", "identity"]
                } else {
                    ["identity", "gzip"]
                } {
                    let before = gateway.http_cache.len();
                    let response = client
                        .get(format!("http://{address}{path}"))
                        .header(ACCEPT_ENCODING, encoding)
                        .send()
                        .await?;
                    let is_gzip = response
                        .headers()
                        .get(CONTENT_ENCODING)
                        .is_some_and(|value| value == "gzip");
                    assert_eq!(is_gzip, encoding == "gzip");
                    let bytes = response.bytes().await?;
                    if is_gzip {
                        let mut decoded = Vec::new();
                        flate2::read::GzDecoder::new(bytes.as_ref()).read_to_end(&mut decoded)?;
                        assert_eq!(decoded, BODY);
                    } else {
                        assert_eq!(bytes.as_ref(), BODY);
                    }
                    if !vary && gzip_first && encoding == "gzip" {
                        assert_eq!(
                            gateway.http_cache.len(),
                            before,
                            "encoded origin stored without AE cache dimension"
                        );
                    }
                }
            }
        }
    }
    assert!(count.load(Ordering::Relaxed) > 0);
    assert!(metadata_errors.is_empty(), "{}", metadata_errors.join("; "));
    Ok(())
}

#[tokio::test]
async fn actual_h1_h2_h3_head_uses_get_metadata_without_body_and_headers_append_in_order(
) -> Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    const BODY: &[u8] = b"get-metadata-fixture";
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = listener.local_addr()?;
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_origin = seen.clone();
    let _origin = Task(tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let seen = seen_origin.clone();
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| {
                    seen.lock().unwrap().push(request.method().clone());
                    let is_head = request.method() == Method::HEAD;
                    let visitor = request
                        .headers()
                        .get("x-visitor-method")
                        .cloned()
                        .unwrap_or(HeaderValue::from_static("missing"));
                    async move {
                        let body = if is_head {
                            Bytes::new()
                        } else {
                            Bytes::from_static(BODY)
                        };
                        let mut response = Response::builder()
                            .status(if is_head {
                                StatusCode::NOT_FOUND
                            } else {
                                StatusCode::OK
                            })
                            .header(CONTENT_LENGTH, body.len())
                            .header(CONTENT_TYPE, "text/plain")
                            .header("etag", "\"get-fixture\"")
                            .header("x-visitor-method", visitor)
                            .header("x-origin-method", if is_head { "HEAD" } else { "GET" })
                            .header(
                                "strict-transport-security",
                                "max-age=31536000; includeSubDomains",
                            )
                            .body(Full::new(body))
                            .unwrap();
                        response.headers_mut().append(
                            http::header::SET_COOKIE,
                            HeaderValue::from_static(
                                "a=one; Expires=Wed, 21 Oct 2030 07:28:00 GMT; Path=/",
                            ),
                        );
                        response.headers_mut().append(
                            http::header::SET_COOKIE,
                            HeaderValue::from_static("b=two; Secure; Path=/"),
                        );
                        Ok::<_, Infallible>(response)
                    }
                });
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    }));
    let root = TestDir::new("upstream-method-wire");
    let script = root.join("policy.ts");
    std::fs::write(
        &script,
        format!(
            r#"export default {{name:"method-fixture", access(message) {{
      const c=message.ctx;
      return {{ upstream:"http://{origin}", upstream_method:c.path==="/default"?undefined:(c.path==="/origin-head"?"HEAD":(c.method==="HEAD"?"GET":undefined)),
        cookie_secure:true, set_headers:{{"X-Visitor-Method":c.method}}, response_headers:c.path==="/replace"?{{
          "Strict-Transport-Security":"max-age=15552000", "Set-Cookie":"c=three; Path=/"
        }}:{{"Strict-Transport-Security":{{value:"max-age=15552000",append:true,statuses:[200]}},
          "Set-Cookie":{{value:"c=three; Path=/",append:true,statuses:[200]}}}} }};
    }} }};"#
        ),
    )?;
    let mut config = GatewayConfig {
        root_dir: root.to_path_buf(),
        ..Default::default()
    };
    config.http.tls.cert_path = root.join("cert.pem");
    config.http.tls.key_path = root.join("key.pem");
    config.http.tls_bind.clear();
    config.http.h3_bind.clear();
    config.http.error_pages.enabled = false;
    config.http.redirect_http_to_https = false;
    config.script.enabled = true;
    config.script.entry = script;
    config.script.cwd = Some(root.to_path_buf());
    config.plugins.enabled = false;
    config.admin.enabled = false;
    config.monitoring.enabled = false;
    config.logging.access_log = false;
    config.runtime.performance.enabled = false;
    let gateway = Gateway::from_config(root.join("unused.yaml"), config).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = gateway.clone();
    let _proxy = Task(tokio::spawn(async move {
        loop {
            let (stream, peer) = listener.accept().await.unwrap();
            let gateway = server.clone();
            tokio::spawn(async move {
                gateway
                    .serve_plain_hyper_connection(stream, Bytes::new(), peer, 0)
                    .await;
            });
        }
    }));
    for h2 in [false, true] {
        let mut builder = reqwest::Client::builder().no_proxy();
        if h2 {
            builder = builder.http2_prior_knowledge();
        }
        let client = builder.build()?;
        for _ in 0..2 {
            let response = client
                .head(format!("http://{address}/head?fixture=yes"))
                .send()
                .await?;
            assert_eq!(
                response.version(),
                if h2 {
                    Version::HTTP_2
                } else {
                    Version::HTTP_11
                }
            );
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[CONTENT_LENGTH], BODY.len().to_string());
            assert_eq!(response.headers()["etag"], "\"get-fixture\"");
            assert_eq!(response.headers()["x-origin-method"], "GET");
            assert_eq!(response.headers()["x-visitor-method"], "HEAD");
            assert!(response.bytes().await?.is_empty());
            assert_eq!(seen.lock().unwrap().last(), Some(&Method::GET));
        }
        let response = client
            .head(format!("http://{address}/default"))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.bytes().await?.is_empty());
        for append in [true, false] {
            let response = client
                .get(format!(
                    "http://{address}/{}",
                    if append { "append" } else { "replace" }
                ))
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let hsts = response
                .headers()
                .get_all("strict-transport-security")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                hsts,
                if append {
                    vec!["max-age=31536000; includeSubDomains", "max-age=15552000"]
                } else {
                    vec!["max-age=15552000"]
                }
            );
            let cookies = response
                .headers()
                .get_all(http::header::SET_COOKIE)
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(cookies.len(), if append { 3 } else { 1 });
            if append {
                assert!(cookies[0].starts_with("a=one; Expires="));
                assert!(cookies[1].starts_with("b=two;"));
            }
            assert!(cookies.last().unwrap().starts_with("c=three;"));
            assert!(cookies
                .iter()
                .all(|cookie| cookie.matches("; Secure").count() == 1));
            assert_eq!(response.bytes().await?.as_ref(), BODY);
        }
        let response = client
            .get(format!("http://{address}/origin-head"))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[CONTENT_LENGTH], "0");
        assert!(response.bytes().await?.is_empty());
    }
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
    let address = socket.local_addr()?;
    drop(socket);
    let server = gateway.clone();
    let _h3 = Task(tokio::spawn(async move {
        server.run_http3(address.to_string()).await.unwrap();
    }));
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut std::io::Cursor::new(std::fs::read(
        &gateway.bootstrap_config.http.tls.cert_path,
    )?)) {
        roots.add(cert?)?;
    }
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )));
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        endpoint.connect(address, "localhost")?,
    )
    .await??;
    let (mut driver, mut requests) = h3::client::new(h3_quinn::Connection::new(connection)).await?;
    let _driver = Task(tokio::spawn(async move {
        let _ = driver.wait_idle().await;
    }));
    let mut stream = requests
        .send_request(
            Request::builder()
                .method(Method::HEAD)
                .uri("https://localhost/head")
                .body(())?,
        )
        .await?;
    stream.finish().await?;
    let response = tokio::time::timeout(Duration::from_secs(5), stream.recv_response()).await??;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_LENGTH], BODY.len().to_string());
    assert_eq!(response.headers()["x-origin-method"], "GET");
    assert_eq!(response.headers()["x-visitor-method"], "HEAD");
    let mut size = 0;
    while let Some(chunk) =
        tokio::time::timeout(Duration::from_secs(5), stream.recv_data()).await??
    {
        size += chunk.remaining();
    }
    assert_eq!(size, 0);
    Ok(())
}
