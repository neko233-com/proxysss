use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::{GatewayConfig, TlsMode};
use crate::verify::harness::{
    cleanup, reserve_port, spawn_gateway, spawn_json_echo_upstream, spawn_sse_hold_upstream,
    temp_root, wait_http_ok, wait_tcp_ready, write_config,
};

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

async fn read_http_response(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
) -> Result<(u16, serde_json::Value)> {
    let deadline = Duration::from_secs(3);
    loop {
        if let Some(head_end) = find_header_end(buffer) {
            let head = std::str::from_utf8(&buffer[..head_end])?.to_string();
            let status = head
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|code| code.parse::<u16>().ok())
                .context("missing HTTP status")?;
            let content_length = head
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .context("missing content-length")?;
            let total = head_end + 4 + content_length;
            while buffer.len() < total {
                let mut chunk = [0_u8; 4096];
                let read = tokio::time::timeout(deadline, stream.read(&mut chunk)).await??;
                if read == 0 {
                    anyhow::bail!("connection closed before full response body");
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            let body = serde_json::from_slice(&buffer[head_end + 4..total])?;
            buffer.drain(..total);
            return Ok((status, body));
        }
        let mut chunk = [0_u8; 4096];
        let read = tokio::time::timeout(deadline, stream.read(&mut chunk)).await??;
        if read == 0 {
            anyhow::bail!("connection closed before response headers");
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

#[tokio::test]
async fn integration_e2e_http_reverse_proxy_roundtrip() -> Result<()> {
    let root = temp_root("proxysss-e2e-http");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: integration-e2e
        path_prefix: /proxy
        upstream: http://127.0.0.1:{upstream_port}
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/proxy/grpc-health");
    wait_http_ok(&url).await?;

    let response = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/grpc")
        .body(vec![0_u8; 8])
        .send()
        .await?;
    assert!(response.status().is_success());
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["content_type"], "application/grpc");
    assert_eq!(payload["body_len"], 8);

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_e2e_http_reverse_proxy_plain_fast_lane_keeps_forwarding_headers() -> Result<()>
{
    let root = temp_root("proxysss-e2e-http-forwarding");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: integration-e2e-forward
        path_prefix: /proxy
        strip_prefix: true
        upstream: http://127.0.0.1:{upstream_port}
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/proxy/ping");
    wait_http_ok(&url).await?;

    let payload: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .header("x-forwarded-for", "198.51.100.10")
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(payload["path"], "/ping");
    assert_eq!(payload["x_real_ip"], "127.0.0.1");
    assert_eq!(
        payload["x_forwarded_for"],
        serde_json::Value::String("198.51.100.10, 127.0.0.1".to_string())
    );
    assert_eq!(
        payload["x_forwarded_host"],
        serde_json::Value::String(format!("127.0.0.1:{gateway_port}"))
    );
    assert_eq!(payload["x_forwarded_proto"], "http");
    assert!(payload["forwarded"]
        .as_str()
        .unwrap_or_default()
        .contains("for=127.0.0.1"));

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_e2e_ai_proxy_streams_sse_without_buffering() -> Result<()> {
    let root = temp_root("proxysss-e2e-ai-sse");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_sse_hold_upstream(upstream_port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  ai_proxy:
    enabled: true
    routes:
      - name: new-api-stream
        provider: new-api
        match_host: ai.local
        path_prefix: /v1
        upstream: http://127.0.0.1:{upstream_port}
        rewrite_base_path: /v1
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/v1/chat/completions");
    let mut stream = reqwest::Client::new()
        .get(&url)
        .header("host", "ai.local")
        .send()
        .await?
        .bytes_stream();

    let first = tokio::time::timeout(
        Duration::from_millis(1500),
        futures_util::StreamExt::next(&mut stream),
    )
    .await?
    .expect("first SSE chunk")?;
    assert!(
        String::from_utf8_lossy(&first).contains("data: first"),
        "first chunk was {:?}",
        first
    );

    cleanup(&root);
    Ok(())
}

#[test]
fn integration_e2e_http3_bind_is_validated() -> Result<()> {
    let mut config = GatewayConfig::default();
    config.http.h3_bind = "127.0.0.1:443".to_string();
    config.http.tls_bind = "127.0.0.1:443".to_string();
    config.http.tls.mode = TlsMode::SelfSigned;
    config.validate()?;
    Ok(())
}

#[tokio::test]
async fn integration_e2e_ai_proxy_raw_sse_fast_lane_keeps_forwarding_and_metadata_headers(
) -> Result<()> {
    let root = temp_root("proxysss-e2e-ai-sse-headers");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  ai_proxy:
    enabled: true
    routes:
      - name: new-api-stream
        provider: new-api
        match_host: ai.local
        path_prefix: /v1
        upstream: http://127.0.0.1:{upstream_port}
        rewrite_base_path: /v1
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/v1/chat/completions");
    let payload: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .header("host", "ai.local")
        .header("accept", "text/event-stream")
        .header("x-forwarded-for", "198.51.100.55")
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(payload["path"], "/v1/chat/completions");
    assert_eq!(payload["ai_route"], "new-api-stream");
    assert_eq!(payload["ai_provider"], "new-api");
    assert_eq!(payload["ai_original_path"], "/v1/chat/completions");
    assert_eq!(payload["x_real_ip"], "127.0.0.1");
    assert_eq!(
        payload["x_forwarded_for"],
        serde_json::Value::String("198.51.100.55, 127.0.0.1".to_string())
    );
    assert_eq!(payload["x_forwarded_host"], "ai.local");
    assert_eq!(payload["x_forwarded_proto"], "http");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_e2e_https_http2_reverse_proxy_roundtrip() -> Result<()> {
    let root = temp_root("proxysss-e2e-https-h2");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let cert_path = root.join("proxysss-test.crt");
    let key_path = root.join("proxysss-test.key");
    let cert_path = cert_path.display().to_string().replace('\\', "/");
    let key_path = key_path.display().to_string().replace('\\', "/");

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: ''
  tls_bind: 127.0.0.1:{gateway_port}
  h3_bind: ''
  tls:
    mode: self_signed
    generate_self_signed_if_missing: true
    cert_path: '{cert_path}'
    key_path: '{key_path}'
    server_name: localhost
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: integration-e2e-h2
        path_prefix: /proxy
        strip_prefix: true
        upstream: http://127.0.0.1:{upstream_port}
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("https://127.0.0.1:{gateway_port}/proxy/ping");
    let response = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()?
        .get(&url)
        .send()
        .await?;
    assert_eq!(response.version(), reqwest::Version::HTTP_2);
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["path"], "/ping");
    assert_eq!(payload["x_forwarded_proto"], "https");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_e2e_admin_route_upsert_refreshes_plain_lane_keepalive() -> Result<()> {
    let root = temp_root("proxysss-e2e-admin-route-refresh");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let admin_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let token = "e2e-admin-route-refresh-token-123";

    let config_path = write_config(
        &root,
        &format!(
            r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: true
  bind: 127.0.0.1:{admin_port}
  enable_write_ops: true
  bearer_token: "{token}"
security:
  block_ssrf_targets: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: lane
        path_prefix: /proxy
        upstream: http://127.0.0.1:{upstream_port}
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;
    wait_tcp_ready(&format!("127.0.0.1:{admin_port}")).await?;

    let mut stream = TcpStream::connect(("127.0.0.1", gateway_port)).await?;
    let mut buffer = Vec::new();
    let host = format!("127.0.0.1:{gateway_port}");

    stream
        .write_all(
            format!(
                "GET /proxy/echo HTTP/1.1\r\nHost: {host}\r\nConnection: keep-alive\r\n\r\n"
            )
            .as_bytes(),
        )
        .await?;
    let (status, body) = read_http_response(&mut stream, &mut buffer).await?;
    assert_eq!(status, 200);
    assert_eq!(body["path"], "/proxy/echo");

    let response = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{admin_port}/v1/reverse-proxy-routes/upsert"
        ))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "name": "lane",
            "path_prefix": "/proxy",
            "strip_prefix": true,
            "upstream": format!("http://127.0.0.1:{upstream_port}"),
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    assert_eq!(status, reqwest::StatusCode::OK, "admin upsert failed: {body}");

    // A fresh connection must already see the new route; this separates a
    // reload failure from a stale keep-alive lane.
    let fresh: serde_json::Value = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{gateway_port}/proxy/echo"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        fresh["path"], "/echo",
        "fresh request after admin upsert: {fresh}"
    );

    // The same keep-alive connection was served by the raw lane before the
    // write. It must not keep the retired route snapshot.
    stream
        .write_all(
            format!(
                "GET /proxy/echo HTTP/1.1\r\nHost: {host}\r\nConnection: keep-alive\r\n\r\n"
            )
            .as_bytes(),
        )
        .await?;
    let (status, body) = read_http_response(&mut stream, &mut buffer).await?;
    assert_eq!(status, 200);
    assert_eq!(body["path"], "/echo");

    cleanup(&root);
    Ok(())
}
