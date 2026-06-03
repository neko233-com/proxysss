use std::collections::{hash_map::DefaultHasher, BTreeMap, HashSet};
use std::convert::Infallible;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use bytes::{Buf, Bytes, BytesMut};
use dashmap::DashMap;
use h3::server::Connection as H3Connection;
use http::header::{AUTHORIZATION, COOKIE, HOST};
use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri, Version};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;
use url::Url;
use uuid::Uuid;
use serde::Deserialize;

use crate::config::{
    AcmeChallengeType, AdminConfig, GatewayConfig, HttpAffinityConfig, LoadBalanceAlgorithm,
    StreamAffinityConfig, TcpListenerConfig, TlsMode, UdpListenerConfig,
};
use crate::install;
use crate::script::{HttpContext, RouteDecision, ScriptPluginSpec, ScriptRuntime, StreamContext};

#[derive(Clone)]
pub struct Gateway {
    config_path: PathBuf,
    bootstrap_config: GatewayConfig,
    dynamic: Arc<RwLock<Arc<DynamicState>>>,
    stats: Arc<GatewayStats>,
    sticky_affinity: Arc<DashMap<String, StickyEntry>>,
    round_robin_state: Arc<DashMap<String, u64>>,
    upstream_runtime: Arc<DashMap<String, UpstreamRuntimeState>>,
}

struct DynamicState {
    config: GatewayConfig,
    http_client: reqwest::Client,
    script: Arc<ScriptRuntime>,
}

struct GatewayHttpResponse {
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
    upstream: String,
}

#[derive(Clone)]
struct StickyEntry {
    upstream: String,
    expires_at: Instant,
}

#[derive(Clone)]
struct UpstreamRuntimeState {
    consecutive_failures: u32,
    quarantined_until: Option<Instant>,
    active_connections: u64,
}

#[derive(Default)]
struct GatewayStats {
    http_requests: AtomicU64,
    http_errors: AtomicU64,
    tcp_sessions_total: AtomicU64,
    tcp_sessions_active: AtomicU64,
    udp_packets_total: AtomicU64,
    udp_bytes_total: AtomicU64,
    reload_success_total: AtomicU64,
    reload_failure_total: AtomicU64,
    admin_requests_total: AtomicU64,
    admin_auth_fail_total: AtomicU64,
    script_fail_total: AtomicU64,
}

impl Default for UpstreamRuntimeState {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            quarantined_until: None,
            active_connections: 0,
        }
    }
}

struct UpstreamLease {
    runtime: Arc<DashMap<String, UpstreamRuntimeState>>,
    key: String,
}

impl UpstreamLease {
    fn acquire(runtime: Arc<DashMap<String, UpstreamRuntimeState>>, key: String) -> Self {
        {
            let mut entry = runtime.entry(key.clone()).or_default();
            entry.active_connections = entry.active_connections.saturating_add(1);
        }

        Self { runtime, key }
    }
}

impl Drop for UpstreamLease {
    fn drop(&mut self) {
        if let Some(mut entry) = self.runtime.get_mut(&self.key) {
            entry.active_connections = entry.active_connections.saturating_sub(1);
        }
    }
}

