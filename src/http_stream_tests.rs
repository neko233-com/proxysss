use super::*;

#[test]
fn raw_upstream_framing_rejects_ambiguity_before_forwarding() {
    for headers in [
        "Content-Length: 3\r\nContent-Length: 4\r\n",
        "Content-Length: +3\r\n",
        "Content-Length: 3\r\nTransfer-Encoding: chunked\r\n",
        "Transfer-Encoding: gzip, chunked\r\n",
        "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
    ] {
        let head = Bytes::from(format!("HTTP/1.1 200 OK\r\n{headers}\r\n"));
        assert!(
            parse_raw_fast_http_response_head(head.clone(), None, true).is_err(),
            "{headers}"
        );
        assert!(parse_http_response_head(&head).is_err(), "{headers}");
    }
    let parsed = parse_raw_fast_http_response_head(
        Bytes::from_static(b"HTTP/1.1 200 OK\r\nContent-Length: 3, 3\r\nContent-Length: 3\r\n\r\n"),
        None,
        true,
    )
    .unwrap();
    assert_eq!(parsed.content_length, Some(3));
}

#[test]
fn raw_chunk_scanner_consumes_only_the_message_for_every_fragmentation() {
    let message = b"3;key=value\r\nabc\r\n0\r\nx-final: yes\r\n\r\n";
    let wire = [
        message.as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\npoison",
    ]
    .concat();
    for step in [1, 2, 3, 7, 4096] {
        let mut scanner = ChunkedBodyScanner::default();
        let mut consumed = 0;
        let mut complete = false;
        for part in wire.chunks(step) {
            let (count, done) = scanner.observe_prefix(part).unwrap();
            consumed += count;
            if done {
                complete = true;
                break;
            }
        }
        assert!(complete);
        assert_eq!(consumed, message.len());
    }
    for wire in [
        b"+3\r\nabc\r\n0\r\n\r\n".as_slice(),
        b"0\r\ninvalid-trailer\r\n\r\n",
    ] {
        assert!(ChunkedBodyScanner::default().observe_prefix(wire).is_err());
    }
    let oversized = format!("1;{}\r\na\r\n0\r\n\r\n", "x".repeat(16384));
    assert!(ChunkedBodyScanner::default()
        .observe_prefix(oversized.as_bytes())
        .is_err());
}

#[tokio::test]
async fn raw_relays_drop_extra_bytes_and_report_premature_eof() {
    let mut source = &b"3\r\nabc\r\n0\r\n\r\npoison"[..];
    let (mut output, mut reader) = tokio::io::duplex(4096);
    let reusable =
        relay_passthrough_chunked_http_body(&mut source, &mut output, None, Duration::from_secs(1))
            .await
            .unwrap();
    assert!(!reusable);
    drop(output);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes, b"3\r\nabc\r\n0\r\n\r\n");
    let (mut output, _reader) = tokio::io::duplex(4096);
    let mut short = &b"abc"[..];
    assert!(
        relay_fixed_http_body(&mut short, &mut output, None, 4, Duration::from_secs(1))
            .await
            .is_err()
    );
    let (mut output, _reader) = tokio::io::duplex(4096);
    let mut short = &b"3\r\nabc\r\n"[..];
    assert!(relay_passthrough_chunked_http_body(
        &mut short,
        &mut output,
        None,
        Duration::from_secs(1)
    )
    .await
    .is_err());
}

