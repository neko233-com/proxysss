use super::*;
use crate::test_support::TestDir;
use crate::verify::harness::{ensure_rustls_crypto_provider, reserve_port};
use futures_util::{SinkExt, StreamExt};

struct Task(tokio::task::JoinHandle<()>);
impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn config(root: &Path) -> GatewayConfig {
    ensure_rustls_crypto_provider();
    let mut config = GatewayConfig::default();
    config.http.plain_bind = "127.0.0.1:0".into();
    config.http.tls_bind.clear();
    config.http.h3_bind.clear();
    config.http.tls.mode = TlsMode::SelfSigned;
    config.http.tls.cert_path = root.join("cert.pem");
    config.http.tls.key_path = root.join("key.pem");
    config.script.enabled = false;
    config.plugins.enabled = false;
    config.admin.enabled = false;
    config.logging.access_log = false;
    config.runtime.hot_reload.enabled = false;
    config.runtime.performance.enabled = true;
    config.security.ddos.enabled = false;
    config.security.dynamic_blacklist.enabled = false;
    config.services.access_control.http.enabled = false;
    config.services.rate_limit.http.enabled = false;
    config.services.response_policy.compression.enabled = false;
    config.services.response_policy.cache.enabled = false;
    config.load_balance.retries.enabled = false;
    config.load_balance.active_health.enabled = false;
    config.load_balance.passive_health.enabled = false;
    config.affinity.enabled = false;
    config.monitoring.domain_stats_hosts = vec!["localhost".into(), "127.0.0.1".into()];
    config
}

fn route(name: &str, prefix: &str, upstream: &str) -> ReverseProxyRouteConfig {
    serde_yaml::from_str(&format!(
        "name: {name}\npath_prefix: {prefix}\nupstream: {upstream}\n"
    ))
    .unwrap()
}

fn assert_counts(gateway: &Gateway, requests: u64, errors: u64) {
    assert_eq!(gateway.stats.http_requests_total(), requests);
    assert_eq!(gateway.stats.http_errors.load(Ordering::Relaxed), errors);
    let json = gateway.stats.snapshot_json();
    assert_eq!(json["http_requests"], requests);
    assert_eq!(json["http_errors"], errors);
    let domains = json["domain_requests"]
        .as_object()
        .expect("domain counters");
    assert_eq!(
        domains
            .values()
            .map(|count| count.as_u64().unwrap())
            .sum::<u64>(),
        requests
    );
    let prometheus = gateway.stats.snapshot_prometheus();
    assert!(prometheus
        .lines()
        .any(|line| line == format!("proxysss_http_requests_total {requests}")));
    assert!(prometheus
        .lines()
        .any(|line| line == format!("proxysss_http_errors_total {errors}")));
    assert!(prometheus.contains("# TYPE proxysss_domain_http_requests_total counter"));
}

#[test]
fn domain_statistics_discover_bounded_hosts_and_can_be_disabled() {
    let mut config = GatewayConfig::default();
    assert!(config.monitoring.domain_stats_enabled);
    config.monitoring.domain_stats_hosts = vec!["explicit.example.com".into()];
    config.http.tls.acme.domains = vec!["*.example.com".into(), "Origin.Example.com".into()];
    let initial = config.domain_stats_hosts();
    assert_eq!(initial, ["explicit.example.com", "origin.example.com"]);
    config.http.tls.auto_https.domains = (0..300).map(|i| format!("h{i}.example.com")).collect();
    let hosts = config.domain_stats_hosts();
    assert_eq!(hosts.len(), 256);
    assert_eq!(hosts[0], "explicit.example.com");
    assert!(!hosts.iter().any(|host| host.contains('*')));
    config.monitoring.domain_stats_enabled = false;
    assert!(config.domain_stats_hosts().is_empty());
    let default: crate::config::MonitoringConfig = serde_yaml::from_str("{}").unwrap();
    assert!(default.domain_stats_enabled);
}