impl Gateway {
    pub async fn from_config(config_path: PathBuf, config: GatewayConfig) -> Result<Arc<Self>> {
        prepare_tls_material(&config)?;

        let dynamic = Arc::new(build_dynamic_state(config.clone()).await?);

        Ok(Arc::new(Self {
            config_path,
            bootstrap_config: config,
            dynamic: Arc::new(RwLock::new(dynamic)),
            stats: Arc::new(GatewayStats::default()),
            sticky_affinity: Arc::new(DashMap::new()),
            round_robin_state: Arc::new(DashMap::new()),
            upstream_runtime: Arc::new(DashMap::new()),
        }))
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut tasks = JoinSet::new();

        if self.bootstrap_config.runtime.hot_reload.enabled {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_hot_reload_loop().await });
        }

        if self.bootstrap_config.http.tls.mode == TlsMode::AcmeExternal {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_acme_renew_loop().await });
        }

        if !self.bootstrap_config.http.plain_bind.trim().is_empty() {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_plain_http().await });
        }

        if !self.bootstrap_config.http.tls_bind.trim().is_empty() {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_tls_http().await });
        }

        if !self.bootstrap_config.http.h3_bind.trim().is_empty() {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_http3().await });
        }

        for listener in self.bootstrap_config.tcp.listeners.clone() {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_tcp_listener(listener).await });
        }

        for listener in self.bootstrap_config.udp.listeners.clone() {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_udp_listener(listener).await });
        }

        if self.bootstrap_config.admin.enabled {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_admin_server().await });
        }

        while let Some(result) = tasks.join_next().await {
            result??;
        }

        Ok(())
    }

    async fn run_hot_reload_loop(self: Arc<Self>) -> Result<()> {
        let mut last_hash = read_file_hash(&self.config_path).unwrap_or(0);

        loop {
            let interval_ms = {
                let state = self.current_state().await;
                state.config.runtime.hot_reload.interval_ms.max(200)
            };
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;

            let hash = match read_file_hash(&self.config_path) {
                Ok(value) => value,
                Err(error) => {
                    self.stats.reload_failure_total.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(?error, path = %self.config_path.display(), "hot reload failed to read config hash");
                    continue;
                }
            };

            if hash == last_hash {
                continue;
            }

            match self.reload_from_disk().await {
                Ok(()) => {
                    last_hash = hash;
                    self.stats.reload_success_total.fetch_add(1, Ordering::Relaxed);
                }
                Err(error) => {
                    self.stats.reload_failure_total.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(?error, path = %self.config_path.display(), "hot reload rejected new config");
                }
            }
        }
    }

    async fn run_acme_renew_loop(self: Arc<Self>) -> Result<()> {
        let tls = self.bootstrap_config.http.tls.clone();
        let renew_every = Duration::from_secs(tls.acme.renew_interval_hours.max(1) * 3600);

        loop {
            tokio::time::sleep(renew_every).await;
            let tls = tls.clone();
            let renew_result = tokio::task::spawn_blocking(move || run_acme_command(&tls, true)).await;

            match renew_result {
                Ok(Ok(())) => tracing::info!("acme renewal succeeded"),
                Ok(Err(error)) => tracing::warn!(?error, "acme renewal failed"),
                Err(error) => tracing::warn!(?error, "acme renewal task join failed"),
            }
        }
    }

    async fn run_admin_server(self: Arc<Self>) -> Result<()> {
        let bind_addr: SocketAddr = self
            .bootstrap_config
            .admin
            .bind
            .parse()
            .context("invalid admin.bind address")?;
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind admin listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "admin listener ready");

        loop {
            let (stream, remote_addr) = listener.accept().await.context("admin accept failed")?;
            let gateway = self.clone();

            tokio::spawn(async move {
                let service = service_fn(move |request| {
                    let gateway = gateway.clone();
                    async move { gateway.handle_admin_request(request, remote_addr).await }
                });

                let result = AutoBuilder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(TokioIo::new(stream), service)
                    .await;

                if let Err(error) = result {
                    tracing::warn!(?error, %remote_addr, "admin connection failed");
                }
            });
        }
    }

    async fn handle_admin_request(
        self: Arc<Self>,
        mut request: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        self.stats.admin_requests_total.fetch_add(1, Ordering::Relaxed);

        let method = request.method().clone();
        let path = request.uri().path().to_string();

        if path == "/healthz" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "service": "proxysss", "remote_addr": remote_addr.to_string()}),
            ));
        }

        let state = self.current_state().await;
        if !is_authorized(request.headers().get(AUTHORIZATION), &state.config.admin) {
            self.stats.admin_auth_fail_total.fetch_add(1, Ordering::Relaxed);
            return Ok(text_response(StatusCode::UNAUTHORIZED, "unauthorized"));
        }

        if method == Method::GET && path == "/v1/stats" {
            return Ok(json_response(StatusCode::OK, self.stats.snapshot_json()));
        }

        if method == Method::GET && path == "/v1/upstreams" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "items": self.upstream_runtime_snapshot()}),
            ));
        }

        if method == Method::GET && path == "/v1/config" {
            if !state.config.admin.expose_config {
                return Ok(text_response(StatusCode::FORBIDDEN, "config endpoint disabled"));
            }
            return Ok(json_response(StatusCode::OK, sanitize_config(&state.config)));
        }

        if method == Method::GET && path == "/v1/plugins" {
            if !state.config.plugins.enabled {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugins are disabled by config"));
            }
            if !state.config.plugins.allow_admin_manage {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugin management disabled"));
            }

            match state.script.list_plugins().await {
                Ok(plugins) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "plugins": plugins}),
                    ));
                }
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_GATEWAY,
                        serde_json::json!({"ok": false, "error": error.to_string()}),
                    ));
                }
            }
        }

        if method == Method::POST && path == "/v1/plugins/load" {
            if !state.config.plugins.enabled {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugins are disabled by config"));
            }
            if !state.config.admin.enable_write_ops || !state.config.plugins.allow_admin_manage {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugin write operations disabled"));
            }

            let body = match request.body_mut().collect().await {
                Ok(body) => body.to_bytes(),
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid body: {error}")}),
                    ));
                }
            };

            let spec = match serde_json::from_slice::<ScriptPluginSpec>(&body) {
                Ok(spec) => spec,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid plugin spec: {error}")}),
                    ));
                }
            };

            match state.script.load_plugin(spec).await {
                Ok(data) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "data": data}),
                    ));
                }
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": error.to_string()}),
                    ));
                }
            }
        }

        if method == Method::POST && path == "/v1/plugins/unload" {
            if !state.config.plugins.enabled {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugins are disabled by config"));
            }
            if !state.config.admin.enable_write_ops || !state.config.plugins.allow_admin_manage {
                return Ok(text_response(StatusCode::FORBIDDEN, "plugin write operations disabled"));
            }

            let body = match request.body_mut().collect().await {
                Ok(body) => body.to_bytes(),
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid body: {error}")}),
                    ));
                }
            };

            let unload = match serde_json::from_slice::<PluginUnloadRequest>(&body) {
                Ok(data) => data,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid unload request: {error}")}),
                    ));
                }
            };

            match state.script.unload_plugin(&unload.name).await {
                Ok(data) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "data": data}),
                    ));
                }
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": error.to_string()}),
                    ));
                }
            }
        }

        if method == Method::POST && path == "/v1/reload" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(StatusCode::FORBIDDEN, "write operations disabled"));
            }

            match self.reload_from_disk().await {
                Ok(()) => {
                    self.stats.reload_success_total.fetch_add(1, Ordering::Relaxed);
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "message": "reloaded"}),
                    ));
                }
                Err(error) => {
                    self.stats.reload_failure_total.fetch_add(1, Ordering::Relaxed);
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": error.to_string()}),
                    ));
                }
            }
        }

        Ok(text_response(StatusCode::NOT_FOUND, "not found"))
    }

    async fn reload_from_disk(&self) -> Result<()> {
        let new_config = GatewayConfig::load(&self.config_path)?;
        ensure_reload_compatible(&self.bootstrap_config, &new_config)?;
        prepare_tls_material(&new_config)?;

        let new_state = Arc::new(build_dynamic_state(new_config.clone()).await?);
        {
            let mut state = self.dynamic.write().await;
            *state = new_state;
        }

        for warning in new_config.warnings() {
            tracing::warn!(warning, "configuration warning");
        }

        tracing::info!(path = %self.config_path.display(), "configuration reloaded");
        Ok(())
    }

    async fn run_plain_http(self: Arc<Self>) -> Result<()> {
        let bind_addr: SocketAddr = self
            .bootstrap_config
            .http
            .plain_bind
            .parse()
            .context("invalid http.plain_bind address")?;
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind plain http listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "plain http listener ready");

        loop {
            let (stream, remote_addr) = listener.accept().await.context("plain http accept failed")?;
            let gateway = self.clone();

            tokio::spawn(async move {
                let service = service_fn(move |request| {
                    let gateway = gateway.clone();
                    async move { gateway.handle_hyper_request(request, remote_addr, "http").await }
                });

                let result = AutoBuilder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(TokioIo::new(stream), service)
                    .await;

                if let Err(error) = result {
                    tracing::warn!(?error, %remote_addr, "plain http connection failed");
                }
            });
        }
    }

    async fn run_tls_http(self: Arc<Self>) -> Result<()> {
        let bind_addr: SocketAddr = self
            .bootstrap_config
            .http
            .tls_bind
            .parse()
            .context("invalid http.tls_bind address")?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(self.build_rustls_server_config(vec![b"h2".to_vec(), b"http/1.1".to_vec()])?));
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind tls http listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "tls http listener ready");

        loop {
            let (stream, remote_addr) = listener.accept().await.context("tls http accept failed")?;
            let acceptor = tls_acceptor.clone();
            let gateway = self.clone();

            tokio::spawn(async move {
                let tls_stream = match acceptor.accept(stream).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, "tls handshake failed");
                        return;
                    }
                };

                let service = service_fn(move |request| {
                    let gateway = gateway.clone();
                    async move { gateway.handle_hyper_request(request, remote_addr, "https").await }
                });

                let result = AutoBuilder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(TokioIo::new(tls_stream), service)
                    .await;

                if let Err(error) = result {
                    tracing::warn!(?error, %remote_addr, "tls http connection failed");
                }
            });
        }
    }

    async fn run_http3(self: Arc<Self>) -> Result<()> {
        let bind_addr: SocketAddr = self
            .bootstrap_config
            .http
            .h3_bind
            .parse()
            .context("invalid http.h3_bind address")?;

        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(
            self.build_rustls_server_config(vec![b"h3".to_vec()])?,
        )?));
        let transport = Arc::get_mut(&mut server_config.transport).context("failed to configure quic transport")?;
        transport.keep_alive_interval(Some(Duration::from_secs(15)));

        let endpoint = quinn::Endpoint::server(server_config, bind_addr)
            .with_context(|| format!("failed to bind http3 listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "http3 listener ready");

        loop {
            let Some(connecting) = endpoint.accept().await else {
                return Err(anyhow!("http3 endpoint stopped accepting"));
            };

            let gateway = self.clone();
            tokio::spawn(async move {
                let connection = match connecting.await {
                    Ok(connection) => connection,
                    Err(error) => {
                        tracing::warn!(?error, "quic connection failed");
                        return;
                    }
                };

                let remote_addr = connection.remote_address();
                let quinn_connection = h3_quinn::Connection::new(connection);
                let mut h3_connection = match H3Connection::new(quinn_connection).await {
                    Ok(connection) => connection,
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, "failed to establish h3 connection");
                        return;
                    }
                };

                loop {
                    match h3_connection.accept().await {
                        Ok(Some(request_resolver)) => {
                            let (request, mut stream) = match request_resolver.resolve_request().await {
                                Ok(value) => value,
                                Err(error) => {
                                    tracing::warn!(?error, %remote_addr, "failed resolving h3 request");
                                    continue;
                                }
                            };

                            let (parts, _) = request.into_parts();
                            let mut body = BytesMut::new();

                            loop {
                                match stream.recv_data().await {
                                    Ok(Some(mut chunk)) => {
                                        while chunk.has_remaining() {
                                            let data = chunk.chunk();
                                            if data.is_empty() {
                                                break;
                                            }
                                            body.extend_from_slice(data);
                                            let advance = data.len();
                                            chunk.advance(advance);
                                        }
                                    }
                                    Ok(None) => break,
                                    Err(error) => {
                                        tracing::warn!(?error, %remote_addr, "failed reading h3 body");
                                        break;
                                    }
                                }
                            }

                            let response = match gateway
                                .dispatch_http(
                                    parts.method,
                                    parts.uri,
                                    parts.headers,
                                    body.freeze(),
                                    remote_addr,
                                    "https",
                                    "HTTP/3",
                                )
                                .await
                            {
                                Ok(response) => response,
                                Err(error) => {
                                    gateway.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!(?error, %remote_addr, "http3 request dispatch failed");
                                    GatewayHttpResponse::error(StatusCode::BAD_GATEWAY, error.to_string())
                                }
                            };

                            let mut builder = Response::builder().status(response.status);
                            for (name, value) in &response.headers {
                                builder = builder.header(name, value);
                            }

                            let response_head = match builder.body(()) {
                                Ok(response) => response,
                                Err(error) => {
                                    tracing::warn!(?error, %remote_addr, "failed building h3 response head");
                                    continue;
                                }
                            };

                            if let Err(error) = stream.send_response(response_head).await {
                                tracing::warn!(?error, %remote_addr, "failed sending h3 response head");
                                continue;
                            }

                            if !response.body.is_empty() {
                                if let Err(error) = stream.send_data(response.body).await {
                                    tracing::warn!(?error, %remote_addr, "failed sending h3 response body");
                                    continue;
                                }
                            }

                            if let Err(error) = stream.finish().await {
                                tracing::warn!(?error, %remote_addr, "failed finishing h3 stream");
                            }
                        }
                        Ok(None) => break,
                        Err(error) => {
                            tracing::warn!(?error, %remote_addr, "h3 accept failed");
                            break;
                        }
                    }
                }
            });
        }
    }

    async fn run_tcp_listener(self: Arc<Self>, listener_config: TcpListenerConfig) -> Result<()> {
        let bind_addr: SocketAddr = listener_config
            .bind
            .parse()
            .with_context(|| format!("invalid tcp bind {}", listener_config.bind))?;
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind tcp listener {}", bind_addr))?;

        tracing::info!(listener = %listener_config.name, bind = %bind_addr, "tcp listener ready");

        loop {
            let (mut inbound, remote_addr) = listener.accept().await.context("tcp accept failed")?;
            let gateway = self.clone();
            let listener_name = listener_config.name.clone();
            self.stats.tcp_sessions_total.fetch_add(1, Ordering::Relaxed);
            self.stats.tcp_sessions_active.fetch_add(1, Ordering::Relaxed);

            tokio::spawn(async move {
                let request_id = Uuid::new_v4().to_string();
                if let Err(error) = async {
                    let state = gateway.current_state().await;
                    let stream_cfg = state.config.affinity.stream.clone();
                    let mut first_payload = BytesMut::new();

                    if stream_cfg.peek_bytes > 0 {
                        let mut buffer = vec![0_u8; stream_cfg.peek_bytes];
                        let read_result = tokio::time::timeout(
                            Duration::from_millis(stream_cfg.peek_timeout_ms.max(1)),
                            inbound.read(&mut buffer),
                        )
                        .await;

                        if let Ok(Ok(size)) = read_result {
                            if size > 0 {
                                first_payload.extend_from_slice(&buffer[..size]);
                            }
                        }
                    }

                    let player_id = extract_stream_player_id(&first_payload, &stream_cfg);
                    let preview = if first_payload.is_empty() {
                        None
                    } else {
                        Some(first_packet_preview(&first_payload))
                    };

                    let route = state
                        .script
                        .route_tcp(StreamContext {
                            request_id: request_id.clone(),
                            listener: listener_name.clone(),
                            protocol: "tcp".to_string(),
                            remote_addr: remote_addr.to_string(),
                            player_id: player_id.clone(),
                            first_packet_preview: preview,
                            payload_len: first_payload.len(),
                        })
                        .await
                        .map_err(|error| {
                            gateway.stats.script_fail_total.fetch_add(1, Ordering::Relaxed);
                            error
                        })?;

                    let upstream_plan = gateway.select_upstream_plan(
                        &state.config,
                        &route,
                        "tcp",
                        Some(&listener_name),
                        player_id.as_deref(),
                        Some(&remote_addr.to_string()),
                    );
                    let max_attempts = if state.config.load_balance.retries.enabled {
                        (state.config.load_balance.retries.max_retries as usize)
                            .saturating_add(1)
                            .min(upstream_plan.len().max(1))
                    } else {
                        1
                    };

                    let mut selected: Option<(TcpStream, UpstreamLease, String)> = None;
                    let mut last_error: Option<anyhow::Error> = None;

                    for upstream in upstream_plan.iter().take(max_attempts) {
                        let lease = gateway.acquire_upstream_lease("tcp", Some(&listener_name), upstream);
                        match TcpStream::connect(upstream).await {
                            Ok(stream) => {
                                gateway.on_upstream_success("tcp", Some(&listener_name), upstream);
                                selected = Some((stream, lease, upstream.clone()));
                                break;
                            }
                            Err(error) => {
                                gateway.on_upstream_failure(&state.config, "tcp", Some(&listener_name), upstream);
                                last_error = Some(anyhow!("failed to connect tcp upstream {upstream}: {error}"));
                            }
                        }
                    }

                    let (mut outbound, _lease, upstream) = selected
                        .ok_or_else(|| last_error.unwrap_or_else(|| anyhow!("failed to connect any tcp upstream")))?;

                    if !first_payload.is_empty() {
                        outbound
                            .write_all(&first_payload)
                            .await
                            .context("failed to forward peeked tcp payload")?;
                    }

                    copy_bidirectional(&mut inbound, &mut outbound)
                        .await
                        .context("tcp proxy copy failed")?;

                    gateway.on_upstream_success("tcp", Some(&listener_name), &upstream);

                    Ok::<_, anyhow::Error>(())
                }
                .await
                {
                    tracing::warn!(?error, request_id, listener = %listener_name, %remote_addr, "tcp session failed");
                }

                gateway.stats.tcp_sessions_active.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }

    async fn run_udp_listener(self: Arc<Self>, listener_config: UdpListenerConfig) -> Result<()> {
        let bind_addr: SocketAddr = listener_config
            .bind
            .parse()
            .with_context(|| format!("invalid udp bind {}", listener_config.bind))?;
        let listener_socket = Arc::new(
            UdpSocket::bind(bind_addr)
                .await
                .with_context(|| format!("failed to bind udp listener {}", bind_addr))?,
        );
        let associations = Arc::new(DashMap::<SocketAddr, Arc<UdpSocket>>::new());

        tracing::info!(listener = %listener_config.name, bind = %bind_addr, "udp listener ready");

        loop {
            let mut buffer = vec![0_u8; 65_536];
            let (received, client_addr) = listener_socket.recv_from(&mut buffer).await.context("udp recv failed")?;
            self.stats.udp_packets_total.fetch_add(1, Ordering::Relaxed);
            self.stats
                .udp_bytes_total
                .fetch_add(received as u64, Ordering::Relaxed);

            let payload = buffer[..received].to_vec();
            let gateway = self.clone();
            let listener_name = listener_config.name.clone();
            let listener_socket = listener_socket.clone();
            let associations = associations.clone();

            tokio::spawn(async move {
                let request_id = Uuid::new_v4().to_string();

                if let Err(error) = async {
                    let state = gateway.current_state().await;

                    let upstream_socket = if let Some(existing) = associations.get(&client_addr) {
                        existing.clone()
                    } else {
                        let player_id = extract_stream_player_id(&payload, &state.config.affinity.stream);
                        let route = state
                            .script
                            .route_udp(StreamContext {
                                request_id: request_id.clone(),
                                listener: listener_name.clone(),
                                protocol: "udp".to_string(),
                                remote_addr: client_addr.to_string(),
                                player_id: player_id.clone(),
                                first_packet_preview: Some(first_packet_preview(&payload)),
                                payload_len: payload.len(),
                            })
                            .await
                            .map_err(|error| {
                                gateway.stats.script_fail_total.fetch_add(1, Ordering::Relaxed);
                                error
                            })?;

                        let upstream_plan = gateway.select_upstream_plan(
                            &state.config,
                            &route,
                            "udp",
                            Some(&listener_name),
                            player_id.as_deref(),
                            Some(&client_addr.to_string()),
                        );
                        let max_attempts = if state.config.load_balance.retries.enabled {
                            (state.config.load_balance.retries.max_retries as usize)
                                .saturating_add(1)
                                .min(upstream_plan.len().max(1))
                        } else {
                            1
                        };

                        let mut selected: Option<(Arc<UdpSocket>, UpstreamLease)> = None;
                        let mut last_error: Option<anyhow::Error> = None;

                        for upstream in upstream_plan.iter().take(max_attempts) {
                            let upstream_addr: SocketAddr = match upstream.parse() {
                                Ok(value) => value,
                                Err(error) => {
                                    gateway.on_upstream_failure(&state.config, "udp", Some(&listener_name), upstream);
                                    last_error = Some(anyhow!("invalid udp upstream {upstream}: {error}"));
                                    continue;
                                }
                            };

                            let bind_any = if upstream_addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
                            let socket = match UdpSocket::bind(bind_any).await {
                                Ok(value) => Arc::new(value),
                                Err(error) => {
                                    gateway.on_upstream_failure(&state.config, "udp", Some(&listener_name), upstream);
                                    last_error = Some(anyhow!("failed to bind udp upstream socket: {error}"));
                                    continue;
                                }
                            };

                            match socket.connect(upstream_addr).await {
                                Ok(()) => {
                                    gateway.on_upstream_success("udp", Some(&listener_name), upstream);
                                    let lease = gateway.acquire_upstream_lease("udp", Some(&listener_name), upstream);
                                    selected = Some((socket, lease));
                                    break;
                                }
                                Err(error) => {
                                    gateway.on_upstream_failure(&state.config, "udp", Some(&listener_name), upstream);
                                    last_error = Some(anyhow!("failed to connect udp upstream {upstream}: {error}"));
                                }
                            }
                        }

                        let (socket, lease) = selected
                            .ok_or_else(|| last_error.unwrap_or_else(|| anyhow!("failed to connect any udp upstream")))?;

                        let read_socket = socket.clone();
                        let send_socket = listener_socket.clone();
                        associations.insert(client_addr, socket.clone());

                        tokio::spawn(async move {
                            let _lease = lease;
                            let mut response = vec![0_u8; 65_536];
                            loop {
                                match read_socket.recv(&mut response).await {
                                    Ok(size) => {
                                        if let Err(error) = send_socket.send_to(&response[..size], client_addr).await {
                                            tracing::warn!(?error, %client_addr, "failed relaying udp response to client");
                                            break;
                                        }
                                    }
                                    Err(error) => {
                                        tracing::warn!(?error, %client_addr, "udp upstream association closed");
                                        break;
                                    }
                                }
                            }
                        });

                        socket
                    };

                    upstream_socket
                        .send(&payload)
                        .await
                        .context("failed forwarding udp payload to upstream")?;

                    Ok::<_, anyhow::Error>(())
                }
                .await
                {
                    tracing::warn!(?error, request_id, listener = %listener_name, %client_addr, "udp session failed");
                }
            });
        }
    }

    async fn handle_hyper_request(
        self: Arc<Self>,
        mut request: Request<Incoming>,
        remote_addr: SocketAddr,
        scheme: &'static str,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        self.stats.http_requests.fetch_add(1, Ordering::Relaxed);

        let started = Instant::now();
        let version = version_label(request.version());
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = match request.body_mut().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(error) => {
                self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?error, %remote_addr, "failed collecting request body");
                return Ok(GatewayHttpResponse::error(StatusCode::BAD_REQUEST, "invalid request body").into_hyper());
            }
        };

        let response = match self
            .dispatch_http(method, uri, headers, body, remote_addr, scheme, version)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?error, %remote_addr, scheme, "http request dispatch failed");
                GatewayHttpResponse::error(StatusCode::BAD_GATEWAY, error.to_string())
            }
        };

        let elapsed = started.elapsed();
        if response.status.is_server_error() {
            self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
        }

        let state = self.current_state().await;
        if state.config.logging.access_log {
            let request_id = Uuid::new_v4().to_string();
            if should_sample(state.config.logging.access_sample_rate, &request_id) {
                let slow = elapsed.as_millis() as u64 >= state.config.logging.slow_request_ms;
                if slow {
                    tracing::warn!(
                        target: "access",
                        request_id,
                        method = %request.method(),
                        path = %request.uri().path(),
                        status = %response.status.as_u16(),
                        latency_ms = elapsed.as_millis() as u64,
                        upstream = %response.upstream,
                        remote_addr = %remote_addr,
                        "slow access"
                    );
                } else {
                    tracing::info!(
                        target: "access",
                        request_id,
                        method = %request.method(),
                        path = %request.uri().path(),
                        status = %response.status.as_u16(),
                        latency_ms = elapsed.as_millis() as u64,
                        upstream = %response.upstream,
                        remote_addr = %remote_addr,
                        "access"
                    );
                }
            }
        }

        Ok(response.into_hyper())
    }

    async fn dispatch_http(
        &self,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
    ) -> Result<GatewayHttpResponse> {
        let state = self.current_state().await;

        let host = headers
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
            .or_else(|| uri.authority().map(|authority| authority.as_str().to_string()))
            .unwrap_or_else(|| "localhost".to_string());

        let player_id = extract_http_player_id(&uri, &headers, &state.config.affinity.http);
        let request_id = Uuid::new_v4().to_string();

        let route = state
            .script
            .route_http(HttpContext {
                request_id: request_id.clone(),
                host: host.clone(),
                method: method.as_str().to_string(),
                path: uri.path().to_string(),
                query: uri.query().map(|value| value.to_string()),
                scheme: scheme.to_string(),
                version: version.to_string(),
                remote_addr: remote_addr.to_string(),
                player_id: player_id.clone(),
                headers: header_map_to_btree(&headers),
                body_len: body.len(),
            })
            .await
            .map_err(|error| {
                self.stats.script_fail_total.fetch_add(1, Ordering::Relaxed);
                error
            })?;

        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route,
            "http",
            None,
            route.affinity_key.as_deref().or(player_id.as_deref()),
            Some(&remote_addr.to_string()),
        );
        let max_attempts = if state.config.load_balance.retries.enabled {
            (state.config.load_balance.retries.max_retries as usize)
                .saturating_add(1)
                .min(upstream_plan.len().max(1))
        } else {
            1
        };

        let mut last_error: Option<anyhow::Error> = None;

        for (attempt, upstream) in upstream_plan.iter().take(max_attempts).enumerate() {
            let upstream_url = build_upstream_url(upstream, &route, &uri)?;
            let upstream_headers = build_upstream_headers(&headers, &route, &host, remote_addr, scheme)?;
            let _lease = self.acquire_upstream_lease("http", None, upstream);

            let send_result = state
                .http_client
                .request(method.clone(), upstream_url)
                .headers(upstream_headers)
                .body(body.clone())
                .send()
                .await;

            let upstream_response = match send_result {
                Ok(response) => response,
                Err(error) => {
                    self.on_upstream_failure(&state.config, "http", None, upstream);
                    last_error = Some(anyhow!("upstream request failed: {error}"));
                    continue;
                }
            };

            let status = upstream_response.status();
            let response_headers = upstream_response
                .headers()
                .iter()
                .filter(|(name, _)| !is_hop_header(name.as_str()))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect::<Vec<_>>();
            let response_body = match upstream_response.bytes().await {
                Ok(body_bytes) => body_bytes,
                Err(error) => {
                    self.on_upstream_failure(&state.config, "http", None, upstream);
                    last_error = Some(anyhow!("failed reading upstream response body: {error}"));
                    continue;
                }
            };

            if status.is_server_error() && attempt + 1 < max_attempts {
                self.on_upstream_failure(&state.config, "http", None, upstream);
                last_error = Some(anyhow!("upstream {upstream} returned server error {}", status.as_u16()));
                continue;
            }

            self.on_upstream_success("http", None, upstream);
            return Ok(GatewayHttpResponse {
                status,
                headers: response_headers,
                body: response_body,
                upstream: upstream.clone(),
            });
        }

        Err(last_error.unwrap_or_else(|| anyhow!("upstream request failed after retries")))
    }

    async fn current_state(&self) -> Arc<DynamicState> {
        self.dynamic.read().await.clone()
    }

    fn select_upstream(
        &self,
        config: &GatewayConfig,
        route: &RouteDecision,
        protocol: &str,
        listener: Option<&str>,
        affinity_key: Option<&str>,
        remote_addr: Option<&str>,
    ) -> String {
        self.select_upstream_plan(config, route, protocol, listener, affinity_key, remote_addr)
            .into_iter()
            .next()
            .unwrap_or_else(|| route.upstream.clone())
    }

    fn select_upstream_plan(
        &self,
        config: &GatewayConfig,
        route: &RouteDecision,
        protocol: &str,
        listener: Option<&str>,
        affinity_key: Option<&str>,
        remote_addr: Option<&str>,
    ) -> Vec<String> {
        let raw_candidates = normalize_candidates(route);
        if raw_candidates.is_empty() {
            return vec![route.upstream.clone()];
        }

        let candidates = self.filter_healthy_candidates(config, protocol, listener, raw_candidates);
        if candidates.len() == 1 {
            return candidates;
        }

        let scope_base = format!("{}:{}", protocol, listener.unwrap_or("default"));
        let selected_key = if config.affinity.enabled {
            affinity_key
                .map(|value| value.to_string())
                .or_else(|| {
                    if config.affinity.fallback_to_remote_addr {
                        remote_addr.map(|value| value.to_string())
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        match config.load_balance.algorithm {
            LoadBalanceAlgorithm::RoundRobin => self.select_round_robin_plan(&scope_base, candidates),
            LoadBalanceAlgorithm::LeastConnections => {
                self.select_least_connections_plan(protocol, listener, candidates)
            }
            LoadBalanceAlgorithm::SourceHash => self.select_source_hash_plan(selected_key.as_deref(), candidates),
            LoadBalanceAlgorithm::Rendezvous => {
                self.select_rendezvous_plan(config, &scope_base, selected_key.as_deref(), candidates)
            }
        }
    }

    fn select_rendezvous_plan(
        &self,
        config: &GatewayConfig,
        scope_base: &str,
        selected_key: Option<&str>,
        candidates: Vec<String>,
    ) -> Vec<String> {
        let key = selected_key.unwrap_or(scope_base);
        let mut ranked = rendezvous_rank(key, &candidates);
        if ranked.is_empty() {
            return candidates;
        }

        let Some(affinity_key) = selected_key else {
            return ranked;
        };

        let scope = format!("{}:{}", scope_base, affinity_key);
        let now = Instant::now();

        if let Some(entry) = self.sticky_affinity.get(&scope) {
            if entry.expires_at > now && ranked.iter().any(|item| item == &entry.upstream) {
                if let Some(position) = ranked.iter().position(|item| item == &entry.upstream) {
                    ranked.swap(0, position);
                }
                return ranked;
            }
        }

        let selected = ranked[0].clone();
        self.sticky_affinity.insert(
            scope,
            StickyEntry {
                upstream: selected,
                expires_at: now + Duration::from_secs(config.affinity.sticky_ttl_secs),
            },
        );

        ranked
    }

    fn select_source_hash_plan(&self, selected_key: Option<&str>, candidates: Vec<String>) -> Vec<String> {
        let Some(key) = selected_key else {
            return candidates;
        };
        if candidates.len() <= 1 {
            return candidates;
        }

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let start = (hasher.finish() as usize) % candidates.len();

        let mut ordered = Vec::with_capacity(candidates.len());
        for index in 0..candidates.len() {
            ordered.push(candidates[(start + index) % candidates.len()].clone());
        }
        ordered
    }

    fn select_round_robin_plan(&self, scope_base: &str, candidates: Vec<String>) -> Vec<String> {
        if candidates.len() <= 1 {
            return candidates;
        }

        let start = {
            let mut entry = self.round_robin_state.entry(scope_base.to_string()).or_insert(0);
            let start = (*entry as usize) % candidates.len();
            *entry = entry.saturating_add(1);
            start
        };

        let mut ordered = Vec::with_capacity(candidates.len());
        for index in 0..candidates.len() {
            ordered.push(candidates[(start + index) % candidates.len()].clone());
        }
        ordered
    }

    fn select_least_connections_plan(
        &self,
        protocol: &str,
        listener: Option<&str>,
        mut candidates: Vec<String>,
    ) -> Vec<String> {
        candidates.sort_by(|left, right| {
            let left_connections = self.upstream_active_connections(protocol, listener, left);
            let right_connections = self.upstream_active_connections(protocol, listener, right);
            left_connections
                .cmp(&right_connections)
                .then_with(|| left.cmp(right))
        });
        candidates
    }

    fn filter_healthy_candidates(
        &self,
        config: &GatewayConfig,
        protocol: &str,
        listener: Option<&str>,
        candidates: Vec<String>,
    ) -> Vec<String> {
        if !config.load_balance.passive_health.enabled {
            return candidates;
        }

        let now = Instant::now();
        let mut available = Vec::new();

        for candidate in &candidates {
            let key = runtime_scope_key(protocol, listener, candidate);
            let healthy = match self.upstream_runtime.get(&key) {
                Some(state) => match state.quarantined_until {
                    Some(until) => until <= now,
                    None => true,
                },
                None => true,
            };

            if healthy {
                available.push(candidate.clone());
            }
        }

        if available.is_empty() {
            candidates
        } else {
            available
        }
    }

    fn on_upstream_success(&self, protocol: &str, listener: Option<&str>, upstream: &str) {
        let key = runtime_scope_key(protocol, listener, upstream);
        let mut entry = self.upstream_runtime.entry(key).or_default();
        entry.consecutive_failures = 0;
        entry.quarantined_until = None;
    }

    fn on_upstream_failure(
        &self,
        config: &GatewayConfig,
        protocol: &str,
        listener: Option<&str>,
        upstream: &str,
    ) {
        if !config.load_balance.passive_health.enabled {
            return;
        }

        let key = runtime_scope_key(protocol, listener, upstream);
        let mut entry = self.upstream_runtime.entry(key).or_default();
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        if entry.consecutive_failures >= config.load_balance.passive_health.fail_threshold {
            entry.quarantined_until = Some(
                Instant::now() + Duration::from_secs(config.load_balance.passive_health.quarantine_secs),
            );
        }
    }

    fn acquire_upstream_lease(
        &self,
        protocol: &str,
        listener: Option<&str>,
        upstream: &str,
    ) -> UpstreamLease {
        let key = runtime_scope_key(protocol, listener, upstream);
        UpstreamLease::acquire(self.upstream_runtime.clone(), key)
    }

    fn upstream_active_connections(&self, protocol: &str, listener: Option<&str>, upstream: &str) -> u64 {
        let key = runtime_scope_key(protocol, listener, upstream);
        self.upstream_runtime
            .get(&key)
            .map(|entry| entry.active_connections)
            .unwrap_or(0)
    }

    fn upstream_runtime_snapshot(&self) -> Vec<serde_json::Value> {
        let now = Instant::now();
        let mut result = self
            .upstream_runtime
            .iter()
            .map(|entry| {
                let value = entry.value();
                let remaining = value
                    .quarantined_until
                    .map(|until| until.saturating_duration_since(now).as_secs())
                    .unwrap_or(0);
                serde_json::json!({
                    "key": entry.key(),
                    "consecutive_failures": value.consecutive_failures,
                    "active_connections": value.active_connections,
                    "quarantine_remaining_secs": remaining,
                    "healthy": value.quarantined_until.map(|until| until <= now).unwrap_or(true),
                })
            })
            .collect::<Vec<_>>();

        result.sort_by(|left, right| {
            left.get("key")
                .and_then(|value| value.as_str())
                .cmp(&right.get("key").and_then(|value| value.as_str()))
        });
        result
    }

    fn build_rustls_server_config(&self, alpn_protocols: Vec<Vec<u8>>) -> Result<rustls::ServerConfig> {
        let certs = load_certs(&self.bootstrap_config.http.tls.cert_path)?;
        let key = load_private_key(&self.bootstrap_config.http.tls.key_path)?;

        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("failed building rustls server config")?;
        server_config.alpn_protocols = alpn_protocols;
        Ok(server_config)
    }
}

impl GatewayHttpResponse {
    fn error(status: StatusCode, message: impl Into<String>) -> Self {
        let body = Bytes::from(message.into());
        Self {
            status,
            headers: vec![(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            body,
            upstream: "-".to_string(),
        }
    }

    fn into_hyper(self) -> Response<Full<Bytes>> {
        let mut builder = Response::builder().status(self.status);
        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }
        builder
            .body(Full::new(self.body))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from_static(b"response build failure")))
                    .expect("static response build should never fail")
            })
    }
}