#[tokio::test]
async fn upstream_idle_timeout_resets_on_progress_and_excludes_downstream_pauses() {
    let stream = futures::stream::unfold(0, |index| async move {
        if index == 12 {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        Some((
            Ok::<_, anyhow::Error>(Frame::data(Bytes::from_static(b"x"))),
            index + 1,
        ))
    });
    let body = GatewayBody::Stream(StreamBody::new(stream).boxed_unsync());
    let mut body = upstream_idle_body(body, Duration::from_millis(150));
    assert!(body.frame().await.unwrap().is_ok());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(body.collect().await.unwrap().to_bytes().len(), 11);
    let pending = futures::stream::pending::<Result<Frame<Bytes>>>();
    let body = GatewayBody::Stream(StreamBody::new(pending).boxed_unsync());
    let mut body = upstream_idle_body(body, Duration::from_millis(10));
    assert!(body.frame().await.unwrap().is_err());
}

#[tokio::test]
async fn raw_handshake_skips_informational_responses_and_preserves_final_body() {
    let mut source = &b"HTTP/1.1 103 Early Hints\r\nLink: </asset>; rel=preload\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc"[..];
    let (status, _, leftover) = read_http_response_head(&mut source).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(leftover.unwrap().as_ref(), b"abc");
}

#[tokio::test]
async fn inbound_body_limits_validate_declared_and_streamed_lengths_and_idle() {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("5"));
    let mut full = Full::new(Bytes::from_static(b"hello"));
    assert_eq!(
        collect_request_body_if_needed(
            &Method::PUT,
            &headers,
            &mut full,
            4,
            Duration::from_secs(1)
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut full = Full::new(Bytes::from_static(b"hell"));
    assert_eq!(
        collect_request_body_if_needed(
            &Method::PUT,
            &headers,
            &mut full,
            5,
            Duration::from_secs(1)
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::BAD_REQUEST
    );
    headers.clear();
    // HTTP/2 and HTTP/3 may carry GET data without a Content-Length header.
    let mut full = Full::new(Bytes::from_static(b"hello"));
    assert_eq!(
        collect_request_body_if_needed(
            &Method::GET,
            &headers,
            &mut full,
            5,
            Duration::from_secs(1)
        )
        .await
        .unwrap()
        .as_ref(),
        b"hello"
    );
    let chunks = futures::stream::iter([
        Ok::<_, anyhow::Error>(Frame::data(Bytes::from_static(b"abc"))),
        Ok(Frame::data(Bytes::from_static(b"def"))),
    ]);
    let mut body = StreamBody::new(chunks);
    assert_eq!(
        collect_request_body_if_needed(
            &Method::POST,
            &headers,
            &mut body,
            5,
            Duration::from_secs(1)
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut pending = StreamBody::new(futures::stream::pending::<Result<Frame<Bytes>>>());
    assert_eq!(
        collect_request_body_if_needed(
            &Method::POST,
            &headers,
            &mut pending,
            5,
            Duration::from_millis(10)
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::REQUEST_TIMEOUT
    );
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("+5"));
    let mut full = Full::new(Bytes::new());
    assert_eq!(
        collect_request_body_if_needed(
            &Method::POST,
            &headers,
            &mut full,
            5,
            Duration::from_secs(1)
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn admin_config_redacts_file_service_and_monitor_passwords() {
    let mut config = GatewayConfig::default();
    config.services.webdav.auth.password = "dav-secret-sentinel".into();
    config.admin.monitor.password = "monitor-secret-sentinel".into();
    let value = sanitize_config(&config);
    assert_eq!(value["services"]["webdav"]["auth"]["password"], "***");
    assert_eq!(value["admin"]["monitor"]["password"], "***");
    assert!(!value.to_string().contains("secret-sentinel"));
}

#[tokio::test]
async fn https_upstream_sse_outlives_total_timeout_but_stalled_body_fails() -> Result<()> {
    crate::verify::harness::ensure_rustls_crypto_provider();
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let tls = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate.cert.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.key_pair.serialize_der())
                .into(),
        )?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (socket, _) = listener.accept().await?;
            socket.set_nodelay(true)?;
            let mut stream = acceptor.accept(socket).await?;
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let byte = stream.read_u8().await?;
                head.push(byte);
            }
            let stalled = head.starts_with(b"GET /stall ");
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n1\r\nx\r\n").await?;
            stream.flush().await?;
            if stalled {
                tokio::time::sleep(Duration::from_secs(1)).await;
                break;
            }
            for _ in 0..10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                stream.write_all(b"1\r\nx\r\n").await?;
                stream.flush().await?;
            }
            stream.write_all(b"0\r\n\r\n").await?;
            stream.flush().await?;
        }
        Ok::<_, anyhow::Error>(())
    });
    let mut config = GatewayConfig::default();
    config.script.enabled = false;
    config.plugins.enabled = false;
    config.http.allow_insecure_upstreams = true;
    config.http.request_timeout_ms = 500;
    let state = build_dynamic_state(config).await?;
    let response = state
        .dispatch_upstream_http(
            Method::GET,
            UpstreamHttpTarget::Reqwest(format!("https://{address}/live").parse()?),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await?;
    assert_eq!(response.bytes().await?.as_ref(), b"xxxxxxxxxxx");
    let response = state
        .dispatch_upstream_http(
            Method::GET,
            UpstreamHttpTarget::Reqwest(format!("https://{address}/stall").parse()?),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await?;
    assert!(response.bytes().await.is_err());
    server.await??;
    Ok(())
}