#[test]
fn domain_statistics_are_bounded_to_configured_hosts_and_survive_reload() {
    let stats = GatewayStats::default();
    stats.configure_domain_stats(&["a.example.com".into()]);
    stats.record_domain_request(Some("A.Example.Com:443"));
    stats.record_domain_request(Some("unlisted.example.com"));
    assert_eq!(stats.snapshot_json()["domain_requests"]["a.example.com"], 1);
    assert_eq!(
        stats.snapshot_json()["domain_requests"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    stats.configure_domain_stats(&["a.example.com".into(), "b.example.com".into()]);
    stats.record_domain_request(Some("a.example.com"));
    assert_eq!(stats.snapshot_json()["domain_requests"]["a.example.com"], 2);
    stats.configure_domain_stats(&[]);
    stats.record_domain_request(Some("a.example.com"));
    assert!(stats.snapshot_json()["domain_requests"]
        .as_object()
        .unwrap()
        .is_empty());
}

async fn frontend(gateway: Arc<Gateway>, tls: bool, hyper_only: bool) -> (SocketAddr, Task) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let acceptor = tls.then(|| {
        TlsAcceptor::from(Arc::new(
            build_rustls_server_config(
                &gateway.bootstrap_config,
                gateway.acme_tls_alpn_certs.clone(),
                gateway.on_demand_certs.clone(),
                gateway.on_demand_trigger.clone(),
                default_tls_alpn_protocols(),
            )
            .unwrap(),
        ))
    });
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, peer)) = accepted else {break};
                    let gateway = gateway.clone(); let acceptor = acceptor.clone();
                    connections.spawn(async move {
                        if let Some(acceptor) = acceptor {
                            gateway.serve_tls_http_connection(stream,acceptor,peer,0).await;
                        } else if hyper_only {
                            gateway.serve_plain_hyper_connection(stream,Bytes::new(),peer,0).await;
                        } else {
                            gateway.serve_plain_http_connection(stream,peer,0,Bytes::new()).await;
                        }
                    });
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    (address, Task(task))
}

async fn backend() -> (SocketAddr, Task) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream,_))=accepted else {break};
                    connections.spawn(async move {
                        let service = service_fn(|request: Request<Incoming>| async move {
                            let status = if request.uri().path().ends_with("/error") {StatusCode::SERVICE_UNAVAILABLE}
                                else if request.uri().path().ends_with("/missing") {StatusCode::NOT_FOUND} else {StatusCode::OK};
                            request.into_body().collect().await.unwrap();
                            Ok::<_,Infallible>(Response::builder().status(status).header("content-type","text/event-stream")
                                .body(Full::new(Bytes::from_static(b"data: test\n\n"))).unwrap())
                        });
                        let _ = optimized_http_server_builder().serve_connection(TokioIo::new(stream),service).await;
                    });
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    (address, Task(task))
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn static_keepalive_cached_heads_sendfile_and_hyper_fallback_count_once() -> Result<()> {
    let root = TestDir::new("http-metrics-static");
    std::fs::write(root.join("asset.txt"), b"content")?;
    let file = std::fs::File::create(root.join("large.bin"))?;
    file.set_len(8 * 1024 * 1024)?;
    let mut config = config(&root);
    config.runtime.performance.traffic_profile = RuntimePerformanceTrafficProfile::Balanced;
    config.services.static_sites = vec![serde_yaml::from_str(&format!(
        "name: assets\npath_prefix: /assets\nroot: '{}'\n",
        root.display().to_string().replace('\\', "/")
    ))?];
    let gateway = Gateway::from_config(root.join("proxysss.yaml"), config).await?;
    assert!(gateway.bootstrap_fast_lane.plain_http_static_sendfile);
    let (address, _server) = frontend(gateway.clone(), false, false).await;
    let mut client = TcpStream::connect(address).await?;
    let mut count = 0;
    for (method, target, extra, expected, body) in [
        (
            "GET",
            "/assets/asset.txt",
            "",
            StatusCode::OK,
            b"content".as_slice(),
        ),
        ("GET", "/assets/asset.txt", "", StatusCode::OK, b"content"),
        (
            "GET",
            "/assets/asset.txt",
            "X-Client: changed\r\n",
            StatusCode::OK,
            b"content",
        ),
        ("HEAD", "/assets/asset.txt", "", StatusCode::OK, b""),
        (
            "GET",
            "/assets/asset.txt?new-cache-key",
            "",
            StatusCode::OK,
            b"content",
        ),
        ("GET", "/assets/large.bin", "", StatusCode::OK, b""),
        ("GET", "/assets/large.bin", "", StatusCode::OK, b""),
        (
            "GET",
            "/assets/asset.txt",
            "Range: bytes=1-3\r\n",
            StatusCode::PARTIAL_CONTENT,
            b"ont",
        ),
        ("GET", "/assets/missing", "", StatusCode::NOT_FOUND, b""),
    ] {
        client
            .write_all(
                format!("{method} {target} HTTP/1.1\r\nHost: localhost\r\n{extra}\r\n").as_bytes(),
            )
            .await?;
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            read_raw_fast_http_response_head(&mut client, true),
        )
        .await??;
        assert_eq!(response.status, expected);
        let mut received = response.leftover.unwrap_or_default().to_vec();
        let length = if method == "HEAD" {
            0
        } else {
            response.content_length.unwrap() as usize
        };
        if received.len() < length {
            let offset = received.len();
            received.resize(length, 0);
            tokio::time::timeout(
                Duration::from_secs(5),
                client.read_exact(&mut received[offset..]),
            )
            .await??;
        }
        if target.ends_with("large.bin") {
            assert_eq!(received.len(), 8 * 1024 * 1024);
            assert!(received.iter().all(|b| *b == 0));
        } else if expected != StatusCode::NOT_FOUND {
            assert_eq!(received, body);
        }
        count += 1;
        assert_counts(
            &gateway,
            count,
            u64::from(expected == StatusCode::NOT_FOUND),
        );
    }
    Ok(())
}