impl GatewayStats {
    fn snapshot_json(&self) -> serde_json::Value {
        serde_json::json!({
            "http_requests": self.http_requests.load(Ordering::Relaxed),
            "http_errors": self.http_errors.load(Ordering::Relaxed),
            "tcp_sessions_total": self.tcp_sessions_total.load(Ordering::Relaxed),
            "tcp_sessions_active": self.tcp_sessions_active.load(Ordering::Relaxed),
            "udp_packets_total": self.udp_packets_total.load(Ordering::Relaxed),
            "udp_bytes_total": self.udp_bytes_total.load(Ordering::Relaxed),
            "reload_success_total": self.reload_success_total.load(Ordering::Relaxed),
            "reload_failure_total": self.reload_failure_total.load(Ordering::Relaxed),
            "admin_requests_total": self.admin_requests_total.load(Ordering::Relaxed),
            "admin_auth_fail_total": self.admin_auth_fail_total.load(Ordering::Relaxed),
            "script_fail_total": self.script_fail_total.load(Ordering::Relaxed),
        })
    }
}

async fn build_dynamic_state(config: GatewayConfig) -> Result<DynamicState> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(config.http.allow_insecure_upstreams)
        .http2_adaptive_window(true)
        .http2_keep_alive_interval(Some(Duration::from_secs(15)))
        .http2_keep_alive_while_idle(true)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .pool_max_idle_per_host(1024)
        .timeout(Duration::from_millis(config.http.request_timeout_ms.max(1)))
        .build()
        .context("failed to build upstream http client")?;

    let script = Arc::new(ScriptRuntime::spawn(&config.script)?);

    auto_load_plugins(&config, &script).await?;

    Ok(DynamicState {
        config,
        http_client: client,
        script,
    })
}

