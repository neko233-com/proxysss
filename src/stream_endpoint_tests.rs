use super::*;
use crate::verify::harness::{
    base_gateway_yaml, cleanup, reserve_port, spawn_gateway, temp_root, write_config,
};
use serde_json::json;

#[test]
fn config_checks_stream_primary_pool_and_weight_targets_without_dns() -> Result<()> {
    let base: GatewayConfig = serde_yaml::from_str(&base_gateway_yaml(8080))?;
    for (target, valid) in [
        ("127.0.0.1:9000", true),
        ("[::1]:9000", true),
        ("game-server.namespace.svc.cluster.local:9000", true),
        ("game_backend:9000", true),
        ("localhost.:9000", true),
        ("tcp://127.0.0.1:9000", false),
        ("udp://localhost:9000", false),
        ("http://localhost:9000/path", false),
        ("localhost:9000/path", false),
        (" localhost:9000", false),
        ("localhost:9000 ", false),
        ("::1:9000", false),
        ("[::1]:0", false),
        ("localhost:0", false),
        ("localhost:65536", false),
        ("localhost:+9000", false),
        ("localhost", false),
        (":9000", false),
        ("bad..name:9000", false),
        ("999.1.1.1:9000", false),
        ("host\nname:9000", false),
    ] {
        for family in ["tcp", "udp", "stream", "ftp"] {
            for field in ["upstream", "upstreams", "upstream_weights"] {
                if family == "ftp" && field != "upstream" {
                    continue;
                }
                let mut config = base.clone();
                let mut item =
                    json!({"name":"echo", "bind":"127.0.0.1:9001", "upstream":"localhost:9000"});
                item[field] = match field {
                    "upstreams" => json!([target]),
                    "upstream_weights" => json!({(target): 1}),
                    _ => json!(target),
                };
                match family {
                    "tcp" => config.tcp.listeners.push(serde_json::from_value(item)?),
                    "udp" => config.udp.listeners.push(serde_json::from_value(item)?),
                    "stream" => {
                        item.as_object_mut().unwrap().remove("bind");
                        item["listen"] = json!("127.0.0.1:9001");
                        config.tcp.stream_routes.push(serde_json::from_value(item)?);
                    }
                    _ => {
                        config.services.ftp.enabled = true;
                        config.services.ftp.upstream = target.into();
                    }
                }
                let result = config.validate();
                assert_eq!(
                    result.is_ok(),
                    valid,
                    "{family}.{field} {target:?}: {result:?}"
                );
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn udp_hostname_and_ipv6_forward_multiple_datagrams_and_probe() -> Result<()> {
    let localhost_ip = tokio::net::lookup_host(("localhost", 1))
        .await?
        .next()
        .unwrap()
        .ip();
    for (host, ip) in [("localhost", localhost_ip), ("[::1]", "::1".parse()?)] {
        let upstream = UdpSocket::bind(SocketAddr::new(ip, 0)).await?;
        let address = format!("{host}:{}", upstream.local_addr()?.port());
        let (peer_tx, mut peers) = tokio::sync::mpsc::unbounded_channel();
        let echo = tokio::spawn(async move {
            let mut buffer = [0_u8; 2048];
            while let Ok((size, peer)) = upstream.recv_from(&mut buffer).await {
                let _ = peer_tx.send(peer);
                if upstream.send_to(&buffer[..size], peer).await.is_err() {
                    break;
                }
            }
        });
        let root = temp_root("stream-hostname");
        let port = reserve_port().await?;
        let udp_port = reserve_port().await?;
        let yaml = format!("{}\nudp:\n  listeners:\n    - name: echo\n      bind: 127.0.0.1:{udp_port}\n      upstream: '{address}'\n", base_gateway_yaml(port));
        let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = UdpSocket::bind("127.0.0.1:0").await?;
        client.connect(("127.0.0.1", udp_port)).await?;
        let mut first_peer = None;
        for index in 0..8 {
            let payload = format!("hostname-udp-{index}");
            client.send(payload.as_bytes()).await?;
            let mut buffer = [0; 128];
            let count =
                tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buffer)).await??;
            assert_eq!(&buffer[..count], payload.as_bytes());
            let peer = peers.recv().await.unwrap();
            assert_eq!(peer.ip(), ip);
            assert_eq!(
                *first_peer.get_or_insert(peer),
                peer,
                "association must reuse its socket"
            );
        }
        let settings = resolve_global_active_health_config(&ActiveHealthConfig::default());
        let outcome = probe_udp_upstream(&address, &settings).await;
        assert!(outcome.0, "UDP hostname/IPv6 health probe: {outcome:?}");
        runner.abort();
        echo.abort();
        cleanup(&root);
    }
    Ok(())
}

#[tokio::test]
async fn invalid_stream_admin_updates_and_reload_preserve_config_and_live_state() -> Result<()> {
    let root = temp_root("stream-target-admin");
    let mut config: GatewayConfig =
        serde_yaml::from_str(&base_gateway_yaml(reserve_port().await?))?;
    let admin_port = reserve_port().await?;
    config.admin.enabled = true;
    config.admin.bind = format!("127.0.0.1:{admin_port}");
    config.admin.enable_write_ops = true;
    config.admin.bearer_token = "stream-target-test-token".into();
    let original = serde_yaml::to_string(&config)?;
    let path = write_config(&root, &original)?;
    let (gateway, runner) = spawn_gateway(path.clone()).await?;
    let state = gateway.current_state().await;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    crate::verify::harness::wait_http_ok(&format!("http://127.0.0.1:{admin_port}/healthz")).await?;
    for (surface, mut payload) in [
        (
            "tcp-listeners",
            json!({"name":"bad", "bind":"127.0.0.1:19001"}),
        ),
        (
            "udp-listeners",
            json!({"name":"bad", "bind":"127.0.0.1:19002"}),
        ),
        (
            "stream-routes",
            json!({"name":"bad", "listen":"127.0.0.1:19003"}),
        ),
    ] {
        for target in [
            "tcp://127.0.0.1:9000",
            "metadata.google.internal:9000",
            "METADATA.GOOGLE.INTERNAL.:9000",
            "169.254.169.254:9000",
        ] {
            payload["upstream"] = json!(target);
            let response = client
                .post(format!("http://127.0.0.1:{admin_port}/v1/{surface}/upsert"))
                .bearer_auth("stream-target-test-token")
                .json(&payload)
                .send()
                .await?;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{surface}: {target}"
            );
            if !target.starts_with("tcp://") {
                assert!(
                    response.text().await?.contains("blocked"),
                    "must apply the host/IP policy"
                );
            }
            assert_eq!(fs::read_to_string(&path)?, original);
            assert!(Arc::ptr_eq(&state, &gateway.current_state().await));
        }
    }
    let invalid = format!("{original}\ntcp:\n  listeners:\n    - name: bad\n      bind: 127.0.0.1:19004\n      upstream: 'bad:99999'\n");
    // Replace the existing empty tcp section through serde to avoid exercising
    // duplicate YAML keys instead of the endpoint validator.
    let mut value: serde_yaml::Value = serde_yaml::from_str(&original)?;
    let candidate: serde_yaml::Value = serde_yaml::from_str(&invalid[original.len()..])?;
    value["tcp"] = candidate["tcp"].clone();
    fs::write(&path, serde_yaml::to_string(&value)?)?;
    let error = gateway
        .reload_from_disk()
        .await
        .expect_err("invalid reload");
    assert!(format!("{error:#}").contains("stream upstream"));
    assert!(Arc::ptr_eq(&state, &gateway.current_state().await));
    fs::write(&path, original)?;
    runner.abort();
    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn stream_script_validates_pool_and_weight_targets_before_returning_route() -> Result<()> {
    let root = temp_root("stream-target-script");
    fs::create_dir_all(&root)?;
    let entry = root.join("gateway.ts");
    fs::write(
        &entry,
        r#"export default {
        preread(message) {
            if (message.ctx.listener === 'pool') return {upstream:'localhost:9000', upstreams:['tcp://bad:9000']};
            if (message.ctx.listener === 'weights') return {upstream:'localhost:9000', upstream_weights:{'udp://bad:9000':1}};
            if (message.ctx.listener === 'primary') return {upstream:'http://bad:9000'};
            return {upstream:'localhost:9000', upstreams:['[::1]:9000']};
        }
    };"#,
    )?;
    let runtime = ScriptRuntime::spawn(
        &crate::config::ScriptConfig {
            enabled: true,
            entry,
            cwd: Some(root.clone()),
            ..Default::default()
        },
        &BTreeMap::new(),
    )?;
    for listener in ["primary", "pool", "weights", "valid"] {
        let context = StreamContext {
            request_id: "test".into(),
            listener: listener.into(),
            protocol: "tcp".into(),
            remote_addr: "127.0.0.1:1234".into(),
            player_id: None,
            first_packet_preview: None,
            payload_len: 0,
        };
        assert_eq!(
            runtime.route_tcp(context.clone()).await.is_ok(),
            listener == "valid"
        );
        assert_eq!(
            runtime.route_udp(context).await.is_ok(),
            listener == "valid"
        );
    }
    drop(runtime);
    cleanup(&root);
    Ok(())
}

#[test]
fn stream_admin_host_policy_uses_host_component_and_respects_switches() -> Result<()> {
    for target in [
        "metadata.google.internal:9000",
        "METADATA.GOOGLE.INTERNAL.:9000",
        "169.254.169.254:9000",
    ] {
        for pool in [false, true] {
            let mut item = json!({"name":"stream", "upstream":"public.example:9000", "upstreams":[], "bind":"127.0.0.1:19001"});
            if pool {
                item["upstreams"] = json!([target]);
            } else {
                item["upstream"] = json!(target);
            }
            let tcp: TcpListenerConfig = serde_json::from_value(item.clone())?;
            let udp: UdpListenerConfig = serde_json::from_value(item.clone())?;
            item.as_object_mut().unwrap().remove("bind");
            item["listen"] = json!("127.0.0.1:19001");
            let route: StreamRouteConfig = serde_json::from_value(item)?;
            let mut policy = crate::config::SecurityConfig::default();
            assert!(security::validate_tcp_listener_mutation(&tcp, &policy).is_err());
            assert!(security::validate_udp_listener_mutation(&udp, &policy).is_err());
            assert!(security::validate_stream_route_mutation(&route, &policy).is_err());
            policy.block_ssrf_targets = false;
            security::validate_tcp_listener_mutation(&tcp, &policy)?;
            security::validate_udp_listener_mutation(&udp, &policy)?;
            security::validate_stream_route_mutation(&route, &policy)?;
            policy.block_ssrf_targets = true;
            policy.validate_admin_mutations = false;
            security::validate_stream_route_mutation(&route, &policy)?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn udp_probe_deadline_includes_all_work_and_reports_no_reply() -> Result<()> {
    let silent = UdpSocket::bind("127.0.0.1:0").await?;
    let mut settings = resolve_global_active_health_config(&ActiveHealthConfig::default());
    settings.timeout_ms = 100;
    settings.udp_expect_response = true;
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        probe_udp_upstream(&silent.local_addr()?.to_string(), &settings),
    )
    .await?;
    assert!(!outcome.0);
    assert!(outcome.2.unwrap().contains("probe timeout"));
    Ok(())
}
