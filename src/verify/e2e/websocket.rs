use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::verify::harness::{
    cleanup, reserve_port, spawn_gateway, spawn_ws_echo_upstream, temp_root, write_config,
};

#[tokio::test]
async fn integration_e2e_websocket_reverse_proxy_echoes() -> Result<()> {
    let root = temp_root("proxysss-e2e-ws");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_ws_echo_upstream(upstream_port).await;
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
      - name: integration-e2e-ws
        path_prefix: /proxy
        upstream: ws://127.0.0.1:{upstream_port}
"#
        ),
    )?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let gateway_url = format!("ws://127.0.0.1:{gateway_port}/proxy/");
    for _ in 0..100 {
        if connect_async(&gateway_url).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let (mut client, _) = connect_async(&gateway_url)
        .await
        .context("websocket client failed to connect through proxysss")?;
    client.send(Message::Text("integration-e2e".into())).await?;
    let echoed = client
        .next()
        .await
        .context("websocket client read timed out")?
        .context("websocket client read failed")?;
    assert_eq!(echoed, Message::Text("integration-e2e".into()));

    cleanup(&root);
    Ok(())
}