async fn auto_load_plugins(config: &GatewayConfig, script: &Arc<ScriptRuntime>) -> Result<()> {
    if !config.plugins.enabled {
        return Ok(());
    }

    let dir = &config.plugins.auto_load_dir;
    if !dir.exists() {
        return Ok(());
    }

    let extension_set: HashSet<String> = config
        .plugins
        .extensions
        .iter()
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();

    let mut candidates = Vec::<PathBuf>::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to scan plugin directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read plugin entry under {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();

        if extension_set.contains(&ext) {
            candidates.push(path);
        }
    }

    candidates.sort();

    for path in candidates {
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "plugin".to_string());

        script
            .load_plugin(ScriptPluginSpec {
                name: name.clone(),
                module_path: path.to_string_lossy().to_string(),
                priority: 0,
                enabled: true,
                config: serde_json::Value::Null,
            })
            .await
            .with_context(|| format!("failed to auto-load plugin {}", path.display()))?;

        tracing::info!(plugin = %name, path = %path.display(), "plugin auto-loaded");
    }

    Ok(())
}

fn ensure_reload_compatible(old: &GatewayConfig, new: &GatewayConfig) -> Result<()> {
    if old.logging.format != new.logging.format || old.logging.filter != new.logging.filter {
        return Err(anyhow!("logging.format/logging.filter changes require restart"));
    }

    if old.http.plain_bind != new.http.plain_bind
        || old.http.tls_bind != new.http.tls_bind
        || old.http.h3_bind != new.http.h3_bind
    {
        return Err(anyhow!(
            "listener bind changes require restart (http.plain_bind/http.tls_bind/http.h3_bind)"
        ));
    }

    if old.admin.enabled != new.admin.enabled {
        return Err(anyhow!("admin.enabled change requires restart"));
    }

    if old.admin.enabled && old.admin.bind != new.admin.bind {
        return Err(anyhow!("admin.bind change requires restart"));
    }

    if listener_signature_tcp(&old.tcp.listeners) != listener_signature_tcp(&new.tcp.listeners) {
        return Err(anyhow!("tcp listener set changed; restart required"));
    }

    if listener_signature_udp(&old.udp.listeners) != listener_signature_udp(&new.udp.listeners) {
        return Err(anyhow!("udp listener set changed; restart required"));
    }

    if old.http.tls.mode != new.http.tls.mode {
        return Err(anyhow!("tls mode changes require restart"));
    }

    Ok(())
}