#[tokio::test]
async fn raw_reverse_sse_and_hyper_responses_share_exact_request_and_error_counts() -> Result<()> {
    let (upstream, _backend) = backend().await;
    for mode in 0..3 {
        let root = TestDir::new("http-metrics-proxy");
        let mut config = config(&root);
        config.runtime.performance.enabled = mode != 2;
        config.services.reverse_proxy.routes =
            vec![route("proxy", "/", &format!("http://{upstream}"))];
        config.services.ai_proxy=serde_yaml::from_str(&format!("enabled: true\nroutes:\n  - name: events\n    path_prefix: /events\n    upstream: http://{upstream}\n"))?;
        let gateway = Gateway::from_config(root.join("proxysss.yaml"), config).await?;
        assert_eq!(gateway.bootstrap_fast_lane.raw_reverse_proxy, mode != 2);
        let (address, _server) = frontend(gateway.clone(), false, mode == 1).await;
        let client = reqwest::Client::builder()
            .http1_only()
            .timeout(Duration::from_secs(5))
            .build()?;
        let mut count = 0;
        let mut errors = 0;
        for (path, status) in [
            ("/ok", StatusCode::OK),
            ("/ok", StatusCode::OK),
            ("/error", StatusCode::SERVICE_UNAVAILABLE),
            ("/missing", StatusCode::NOT_FOUND),
            ("/events/ok", StatusCode::OK),
            ("/events/error", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let response = client
                .get(format!("http://{address}{path}"))
                .header("accept", "text/event-stream")
                .send()
                .await?;
            assert_eq!(response.status(), status);
            let _ = response.bytes().await?;
            count += 1;
            errors += u64::from(status.is_client_error() || status.is_server_error());
            assert_counts(&gateway, count, errors);
        }
        let response = client
            .post(format!("http://{address}/ok"))
            .body("payload")
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let _ = response.bytes().await?;
        assert_counts(&gateway, count + 1, errors);
    }
    Ok(())
}

#[tokio::test]
async fn tls_http1_http2_count_generated_and_forwarded_errors_once() -> Result<()> {
    let (upstream, _backend) = backend().await;
    let unavailable = reserve_port().await?;
    for h2 in [false, true] {
        let root = TestDir::new("http-metrics-tls");
        let mut config = config(&root);
        config.http.tls_bind = "127.0.0.1:0".into();
        config.http.max_request_body_bytes = 16;
        // Force dispatch_http so a connect failure reaches the common finisher.
        config.load_balance.passive_health.enabled = true;
        config.services.reverse_proxy.routes = vec![
            route("proxy", "/", &format!("http://{upstream}")),
            route(
                "unavailable",
                "/offline",
                &format!("http://127.0.0.1:{unavailable}"),
            ),
        ];
        let gateway = Gateway::from_config(root.join("proxysss.yaml"), config).await?;
        let (address, _server) = frontend(gateway.clone(), true, false).await;
        let builder = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(5));
        let client = if h2 {
            builder.http2_prior_knowledge()
        } else {
            builder.http1_only()
        }
        .build()?;
        let mut count = 0;
        let mut errors = 0;
        for (path, status) in [
            ("/ok", StatusCode::OK),
            ("/missing", StatusCode::NOT_FOUND),
            ("/error", StatusCode::SERVICE_UNAVAILABLE),
            ("/offline", StatusCode::BAD_GATEWAY),
        ] {
            let response = client
                .get(format!("https://{address}{path}"))
                .send()
                .await?;
            assert_eq!(
                response.version(),
                if h2 {
                    http::Version::HTTP_2
                } else {
                    http::Version::HTTP_11
                }
            );
            assert_eq!(response.status(), status);
            let _ = response.bytes().await?;
            count += 1;
            errors += u64::from(status.is_client_error() || status.is_server_error());
            assert_counts(&gateway, count, errors);
        }
        let response = client
            .post(format!("https://{address}/ok"))
            .body("x".repeat(32))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let _ = response.bytes().await?;
        assert_counts(&gateway, count + 1, errors + 1);
    }
    Ok(())
}

