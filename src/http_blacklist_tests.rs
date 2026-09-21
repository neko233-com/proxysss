use super::*;
use crate::test_support::TestDir;
use crate::verify::harness;
use futures_util::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite};

#[tokio::test]
async fn memory_only_admin_bans_cover_http_keepalive_h2_ai_and_websocket_admission() -> Result<()> {
    let root = TestDir::new("http-memory-blacklist");
    let http_port = harness::reserve_port().await?;
    let tls_port = harness::reserve_port().await?;
    let admin_port = harness::reserve_port().await?;
    let upstream_port = harness::reserve_port().await?;
    let ws_port = harness::reserve_port().await?;
    let upstream = harness::spawn_json_echo_upstream(upstream_port).await?;
    let _upstream_abort = AbortTaskOnDrop(upstream.abort_handle());
    let ws_upstream = harness::spawn_ws_echo_upstream(ws_port).await;
    let _ws_abort = AbortTaskOnDrop(ws_upstream.abort_handle());
    harness::wait_tcp_ready(&format!("127.0.0.1:{ws_port}")).await?;

    let mut config: GatewayConfig = serde_yaml::from_str(&harness::base_gateway_yaml(http_port))?;
    config.http.tls_bind = format!("127.0.0.1:{tls_port}");
    config.http.tls.mode = TlsMode::SelfSigned;
    config.http.tls.generate_self_signed_if_missing = true;
    config.http.tls.cert_path = root.join("cert.pem");
    config.http.tls.key_path = root.join("key.pem");
    config.admin.enabled = true;
    config.admin.bind = format!("127.0.0.1:{admin_port}");
    config.admin.enable_write_ops = true;
    config.admin.bearer_token = "blacklist-admission-test-token-12345".into();
    config.security.ddos.enabled = false;
    config.security.dynamic_blacklist.enabled = false;
    config.load_balance.retries.enabled = false;
    config.load_balance.passive_health.enabled = false;
    config.affinity.enabled = false;
    config.services.reverse_proxy.routes = serde_yaml::from_str(&format!(
        "- name: http\n  path_prefix: /proxy\n  upstream: http://127.0.0.1:{upstream_port}\n- name: ws\n  path_prefix: /ws\n  upstream: ws://127.0.0.1:{ws_port}\n"
    ))?;
    config.services.ai_proxy = serde_yaml::from_str(&format!(
        "enabled: true\nroutes:\n  - name: ai\n    provider: new-api\n    path_prefix: /v1\n    rewrite_base_path: /v1\n    upstream: http://127.0.0.1:{upstream_port}\n"
    ))?;
    let config_path = harness::write_config(&root, &serde_yaml::to_string(&config)?)?;
    let (_gateway, task) = harness::spawn_gateway(config_path).await?;
    let _gateway_abort = AbortTaskOnDrop(task.abort_handle());
    harness::wait_tcp_ready(&config.admin.bind).await?;
    let admin = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    let tls = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(3))
        .build()?;
    // One explicit HTTP/1 connection survives the add/remove operations. This
    // also exercises repeated identical heads that previously hit a raw cache.
    let stream = TcpStream::connect(("127.0.0.1", http_port)).await?;
    let (mut plain, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
    let driver = tokio::spawn(connection);
    let _driver_abort = AbortTaskOnDrop(driver.abort_handle());
    let endpoint = format!("http://127.0.0.1:{admin_port}/v1/security/blacklist");
    let ws_url = format!("ws://127.0.0.1:{http_port}/ws/echo");
    let (mut established_ws, _) = connect_async(&ws_url).await?;

    for (action, expected) in [
        (None, StatusCode::OK),
        (Some("add"), StatusCode::TOO_MANY_REQUESTS),
        (Some("remove"), StatusCode::OK),
    ] {
        if let Some(action) = action {
            let response = admin
                .post(format!("{endpoint}/{action}"))
                .bearer_auth(&config.admin.bearer_token)
                .json(&serde_json::json!({"ip":"127.0.0.1","ban_secs":300}))
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.json::<serde_json::Value>().await?["persisted"],
                false
            );
        }
        for path in ["/proxy/echo", "/v1/chat/completions"] {
            let request = Request::builder()
                .uri(path)
                .header(HOST, format!("127.0.0.1:{http_port}"))
                .header(ACCEPT, "text/event-stream")
                .body(Full::new(Bytes::new()))?;
            let response =
                tokio::time::timeout(Duration::from_secs(3), plain.send_request(request)).await??;
            assert_eq!(response.status(), expected, "HTTP/1 {action:?} {path}");
            let body = tokio::time::timeout(Duration::from_secs(3), response.into_body().collect())
                .await??
                .to_bytes();
            if expected == StatusCode::OK {
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&body)?["path"],
                    path
                );
            }
            let response = tls
                .get(format!("https://127.0.0.1:{tls_port}{path}"))
                .header(ACCEPT, "text/event-stream")
                .send()
                .await?;
            assert_eq!(response.version(), reqwest::Version::HTTP_2);
            assert_eq!(response.status(), expected, "H2 {action:?} {path}");
            response.bytes().await?;
        }
        let new_ws = tokio::time::timeout(Duration::from_secs(3), connect_async(&ws_url)).await?;
        if expected == StatusCode::OK {
            let (mut ws, _) = new_ws?;
            ws.close(None).await?;
        } else {
            match new_ws {
                Err(tungstenite::Error::Http(response)) => assert_eq!(response.status(), expected),
                other => panic!("new WebSocket admission bypassed memory blacklist: {other:?}"),
            }
        }
        // Admission changes must not arbitrarily terminate established game
        // sessions. Their lifecycle still follows the existing relay policy.
        let message = tungstenite::Message::Text(format!("phase-{action:?}").into());
        established_ws.send(message.clone()).await?;
        let echoed = tokio::time::timeout(Duration::from_secs(3), established_ws.next())
            .await?
            .context("existing WebSocket closed after admission update")??;
        assert_eq!(echoed, message);
    }
    established_ws.close(None).await?;
    Ok(())
}