fn listener_signature_tcp(listeners: &[TcpListenerConfig]) -> HashSet<(String, String)> {
    listeners
        .iter()
        .map(|listener| (listener.name.clone(), listener.bind.clone()))
        .collect()
}

fn listener_signature_udp(listeners: &[UdpListenerConfig]) -> HashSet<(String, String)> {
    listeners
        .iter()
        .map(|listener| (listener.name.clone(), listener.bind.clone()))
        .collect()
}

fn prepare_tls_material(config: &GatewayConfig) -> Result<()> {
    match config.http.tls.mode {
        TlsMode::SelfSigned => {
            if config.http.tls.generate_self_signed_if_missing {
                install::ensure_cert_pair(
                    &config.http.tls.cert_path,
                    &config.http.tls.key_path,
                    &config.http.tls.server_name,
                    false,
                )?;
            }

            if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                return Err(anyhow!(
                    "tls.mode=self_signed but cert/key files are missing: {} {}",
                    config.http.tls.cert_path.display(),
                    config.http.tls.key_path.display()
                ));
            }
        }
        TlsMode::Manual => {
            if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                return Err(anyhow!(
                    "tls.mode=manual requires existing cert/key: {} {}",
                    config.http.tls.cert_path.display(),
                    config.http.tls.key_path.display()
                ));
            }
        }
        TlsMode::AcmeExternal => {
            if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                run_acme_command(&config.http.tls, false)?;
            }
            if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                return Err(anyhow!(
                    "acme command did not produce cert/key files: {} {}",
                    config.http.tls.cert_path.display(),
                    config.http.tls.key_path.display()
                ));
            }
        }
    }

    Ok(())
}

