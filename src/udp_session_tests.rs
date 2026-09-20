use super::*;
use crate::verify::harness::{
    base_gateway_yaml, cleanup, ensure_rustls_crypto_provider, reserve_port, temp_root,
    write_config,
};

struct Fixture {
    root: PathBuf,
    config: GatewayConfig,
    gateway: Arc<Gateway>,
    address: SocketAddr,
    backend: SocketAddr,
    runner: JoinHandle<Result<()>>,
    echo: JoinHandle<()>,
}

impl Fixture {
    async fn new(mode: &str, cap: usize, ttl: u64) -> Result<Self> {
        ensure_rustls_crypto_provider();
        let root = temp_root("udp-admission");
        let backend_ip = tokio::net::lookup_host(("localhost", 1))
            .await?
            .next()
            .unwrap()
            .ip();
        let socket = UdpSocket::bind(SocketAddr::new(backend_ip, 0)).await?;
        let backend = socket.local_addr()?;
        let echo = tokio::spawn(async move {
            let mut buffer = [0_u8; 4096];
            while let Ok((size, peer)) = socket.recv_from(&mut buffer).await {
                if socket.send_to(&buffer[..size], peer).await.is_err() {
                    break;
                }
            }
        });
        let address = SocketAddr::from(([127, 0, 0, 1], reserve_port().await?));
        let mut config: GatewayConfig =
            serde_yaml::from_str(&base_gateway_yaml(reserve_port().await?))?;
        let mut listener: UdpListenerConfig = serde_json::from_value(serde_json::json!({
            "name":"bounded", "bind":address.to_string(), "max_associations":cap,
            "session_ttl_secs":ttl, "upstream":backend.to_string()
        }))?;
        if mode == "dns" {
            listener.upstream = format!("localhost:{}", backend.port());
        }
        if mode == "script" || mode == "failed-script" {
            listener.upstream.clear();
            config.script.enabled = true;
            config.script.entry = root.join("gateway.ts");
            config.script.timeout_ms = 10_000;
            fs::create_dir_all(&root)?;
            let target = if mode == "failed-script" {
                "udp://invalid:1".into()
            } else {
                backend.to_string()
            };
            fs::write(&config.script.entry, format!("export default {{ preread() {{ const until=Date.now()+30; while(Date.now()<until){{}}; return {{upstream:{target:?}}}; }} }};"))?;
        }
        config.udp.listeners.push(listener.clone());
        let path = write_config(&root, &serde_yaml::to_string(&config)?)?;
        let gateway = Gateway::from_config(path, config.clone()).await?;
        let listener_gateway = gateway.clone();
        let runner = tokio::spawn(async move { listener_gateway.run_udp_listener(listener).await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!runner.is_finished(), "UDP listener must stay alive");
        Ok(Self {
            root,
            config,
            gateway,
            address,
            backend,
            runner,
            echo,
        })
    }

    async fn client(&self) -> Result<Arc<UdpSocket>> {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
        socket.connect(self.address).await?;
        Ok(socket)
    }

    fn active(&self) -> u64 {
        self.gateway
            .upstream_runtime
            .iter()
            .map(|entry| entry.active_connections)
            .sum()
    }

    async fn reload(&mut self) -> Result<()> {
        fs::write(
            self.root.join("proxysss.yaml"),
            serde_yaml::to_string(&self.config)?,
        )?;
        self.gateway.reload_from_disk().await?;
        // Receiver snapshots refresh on traffic at most once per second.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.runner.abort();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Ok(socket) = std::net::UdpSocket::bind(self.address) {
                    drop(socket);
                    if self.active() == 0 {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.runner.abort();
        self.echo.abort();
        cleanup(&self.root);
    }
}

async fn echo(client: &UdpSocket, payload: &[u8], deadline_ms: u64) -> bool {
    if client.send(payload).await.is_err() {
        return false;
    }
    let mut buffer = [0_u8; 4096];
    matches!(tokio::time::timeout(Duration::from_millis(deadline_ms), client.recv(&mut buffer)).await,
        Ok(Ok(size)) if &buffer[..size] == payload)
}

#[tokio::test]
async fn udp_concurrent_numeric_dns_and_script_admission_never_exceeds_cap() -> Result<()> {
    for mode in ["numeric", "dns", "script"] {
        let fixture = Fixture::new(mode, 1, 180).await?;
        let mut clients = Vec::new();
        let mut attempts = JoinSet::new();
        let barrier = Arc::new(tokio::sync::Barrier::new(24));
        for index in 0..24 {
            let client = fixture.client().await?;
            clients.push(client.clone());
            let barrier = barrier.clone();
            attempts.spawn(async move {
                barrier.wait().await;
                (index, echo(&client, b"first", 500).await)
            });
        }
        let mut accepted = Vec::new();
        while let Some(result) = attempts.join_next().await {
            let (index, success) = result?;
            if success {
                accepted.push(index);
            }
        }
        assert_eq!(accepted.len(), 1, "{mode}: accepted clients");
        assert_eq!(fixture.active(), 1, "{mode}: actual upstream leases");
        let survivor = &clients[accepted[0]];
        assert!(echo(survivor, b"still-live", 500).await);
        let rejected = &clients[(accepted[0] + 1) % clients.len()];
        assert!(!echo(rejected, b"full", 100).await);
        assert!(echo(survivor, b"not-evicted", 500).await);
        assert_eq!(fixture.active(), 1);
        fixture.stop().await?;
    }
    Ok(())
}

#[tokio::test]
async fn udp_failed_routes_release_reservations_and_reload_can_recover() -> Result<()> {
    let mut fixture = Fixture::new("failed-script", 1, 180).await?;
    for _ in 0..3 {
        let client = fixture.client().await?;
        assert!(!echo(&client, b"bad-route", 120).await);
    }
    assert_eq!(fixture.active(), 0);
    // A failed route did not consume capacity permanently; use the same listener.
    fixture.config.script.enabled = false;
    fixture.config.udp.listeners[0].upstream = fixture.backend.to_string();
    fixture.reload().await?;
    let client = fixture.client().await?;
    assert!(echo(&client, b"recovered", 500).await);
    fixture.stop().await?;
    Ok(())
}

#[tokio::test]
async fn udp_reload_capacity_and_ttl_apply_to_new_sessions_without_evicting_existing() -> Result<()>
{
    let mut fixture = Fixture::new("numeric", 1, 180).await?;
    let old = fixture.client().await?;
    assert!(echo(&old, b"old", 500).await);
    fixture.config.udp.listeners[0].max_associations = 2;
    fixture.config.udp.listeners[0].session_ttl_secs = 1;
    fixture.reload().await?;
    let short = fixture.client().await?;
    assert!(echo(&short, b"short", 500).await);
    assert_eq!(fixture.active(), 2);
    tokio::time::timeout(Duration::from_secs(4), async {
        while fixture.active() != 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    assert!(echo(&old, b"original-ttl", 500).await);
    let replacement = fixture.client().await?;
    assert!(echo(&replacement, b"freed-capacity", 500).await);
    fixture.config.udp.listeners[0].max_associations = 1;
    fixture.reload().await?;
    let denied = fixture.client().await?;
    assert!(!echo(&denied, b"lower-cap", 100).await);
    assert!(echo(&old, b"survived-lower-cap", 500).await);
    fixture.config.udp.listeners[0].max_associations = 0;
    fixture.config.udp.listeners[0].session_ttl_secs = 180;
    fixture.reload().await?;
    let mut held = Vec::new();
    for _ in 0..8 {
        let client = fixture.client().await?;
        assert!(echo(&client, b"unlimited", 500).await);
        held.push(client);
    }
    assert!(fixture.active() >= 9);
    fixture.stop().await?;
    Ok(())
}

#[tokio::test]
async fn udp_retirement_wakes_silent_reader_and_releases_weak_cached_socket() -> Result<()> {
    let state = Arc::new(UdpPruneState::new());
    let table = Arc::new(DashMap::new());
    let client: SocketAddr = "127.0.0.1:32101".parse()?;
    let upstream = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
    let upstream_addr = upstream.local_addr()?;
    let association = Arc::new(UdpAssociation {
        socket: upstream,
        last_seen_epoch: AtomicU64::new(now_unix_secs()),
        active: AtomicBool::new(true),
        session_ttl_secs: 180,
        retired: tokio::sync::Notify::new(),
        _permit: state.try_reserve(1),
    });
    let weak = Arc::downgrade(&association);
    publish_udp_association(&table, client, association.clone(), &state)?;
    let runtime = Arc::new(DashMap::new());
    let reader = spawn_udp_association_reader(
        Arc::new(UdpSocket::bind("127.0.0.1:0").await?),
        table.clone(),
        association.clone(),
        client,
        UpstreamLease::acquire(runtime.clone(), "test".into()),
    );
    let mut waiters = JoinSet::new();
    for _ in 0..2 {
        let association = association.clone();
        waiters.spawn(async move {
            association.wait_retired().await;
        });
    }
    tokio::task::yield_now().await;
    retire_udp_association(&table, client, &association);
    // A waiter registered after retirement must also complete immediately.
    tokio::time::timeout(Duration::from_secs(1), association.wait_retired()).await?;
    drop(association);
    tokio::time::timeout(Duration::from_secs(1), reader).await??;
    while let Some(result) =
        tokio::time::timeout(Duration::from_secs(1), waiters.join_next()).await?
    {
        result?;
    }
    assert!(table.is_empty());
    assert!(weak.upgrade().is_none());
    assert_eq!(state.sessions.load(Ordering::Acquire), 0);
    assert_eq!(runtime.get("test").unwrap().active_connections, 0);
    // Prove the old file descriptor/socket is actually closed, not only uncounted.
    let _rebound = std::net::UdpSocket::bind(upstream_addr)?;
    assert!(state.try_reserve(1).is_some());
    Ok(())
}

#[tokio::test]
async fn udp_continuous_backend_replies_do_not_extend_client_idle_ttl() -> Result<()> {
    let state = Arc::new(UdpPruneState::new());
    let table = Arc::new(DashMap::new());
    let client = UdpSocket::bind("127.0.0.1:0").await?;
    let client_addr = client.local_addr()?;
    let backend = UdpSocket::bind("127.0.0.1:0").await?;
    let upstream = UdpSocket::bind("127.0.0.1:0").await?;
    upstream.connect(backend.local_addr()?).await?;
    backend.connect(upstream.local_addr()?).await?;
    let association = Arc::new(UdpAssociation {
        socket: Arc::new(upstream),
        last_seen_epoch: AtomicU64::new(now_unix_secs()),
        active: AtomicBool::new(true),
        session_ttl_secs: 1,
        retired: tokio::sync::Notify::new(),
        _permit: state.try_reserve(1),
    });
    let weak = Arc::downgrade(&association);
    publish_udp_association(&table, client_addr, association.clone(), &state)?;
    let runtime = Arc::new(DashMap::new());
    let reader = spawn_udp_association_reader(
        Arc::new(UdpSocket::bind("127.0.0.1:0").await?),
        table.clone(),
        association,
        client_addr,
        UpstreamLease::acquire(runtime.clone(), "test".into()),
    );
    let sender = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(10));
        loop {
            tick.tick().await;
            if backend.send(b"unsolicited").await.is_err() {
                break;
            }
        }
    });
    let mut buffer = [0_u8; 64];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), client.recv(&mut buffer)).await??,
        b"unsolicited".len()
    );
    let result = tokio::time::timeout(Duration::from_secs(4), reader).await;
    sender.abort();
    result??;
    assert!(weak.upgrade().is_none());
    assert!(table.is_empty());
    assert_eq!(state.sessions.load(Ordering::Acquire), 0);
    assert_eq!(runtime.get("test").unwrap().active_connections, 0);
    Ok(())
}

