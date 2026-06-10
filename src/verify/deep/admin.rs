use anyhow::Result;
use std::error::Error;

use crate::install;
use crate::verify::harness::{
    cleanup, ensure_rustls_crypto_provider, reserve_port, spawn_gateway, temp_root, wait_http_ok,
    write_config,
};

#[tokio::test]
async fn integration_deep_admin_health_endpoint() -> Result<()> {
    let root = temp_root("proxysss-deep-admin");
    let gateway_port = reserve_port().await?;
    let admin_port = reserve_port().await?;
    let yaml = format!(
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
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
admin:
  enabled: true
  bind: 127.0.0.1:{admin_port}
  username: root
  password: root
  enable_write_ops: false
  expose_config: false
"#
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    wait_http_ok(&format!("http://127.0.0.1:{admin_port}/healthz")).await?;
    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_admin_https_api_over_tls() -> Result<()> {
    let root = temp_root("proxysss-deep-admin-https");
    let tls_port = reserve_port().await?;
    let admin_port = reserve_port().await?;
    let cert_path = root.join("certs").join("gateway.crt");
    let key_path = root.join("certs").join("gateway.key");
    install::ensure_cert_pair(&cert_path, &key_path, "localhost", false)?;
    let cert_fwd = cert_path.display().to_string().replace('\\', "/");
    let key_fwd = key_path.display().to_string().replace('\\', "/");
    let blacklist_fwd = root
        .join("blacklist.json")
        .display()
        .to_string()
        .replace('\\', "/");

    let yaml = format!(
        r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: ''
  tls_bind: 127.0.0.1:{tls_port}
  h3_bind: ''
  tls:
    mode: manual
    cert_path: {cert_path}
    key_path: {key_path}
    generate_self_signed_if_missing: false
    server_name: localhost
script:
  enabled: false
plugins:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
admin:
  enabled: true
  bind: 127.0.0.1:{admin_port}
  username: ops
  password: change-me
  bearer_token: cluster-test-token
  enable_write_ops: true
  https:
    enabled: true
    path_prefix: /_proxysss/admin
security:
  dynamic_blacklist:
    enabled: true
    path: {blacklist_path}
"#,
        cert_path = cert_fwd,
        key_path = key_fwd,
        blacklist_path = blacklist_fwd,
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;
    ensure_rustls_crypto_provider();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let stats_url = format!("https://127.0.0.1:{tls_port}/_proxysss/admin/v1/stats");
    let mut last_error = String::new();
    for _ in 0..100 {
        match client
            .get(&stats_url)
            .header("Authorization", "Bearer cluster-test-token")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let body: serde_json::Value = response.json().await?;
                    assert!(body.get("http_requests").is_some());
                    cleanup(&root);
                    return Ok(());
                }
                last_error = format!("status {}", response.status());
            }
            Err(error) => {
                let mut chain = error.to_string();
                let mut src = error.source();
                while let Some(next) = src {
                    chain.push_str(" -> ");
                    chain.push_str(&next.to_string());
                    src = next.source();
                }
                last_error = chain;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("HTTPS admin API did not become ready at {stats_url}: {last_error}");
}

#[tokio::test]
async fn integration_deep_admin_sni_certificate_upsert() -> Result<()> {
    let root = temp_root("proxysss-deep-admin-sni");
    let admin_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let yaml = format!(
        r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
  tls:
    mode: self_signed
    generate_self_signed_if_missing: true
    cert_path: {cert_path}
    key_path: {key_path}
script:
  enabled: false
plugins:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
admin:
  enabled: true
  bind: 127.0.0.1:{admin_port}
  username: ops
  password: change-me
  bearer_token: sni-test-token
  enable_write_ops: true
"#,
        cert_path = root
            .join("certs")
            .join("default.crt")
            .display()
            .to_string()
            .replace('\\', "/"),
        key_path = root
            .join("certs")
            .join("default.key")
            .display()
            .to_string()
            .replace('\\', "/"),
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let client = reqwest::Client::new();
    let upsert_url = format!("http://127.0.0.1:{admin_port}/v1/tls/sni-certificates/upsert");
    let response = client
        .post(&upsert_url)
        .header("Authorization", "Bearer sni-test-token")
        .json(&serde_json::json!({
            "domains": ["api.internal.test"],
            "cert_pem": "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n",
            "key_pem": "-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n",
        }))
        .send()
        .await?;
    assert!(
        response.status().is_success(),
        "upsert failed: {}",
        response.text().await?
    );

    let list_url = format!("http://127.0.0.1:{admin_port}/v1/tls/sni-certificates");
    let listed = client
        .get(&list_url)
        .header("Authorization", "Bearer sni-test-token")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let items = listed
        .get("items")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]
            .get("domains")
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(1)
    );

    cleanup(&root);
    Ok(())
}