fn run_acme_command(tls: &crate::config::TlsConfig, renew_only: bool) -> Result<()> {
    let acme = &tls.acme;
    let primary = acme
        .domains
        .first()
        .ok_or_else(|| anyhow!("http.tls.acme.domains cannot be empty"))?
        .clone();

    if let Some(parent) = tls.cert_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cert directory {}", parent.display()))?;
    }
    if let Some(parent) = tls.key_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }

    let mut issue = Command::new(&acme.client);
    if renew_only {
        issue.arg("--renew");
    } else {
        issue.arg("--issue");
        issue.arg("--standalone");
    }

    for domain in &acme.domains {
        issue.arg("-d").arg(domain);
    }

    if !acme.email.trim().is_empty() {
        issue.arg("--accountemail").arg(&acme.email);
    }

    if acme.directory_production {
        issue.arg("--server").arg("letsencrypt");
    } else {
        issue.arg("--server").arg("letsencrypt_test");
    }

    match acme.challenge {
        AcmeChallengeType::TlsAlpn01 => {
            issue.arg("--alpn");
        }
        AcmeChallengeType::Http01 => {
            issue.arg("--standalone");
        }
    }

    for arg in &acme.extra_args {
        issue.arg(arg);
    }

    run_command_checked(issue, "issue/renew acme certificate")?;

    let mut install_cmd = Command::new(&acme.client);
    install_cmd
        .arg("--install-cert")
        .arg("-d")
        .arg(&primary)
        .arg("--key-file")
        .arg(tls.key_path.to_string_lossy().to_string())
        .arg("--fullchain-file")
        .arg(tls.cert_path.to_string_lossy().to_string())
        .arg("--reloadcmd")
        .arg("true");

    run_command_checked(install_cmd, "install acme certificate")
}

