use anyhow::Result;

use crate::verify::harness::{
    cleanup, reserve_port, spawn_gateway, temp_root, wait_http_ok, write_config,
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