#[tokio::test]
async fn websocket_and_wss_count_handshake_before_messages_and_fallback_once() -> Result<()> {
    let (upstream, _backend) = backend().await;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let echo = listener.local_addr()?;
    let _echo = Task(tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                result=listener.accept()=>{
                    let Ok((stream,_))=result else {break};
                    connections.spawn(async move {
                        let Ok(mut socket)=tokio_tungstenite::accept_async(stream).await else{return};
                        while let Some(Ok(message))=socket.next().await {if socket.send(message).await.is_err(){break}}
                    });
                }
                _=connections.join_next(),if !connections.is_empty()=>{}
            }
        }
    }));
    let mut cases = vec![
        (false, false, false),
        (true, false, false),
        (false, true, false),
    ];
    if cfg!(target_os = "linux") {
        cases.push((false, false, true));
    }
    for (tls, hyper_only, balanced) in cases {
        let root = TestDir::new("http-metrics-websocket");
        let mut config = config(&root);
        if balanced {
            config.runtime.performance.traffic_profile = RuntimePerformanceTrafficProfile::Balanced;
        }
        if tls {
            config.http.tls_bind = "127.0.0.1:0".into();
        }
        let unavailable = reserve_port().await?;
        config.services.reverse_proxy.routes = vec![
            route("echo", "/ws", &format!("ws://{echo}")),
            route("rejected", "/reject", &format!("ws://{upstream}")),
            route(
                "offline",
                "/offline",
                &format!("ws://127.0.0.1:{unavailable}"),
            ),
        ];
        let gateway = Gateway::from_config(root.join("proxysss.yaml"), config).await?;
        assert!(gateway.bootstrap_fast_lane.raw_websocket_proxy);
        let (address, _server) = frontend(gateway.clone(), tls, hyper_only).await;
        let connector = if tls {
            Some(tokio_tungstenite::Connector::Rustls(Arc::new(client_tls(
                &gateway.bootstrap_config,
            )?)))
        } else {
            None
        };
        let scheme = if tls { "wss" } else { "ws" };
        let (mut socket, response) = tokio_tungstenite::connect_async_tls_with_config(
            format!("{scheme}://{address}/ws"),
            None,
            false,
            connector.clone(),
        )
        .await?;
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_counts(&gateway, 1, 0);
        tokio::time::timeout(Duration::from_secs(3), async {
            while gateway
                .stats
                .websocket_sessions_active
                .load(Ordering::Relaxed)
                != 1
            {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert_eq!(
            gateway
                .stats
                .websocket_sessions_total
                .load(Ordering::Relaxed),
            1
        );
        for _ in 0..3 {
            socket
                .send(tokio_tungstenite::tungstenite::Message::text(
                    "metrics-probe",
                ))
                .await?;
            assert_eq!(socket.next().await.unwrap()?.into_text()?, "metrics-probe");
            assert_counts(&gateway, 1, 0);
        }
        let _ = socket.close(None).await;
        drop(socket);
        tokio::time::timeout(Duration::from_secs(3), async {
            while gateway
                .stats
                .websocket_sessions_active
                .load(Ordering::Relaxed)
                != 0
            {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let rejected = tokio_tungstenite::connect_async_tls_with_config(
            format!("{scheme}://{address}/reject/error"),
            None,
            false,
            connector.clone(),
        )
        .await;
        assert!(
            matches!(rejected,Err(tokio_tungstenite::tungstenite::Error::Http(response)) if response.status()==StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_counts(&gateway, 2, 1);
        let failed = tokio_tungstenite::connect_async_tls_with_config(
            format!("{scheme}://{address}/offline"),
            None,
            false,
            connector,
        )
        .await;
        assert!(failed.is_err());
        assert_counts(&gateway, 3, if tls || hyper_only { 2 } else { 1 });
        assert_eq!(
            gateway
                .stats
                .websocket_sessions_total
                .load(Ordering::Relaxed),
            1
        );
    }
    Ok(())
}

fn client_tls(config: &GatewayConfig) -> Result<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut std::io::Cursor::new(std::fs::read(
        &config.http.tls.cert_path,
    )?)) {
        roots.add(certificate?)?;
    }
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}

#[tokio::test]
async fn http3_counts_dispatch_errors_and_rejected_bodies_once() -> Result<()> {
    let (upstream, _backend) = backend().await;
    let root = TestDir::new("http-metrics-h3");
    let mut config = config(&root);
    let port = reserve_port().await?;
    let unavailable = reserve_port().await?;
    config.http.h3_bind = format!("127.0.0.1:{port}");
    config.http.max_request_body_bytes = 16;
    config.http.max_buffered_request_body_bytes = 4;
    config.services.reverse_proxy.routes = vec![
        route("proxy", "/", &format!("http://{upstream}")),
        route(
            "offline",
            "/offline",
            &format!("http://127.0.0.1:{unavailable}"),
        ),
    ];
    let gateway = Gateway::from_config(root.join("proxysss.yaml"), config).await?;
    let server = gateway.clone();
    let bind = server.bootstrap_config.http.h3_bind.clone();
    let _server = Task(tokio::spawn(async move {
        server.run_http3(bind).await.unwrap();
    }));
    let mut tls = client_tls(&gateway.bootstrap_config)?;
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )));
    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        endpoint.connect(format!("127.0.0.1:{port}").parse()?, "localhost")?,
    )
    .await??;
    let (mut driver, mut requests) = h3::client::new(h3_quinn::Connection::new(connection)).await?;
    let _driver = Task(tokio::spawn(async move {
        let _ = driver.wait_idle().await;
    }));
    let mut count = 0;
    let mut errors = 0;
    for (path, status) in [
        ("/ok", StatusCode::OK),
        ("/missing", StatusCode::NOT_FOUND),
        ("/error", StatusCode::SERVICE_UNAVAILABLE),
        ("/offline", StatusCode::BAD_GATEWAY),
    ] {
        let mut stream = requests
            .send_request(Request::get(format!("https://localhost{path}")).body(())?)
            .await?;
        stream.finish().await?;
        let response =
            tokio::time::timeout(Duration::from_secs(5), stream.recv_response()).await??;
        assert_eq!(response.status(), status);
        while stream.recv_data().await?.is_some() {}
        count += 1;
        errors += u64::from(status.is_client_error() || status.is_server_error());
        assert_counts(&gateway, count, errors);
    }
    let mut stream = requests
        .send_request(
            Request::post("https://localhost/ok")
                .header("content-length", "32")
                .body(())?,
        )
        .await?;
    stream.finish().await?;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), stream.recv_response())
            .await??
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    while stream.recv_data().await?.is_some() {}
    assert_counts(&gateway, count + 1, errors + 1);
    let mut reserved = gateway.buffered_request_bodies.reservation();
    reserved.reserve(4).unwrap();
    let held = reserved.finish(Bytes::from_static(b"1234"));
    let mut stream = requests
        .send_request(Request::post("https://localhost/ok").body(())?)
        .await?;
    stream.send_data(Bytes::from_static(b"x")).await?;
    let _ = stream.finish().await;
    let response = tokio::time::timeout(Duration::from_secs(5), stream.recv_response()).await??;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    while stream.recv_data().await?.is_some() {}
    assert_eq!(gateway.buffered_request_bodies.used(), 4);
    drop(held);
    let mut stream = requests
        .send_request(Request::post("https://localhost/ok").body(())?)
        .await?;
    stream.send_data(Bytes::from_static(b"x")).await?;
    stream.finish().await?;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), stream.recv_response())
            .await??
            .status(),
        StatusCode::OK
    );
    while stream.recv_data().await?.is_some() {}
    assert_counts(&gateway, count + 3, errors + 2);
    endpoint.close(0_u32.into(), b"test complete");
    Ok(())
}