fn run_command_checked(mut command: Command, description: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to {description}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow!("{description} failed: {stderr}"))
}

fn build_upstream_url(base_upstream: &str, route: &RouteDecision, uri: &Uri) -> Result<Url> {
    let upstream = if base_upstream.starts_with("http://") || base_upstream.starts_with("https://") {
        base_upstream.to_string()
    } else {
        format!("http://{}", base_upstream)
    };
    let mut url = Url::parse(&upstream).with_context(|| format!("invalid upstream url {}", base_upstream))?;

    let rewritten = route
        .rewrite_path
        .clone()
        .unwrap_or_else(|| {
            uri.path_and_query()
                .map(|value| value.as_str().to_string())
                .unwrap_or_else(|| uri.path().to_string())
        });

    let (path, query) = match rewritten.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (rewritten, uri.query().map(|value| value.to_string())),
    };

    url.set_path(&path);
    url.set_query(query.as_deref());
    Ok(url)
}

fn build_upstream_headers(
    original: &HeaderMap,
    route: &RouteDecision,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    for (name, value) in original {
        if is_hop_header(name.as_str()) || name == HOST {
            continue;
        }
        headers.append(name.clone(), value.clone());
    }

    for header_name in &route.strip_headers {
        if let Ok(name) = HeaderName::from_bytes(header_name.as_bytes()) {
            headers.remove(name);
        }
    }

    for (name, value) in &route.set_headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid request header name {name}"))?;
        let value = HeaderValue::from_str(value)
            .with_context(|| format!("invalid request header value for {name}"))?;
        headers.insert(name, value);
    }

    headers.insert(
        HeaderName::from_static("x-forwarded-for"),
        HeaderValue::from_str(&remote_addr.ip().to_string()).context("invalid x-forwarded-for header")?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-host"),
        HeaderValue::from_str(host).context("invalid x-forwarded-host header")?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_str(scheme).context("invalid x-forwarded-proto header")?,
    );

    Ok(headers)
}

