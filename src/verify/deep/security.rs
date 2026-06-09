use anyhow::Result;

use crate::verify::harness::{
    base_gateway_yaml, cleanup, reserve_port, spawn_gateway, temp_root, wait_http_status,
    write_config,
};

#[tokio::test]
async fn integration_deep_access_control_denies_client() -> Result<()> {
    let root = temp_root("proxysss-deep-acl");
    let gateway_port = reserve_port().await?;
    let yaml = format!(
        r#"{base}
services:
  access_control:
    http:
      enabled: true
      allow: [198.51.100.0/24]
      deny: []
      status: 403
"#,
        base = base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/");
    wait_http_status(&url, |status| status == reqwest::StatusCode::FORBIDDEN).await?;

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_rate_limit_blocks_excess_traffic() -> Result<()> {
    let root = temp_root("proxysss-deep-ratelimit");
    let gateway_port = reserve_port().await?;
    let yaml = format!(
        r#"{base}
services:
  rate_limit:
    http:
      enabled: true
      requests: 3
      window_ms: 60000
      burst: 0
      max_connections: 0
"#,
        base = base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/");
    let client = reqwest::Client::new();
    let mut saw_429 = false;
    for _ in 0..12 {
        if client.get(&url).send().await?.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(saw_429, "expected HTTP 429 from rate limiter");

    cleanup(&root);
    Ok(())
}
