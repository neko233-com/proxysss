use std::time::Duration;

use anyhow::Result;

use crate::config::{GatewayConfig, TlsMode};
use crate::verify::harness::{
    cleanup, reserve_port, spawn_gateway, spawn_json_echo_upstream, spawn_sse_hold_upstream,
    temp_root, wait_http_ok, write_config,
};

#[tokio::test]
async fn integration_e2e_http_reverse_proxy_roundtrip() -> Result<()> {
    let root = temp_root("proxysss-e2e-http");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await;
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