fn header_map_to_btree(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| value.to_str().ok().map(|value| (name.as_str().to_string(), value.to_string())))
        .collect()
}

fn normalize_candidates(route: &RouteDecision) -> Vec<String> {
    if route.upstreams.is_empty() {
        return vec![route.upstream.clone()];
    }

    let mut candidates = Vec::new();
    for item in &route.upstreams {
        if !item.trim().is_empty() {
            candidates.push(item.clone());
        }
    }
    if candidates.is_empty() {
        candidates.push(route.upstream.clone());
    }
    candidates
}

fn runtime_scope_key(protocol: &str, listener: Option<&str>, upstream: &str) -> String {
    format!("{}:{}:{}", protocol, listener.unwrap_or("default"), upstream)
}

fn rendezvous_rank(key: &str, candidates: &[String]) -> Vec<String> {
    let mut scored = candidates
        .iter()
        .map(|candidate| {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            candidate.hash(&mut hasher);
            (hasher.finish(), candidate.clone())
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

fn extract_http_player_id(uri: &Uri, headers: &HeaderMap, cfg: &HttpAffinityConfig) -> Option<String> {
    if let Some(query) = uri.query() {
        let target_keys: HashSet<String> = cfg.query_keys.iter().map(|key| key.to_ascii_lowercase()).collect();
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if target_keys.contains(&key.to_ascii_lowercase()) {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    for header_key in &cfg.header_keys {
        if let Some(value) = headers.get(header_key) {
            if let Ok(value) = value.to_str() {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    if let Some(cookie) = headers.get(COOKIE).and_then(|value| value.to_str().ok()) {
        let target_keys: HashSet<String> = cfg.cookie_keys.iter().map(|key| key.to_ascii_lowercase()).collect();
        for chunk in cookie.split(';') {
            let trimmed = chunk.trim();
            if let Some((name, value)) = trimmed.split_once('=') {
                if target_keys.contains(&name.trim().to_ascii_lowercase()) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_stream_player_id(payload: &[u8], cfg: &StreamAffinityConfig) -> Option<String> {
    if payload.is_empty() {
        return None;
    }

    let text = String::from_utf8_lossy(payload);
    let delimiters: HashSet<char> = cfg
        .probe_delimiters
        .iter()
        .filter_map(|value| value.chars().next())
        .collect();

    for prefix in &cfg.probe_prefixes {
        if let Some(position) = text.find(prefix) {
            let remainder = &text[position + prefix.len()..];
            let mut end = remainder.len();
            for (index, ch) in remainder.char_indices() {
                if delimiters.contains(&ch) {
                    end = index;
                    break;
                }
            }

            let candidate = remainder[..end].trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

fn first_packet_preview(payload: &[u8]) -> String {
    let limit = payload.len().min(96);
    String::from_utf8_lossy(&payload[..limit]).replace('\n', "\\n").replace('\r', "\\r")
}

fn should_sample(sample_rate: f64, seed: &str) -> bool {
    if sample_rate >= 1.0 {
        return true;
    }
    if sample_rate <= 0.0 {
        return false;
    }

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let score = hasher.finish();
    let threshold = (sample_rate * (u64::MAX as f64)) as u64;
    score <= threshold
}

fn is_authorized(header: Option<&HeaderValue>, admin: &AdminConfig) -> bool {
    let Some(header) = header else {
        return false;
    };

    let Ok(value) = header.to_str() else {
        return false;
    };

    if !value.starts_with("Basic ") {
        return false;
    }

    let encoded = &value[6..];
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };

    let Ok(decoded) = String::from_utf8(decoded) else {
        return false;
    };

    let Some((username, password)) = decoded.split_once(':') else {
        return false;
    };

    username == admin.username && password == admin.password
}

#[derive(Debug, Deserialize)]
struct PluginUnloadRequest {
    name: String,
}

fn sanitize_config(config: &GatewayConfig) -> serde_json::Value {
    let mut value = serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({}));

    if let Some(admin) = value.get_mut("admin").and_then(|item| item.as_object_mut()) {
        admin.insert("password".to_string(), serde_json::Value::String("***".to_string()));
    }

    value
}

fn json_response(status: StatusCode, payload: serde_json::Value) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(payload.to_string())))
        .unwrap_or_else(|_| text_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to build json response"))
}

fn text_response(status: StatusCode, body: impl Into<String>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(body.into())))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from_static(b"failed to build text response")))
                .expect("static response build should never fail")
        })
}

fn is_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-connection"
    )
}

fn version_label(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/unknown",
    }
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = std::fs::File::open(path).with_context(|| format!("failed to open certificate {}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse certificate pem")
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = std::fs::File::open(path).with_context(|| format!("failed to open private key {}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .context("failed to parse private key pem")?
        .ok_or_else(|| anyhow!("no private key found in {}", path.display()))
}

fn read_file_hash(path: &Path) -> Result<u64> {
    let bytes = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AdminConfig, StreamAffinityConfig};

    #[test]
    fn runtime_scope_key_contains_protocol_listener_and_upstream() {
        let key = runtime_scope_key("tcp", Some("game-login"), "127.0.0.1:7001");
        assert_eq!(key, "tcp:game-login:127.0.0.1:7001");
    }

    #[test]
    fn rendezvous_rank_is_deterministic_and_complete() {
        let candidates = vec![
            "127.0.0.1:7001".to_string(),
            "127.0.0.1:7002".to_string(),
            "127.0.0.1:7003".to_string(),
        ];

        let ranked_a = rendezvous_rank("player-100", &candidates);
        let ranked_b = rendezvous_rank("player-100", &candidates);

        assert_eq!(ranked_a, ranked_b);
        assert_eq!(ranked_a.len(), candidates.len());
        for item in &candidates {
            assert!(ranked_a.contains(item));
        }
    }

    #[test]
    fn extract_stream_player_id_uses_prefix_and_delimiters() {
        let cfg = StreamAffinityConfig::default();
        let payload = b"hello|playerId=abc123;region=cn";
        let player_id = extract_stream_player_id(payload, &cfg);
        assert_eq!(player_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn is_authorized_accepts_basic_auth() {
        let admin = AdminConfig {
            username: "root".to_string(),
            password: "root".to_string(),
            ..AdminConfig::default()
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode("root:root");
        let header = HeaderValue::from_str(&format!("Basic {encoded}")).expect("valid header");

        assert!(is_authorized(Some(&header), &admin));
    }

    #[test]
    fn normalize_candidates_preserves_non_empty_items() {
        let route = RouteDecision {
            upstream: "127.0.0.1:7001".to_string(),
            upstreams: vec![
                "127.0.0.1:7001".to_string(),
                "".to_string(),
                "127.0.0.1:7002".to_string(),
            ],
            affinity_key: None,
            rewrite_path: None,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
        };

        let candidates = normalize_candidates(&route);
        assert_eq!(candidates, vec!["127.0.0.1:7001".to_string(), "127.0.0.1:7002".to_string()]);
    }
}