#[tokio::test]
async fn udp_pressure_preserves_live_entries_and_reader_abort_cleans_up() -> Result<()> {
    let state = Arc::new(UdpPruneState::new());
    let table = Arc::new(DashMap::new());
    let mut weak_entries = Vec::new();
    let runtime = Arc::new(DashMap::new());
    let mut readers = Vec::new();
    for port in 32200..32216 {
        let client = SocketAddr::from(([127, 0, 0, 1], port));
        let association = Arc::new(UdpAssociation {
            socket: Arc::new(UdpSocket::bind("127.0.0.1:0").await?),
            last_seen_epoch: AtomicU64::new(now_unix_secs()),
            active: AtomicBool::new(true),
            session_ttl_secs: 180,
            retired: tokio::sync::Notify::new(),
            _permit: state.try_reserve(16),
        });
        weak_entries.push(Arc::downgrade(&association));
        publish_udp_association(&table, client, association.clone(), &state)?;
        readers.push(spawn_udp_association_reader(
            Arc::new(UdpSocket::bind("127.0.0.1:0").await?),
            table.clone(),
            association,
            client,
            UpstreamLease::acquire(runtime.clone(), "test".into()),
        ));
    }
    state.last_prune_epoch.store(0, Ordering::Relaxed);
    maybe_prune_udp_associations(&table, &state, 16);
    assert_eq!(
        table.len(),
        16,
        "pressure must not evict live game sessions"
    );
    assert!(state.try_reserve(16).is_none());
    // Abort before some readers have even been polled: their drop guard must run.
    for reader in &readers {
        reader.abort();
    }
    for reader in readers {
        let _ = reader.await;
    }
    assert!(table.is_empty());
    assert_eq!(state.sessions.load(Ordering::Acquire), 0);
    assert!(weak_entries.iter().all(|entry| entry.upgrade().is_none()));
    assert_eq!(runtime.get("test").unwrap().active_connections, 0);
    drop(UdpListenerLifetime {
        state: state.clone(),
        associations: table.clone(),
    });
    assert!(
        state.try_reserve(0).is_none(),
        "closed listener cannot admit even unlimited sessions"
    );
    Ok(())
}
