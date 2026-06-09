use std::time::Duration;

use anyhow::Result;

use crate::verify::harness::{
    base_gateway_yaml, cleanup, reserve_port, spawn_gateway, spawn_tcp_echo_upstream,
    spawn_udp_echo_upstream, tcp_roundtrip, temp_root, udp_roundtrip, write_config,
};

#[tokio::test]
async fn integration_deep_tcp_listener_proxies_payload() -> Result<()> {
    let root = temp_root("proxysss-deep-tcp");
    let gateway_port = reserve_port().await?;
    let upstream_port = reserve_port().await?;
    let tcp_listen_port = reserve_port().await?;
    let _upstream = spawn_tcp_echo_upstream(upstream_port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
tcp:
  listeners:
    - name: echo-tcp
      bind: 127.0.0.1:{tcp_listen_port}
      upstream: 127.0.0.1:{upstream_port}
"#,
        base = base_gateway_yaml(gateway_port),
        tcp_listen_port = tcp_listen_port,
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;
    let payload = b"tcp-deep-e2e";
    assert_eq!(
        tcp_roundtrip(&format!("127.0.0.1:{tcp_listen_port}"), payload).await?,
        payload
    );

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_udp_listener_proxies_payload() -> Result<()> {
    let root = temp_root("proxysss-deep-udp");
    let gateway_port = reserve_port().await?;
    let upstream_port = reserve_port().await?;
    let udp_listen_port = reserve_port().await?;

    let _upstream = match spawn_udp_echo_upstream(upstream_port).await {
        Ok(handle) => handle,
        Err(error) if cfg!(windows) => {
            eprintln!("skipping udp deep test on Windows: {error}");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
udp:
  listeners:
    - name: echo-udp
      bind: 127.0.0.1:{udp_listen_port}
      upstream: 127.0.0.1:{upstream_port}
"#,
        base = base_gateway_yaml(gateway_port),
        udp_listen_port = udp_listen_port,
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    tokio::time::sleep(Duration::from_millis(200)).await;
    let payload = b"udp-deep-e2e";
    assert_eq!(
        udp_roundtrip(&format!("127.0.0.1:{udp_listen_port}"), payload).await?,
        payload
    );

    cleanup(&root);
    Ok(())
}
