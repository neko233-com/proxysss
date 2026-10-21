use super::*;
use crate::verify::harness::{
    base_gateway_yaml, cleanup, ensure_rustls_crypto_provider, reserve_port,
    spawn_tcp_echo_upstream, spawn_udp_echo_upstream, tcp_roundtrip, temp_root, udp_roundtrip,
    write_config,
};

struct Fixture {
    root: PathBuf,
    config: GatewayConfig,
    gateway: Arc<Gateway>,
    runner: JoinHandle<Result<()>>,
    _abort: AbortTaskOnDrop,
    _tcp_echo: AbortTaskOnDrop,
    _udp_echo: AbortTaskOnDrop,
}

impl Fixture {
    async fn new() -> Result<Self> {
        ensure_rustls_crypto_provider();
        let root = temp_root("listener-lifetime");
        let mut config: GatewayConfig =
            serde_yaml::from_str(&base_gateway_yaml(reserve_port().await?))?;
        // This fixture bypasses GatewayConfig::load, so resolve all TLS material
        // into its own root rather than sharing the process working directory.
        config.root_dir = root.clone();
        config.http.tls.cert_path = root.join("cert.pem");
        config.http.tls.key_path = root.join("key.pem");
        config.http.tls_bind = format!("127.0.0.1:{}", reserve_port().await?);
        config.http.h3_bind = format!("127.0.0.1:{}", reserve_port().await?);
        config.admin.enabled = true;
        config.admin.bind = format!("127.0.0.1:{}", reserve_port().await?);
        let tcp_backend = reserve_port().await?;
        let tcp_echo = spawn_tcp_echo_upstream(tcp_backend).await;
        let udp_backend = reserve_port().await?;
        let udp_echo = spawn_udp_echo_upstream(udp_backend).await?;
        let tcp_bind = format!("127.0.0.1:{}", reserve_port().await?);
        let udp_bind = format!("127.0.0.1:{}", reserve_port().await?);
        config
            .tcp
            .listeners
            .push(serde_json::from_value(serde_json::json!({
                "name":"lifetime-tcp", "bind":tcp_bind,
                "upstream":format!("127.0.0.1:{tcp_backend}")
            }))?);
        config
            .udp
            .listeners
            .push(serde_json::from_value(serde_json::json!({
                "name":"lifetime-udp", "bind":udp_bind,
                "upstream":format!("127.0.0.1:{udp_backend}")
            }))?);
        let path = write_config(&root, &serde_yaml::to_string(&config)?)?;
        let gateway = Gateway::from_config(path, config.clone()).await?;
        let runner = tokio::spawn(gateway.clone().run());
        let fixture = Self {
            root,
            config,
            gateway,
            _abort: AbortTaskOnDrop(runner.abort_handle()),
            runner,
            _tcp_echo: AbortTaskOnDrop(tcp_echo.abort_handle()),
            _udp_echo: AbortTaskOnDrop(udp_echo.abort_handle()),
        };
        fixture.wait_ready().await?;
        Ok(fixture)
    }

    async fn wait_ready(&self) -> Result<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(1))
            .build()?;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let plain = client
                    .get(format!("http://{}/", self.config.http.plain_bind))
                    .send()
                    .await;
                let tls = client
                    .get(format!("https://{}/", self.config.http.tls_bind))
                    .send()
                    .await;
                if plain.is_ok_and(|r| r.status().is_success())
                    && tls.is_ok_and(|r| r.status().is_success())
                    && !udp_free(self.config.http.h3_bind.parse()?)
                    && TcpStream::connect(&self.config.admin.bind).await.is_ok()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        assert_eq!(
            tcp_roundtrip(&self.config.tcp.listeners[0].bind, b"tcp-ready").await?,
            b"tcp-ready"
        );
        assert_eq!(
            udp_roundtrip(&self.config.udp.listeners[0].bind, b"udp-ready").await?,
            b"udp-ready"
        );
        Ok(())
    }

    async fn reload(&self, config: &GatewayConfig) -> Result<()> {
        fs::write(
            self.root.join("proxysss.yaml"),
            serde_yaml::to_string(config)?,
        )?;
        self.gateway.reload_from_disk().await
    }

    async fn ports_free(&self, include_plain_admin: bool) -> Result<()> {
        let mut tcp = vec![
            self.config.http.tls_bind.parse()?,
            self.config.tcp.listeners[0].bind.parse()?,
        ];
        if include_plain_admin {
            tcp.push(self.config.http.plain_bind.parse()?);
            tcp.push(self.config.admin.bind.parse()?);
        }
        let udp: [SocketAddr; 2] = [
            self.config.http.h3_bind.parse()?,
            self.config.udp.listeners[0].bind.parse()?,
        ];
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if tcp.iter().all(|addr| tcp_free(*addr)) && udp.iter().all(|addr| udp_free(*addr))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.runner.abort();
        cleanup(&self.root);
    }
}

fn udp_free(address: SocketAddr) -> bool {
    std::net::UdpSocket::bind(address).is_ok()
}

fn tcp_free(address: SocketAddr) -> bool {
    #[cfg(windows)]
    {
        std::net::TcpListener::bind(address).is_ok()
    }
    #[cfg(not(windows))]
    {
        // Permit TIME_WAIT, never SO_REUSEPORT: an orphan accept socket must fail.
        (|| -> std::io::Result<()> {
            let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(SocketProtocol::TCP))?;
            socket.set_reuse_address(true)?;
            socket.bind(&address.into())?;
            socket.listen(8)
        })()
        .is_ok()
    }
}

#[tokio::test]
async fn cancelling_gateway_releases_all_listener_ports_and_udp_leases() -> Result<()> {
    let mut fixture = Fixture::new().await?;
    fixture.runner.abort();
    assert!((&mut fixture.runner).await.unwrap_err().is_cancelled());
    fixture.ports_free(true).await?;
    assert_eq!(
        fixture
            .gateway
            .upstream_runtime
            .iter()
            .filter(|entry| entry.key().starts_with("udp:"))
            .map(|entry| entry.active_connections)
            .sum::<u64>(),
        0
    );
    Ok(())
}

#[tokio::test]
async fn reload_removes_and_recreates_http_tls_tcp_udp_listeners_without_orphans() -> Result<()> {
    let fixture = Fixture::new().await?;
    let mut replacement = fixture.config.clone();
    replacement.http.tls_bind.clear();
    replacement.http.h3_bind.clear();
    replacement.tcp.listeners.clear();
    replacement.udp.listeners.clear();
    fixture.reload(&replacement).await?;
    fixture.ports_free(false).await?;
    fixture.reload(&fixture.config).await?;
    fixture.wait_ready().await?;
    fixture.runner.abort();
    fixture.ports_free(true).await?;
    Ok(())
}
