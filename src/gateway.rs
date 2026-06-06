use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashSet};
use std::convert::Infallible;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use bytes::{Buf, Bytes, BytesMut};
use dashmap::DashMap;
use flate2::write::GzEncoder;
use flate2::Compression;
use h3::server::Connection as H3Connection;
use http::header::{
    ACCEPT_ENCODING, AUTHORIZATION, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE,
    COOKIE, HOST, LOCATION, SET_COOKIE, VARY,
};
use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri, Version,
};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use quinn::crypto::rustls::QuicServerConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::UnixTime;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ClientConfig;
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use serde::Deserialize;
use tokio::io::{copy_bidirectional, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use url::Url;
use uuid::Uuid;

use crate::config::{
    AcmeChallengeType, AdminConfig, DomainRouteConfig, GatewayConfig, HttpAffinityConfig,
    HttpRateLimitConfig, LoadBalanceAlgorithm, RateLimitKey, ResponseCacheConfig,
    ResponseCompressionConfig, ReverseProxyRouteConfig, StaticSiteConfig, StreamAffinityConfig,
    TcpListenerConfig, TlsCertificateConfig, TlsMode, UdpListenerConfig, WebDavConfig,
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
    http_rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    http_cache: Arc<DashMap<String, CachedHttpEntry>>,
}

struct DynamicState {
    config: GatewayConfig,
    http_client: reqwest::Client,
    script: Option<Arc<ScriptRuntime>>,
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

#[derive(Clone, Default)]
struct UpstreamRuntimeState {
    consecutive_failures: u32,
    quarantined_until: Option<Instant>,
    active_connections: u64,
}

#[derive(Clone)]
struct RateLimitBucket {
    window_start: Instant,
    count: u32,
}

#[derive(Clone)]
struct CachedHttpEntry {
    expires_at: Instant,
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
    upstream: String,
}

#[derive(Clone)]
struct HttpRouteConfig {
    decision: RouteDecision,
    compression: ResponseCompressionConfig,
    cache: ResponseCacheConfig,
}

#[derive(Debug)]
struct SniResolver {
    default: Arc<CertifiedKey>,
    by_name: BTreeMap<String, Arc<CertifiedKey>>,
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

struct UpstreamLease {
    runtime: Arc<DashMap<String, UpstreamRuntimeState>>,
    key: String,
}

#[derive(Debug, Default, Deserialize)]
struct AutoLoadPluginMetadata {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    config: serde_json::Value,
}

#[derive(Clone, Debug)]
enum ListenerSpec {
    PlainHttp {
        bind: String,
    },
    TlsHttp {
        bind: String,
        tls_fingerprint: String,
    },
    Http3 {
        bind: String,
        tls_fingerprint: String,
    },
    Tcp(TcpListenerConfig),
    Udp(UdpListenerConfig),
    Admin {
        bind: String,
    },
}

trait ProxyIo: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> ProxyIo for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

type BoxedProxyIo = Box<dyn ProxyIo>;

#[derive(Debug)]
struct InsecureUpstreamVerifier;

impl ServerCertVerifier for InsecureUpstreamVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let server_name = client_hello.server_name()?.to_ascii_lowercase();
        if let Some(certified) = self.by_name.get(&server_name) {
            return Some(certified.clone());
        }

        let labels = server_name.split('.').collect::<Vec<_>>();
        for index in 1..labels.len() {
            let suffix = format!(".{}", labels[index..].join("."));
            if let Some(certified) = self.by_name.get(&suffix) {
                return Some(certified.clone());
            }
        }

        Some(self.default.clone())
    }
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
            http_rate_limits: Arc::new(DashMap::new()),
            http_cache: Arc::new(DashMap::new()),
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

        let gateway = self.clone();
        tasks.spawn(async move { gateway.run_listener_supervisor().await });

        while let Some(result) = tasks.join_next().await {
            result??;
        }

        Ok(())
    }

    async fn run_hot_reload_loop(self: Arc<Self>) -> Result<()> {
        let mut last_hash = reload_fingerprint(&self.config_path).unwrap_or_default();

        loop {
            let interval_ms = {
                let state = self.current_state().await;
                state.config.runtime.hot_reload.interval_ms.max(200)
            };
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            self.prune_sticky_affinity();

            let hash = match reload_fingerprint(&self.config_path) {
                Ok(value) => value,
                Err(error) => {
                    self.stats
                        .reload_failure_total
                        .fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(?error, path = %self.config_path.display(), "hot reload failed to read reload fingerprint");
                    continue;
                }
            };

            if hash == last_hash {
                continue;
            }

            match self.reload_from_disk().await {
                Ok(()) => {
                    last_hash = hash;
                    self.stats
                        .reload_success_total
                        .fetch_add(1, Ordering::Relaxed);
                }
                Err(error) => {
                    self.stats
                        .reload_failure_total
                        .fetch_add(1, Ordering::Relaxed);
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
            let renew_result =
                tokio::task::spawn_blocking(move || run_acme_command(&tls, true)).await;

            match renew_result {
                Ok(Ok(())) => tracing::info!("acme renewal succeeded"),
                Ok(Err(error)) => tracing::warn!(?error, "acme renewal failed"),
                Err(error) => tracing::warn!(?error, "acme renewal task join failed"),
            }
        }
    }

    async fn run_listener_supervisor(self: Arc<Self>) -> Result<()> {
        let mut active = BTreeMap::<String, JoinHandle<Result<()>>>::new();

        loop {
            let state = self.current_state().await;
            let desired = listener_specs(&state.config);
            let desired_keys = desired
                .iter()
                .map(ListenerSpec::key)
                .collect::<BTreeSet<_>>();

            let stale = active
                .keys()
                .filter(|key| !desired_keys.contains(*key))
                .cloned()
                .collect::<Vec<_>>();
            for key in stale {
                if let Some(handle) = active.remove(&key) {
                    handle.abort();
                    tracing::info!(listener = %key, "listener stopped after hot reload");
                }
            }

            for spec in desired {
                let key = spec.key();
                if active.contains_key(&key) {
                    continue;
                }

                let gateway = self.clone();
                let spawn_key = key.clone();
                let handle = tokio::spawn(async move { gateway.run_listener_spec(spec).await });
                active.insert(key, handle);
                tracing::info!(listener = %spawn_key, "listener started");
            }

            let finished = active
                .iter()
                .filter(|(_, handle)| handle.is_finished())
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            for key in finished {
                let Some(handle) = active.remove(&key) else {
                    continue;
                };
                match handle.await {
                    Ok(Ok(())) => tracing::info!(listener = %key, "listener finished"),
                    Ok(Err(error)) => {
                        tracing::warn!(?error, listener = %key, "listener failed; supervisor will retry")
                    }
                    Err(error) if error.is_cancelled() => {}
                    Err(error) => {
                        tracing::warn!(?error, listener = %key, "listener task join failed")
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn run_listener_spec(self: Arc<Self>, spec: ListenerSpec) -> Result<()> {
        match spec {
            ListenerSpec::PlainHttp { bind } => self.run_plain_http(bind).await,
            ListenerSpec::TlsHttp { bind, .. } => self.run_tls_http(bind).await,
            ListenerSpec::Http3 { bind, .. } => self.run_http3(bind).await,
            ListenerSpec::Tcp(listener) => self.run_tcp_listener(listener).await,
            ListenerSpec::Udp(listener) => self.run_udp_listener(listener).await,
            ListenerSpec::Admin { bind } => self.run_admin_server(bind).await,
        }
    }

    async fn run_admin_server(self: Arc<Self>, bind: String) -> Result<()> {
        let bind_addr: SocketAddr = bind.parse().context("invalid admin.bind address")?;
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
        self.stats
            .admin_requests_total
            .fetch_add(1, Ordering::Relaxed);

        let method = request.method().clone();
        let path = request.uri().path().to_string();

        if path == "/healthz" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "service": "proxysss", "remote_addr": remote_addr.to_string()}),
            ));
        }

        let state = self.current_state().await;
        if method == Method::GET && (path == "/" || path == "/index.html") {
            return Ok(html_response(
                StatusCode::OK,
                render_admin_console_html(&state.config),
            ));
        }

        if !is_authorized(request.headers().get(AUTHORIZATION), &state.config.admin) {
            self.stats
                .admin_auth_fail_total
                .fetch_add(1, Ordering::Relaxed);
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
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "config endpoint disabled",
                ));
            }
            return Ok(json_response(
                StatusCode::OK,
                sanitize_config(&state.config),
            ));
        }

        if method == Method::GET && path == "/v1/plugins" {
            if !state.config.plugins.enabled {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugins are disabled by config",
                ));
            }
            if !state.config.plugins.allow_admin_manage {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugin management disabled",
                ));
            }

            let Some(script) = &state.script else {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": "TypeScript runtime is disabled"}),
                ));
            };

            match script.list_plugins().await {
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
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugins are disabled by config",
                ));
            }
            if !state.config.admin.enable_write_ops || !state.config.plugins.allow_admin_manage {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugin write operations disabled",
                ));
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

            let Some(script) = &state.script else {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": "TypeScript runtime is disabled"}),
                ));
            };

            match script.load_plugin(spec).await {
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
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugins are disabled by config",
                ));
            }
            if !state.config.admin.enable_write_ops || !state.config.plugins.allow_admin_manage {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "plugin write operations disabled",
                ));
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

            let Some(script) = &state.script else {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": "TypeScript runtime is disabled"}),
                ));
            };

            match script.unload_plugin(&unload.name).await {
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
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }

            match self.reload_from_disk().await {
                Ok(()) => {
                    self.stats
                        .reload_success_total
                        .fetch_add(1, Ordering::Relaxed);
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "message": "reloaded"}),
                    ));
                }
                Err(error) => {
                    self.stats
                        .reload_failure_total
                        .fetch_add(1, Ordering::Relaxed);
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

    async fn run_plain_http(self: Arc<Self>, bind: String) -> Result<()> {
        let bind_addr: SocketAddr = bind.parse().context("invalid http.plain_bind address")?;
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind plain http listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "plain http listener ready");

        loop {
            let (stream, remote_addr) = listener
                .accept()
                .await
                .context("plain http accept failed")?;
            let gateway = self.clone();

            tokio::spawn(async move {
                let service = service_fn(move |request| {
                    let gateway = gateway.clone();
                    async move {
                        gateway
                            .handle_hyper_request(request, remote_addr, "http")
                            .await
                    }
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

    async fn run_tls_http(self: Arc<Self>, bind: String) -> Result<()> {
        let bind_addr: SocketAddr = bind.parse().context("invalid http.tls_bind address")?;
        let state = self.current_state().await;
        let tls_acceptor = TlsAcceptor::from(Arc::new(build_rustls_server_config(
            &state.config,
            vec![b"h2".to_vec(), b"http/1.1".to_vec()],
        )?));
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("failed to bind tls http listener {}", bind_addr))?;

        tracing::info!(bind = %bind_addr, "tls http listener ready");

        loop {
            let (stream, remote_addr) =
                listener.accept().await.context("tls http accept failed")?;
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
                    async move {
                        gateway
                            .handle_hyper_request(request, remote_addr, "https")
                            .await
                    }
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

    async fn run_http3(self: Arc<Self>, bind: String) -> Result<()> {
        let bind_addr: SocketAddr = bind.parse().context("invalid http.h3_bind address")?;

        let state = self.current_state().await;
        let mut server_config =
            quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(
                build_rustls_server_config(&state.config, vec![b"h3".to_vec()])?,
            )?));
        let transport = Arc::get_mut(&mut server_config.transport)
            .context("failed to configure quic transport")?;
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
                            let (request, mut stream) = match request_resolver
                                .resolve_request()
                                .await
                            {
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
                                    None,
                                )
                                .await
                            {
                                Ok(response) => response,
                                Err(error) => {
                                    gateway.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!(?error, %remote_addr, "http3 request dispatch failed");
                                    GatewayHttpResponse::error(
                                        StatusCode::BAD_GATEWAY,
                                        error.to_string(),
                                    )
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
            let (mut inbound, remote_addr) =
                listener.accept().await.context("tcp accept failed")?;
            let gateway = self.clone();
            let listener_name = listener_config.name.clone();
            self.stats
                .tcp_sessions_total
                .fetch_add(1, Ordering::Relaxed);
            self.stats
                .tcp_sessions_active
                .fetch_add(1, Ordering::Relaxed);

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

                    let route = if listener_name == "ftp" && state.config.services.ftp.enabled {
                        RouteDecision {
                            upstream: state.config.services.ftp.upstream.clone(),
                            upstreams: Vec::new(),
                            affinity_key: player_id.clone(),
                            rewrite_path: None,
                            set_headers: BTreeMap::new(),
                            strip_headers: Vec::new(),
                            status: None,
                            content_type: None,
                        }
                    } else if let Some(route) = configured_tcp_listener_route(
                        &state.config,
                        &listener_name,
                        player_id.clone(),
                    ) {
                        route
                    } else if let Some(script) = &state.script {
                        script
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
                            .inspect_err(|_| {
                                gateway
                                    .stats
                                    .script_fail_total
                                    .fetch_add(1, Ordering::Relaxed);
                            })?
                    } else {
                        return Err(anyhow!(
                            "tcp listener {} has no configured upstream and script runtime is disabled",
                            listener_name
                        ));
                    };

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
                        let lease =
                            gateway.acquire_upstream_lease("tcp", Some(&listener_name), upstream);
                        match TcpStream::connect(upstream).await {
                            Ok(stream) => {
                                gateway.on_upstream_success("tcp", Some(&listener_name), upstream);
                                selected = Some((stream, lease, upstream.clone()));
                                break;
                            }
                            Err(error) => {
                                gateway.on_upstream_failure(
                                    &state.config,
                                    "tcp",
                                    Some(&listener_name),
                                    upstream,
                                );
                                last_error = Some(anyhow!(
                                    "failed to connect tcp upstream {upstream}: {error}"
                                ));
                            }
                        }
                    }

                    let (mut outbound, _lease, upstream) = selected.ok_or_else(|| {
                        last_error.unwrap_or_else(|| anyhow!("failed to connect any tcp upstream"))
                    })?;

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

                gateway
                    .stats
                    .tcp_sessions_active
                    .fetch_sub(1, Ordering::Relaxed);
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
            let (received, client_addr) = listener_socket
                .recv_from(&mut buffer)
                .await
                .context("udp recv failed")?;
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
                        let route = if let Some(route) = configured_udp_listener_route(
                            &state.config,
                            &listener_name,
                            player_id.clone(),
                        ) {
                            route
                        } else if let Some(script) = &state.script {
                            script
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
                                .inspect_err(|_| {
                                    gateway.stats.script_fail_total.fetch_add(1, Ordering::Relaxed);
                                })?
                        } else {
                            return Err(anyhow!(
                                "udp listener {} has no configured upstream and script runtime is disabled",
                                listener_name
                            ));
                        };

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
        let on_upgrade = websocket_upgrade_requested(request.headers())
            .then(|| hyper::upgrade::on(&mut request));
        let version = version_label(request.version());
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        let body = match request.body_mut().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(error) => {
                self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?error, %remote_addr, "failed collecting request body");
                return Ok(GatewayHttpResponse::error(
                    StatusCode::BAD_REQUEST,
                    "invalid request body",
                )
                .into_hyper());
            }
        };

        let response = match self
            .dispatch_http(
                method,
                uri,
                headers,
                body,
                remote_addr,
                scheme,
                version,
                on_upgrade,
            )
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

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_http(
        &self,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
        on_upgrade: Option<OnUpgrade>,
    ) -> Result<GatewayHttpResponse> {
        let state = self.current_state().await;

        let host = headers
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
            .or_else(|| {
                uri.authority()
                    .map(|authority| authority.as_str().to_string())
            })
            .unwrap_or_else(|| "localhost".to_string());

        let player_id = extract_http_player_id(&uri, &headers, &state.config.affinity.http);
        let request_id = Uuid::new_v4().to_string();

        if monitoring_path_matches(&state.config.monitoring, uri.path()) {
            return Ok(json_gateway_response(
                StatusCode::OK,
                self.stats.snapshot_json(),
                "proxysss://metrics",
            ));
        }

        if let Some(response) = self.apply_http_rate_limit(
            &state.config.services.rate_limit.http,
            &host,
            &headers,
            remote_addr,
        ) {
            return Ok(response);
        }

        if state.config.services.webdav.enabled
            && webdav_path_matches(&state.config.services.webdav.path_prefix, uri.path())
        {
            return dispatch_webdav(&state.config.services.webdav, &method, &uri, &headers, body)
                .await;
        }

        if let Some(site) = state
            .config
            .services
            .static_sites
            .iter()
            .find(|site| static_site_path_matches(site, uri.path()))
        {
            return dispatch_static_site(site, &method, &uri).await;
        }

        if let Some(route) = builtin_http_route(uri.path()) {
            return Ok(dispatch_internal_http(&state.config, &route));
        }

        let route = if let Some(route) = configured_http_route(&state.config, &host, &uri) {
            route
        } else if let Some(script) = &state.script {
            HttpRouteConfig {
                decision: script
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
                    .inspect_err(|_| {
                        self.stats.script_fail_total.fetch_add(1, Ordering::Relaxed);
                    })?,
                compression: ResponseCompressionConfig::default(),
                cache: ResponseCacheConfig::default(),
            }
        } else {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "no built-in or YAML route matched; enable script.enabled to use TypeScript routing",
            ));
        };

        if route.decision.upstream.starts_with("proxysss://") {
            let response = dispatch_internal_http(&state.config, &route.decision);
            return finalize_http_response(&headers, &route.compression, response);
        }

        if websocket_upgrade_requested(&headers) || websocket_upstream_requested(&route.decision) {
            let on_upgrade = on_upgrade.ok_or_else(|| {
                anyhow!("websocket route requires an HTTP/1.1 upgrade-capable request")
            })?;
            return self
                .dispatch_websocket(
                    method,
                    uri,
                    headers,
                    body,
                    remote_addr,
                    scheme,
                    &host,
                    &route.decision,
                    on_upgrade,
                )
                .await;
        }

        let cache_key = cache_lookup_key(&route.cache, &method, &host, &uri, &headers);
        if let Some(cache_key) = cache_key.as_deref() {
            if let Some(response) = self.load_cached_http_response(cache_key) {
                return finalize_http_response(&headers, &route.compression, response);
            }
        }

        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route.decision,
            "http",
            None,
            route
                .decision
                .affinity_key
                .as_deref()
                .or(player_id.as_deref()),
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
            let upstream_url = build_upstream_url(upstream, &route.decision, &uri)?;
            let upstream_headers =
                build_upstream_headers(&headers, &route.decision, &host, remote_addr, scheme)?;
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
                last_error = Some(anyhow!(
                    "upstream {upstream} returned server error {}",
                    status.as_u16()
                ));
                continue;
            }

            self.on_upstream_success("http", None, upstream);
            let response = GatewayHttpResponse {
                status,
                headers: response_headers,
                body: response_body,
                upstream: upstream.clone(),
            };
            if let Some(cache_key) = cache_key.as_deref() {
                self.store_cached_http_response(cache_key, &route.cache, &response);
            }
            return finalize_http_response(&headers, &route.compression, response);
        }

        Err(last_error.unwrap_or_else(|| anyhow!("upstream request failed after retries")))
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_websocket(
        &self,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        host: &str,
        route: &RouteDecision,
        on_upgrade: OnUpgrade,
    ) -> Result<GatewayHttpResponse> {
        let state = self.current_state().await;
        let upstream_url = build_upstream_url(&route.upstream, route, &uri)?;
        let upstream_host = upstream_host_header(&upstream_url)?;
        let upstream_headers = build_websocket_upstream_headers(
            &headers,
            route,
            &upstream_host,
            remote_addr,
            scheme,
            host,
        )?;
        let upstream = upstream_url.to_string();
        let mut upstream_io =
            connect_upgrade_upstream(&upstream_url, state.config.http.allow_insecure_upstreams)
                .await
                .with_context(|| format!("failed connecting websocket upstream {upstream}"))?;

        let request_bytes =
            serialize_http_request(&method, &upstream_url, &upstream_headers, &body)?;
        upstream_io
            .write_all(&request_bytes)
            .await
            .with_context(|| format!("failed sending websocket handshake to {upstream}"))?;

        let (status, response_headers, leftover) = read_http_response_head(&mut upstream_io)
            .await
            .with_context(|| format!("failed reading websocket handshake from {upstream}"))?;

        if status != StatusCode::SWITCHING_PROTOCOLS {
            return Ok(GatewayHttpResponse {
                status,
                headers: response_headers,
                body: leftover.unwrap_or_default(),
                upstream,
            });
        }

        let tunnel_upstream = upstream.clone();
        tokio::spawn(async move {
            let result = async {
                let upgraded = on_upgrade
                    .await
                    .context("downstream websocket upgrade failed")?;
                let mut client = TokioIo::new(upgraded);
                if let Some(initial_bytes) = leftover {
                    if !initial_bytes.is_empty() {
                        client
                            .write_all(&initial_bytes)
                            .await
                            .context("failed flushing upstream websocket prelude")?;
                    }
                }
                copy_bidirectional(&mut client, &mut *upstream_io)
                    .await
                    .context("websocket tunnel relay failed")?;
                Ok::<_, anyhow::Error>(())
            }
            .await;

            if let Err(error) = result {
                tracing::warn!(?error, upstream = %tunnel_upstream, "websocket tunnel failed");
            }
        });

        Ok(GatewayHttpResponse {
            status,
            headers: response_headers,
            body: Bytes::new(),
            upstream,
        })
    }

    async fn current_state(&self) -> Arc<DynamicState> {
        self.dynamic.read().await.clone()
    }

    fn load_cached_http_response(&self, key: &str) -> Option<GatewayHttpResponse> {
        let now = Instant::now();
        let entry = self.http_cache.get(key)?;
        if entry.expires_at <= now {
            drop(entry);
            self.http_cache.remove(key);
            return None;
        }
        Some(GatewayHttpResponse {
            status: entry.status,
            headers: entry.headers.clone(),
            body: entry.body.clone(),
            upstream: entry.upstream.clone(),
        })
    }

    fn store_cached_http_response(
        &self,
        key: &str,
        config: &ResponseCacheConfig,
        response: &GatewayHttpResponse,
    ) {
        if !config.enabled
            || response.body.len() > config.max_body_bytes
            || !config
                .statuses
                .iter()
                .any(|status| *status == response.status.as_u16())
            || response.headers.iter().any(|(name, _)| name == SET_COOKIE)
            || cache_control_prevents_storage(&response.headers)
        {
            return;
        }

        self.http_cache.insert(
            key.to_string(),
            CachedHttpEntry {
                expires_at: Instant::now() + Duration::from_secs(config.ttl_secs),
                status: response.status,
                headers: response.headers.clone(),
                body: response.body.clone(),
                upstream: response.upstream.clone(),
            },
        );
    }

    fn apply_http_rate_limit(
        &self,
        config: &HttpRateLimitConfig,
        host: &str,
        headers: &HeaderMap,
        remote_addr: SocketAddr,
    ) -> Option<GatewayHttpResponse> {
        if !config.enabled {
            return None;
        }

        let key = http_rate_limit_key(config, host, headers, remote_addr)?;
        let retry_after = apply_http_rate_limit_to_store(&self.http_rate_limits, config, key)?;
        let status = StatusCode::from_u16(config.status).unwrap_or(StatusCode::TOO_MANY_REQUESTS);
        let mut response = GatewayHttpResponse::bytes(
            status,
            "text/plain; charset=utf-8",
            Bytes::from_static(b"rate limit exceeded"),
            "proxysss://rate-limit",
        );
        response.headers.push((
            HeaderName::from_static("retry-after"),
            HeaderValue::from_str(&retry_after).unwrap_or_else(|_| HeaderValue::from_static("1")),
        ));
        Some(response)
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
            affinity_key.map(|value| value.to_string()).or_else(|| {
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
            LoadBalanceAlgorithm::RoundRobin => {
                self.select_round_robin_plan(&scope_base, candidates)
            }
            LoadBalanceAlgorithm::LeastConnections => {
                self.select_least_connections_plan(protocol, listener, candidates)
            }
            LoadBalanceAlgorithm::SourceHash => {
                self.select_source_hash_plan(selected_key.as_deref(), candidates)
            }
            LoadBalanceAlgorithm::Rendezvous => self.select_rendezvous_plan(
                config,
                &scope_base,
                selected_key.as_deref(),
                candidates,
            ),
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

    fn select_source_hash_plan(
        &self,
        selected_key: Option<&str>,
        candidates: Vec<String>,
    ) -> Vec<String> {
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
            let mut entry = self
                .round_robin_state
                .entry(scope_base.to_string())
                .or_insert(0);
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
                Instant::now()
                    + Duration::from_secs(config.load_balance.passive_health.quarantine_secs),
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

    fn upstream_active_connections(
        &self,
        protocol: &str,
        listener: Option<&str>,
        upstream: &str,
    ) -> u64 {
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

    fn prune_sticky_affinity(&self) {
        let now = Instant::now();
        self.sticky_affinity
            .retain(|_, entry| entry.expires_at > now);
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

    fn html(body: impl Into<String>, upstream: impl Into<String>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            )],
            body: Bytes::from(body.into()),
            upstream: upstream.into(),
        }
    }

    fn redirect(location: impl Into<String>, upstream: impl Into<String>) -> Self {
        Self::redirect_with_status(StatusCode::TEMPORARY_REDIRECT, location, upstream)
    }

    fn redirect_with_status(
        status: StatusCode,
        location: impl Into<String>,
        upstream: impl Into<String>,
    ) -> Self {
        let location = location.into();
        let header =
            HeaderValue::from_str(&location).unwrap_or_else(|_| HeaderValue::from_static("/"));
        Self {
            status,
            headers: vec![(LOCATION, header)],
            body: Bytes::new(),
            upstream: upstream.into(),
        }
    }

    fn bytes(
        status: StatusCode,
        content_type: impl Into<String>,
        body: impl Into<Bytes>,
        upstream: impl Into<String>,
    ) -> Self {
        let content_type = HeaderValue::from_str(&content_type.into())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
        Self {
            status,
            headers: vec![(CONTENT_TYPE, content_type)],
            body: body.into(),
            upstream: upstream.into(),
        }
    }

    fn into_hyper(self) -> Response<Full<Bytes>> {
        let mut builder = Response::builder().status(self.status);
        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }
        builder.body(Full::new(self.body)).unwrap_or_else(|_| {
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

    let script = if config.script.enabled {
        match ScriptRuntime::spawn(&config.script, &default_script_env(&config)) {
            Ok(runtime) => Some(Arc::new(runtime)),
            Err(error) => {
                tracing::warn!(
                    ?error,
                    entry = %config.script.entry.display(),
                    "script runtime failed to start; continuing with YAML-only routing and skipping TypeScript extensions"
                );
                None
            }
        }
    } else {
        None
    };

    if let Some(script_runtime) = &script {
        auto_load_plugins(&config, script_runtime).await?;
    }

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
        let entry = entry
            .with_context(|| format!("failed to read plugin entry under {}", dir.display()))?;
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
        let spec = match load_auto_plugin_spec(&path) {
            Ok(spec) => spec,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    path = %path.display(),
                    "plugin sidecar metadata load failed; plugin will be ignored until the next reload"
                );
                continue;
            }
        };
        let name = spec.name.clone();

        match script.load_plugin(spec).await {
            Ok(_) => {
                tracing::info!(plugin = %name, path = %path.display(), "plugin auto-loaded");
            }
            Err(error) => {
                tracing::warn!(
                    ?error,
                    plugin = %name,
                    path = %path.display(),
                    "plugin auto-load failed; plugin will be ignored until the next reload"
                );
            }
        }
    }

    Ok(())
}

fn load_auto_plugin_spec(path: &Path) -> Result<ScriptPluginSpec> {
    let metadata = load_auto_plugin_metadata(path)?;
    let name = metadata.name.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "plugin".to_string())
    });

    Ok(ScriptPluginSpec {
        name,
        module_path: path.to_string_lossy().to_string(),
        priority: metadata.priority,
        enabled: metadata.enabled,
        config: metadata.config,
    })
}

fn load_auto_plugin_metadata(path: &Path) -> Result<AutoLoadPluginMetadata> {
    for sidecar in plugin_sidecar_paths(path) {
        if !sidecar.exists() {
            continue;
        }

        let body = std::fs::read_to_string(&sidecar)
            .with_context(|| format!("failed to read plugin sidecar {}", sidecar.display()))?;
        let ext = sidecar
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();

        let metadata: anyhow::Result<AutoLoadPluginMetadata> = match ext.as_str() {
            "json" => serde_json::from_str(&body).map_err(Into::into),
            "yaml" | "yml" => serde_yaml::from_str(&body).map_err(Into::into),
            _ => continue,
        };
        let metadata = metadata
            .with_context(|| format!("failed to parse plugin sidecar {}", sidecar.display()))?;

        return Ok(metadata);
    }

    Ok(AutoLoadPluginMetadata::default())
}

fn plugin_sidecar_paths(path: &Path) -> Vec<PathBuf> {
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Vec::new();
    };

    ["yaml", "yml", "json"]
        .into_iter()
        .map(|ext| path.with_file_name(format!("{stem}.plugin.{ext}")))
        .collect()
}

fn listener_specs(config: &GatewayConfig) -> Vec<ListenerSpec> {
    let mut specs = Vec::new();

    if !config.http.plain_bind.trim().is_empty() {
        specs.push(ListenerSpec::PlainHttp {
            bind: config.http.plain_bind.clone(),
        });
    }
    if !config.http.tls_bind.trim().is_empty() {
        specs.push(ListenerSpec::TlsHttp {
            bind: config.http.tls_bind.clone(),
            tls_fingerprint: tls_fingerprint(config),
        });
    }
    if !config.http.h3_bind.trim().is_empty() {
        specs.push(ListenerSpec::Http3 {
            bind: config.http.h3_bind.clone(),
            tls_fingerprint: tls_fingerprint(config),
        });
    }
    for listener in &config.tcp.listeners {
        specs.push(ListenerSpec::Tcp(listener.clone()));
    }
    for listener in &config.udp.listeners {
        specs.push(ListenerSpec::Udp(listener.clone()));
    }
    if config.admin.enabled {
        specs.push(ListenerSpec::Admin {
            bind: config.admin.bind.clone(),
        });
    }

    if config.services.ftp.enabled {
        specs.push(ListenerSpec::Tcp(TcpListenerConfig {
            name: "ftp".to_string(),
            bind: config.services.ftp.bind.clone(),
            upstream: config.services.ftp.upstream.clone(),
            upstreams: Vec::new(),
        }));
    }

    specs
}

impl ListenerSpec {
    fn key(&self) -> String {
        match self {
            ListenerSpec::PlainHttp { bind } => format!("http:{bind}"),
            ListenerSpec::TlsHttp {
                bind,
                tls_fingerprint,
            } => {
                format!("https:{bind}:{tls_fingerprint}")
            }
            ListenerSpec::Http3 {
                bind,
                tls_fingerprint,
            } => {
                format!("h3:{bind}:{tls_fingerprint}")
            }
            ListenerSpec::Tcp(listener) => format!("tcp:{}:{}", listener.name, listener.bind),
            ListenerSpec::Udp(listener) => format!("udp:{}:{}", listener.name, listener.bind),
            ListenerSpec::Admin { bind } => format!("admin:{bind}"),
        }
    }
}

fn read_file_hash(path: &Path) -> Result<u64> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(hasher.finish())
}

fn tls_fingerprint(config: &GatewayConfig) -> String {
    let cert_hash = read_file_hash(&config.http.tls.cert_path).unwrap_or(0);
    let key_hash = read_file_hash(&config.http.tls.key_path).unwrap_or(0);
    format!(
        "{}:{}:{}:{}",
        config.http.tls.mode as u8,
        config.http.tls.cert_path.display(),
        cert_hash,
        key_hash
    )
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

    for certificate in &config.http.tls.certificates {
        if !certificate.cert_path.exists() || !certificate.key_path.exists() {
            return Err(anyhow!(
                "configured sni cert/key files are missing: {} {}",
                certificate.cert_path.display(),
                certificate.key_path.display()
            ));
        }
    }

    Ok(())
}

fn build_rustls_server_config(
    config: &GatewayConfig,
    alpn_protocols: Vec<Vec<u8>>,
) -> Result<rustls::ServerConfig> {
    let mut server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(build_tls_resolver(config)?);
    server_config.alpn_protocols = alpn_protocols;
    Ok(server_config)
}

fn build_tls_resolver(config: &GatewayConfig) -> Result<Arc<dyn ResolvesServerCert>> {
    let default = Arc::new(build_certified_key(
        &config.http.tls.cert_path,
        &config.http.tls.key_path,
    )?);
    let mut by_name = BTreeMap::<String, Arc<CertifiedKey>>::new();

    for certificate in &config.http.tls.certificates {
        let certified = Arc::new(build_certified_key(
            &certificate.cert_path,
            &certificate.key_path,
        )?);
        insert_certificate_domains(&mut by_name, certified, certificate);
    }

    Ok(Arc::new(SniResolver { default, by_name }))
}

fn build_certified_key(cert_path: &Path, key_path: &Path) -> Result<CertifiedKey> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let provider = rustls::crypto::ring::default_provider();
    let signing_key = provider
        .key_provider
        .load_private_key(key)
        .map_err(|error| anyhow!("failed loading private key {}: {error}", key_path.display()))?;
    Ok(CertifiedKey::new(certs, signing_key))
}

fn insert_certificate_domains(
    by_name: &mut BTreeMap<String, Arc<CertifiedKey>>,
    certified: Arc<CertifiedKey>,
    certificate: &TlsCertificateConfig,
) {
    for domain in &certificate.domains {
        by_name.insert(domain.to_ascii_lowercase(), certified.clone());
        if let Some(suffix) = domain.strip_prefix("*.") {
            by_name.insert(
                format!(".{}", suffix.to_ascii_lowercase()),
                certified.clone(),
            );
        }
    }
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
    let upstream = if base_upstream.starts_with("http://")
        || base_upstream.starts_with("https://")
        || base_upstream.starts_with("ws://")
        || base_upstream.starts_with("wss://")
    {
        base_upstream.to_string()
    } else {
        format!("http://{}", base_upstream)
    };
    let mut url =
        Url::parse(&upstream).with_context(|| format!("invalid upstream url {}", base_upstream))?;

    let rewritten = route.rewrite_path.clone().unwrap_or_else(|| {
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

    apply_forwarding_headers(&mut headers, host, remote_addr, scheme)?;

    Ok(headers)
}

fn finalize_http_response(
    request_headers: &HeaderMap,
    compression: &ResponseCompressionConfig,
    mut response: GatewayHttpResponse,
) -> Result<GatewayHttpResponse> {
    if !compression.enabled
        || !accepts_gzip(request_headers)
        || !response_allows_compression(&response)
    {
        return Ok(response);
    }
    if response.body.len() < compression.min_length
        || !content_type_matches(&response, &compression.content_types)
    {
        return Ok(response);
    }

    let compressed = gzip_bytes(&response.body)?;
    response.body = Bytes::from(compressed);
    response.headers.retain(|(name, _)| name != CONTENT_LENGTH);
    response
        .headers
        .push((CONTENT_ENCODING, HeaderValue::from_static("gzip")));
    append_or_insert_header(&mut response.headers, VARY, "accept-encoding")?;
    response.headers.push((
        CONTENT_LENGTH,
        HeaderValue::from_str(&response.body.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    Ok(response)
}

fn gzip_bytes(body: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, body).context("failed to gzip response body")?;
    encoder.finish().context("failed to finalize gzip response")
}

fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').any(|item| item.trim().starts_with("gzip")))
        .unwrap_or(false)
}

fn response_allows_compression(response: &GatewayHttpResponse) -> bool {
    !response.body.is_empty()
        && response.status != StatusCode::NO_CONTENT
        && response.status != StatusCode::NOT_MODIFIED
        && !response
            .headers
            .iter()
            .any(|(name, _)| name == CONTENT_ENCODING)
}

fn content_type_matches(response: &GatewayHttpResponse, patterns: &[String]) -> bool {
    let content_type = response
        .headers
        .iter()
        .find(|(name, _)| name == CONTENT_TYPE)
        .and_then(|(_, value)| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    patterns
        .iter()
        .map(|pattern| pattern.to_ascii_lowercase())
        .any(|pattern| content_type.starts_with(&pattern))
}

fn append_or_insert_header(
    headers: &mut Vec<(HeaderName, HeaderValue)>,
    name: HeaderName,
    value: &str,
) -> Result<()> {
    let existing_name = name.clone();
    if let Some((_, existing)) = headers
        .iter_mut()
        .find(|(header_name, _)| *header_name == existing_name)
    {
        let existing_value = existing
            .to_str()
            .with_context(|| format!("invalid existing header value for {}", name.as_str()))?;
        let merged = format!("{existing_value}, {value}");
        *existing = HeaderValue::from_str(merged.trim_matches(|c| c == ',' || c == ' '))
            .with_context(|| format!("invalid header value for {}", name.as_str()))?;
    } else {
        let header_value = HeaderValue::from_str(value)
            .with_context(|| format!("invalid header value for {}", name.as_str()))?;
        headers.push((name, header_value));
    }
    Ok(())
}

fn cache_lookup_key(
    config: &ResponseCacheConfig,
    method: &Method,
    host: &str,
    uri: &Uri,
    headers: &HeaderMap,
) -> Option<String> {
    if !config.enabled
        || *method != Method::GET
        || headers.contains_key(AUTHORIZATION)
        || headers.contains_key(COOKIE)
    {
        return None;
    }

    Some(format!(
        "{}:{}:{}",
        method.as_str(),
        host.to_ascii_lowercase(),
        uri.path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/")
    ))
}

fn cache_control_prevents_storage(headers: &[(HeaderName, HeaderValue)]) -> bool {
    headers
        .iter()
        .find(|(name, _)| name == CACHE_CONTROL)
        .and_then(|(_, value)| value.to_str().ok())
        .map(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("no-store") || value.contains("private")
        })
        .unwrap_or(false)
}

fn apply_forwarding_headers(
    headers: &mut HeaderMap,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
) -> Result<()> {
    let remote_ip = remote_addr.ip().to_string();
    let xff = append_csv_header(headers.get("x-forwarded-for"), &remote_ip);
    let forwarded = append_forwarded_header(headers.get("forwarded"), &remote_ip, host, scheme);

    headers.insert(
        HeaderName::from_static("x-real-ip"),
        HeaderValue::from_str(&remote_ip).context("invalid x-real-ip header")?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-for"),
        HeaderValue::from_str(&xff).context("invalid x-forwarded-for header")?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-host"),
        HeaderValue::from_str(host).context("invalid x-forwarded-host header")?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_str(scheme).context("invalid x-forwarded-proto header")?,
    );
    headers.insert(
        HeaderName::from_static("forwarded"),
        HeaderValue::from_str(&forwarded).context("invalid forwarded header")?,
    );

    Ok(())
}

fn append_csv_header(existing: Option<&HeaderValue>, next: &str) -> String {
    match existing
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
    {
        Some(value) if !value.is_empty() => format!("{value}, {next}"),
        _ => next.to_string(),
    }
}

fn append_forwarded_header(
    existing: Option<&HeaderValue>,
    remote_ip: &str,
    host: &str,
    scheme: &str,
) -> String {
    let safe_host = sanitize_forwarded_host(host);
    let next = format!(
        "for={};host=\"{}\";proto={scheme}",
        forwarded_for_value(remote_ip),
        safe_host
    );
    match existing
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
    {
        Some(value) if !value.is_empty() => format!("{value}, {next}"),
        _ => next,
    }
}

fn forwarded_for_value(remote_ip: &str) -> String {
    remote_ip
        .parse::<std::net::IpAddr>()
        .map(|ip| match ip {
            std::net::IpAddr::V4(_) => remote_ip.to_string(),
            std::net::IpAddr::V6(_) => format!("\"[{remote_ip}]\""),
        })
        .unwrap_or_else(|_| remote_ip.to_string())
}

fn sanitize_forwarded_host(host: &str) -> String {
    host.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | ':' | '[' | ']'))
        .collect()
}

fn header_map_to_btree(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
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
    format!(
        "{}:{}:{}",
        protocol,
        listener.unwrap_or("default"),
        upstream
    )
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

fn extract_http_player_id(
    uri: &Uri,
    headers: &HeaderMap,
    cfg: &HttpAffinityConfig,
) -> Option<String> {
    if let Some(query) = uri.query() {
        let target_keys: HashSet<String> = cfg
            .query_keys
            .iter()
            .map(|key| key.to_ascii_lowercase())
            .collect();
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
        let target_keys: HashSet<String> = cfg
            .cookie_keys
            .iter()
            .map(|key| key.to_ascii_lowercase())
            .collect();
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
    String::from_utf8_lossy(&payload[..limit])
        .replace('\n', "\\n")
        .replace('\r', "\\r")
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
        admin.insert(
            "password".to_string(),
            serde_json::Value::String("***".to_string()),
        );
    }

    value
}

fn json_response(status: StatusCode, payload: serde_json::Value) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(payload.to_string())))
        .unwrap_or_else(|_| {
            text_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build json response",
            )
        })
}

fn html_response(status: StatusCode, body: impl Into<String>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(body.into())))
        .unwrap_or_else(|_| {
            text_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build html response",
            )
        })
}

fn text_response(status: StatusCode, body: impl Into<String>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(body.into())))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from_static(
                    b"failed to build text response",
                )))
                .expect("static response build should never fail")
        })
}

fn json_gateway_response(
    status: StatusCode,
    payload: serde_json::Value,
    upstream: impl Into<String>,
) -> GatewayHttpResponse {
    GatewayHttpResponse::bytes(
        status,
        "application/json",
        Bytes::from(serde_json::to_vec(&payload).unwrap_or_else(|_| b"{}".to_vec())),
        upstream,
    )
}

fn monitoring_path_matches(config: &crate::config::MonitoringConfig, path: &str) -> bool {
    config.enabled && path == config.path
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

fn websocket_upgrade_requested(headers: &HeaderMap) -> bool {
    let upgrade = headers
        .get("upgrade")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);
    let connection = headers
        .get("connection")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("upgrade"))
        })
        .unwrap_or(false);

    upgrade && connection
}

fn websocket_upstream_requested(route: &RouteDecision) -> bool {
    route.upstream.starts_with("ws://") || route.upstream.starts_with("wss://")
}

fn http_rate_limit_key(
    config: &HttpRateLimitConfig,
    host: &str,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
) -> Option<String> {
    match &config.key {
        RateLimitKey::RemoteAddr => Some(format!("remote:{}", remote_addr.ip())),
        RateLimitKey::Host => Some(format!("host:{}", host.to_ascii_lowercase())),
        RateLimitKey::Header(name) => headers
            .get(name.as_str())
            .and_then(|value| value.to_str().ok())
            .map(|value| format!("header:{}:{}", name.to_ascii_lowercase(), value)),
    }
}

fn apply_http_rate_limit_to_store(
    store: &DashMap<String, RateLimitBucket>,
    config: &HttpRateLimitConfig,
    key: String,
) -> Option<String> {
    let now = Instant::now();
    let window = Duration::from_millis(config.window_ms.max(100));
    let limit = config.requests.saturating_add(config.burst).max(1);

    let mut bucket = store.entry(key).or_insert(RateLimitBucket {
        window_start: now,
        count: 0,
    });

    if now.duration_since(bucket.window_start) >= window {
        bucket.window_start = now;
        bucket.count = 0;
    }

    bucket.count = bucket.count.saturating_add(1);
    if bucket.count <= limit {
        return None;
    }

    Some(
        window
            .saturating_sub(now.duration_since(bucket.window_start))
            .as_secs()
            .max(1)
            .to_string(),
    )
}

fn builtin_http_route(path: &str) -> Option<RouteDecision> {
    let upstream = match path {
        "/" | "/index.html" | "/docs" => "proxysss://welcome",
        "/healthz" => "proxysss://healthz",
        "/admin" => "proxysss://admin",
        path if path.starts_with("/static/") => {
            return Some(RouteDecision {
                upstream: format!("proxysss://static/{}", path.trim_start_matches("/static/")),
                upstreams: Vec::new(),
                affinity_key: None,
                rewrite_path: None,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                status: None,
                content_type: None,
            });
        }
        _ => return None,
    };

    Some(RouteDecision {
        upstream: upstream.to_string(),
        upstreams: Vec::new(),
        affinity_key: None,
        rewrite_path: None,
        set_headers: BTreeMap::new(),
        strip_headers: Vec::new(),
        status: None,
        content_type: None,
    })
}

fn configured_tcp_listener_route(
    config: &GatewayConfig,
    listener_name: &str,
    affinity_key: Option<String>,
) -> Option<RouteDecision> {
    config
        .tcp
        .listeners
        .iter()
        .find(|listener| listener.name == listener_name)
        .and_then(|listener| {
            configured_stream_listener_route(&listener.upstream, &listener.upstreams, affinity_key)
        })
}

fn configured_udp_listener_route(
    config: &GatewayConfig,
    listener_name: &str,
    affinity_key: Option<String>,
) -> Option<RouteDecision> {
    config
        .udp
        .listeners
        .iter()
        .find(|listener| listener.name == listener_name)
        .and_then(|listener| {
            configured_stream_listener_route(&listener.upstream, &listener.upstreams, affinity_key)
        })
}

fn configured_stream_listener_route(
    upstream: &str,
    upstreams: &[String],
    affinity_key: Option<String>,
) -> Option<RouteDecision> {
    let upstream = upstream.trim();
    let mut normalized_upstreams: Vec<String> = upstreams
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect();

    let selected = if !upstream.is_empty() {
        if !normalized_upstreams.iter().any(|item| item == upstream) {
            normalized_upstreams.insert(0, upstream.to_string());
        }
        upstream.to_string()
    } else if let Some(first) = normalized_upstreams.first() {
        first.clone()
    } else {
        return None;
    };

    Some(RouteDecision {
        upstream: selected,
        upstreams: normalized_upstreams,
        affinity_key,
        rewrite_path: None,
        set_headers: BTreeMap::new(),
        strip_headers: Vec::new(),
        status: None,
        content_type: None,
    })
}

pub(crate) fn default_script_env(config: &GatewayConfig) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        "PROXYSSS_VERSION".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    env.insert(
        "PROXYSSS_CONFIG_ROOT".to_string(),
        config.root_dir.to_string_lossy().to_string(),
    );
    env.insert(
        "PROXYSSS_SCRIPT_CWD".to_string(),
        config
            .script
            .cwd
            .clone()
            .unwrap_or_else(|| config.root_dir.clone())
            .to_string_lossy()
            .to_string(),
    );
    env.insert(
        "PROXYSSS_HTTP_BIND".to_string(),
        config.http.plain_bind.clone(),
    );
    env.insert(
        "PROXYSSS_HTTPS_BIND".to_string(),
        config.http.tls_bind.clone(),
    );
    env.insert(
        "PROXYSSS_HTTP3_BIND".to_string(),
        config.http.h3_bind.clone(),
    );
    env.insert(
        "PROXYSSS_ADMIN_BIND".to_string(),
        if config.admin.enabled {
            config.admin.bind.clone()
        } else {
            String::new()
        },
    );
    env.insert(
        "PROXYSSS_PLUGINS_ENABLED".to_string(),
        if config.plugins.enabled { "1" } else { "0" }.to_string(),
    );
    env.insert(
        "PROXYSSS_SCRIPT_ENABLED".to_string(),
        if config.script.enabled { "1" } else { "0" }.to_string(),
    );
    env
}

fn configured_http_route(config: &GatewayConfig, host: &str, uri: &Uri) -> Option<HttpRouteConfig> {
    configured_domain_route(config, host, uri)
        .or_else(|| configured_reverse_proxy_route(config, host, uri))
}

fn configured_domain_route(
    config: &GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig> {
    config
        .services
        .domain_routes
        .iter()
        .filter(|route| domain_route_matches(route, host, uri.path()))
        .max_by_key(|route| route.path_prefix.len())
        .map(|route| domain_route_config(route, uri))
}

fn configured_reverse_proxy_route(
    config: &GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig> {
    config
        .services
        .reverse_proxy
        .routes
        .iter()
        .filter(|route| reverse_proxy_route_matches(route, host, uri.path()))
        .max_by_key(|route| route.path_prefix.len())
        .map(|route| HttpRouteConfig {
            decision: reverse_proxy_route_decision(route, uri),
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
        })
}

fn domain_route_matches(route: &DomainRouteConfig, host: &str, path: &str) -> bool {
    route.domains.iter().any(|item| host_matches(item, host))
        && reverse_proxy_path_matches(&route.path_prefix, path)
}

fn reverse_proxy_route_matches(route: &ReverseProxyRouteConfig, host: &str, path: &str) -> bool {
    if !route.hosts.is_empty() && !route.hosts.iter().any(|item| host_matches(item, host)) {
        return false;
    }

    reverse_proxy_path_matches(&route.path_prefix, path)
}

fn reverse_proxy_path_matches(prefix: &str, path: &str) -> bool {
    let prefix = normalize_route_prefix(prefix);
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn host_matches(pattern: &str, host: &str) -> bool {
    let host = host
        .split_once(':')
        .map(|(host, _)| host)
        .unwrap_or(host)
        .to_ascii_lowercase();
    let pattern = pattern.to_ascii_lowercase();

    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host == suffix || host.ends_with(&format!(".{suffix}"));
    }

    host == pattern
}

fn reverse_proxy_route_decision(route: &ReverseProxyRouteConfig, uri: &Uri) -> RouteDecision {
    let rewrite_path = route.strip_prefix.then(|| {
        let prefix = normalize_route_prefix(&route.path_prefix);
        let suffix = uri.path().strip_prefix(&prefix).unwrap_or(uri.path());
        let path = if suffix.is_empty() {
            "/".to_string()
        } else if suffix.starts_with('/') {
            suffix.to_string()
        } else {
            format!("/{suffix}")
        };
        match uri.query() {
            Some(query) => format!("{path}?{query}"),
            None => path,
        }
    });

    RouteDecision {
        upstream: route.upstream.clone(),
        upstreams: route.upstreams.clone(),
        affinity_key: None,
        rewrite_path,
        set_headers: route.set_headers.clone(),
        strip_headers: route.strip_headers.clone(),
        status: None,
        content_type: None,
    }
}

fn domain_route_config(route: &DomainRouteConfig, uri: &Uri) -> HttpRouteConfig {
    HttpRouteConfig {
        decision: RouteDecision {
            upstream: route.upstream.clone(),
            upstreams: route.upstreams.clone(),
            affinity_key: None,
            rewrite_path: route.strip_prefix.then(|| {
                let prefix = normalize_route_prefix(&route.path_prefix);
                let suffix = uri.path().strip_prefix(&prefix).unwrap_or(uri.path());
                let path = if suffix.is_empty() {
                    "/".to_string()
                } else if suffix.starts_with('/') {
                    suffix.to_string()
                } else {
                    format!("/{suffix}")
                };
                match uri.query() {
                    Some(query) => format!("{path}?{query}"),
                    None => path,
                }
            }),
            set_headers: route.set_headers.clone(),
            strip_headers: route.strip_headers.clone(),
            status: None,
            content_type: None,
        },
        compression: route.compression.clone(),
        cache: route.cache.clone(),
    }
}

fn normalize_route_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

async fn dispatch_static_site(
    site: &StaticSiteConfig,
    method: &Method,
    uri: &Uri,
) -> Result<GatewayHttpResponse> {
    if method != Method::GET && method != Method::HEAD {
        return Ok(GatewayHttpResponse::bytes(
            StatusCode::METHOD_NOT_ALLOWED,
            "text/plain; charset=utf-8",
            Bytes::from_static(b"static site method not allowed"),
            "proxysss://static",
        ));
    }

    let Some(mut target) = static_site_filesystem_path(site, uri.path())? else {
        return Ok(GatewayHttpResponse::error(
            StatusCode::NOT_FOUND,
            "static path not found",
        ));
    };

    let metadata = match tokio::fs::metadata(&target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "static path not found",
            ));
        }
        Err(error) => return Err(error).context("failed reading static metadata"),
    };

    let metadata = if metadata.is_dir() {
        let mut found_index = None;
        for index in &site.index_files {
            let candidate = target.join(index);
            if tokio::fs::metadata(&candidate)
                .await
                .map(|item| item.is_file())
                .unwrap_or(false)
            {
                found_index = Some(candidate);
                break;
            }
        }

        if let Some(index) = found_index {
            target = index;
            tokio::fs::metadata(&target)
                .await
                .context("failed reading static index metadata")?
        } else if site.autoindex {
            return static_autoindex(site, uri.path(), &target).await;
        } else {
            return Ok(GatewayHttpResponse::error(
                StatusCode::FORBIDDEN,
                "static directory listing is disabled",
            ));
        }
    } else {
        metadata
    };

    let body = if method == Method::HEAD {
        Bytes::new()
    } else {
        Bytes::from(
            tokio::fs::read(&target)
                .await
                .context("failed reading static file")?,
        )
    };
    let mut response = GatewayHttpResponse::bytes(
        StatusCode::OK,
        static_content_type(&target),
        body,
        "proxysss://static",
    );
    response.headers.push((
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    Ok(response)
}

async fn static_autoindex(
    site: &StaticSiteConfig,
    request_path: &str,
    target: &Path,
) -> Result<GatewayHttpResponse> {
    let mut entries = tokio::fs::read_dir(target)
        .await
        .context("failed reading static directory")?;
    let mut links = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .context("failed reading static directory entry")?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let href = join_static_href(&site.path_prefix, request_path, &name);
        links.push(format!(
            r#"<li><a href="{}">{}</a></li>"#,
            xml_escape(&href),
            xml_escape(&name)
        ));
    }
    links.sort();

    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Index of {}</title></head><body><h1>Index of {}</h1><ul>{}</ul></body></html>",
        xml_escape(request_path),
        xml_escape(request_path),
        links.join("")
    );

    Ok(GatewayHttpResponse::bytes(
        StatusCode::OK,
        "text/html; charset=utf-8",
        Bytes::from(body),
        "proxysss://static",
    ))
}

fn static_site_path_matches(site: &StaticSiteConfig, path: &str) -> bool {
    webdav_path_matches(&site.path_prefix, path)
}

fn static_site_filesystem_path(
    site: &StaticSiteConfig,
    request_path: &str,
) -> Result<Option<PathBuf>> {
    let prefix = normalize_webdav_prefix(&site.path_prefix);
    if !webdav_path_matches(&prefix, request_path) {
        return Ok(None);
    }

    let relative = request_path
        .strip_prefix(&prefix)
        .unwrap_or("")
        .trim_start_matches('/');
    let decoded = percent_decode_path(relative)?;
    let mut target = site.root.clone();

    for part in decoded.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        let component_path = Path::new(part);
        if component_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(anyhow!("static path escapes root"));
        }
        target.push(part);
    }

    Ok(Some(target))
}

fn join_static_href(prefix: &str, base_path: &str, child_name: &str) -> String {
    let base = if base_path.ends_with('/') {
        base_path.to_string()
    } else if static_site_path_matches(
        &StaticSiteConfig {
            name: "_".to_string(),
            path_prefix: prefix.to_string(),
            root: PathBuf::new(),
            index_files: Vec::new(),
            autoindex: false,
        },
        base_path,
    ) {
        format!("{base_path}/")
    } else {
        format!("{}/", normalize_webdav_prefix(prefix))
    };
    format!("{base}{child_name}")
}

fn static_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

async fn dispatch_webdav(
    config: &WebDavConfig,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<GatewayHttpResponse> {
    let Some(target) = webdav_filesystem_path(config, uri.path())? else {
        return Ok(GatewayHttpResponse::error(
            StatusCode::NOT_FOUND,
            "webdav path not found",
        ));
    };

    let upstream = "proxysss://webdav";
    match method.as_str() {
        "OPTIONS" => Ok(webdav_options_response(upstream)),
        "PROPFIND" => webdav_propfind(config, uri.path(), &target, upstream).await,
        "GET" => webdav_get(&target, false, upstream).await,
        "HEAD" => webdav_get(&target, true, upstream).await,
        "PUT" => {
            if !config.allow_write {
                return Ok(GatewayHttpResponse::error(
                    StatusCode::FORBIDDEN,
                    "webdav writes are disabled",
                ));
            }
            webdav_put(&target, body, upstream).await
        }
        "DELETE" => {
            if !config.allow_write {
                return Ok(GatewayHttpResponse::error(
                    StatusCode::FORBIDDEN,
                    "webdav writes are disabled",
                ));
            }
            webdav_delete(&target, upstream).await
        }
        "MKCOL" => {
            if !config.allow_write {
                return Ok(GatewayHttpResponse::error(
                    StatusCode::FORBIDDEN,
                    "webdav writes are disabled",
                ));
            }
            webdav_mkcol(&target, upstream).await
        }
        "COPY" | "MOVE" => {
            if !config.allow_write {
                return Ok(GatewayHttpResponse::error(
                    StatusCode::FORBIDDEN,
                    "webdav writes are disabled",
                ));
            }
            let Some(destination) = webdav_destination_path(config, headers)? else {
                return Ok(GatewayHttpResponse::error(
                    StatusCode::BAD_REQUEST,
                    "missing webdav Destination header",
                ));
            };
            webdav_copy_or_move(method.as_str(), &target, &destination, upstream).await
        }
        _ => Ok(GatewayHttpResponse::bytes(
            StatusCode::METHOD_NOT_ALLOWED,
            "text/plain; charset=utf-8",
            Bytes::from_static(b"webdav method not allowed"),
            upstream,
        )),
    }
}

fn webdav_options_response(upstream: &str) -> GatewayHttpResponse {
    let mut response = GatewayHttpResponse::bytes(
        StatusCode::NO_CONTENT,
        "text/plain; charset=utf-8",
        Bytes::new(),
        upstream,
    );
    response.headers.push((
        HeaderName::from_static("dav"),
        HeaderValue::from_static("1, 2"),
    ));
    response.headers.push((
        HeaderName::from_static("allow"),
        HeaderValue::from_static("OPTIONS, PROPFIND, GET, HEAD, PUT, DELETE, MKCOL, COPY, MOVE"),
    ));
    response
}

async fn webdav_propfind(
    config: &WebDavConfig,
    request_path: &str,
    target: &Path,
    upstream: &str,
) -> Result<GatewayHttpResponse> {
    let metadata = match tokio::fs::metadata(target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "webdav path not found",
            ));
        }
        Err(error) => return Err(error).context("failed reading webdav metadata"),
    };

    let mut responses = Vec::new();
    responses.push(webdav_prop_response(request_path, &metadata));

    if metadata.is_dir() {
        let mut entries = tokio::fs::read_dir(target)
            .await
            .context("failed reading webdav directory")?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .context("failed reading webdav directory entry")?
        {
            let child_metadata = entry
                .metadata()
                .await
                .context("failed reading webdav child metadata")?;
            let child_name = entry.file_name().to_string_lossy().to_string();
            let href = join_webdav_href(&config.path_prefix, request_path, &child_name);
            responses.push(webdav_prop_response(&href, &child_metadata));
        }
    }

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:">{}</D:multistatus>"#,
        responses.join("")
    );
    Ok(GatewayHttpResponse::bytes(
        StatusCode::from_u16(207).expect("valid multistatus code"),
        "application/xml; charset=utf-8",
        Bytes::from(body),
        upstream,
    ))
}

async fn webdav_get(target: &Path, head_only: bool, upstream: &str) -> Result<GatewayHttpResponse> {
    let metadata = match tokio::fs::metadata(target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "webdav path not found",
            ));
        }
        Err(error) => return Err(error).context("failed reading webdav metadata"),
    };

    if metadata.is_dir() {
        return Ok(GatewayHttpResponse::error(
            StatusCode::FORBIDDEN,
            "webdav GET on directories is disabled",
        ));
    }

    let body = if head_only {
        Bytes::new()
    } else {
        Bytes::from(
            tokio::fs::read(target)
                .await
                .context("failed reading webdav file")?,
        )
    };
    let mut response =
        GatewayHttpResponse::bytes(StatusCode::OK, "application/octet-stream", body, upstream);
    response.headers.push((
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    Ok(response)
}

async fn webdav_put(target: &Path, body: Bytes, upstream: &str) -> Result<GatewayHttpResponse> {
    let existed = tokio::fs::metadata(target).await.is_ok();
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("failed creating webdav parent directory")?;
    }
    tokio::fs::write(target, body)
        .await
        .context("failed writing webdav file")?;
    Ok(GatewayHttpResponse::bytes(
        if existed {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::CREATED
        },
        "text/plain; charset=utf-8",
        Bytes::new(),
        upstream,
    ))
}

async fn webdav_delete(target: &Path, upstream: &str) -> Result<GatewayHttpResponse> {
    let metadata = match tokio::fs::metadata(target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "webdav path not found",
            ));
        }
        Err(error) => return Err(error).context("failed reading webdav metadata"),
    };

    if metadata.is_dir() {
        tokio::fs::remove_dir_all(target)
            .await
            .context("failed deleting webdav directory")?;
    } else {
        tokio::fs::remove_file(target)
            .await
            .context("failed deleting webdav file")?;
    }

    Ok(GatewayHttpResponse::bytes(
        StatusCode::NO_CONTENT,
        "text/plain; charset=utf-8",
        Bytes::new(),
        upstream,
    ))
}

async fn webdav_mkcol(target: &Path, upstream: &str) -> Result<GatewayHttpResponse> {
    if tokio::fs::metadata(target).await.is_ok() {
        return Ok(GatewayHttpResponse::error(
            StatusCode::METHOD_NOT_ALLOWED,
            "webdav collection already exists",
        ));
    }
    tokio::fs::create_dir_all(target)
        .await
        .context("failed creating webdav collection")?;
    Ok(GatewayHttpResponse::bytes(
        StatusCode::CREATED,
        "text/plain; charset=utf-8",
        Bytes::new(),
        upstream,
    ))
}

async fn webdav_copy_or_move(
    method: &str,
    source: &Path,
    destination: &Path,
    upstream: &str,
) -> Result<GatewayHttpResponse> {
    let metadata = match tokio::fs::metadata(source).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "webdav source path not found",
            ));
        }
        Err(error) => return Err(error).context("failed reading webdav source metadata"),
    };

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("failed creating webdav destination parent")?;
    }

    if method == "MOVE" {
        tokio::fs::rename(source, destination)
            .await
            .context("failed moving webdav path")?;
    } else if metadata.is_dir() {
        copy_dir_recursive(source, destination).await?;
    } else {
        tokio::fs::copy(source, destination)
            .await
            .context("failed copying webdav file")?;
    }

    Ok(GatewayHttpResponse::bytes(
        StatusCode::CREATED,
        "text/plain; charset=utf-8",
        Bytes::new(),
        upstream,
    ))
}

async fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];

    while let Some((from, to)) = pending.pop() {
        tokio::fs::create_dir_all(&to)
            .await
            .context("failed creating copied webdav directory")?;
        let mut entries = tokio::fs::read_dir(&from)
            .await
            .context("failed reading copied webdav directory")?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .context("failed reading copied webdav directory entry")?
        {
            let child_from = entry.path();
            let child_to = to.join(entry.file_name());
            let metadata = entry
                .metadata()
                .await
                .context("failed reading copied webdav metadata")?;
            if metadata.is_dir() {
                pending.push((child_from, child_to));
            } else {
                tokio::fs::copy(&child_from, &child_to)
                    .await
                    .context("failed copying webdav file")?;
            }
        }
    }

    Ok(())
}

fn webdav_path_matches(prefix: &str, path: &str) -> bool {
    let prefix = normalize_webdav_prefix(prefix);
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn webdav_filesystem_path(config: &WebDavConfig, request_path: &str) -> Result<Option<PathBuf>> {
    let prefix = normalize_webdav_prefix(&config.path_prefix);
    if !webdav_path_matches(&prefix, request_path) {
        return Ok(None);
    }

    let relative = request_path
        .strip_prefix(&prefix)
        .unwrap_or("")
        .trim_start_matches('/');
    let decoded = percent_decode_path(relative)?;
    let mut target = config.root.clone();

    for part in decoded.split('/') {
        if part.is_empty() {
            continue;
        }
        let component_path = Path::new(part);
        if component_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(anyhow!("webdav path escapes root"));
        }
        if part == "." {
            continue;
        }
        target.push(part);
    }

    Ok(Some(target))
}

fn webdav_destination_path(config: &WebDavConfig, headers: &HeaderMap) -> Result<Option<PathBuf>> {
    let Some(destination) = headers
        .get("destination")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };

    let path = if destination.starts_with("http://") || destination.starts_with("https://") {
        Url::parse(destination)
            .context("invalid webdav Destination URL")?
            .path()
            .to_string()
    } else {
        destination.to_string()
    };

    webdav_filesystem_path(config, &path)
}

fn normalize_webdav_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn percent_decode_path(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(anyhow!("invalid percent-encoded webdav path"));
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .context("invalid percent-encoded webdav path")?;
            let byte =
                u8::from_str_radix(hex, 16).context("invalid percent-encoded webdav path")?;
            output.push(byte);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(output).context("webdav path is not utf-8")
}

fn join_webdav_href(prefix: &str, base_path: &str, child_name: &str) -> String {
    let base = if base_path.ends_with('/') {
        base_path.to_string()
    } else if webdav_path_matches(prefix, base_path) {
        format!("{base_path}/")
    } else {
        format!("{}/", normalize_webdav_prefix(prefix))
    };
    format!("{base}{}", xml_escape(child_name))
}

fn webdav_prop_response(path: &str, metadata: &std::fs::Metadata) -> String {
    let resource_type = if metadata.is_dir() {
        "<D:resourcetype><D:collection/></D:resourcetype>"
    } else {
        "<D:resourcetype/>"
    };
    let content_length = if metadata.is_dir() { 0 } else { metadata.len() };
    format!(
        r#"<D:response><D:href>{}</D:href><D:propstat><D:prop>{}<D:getcontentlength>{}</D:getcontentlength></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"#,
        xml_escape(path),
        resource_type,
        content_length
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn dispatch_internal_http(config: &GatewayConfig, route: &RouteDecision) -> GatewayHttpResponse {
    let upstream = route.upstream.as_str();
    match upstream {
        "proxysss://welcome" => {
            GatewayHttpResponse::html(render_welcome_html(config), "proxysss://welcome")
        }
        "proxysss://healthz" => GatewayHttpResponse::bytes(
            StatusCode::OK,
            "application/json",
            Bytes::from_static(br#"{"ok":true,"service":"proxysss"}"#),
            "proxysss://healthz",
        ),
        "proxysss://admin" => GatewayHttpResponse::redirect(
            format!("http://{}/", config.admin.bind),
            "proxysss://admin",
        ),
        _ if upstream.starts_with("proxysss://redirect/") => {
            let location = upstream.trim_start_matches("proxysss://redirect/");
            let status = route
                .status
                .and_then(|value| StatusCode::from_u16(value).ok())
                .filter(|value| value.is_redirection())
                .unwrap_or(StatusCode::MOVED_PERMANENTLY);
            GatewayHttpResponse::redirect_with_status(status, location, upstream)
        }
        _ if upstream.starts_with("proxysss://static/") => {
            let relative = upstream.trim_start_matches("proxysss://static/");
            match resolve_static_file(&config.root_dir, relative) {
                Ok(path) => match std::fs::read(&path) {
                    Ok(bytes) => GatewayHttpResponse::bytes(
                        StatusCode::OK,
                        route
                            .content_type
                            .clone()
                            .unwrap_or_else(|| guess_content_type(&path).to_string()),
                        Bytes::from(bytes),
                        upstream,
                    ),
                    Err(error) => GatewayHttpResponse::error(
                        StatusCode::NOT_FOUND,
                        format!("static file not found: {error}"),
                    ),
                },
                Err(error) => GatewayHttpResponse::error(StatusCode::FORBIDDEN, error.to_string()),
            }
        }
        _ => GatewayHttpResponse::error(StatusCode::NOT_FOUND, "unknown internal route"),
    }
}

fn resolve_static_file(root_dir: &Path, relative: &str) -> Result<PathBuf> {
    let decoded = percent_decode_path(relative)?;
    let relative_path = PathBuf::from(decoded.trim_start_matches('/'));
    if relative_path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(anyhow!("static path cannot contain parent directory"));
    }

    let root = root_dir
        .canonicalize()
        .unwrap_or_else(|_| root_dir.to_path_buf());
    let candidate = root.join(relative_path);
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to resolve static path {}", candidate.display()))?;
    if !canonical.starts_with(&root) {
        return Err(anyhow!("static path escaped config root"));
    }
    Ok(canonical)
}

fn guess_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn render_welcome_html(_config: &GatewayConfig) -> String {
    let html = r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>proxysss</title>
    <style>
        :root {
            --bg: #07111a;
            --bg-soft: #0c1823;
            --panel: rgba(10, 22, 33, 0.92);
            --panel-strong: rgba(12, 28, 43, 0.96);
            --line: rgba(139, 202, 255, 0.16);
            --text: #f2f7fb;
            --muted: #9ab0c2;
            --cyan: #56d7ff;
            --mint: #77f3bf;
            --gold: #f3d27c;
            --rose: #ff8f8f;
        }
        * { box-sizing: border-box; }
        html, body {
            width: 100%;
            height: 100%;
            overflow: hidden;
        }
        body {
            margin: 0;
            font-family: "Avenir Next", "PingFang SC", "Microsoft YaHei", sans-serif;
            color: var(--text);
            background:
                radial-gradient(circle at 14% 12%, rgba(86, 215, 255, 0.14), transparent 28%),
                radial-gradient(circle at 82% 18%, rgba(119, 243, 191, 0.10), transparent 26%),
                linear-gradient(160deg, var(--bg), var(--bg-soft) 48%, #09131d 100%);
        }
        body::before,
        body::after {
            content: "";
            position: fixed;
            inset: auto;
            pointer-events: none;
            border-radius: 999px;
            filter: blur(18px);
            opacity: 0.5;
        }
        body::before {
            width: 210px;
            height: 210px;
            top: 9%;
            right: 9%;
            background: rgba(86, 215, 255, 0.15);
        }
        body::after {
            width: 180px;
            height: 180px;
            bottom: 7%;
            left: 6%;
            background: rgba(119, 243, 191, 0.11);
        }
        .page {
            position: relative;
            z-index: 1;
            width: min(1200px, calc(100vw - 40px));
            height: min(760px, calc(100vh - 40px));
            margin: 20px auto;
            padding: 24px;
            display: grid;
            grid-template-columns: minmax(320px, 1.15fr) minmax(320px, 0.95fr);
            gap: 18px;
        }
        .hero,
        .panel {
            border: 1px solid var(--line);
            border-radius: 28px;
            background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0)) , var(--panel);
            box-shadow: 0 18px 60px rgba(0, 0, 0, 0.22);
            backdrop-filter: blur(16px);
        }
        .hero {
            padding: 28px;
            display: grid;
            grid-template-rows: auto auto 1fr auto;
            min-height: 0;
        }
        .topbar {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 14px;
        }
        .brand {
            display: flex;
            align-items: center;
            gap: 16px;
        }
        .logo {
            width: 76px;
            height: 76px;
            border-radius: 24px;
            position: relative;
            overflow: hidden;
            background: linear-gradient(145deg, rgba(86, 215, 255, 0.95), rgba(119, 243, 191, 0.88));
            box-shadow: 0 18px 44px rgba(86, 215, 255, 0.22);
        }
        .logo::before,
        .logo::after {
            content: "";
            position: absolute;
            inset: 0;
        }
        .logo::before {
            background: linear-gradient(180deg, rgba(255,255,255,0.24), transparent 46%);
        }
        .logo::after {
            width: 38px;
            height: 38px;
            inset: 19px auto auto 19px;
            border-radius: 14px 14px 14px 4px;
            border: 7px solid #04111a;
            border-right-width: 10px;
            transform: skewY(-8deg);
        }
        .brand-mark {
            display: flex;
            flex-direction: column;
            gap: 4px;
        }
        .eyebrow {
            font-size: 11px;
            letter-spacing: 0.22em;
            text-transform: uppercase;
            color: var(--mint);
        }
        .brand-title {
            font-size: 26px;
            line-height: 1;
            font-weight: 800;
        }
        .version {
            color: var(--muted);
            font-size: 13px;
        }
        .docs-btn {
            display: inline-flex;
            align-items: center;
            gap: 10px;
            text-decoration: none;
            padding: 12px 16px;
            border-radius: 999px;
            color: #04111a;
            font-weight: 800;
            background: linear-gradient(135deg, var(--cyan), var(--mint));
            white-space: nowrap;
        }
        .hero-main {
            display: grid;
            align-content: center;
            gap: 16px;
            min-height: 0;
        }
        .hero-main h1 {
            margin: 0;
            font-size: clamp(42px, 6vw, 78px);
            line-height: 0.92;
            letter-spacing: -0.05em;
        }
        .hero-main p {
            margin: 0;
            max-width: 560px;
            color: var(--muted);
            font-size: clamp(15px, 2vw, 18px);
            line-height: 1.6;
        }
        .protocols {
            display: grid;
            grid-template-columns: repeat(4, minmax(0, 1fr));
            gap: 10px;
        }
        .protocol {
            min-width: 0;
            border-radius: 18px;
            padding: 12px 12px 10px;
            background: rgba(255, 255, 255, 0.04);
            border: 1px solid rgba(255, 255, 255, 0.06);
        }
        .protocol strong {
            display: block;
            font-size: 13px;
            line-height: 1.2;
        }
        .protocol span {
            display: block;
            margin-top: 4px;
            font-size: 11px;
            color: var(--muted);
            line-height: 1.3;
        }
        .notes {
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
        }
        .note {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 10px 12px;
            border-radius: 999px;
            background: rgba(255,255,255,0.04);
            border: 1px solid rgba(255,255,255,0.06);
            font-size: 12px;
            color: var(--muted);
        }
        .note i {
            width: 7px;
            height: 7px;
            border-radius: 999px;
            background: var(--mint);
            display: inline-block;
        }
        .side {
            display: grid;
            gap: 18px;
            min-height: 0;
        }
        .panel {
            padding: 20px 20px 18px;
            min-height: 0;
        }
        .panel h2,
        .panel h3,
        .panel p {
            margin: 0;
        }
        .panel-head {
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            gap: 12px;
            margin-bottom: 14px;
        }
        .panel-head h2 {
            font-size: 20px;
        }
        .panel-head span {
            font-size: 12px;
            color: var(--muted);
        }
        .bench-grid {
            display: grid;
            gap: 12px;
        }
        .bench-card {
            border-radius: 20px;
            padding: 14px 14px 12px;
            background: rgba(255,255,255,0.035);
            border: 1px solid rgba(255,255,255,0.06);
        }
        .bench-card.proxysss {
            background: linear-gradient(135deg, rgba(86, 215, 255, 0.10), rgba(119, 243, 191, 0.08));
            border-color: rgba(86, 215, 255, 0.20);
        }
        .bench-top {
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            gap: 12px;
            margin-bottom: 10px;
        }
        .bench-top strong {
            font-size: 16px;
        }
        .bench-top span {
            font-size: 12px;
            color: var(--muted);
        }
        .metric-row {
            display: grid;
            grid-template-columns: repeat(4, minmax(0, 1fr));
            gap: 10px;
        }
        .metric {
            min-width: 0;
        }
        .metric em {
            display: block;
            font-style: normal;
            color: var(--muted);
            font-size: 11px;
            margin-bottom: 4px;
        }
        .metric strong {
            display: block;
            font-size: 15px;
            line-height: 1.15;
        }
        .bench-note {
            margin-top: 12px;
            color: var(--muted);
            font-size: 12px;
            line-height: 1.45;
        }
        .bench-source {
            margin-top: 12px;
            color: var(--gold);
            font-size: 11px;
            letter-spacing: 0.08em;
            text-transform: uppercase;
        }
        @media (max-width: 980px) {
            .page {
                width: min(100vw - 24px, 920px);
                height: min(100vh - 24px, 900px);
                margin: 12px auto;
                grid-template-columns: 1fr;
            }
            .hero {
                grid-template-rows: auto auto auto auto;
            }
        }
        @media (max-width: 640px) {
            .page {
                width: calc(100vw - 16px);
                height: calc(100vh - 16px);
                margin: 8px auto;
                padding: 10px;
                gap: 10px;
            }
            .hero,
            .panel {
                border-radius: 22px;
            }
            .hero {
                padding: 18px;
                gap: 14px;
            }
            .panel {
                padding: 16px;
            }
            .topbar {
                align-items: flex-start;
                flex-direction: column;
            }
            .logo {
                width: 62px;
                height: 62px;
                border-radius: 20px;
            }
            .logo::after {
                width: 31px;
                height: 31px;
                inset: 15px auto auto 15px;
                border-width: 6px;
                border-right-width: 8px;
            }
            .brand-title {
                font-size: 22px;
            }
            .hero-main h1 {
                font-size: 34px;
            }
            .hero-main p {
                font-size: 14px;
            }
            .protocols,
            .metric-row {
                grid-template-columns: repeat(2, minmax(0, 1fr));
            }
            .protocol {
                padding: 10px;
            }
            .bench-card {
                padding: 12px;
            }
            .bench-top {
                margin-bottom: 8px;
            }
            .bench-top strong {
                font-size: 15px;
            }
            .metric strong {
                font-size: 14px;
            }
        }
    </style>
</head>
<body>
    <main class="page">
        <section class="hero">
            <div class="topbar">
                <div class="brand">
                    <div class="logo" aria-hidden="true"></div>
                    <div class="brand-mark">
                        <div class="eyebrow">General Gateway</div>
                        <div class="brand-title">proxysss</div>
                        <div class="version">v__VERSION__</div>
                    </div>
                </div>
                <a class="docs-btn" href="https://github.com/neko233-com/proxysss#readme" target="_blank" rel="noreferrer">Docs</a>
            </div>
            <div class="hero-main">
                <h1>proxysss</h1>
                <p>通用网关与反向代理。YAML 负责主配置面，TypeScript 作为可选扩展脚本层。</p>
            </div>
            <div class="protocols" aria-label="supported protocols">
                <div class="protocol"><strong>HTTP/1.1</strong><span>Reverse proxy and static delivery</span></div>
                <div class="protocol"><strong>HTTP/2</strong><span>TLS termination and upstream proxying</span></div>
                <div class="protocol"><strong>HTTP/3</strong><span>QUIC listener on the public edge</span></div>
                <div class="protocol"><strong>WebSocket</strong><span>ws and wss upgrade tunneling</span></div>
                <div class="protocol"><strong>TCP</strong><span>Declarative stream listener routing</span></div>
                <div class="protocol"><strong>UDP</strong><span>YAML upstream and upstream pool support</span></div>
                <div class="protocol"><strong>FTP</strong><span>Control-channel passthrough routing</span></div>
                <div class="protocol"><strong>WebDAV</strong><span>Built-in file methods in Rust hot path</span></div>
            </div>
            <div class="notes">
                <div class="note"><i></i><span>Default public ports: 80 / 443</span></div>
                <div class="note"><i></i><span>Config-first runtime model</span></div>
                <div class="note"><i></i><span>Bundled script engine delivery</span></div>
            </div>
        </section>
        <section class="side">
            <article class="panel">
                <div class="panel-head">
                    <h2>Supported Protocols</h2>
                    <span>Edge and stream</span>
                </div>
                <div class="protocols">
                    <div class="protocol"><strong>HTTPS</strong><span>Self-signed, manual, or auto HTTPS</span></div>
                    <div class="protocol"><strong>WSS</strong><span>Secure upgrade with upstream tunneling</span></div>
                    <div class="protocol"><strong>TLS</strong><span>Gateway termination and certificate control</span></div>
                    <div class="protocol"><strong>Static</strong><span>Built-in site serving and autoindex</span></div>
                </div>
            </article>
            <article class="panel">
                <div class="panel-head">
                    <h2>Benchmark Snapshot</h2>
                    <span>Windows loopback, 2026-06-05</span>
                </div>
                <div class="bench-grid">
                    <section class="bench-card proxysss">
                        <div class="bench-top">
                            <strong>proxysss</strong>
                            <span>built-in static site runtime</span>
                        </div>
                        <div class="metric-row">
                            <div class="metric"><em>ops/sec</em><strong>13,272</strong></div>
                            <div class="metric"><em>success</em><strong>398,174</strong></div>
                            <div class="metric"><em>p95</em><strong>42.178 ms</strong></div>
                            <div class="metric"><em>p99</em><strong>44.934 ms</strong></div>
                        </div>
                    </section>
                    <section class="bench-card">
                        <div class="bench-top">
                            <strong>Caddy</strong>
                            <span>file-server</span>
                        </div>
                        <div class="metric-row">
                            <div class="metric"><em>ops/sec</em><strong>11,942</strong></div>
                            <div class="metric"><em>success</em><strong>358,264</strong></div>
                            <div class="metric"><em>p95</em><strong>50.181 ms</strong></div>
                            <div class="metric"><em>p99</em><strong>54.104 ms</strong></div>
                        </div>
                    </section>
                    <section class="bench-card">
                        <div class="bench-top">
                            <strong>nginx</strong>
                            <span>Windows static alias</span>
                        </div>
                        <div class="metric-row">
                            <div class="metric"><em>ops/sec</em><strong>483</strong></div>
                            <div class="metric"><em>errors</em><strong>1,748,575</strong></div>
                            <div class="metric"><em>p95</em><strong>70.809 ms</strong></div>
                            <div class="metric"><em>p99</em><strong>28,958 ms</strong></div>
                        </div>
                    </section>
                </div>
                <p class="bench-note">同一静态 HTML、同一 bench 客户端、concurrency=512、duration=30s。本页展示的是仓库 README 已记录的本地 Windows loopback 数据。</p>
                <div class="bench-source">README benchmark snapshot</div>
            </article>
        </section>
    </main>
</body>
</html>"#;

    html.replace("__VERSION__", env!("CARGO_PKG_VERSION"))
}

fn render_admin_console_html(config: &GatewayConfig) -> String {
    let html = r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>proxysss admin</title>
    <style>
        body { margin: 0; font-family: "Segoe UI", "PingFang SC", sans-serif; background: linear-gradient(160deg, #07111d, #0e1a2f); color: #eef5ff; }
        .wrap { max-width: 980px; margin: 0 auto; padding: 28px 20px 40px; }
        .panel { background: rgba(10, 23, 40, 0.82); border: 1px solid rgba(123, 176, 255, 0.16); border-radius: 22px; padding: 22px; margin-bottom: 18px; }
        h1, h2 { margin-top: 0; }
        input { width: 100%; padding: 12px 14px; margin-top: 8px; margin-bottom: 12px; border-radius: 12px; border: 1px solid rgba(255,255,255,0.12); background: rgba(255,255,255,0.06); color: #eef5ff; }
        button { padding: 12px 16px; border: 0; border-radius: 12px; cursor: pointer; font-weight: 700; background: linear-gradient(135deg, #56d7ff, #7cffb6); color: #04111a; }
        pre { background: rgba(2, 8, 18, 0.72); border-radius: 16px; padding: 16px; overflow: auto; min-height: 180px; }
        .muted { color: #9eb4d1; }
    </style>
</head>
<body>
    <div class="wrap">
        <div class="panel">
            <h1>proxysss admin console</h1>
            <p class="muted">Use the default credentials from your config to inspect stats, upstream state, and loaded plugins.</p>
        </div>
        <div class="panel">
            <h2>Quick Login</h2>
            <label>Username</label>
            <input id="username" value="__ADMIN_USER__" />
            <label>Password</label>
            <input id="password" type="password" value="__ADMIN_PASS__" />
            <button id="load">Load /v1/stats</button>
        </div>
        <div class="panel">
            <h2>Response</h2>
            <pre id="output">Click the button to query /v1/stats</pre>
        </div>
    </div>
    <script>
        const output = document.getElementById('output');
        document.getElementById('load').addEventListener('click', async () => {
            const user = document.getElementById('username').value;
            const pass = document.getElementById('password').value;
            const auth = 'Basic ' + btoa(user + ':' + pass);
            try {
                const response = await fetch('/v1/stats', { headers: { Authorization: auth } });
                output.textContent = await response.text();
            } catch (error) {
                output.textContent = String(error);
            }
        });
    </script>
</body>
</html>"#;

    html.replace("__ADMIN_USER__", &config.admin.username)
        .replace("__ADMIN_PASS__", &config.admin.password)
}

async fn connect_upgrade_upstream(url: &Url, allow_insecure: bool) -> Result<BoxedProxyIo> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("upstream URL missing host"))?
        .to_string();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow!("upstream URL missing port"))?;
    let tcp = TcpStream::connect((host.as_str(), port)).await?;

    if matches!(url.scheme(), "https" | "wss") {
        let client_config = if allow_insecure {
            ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(InsecureUpstreamVerifier))
                .with_no_client_auth()
        } else {
            return Err(anyhow!(
                "wss/https websocket upstreams require http.allow_insecure_upstreams=true in the current build"
            ));
        };
        let connector = TlsConnector::from(Arc::new(client_config));
        let server_name = rustls::pki_types::ServerName::try_from(host)
            .map_err(|_| anyhow!("invalid upstream tls server name"))?;
        let tls = connector.connect(server_name, tcp).await?;
        Ok(Box::new(tls))
    } else {
        Ok(Box::new(tcp))
    }
}

async fn read_http_response_head(
    upstream: &mut BoxedProxyIo,
) -> Result<(StatusCode, Vec<(HeaderName, HeaderValue)>, Option<Bytes>)> {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        if let Some(position) = find_header_end(&buffer) {
            let head = buffer.split_to(position + 4).freeze();
            let leftover = if buffer.is_empty() {
                None
            } else {
                Some(buffer.freeze())
            };
            let (status, headers) = parse_http_response_head(&head)?;
            return Ok((status, headers, leftover));
        }

        if buffer.len() > 64 * 1024 {
            return Err(anyhow!("upstream response headers exceeded 64KiB"));
        }

        let mut chunk = [0_u8; 4096];
        let read = upstream.read(&mut chunk).await?;
        if read == 0 {
            return Err(anyhow!("upstream closed during handshake"));
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

fn parse_http_response_head(head: &[u8]) -> Result<(StatusCode, Vec<(HeaderName, HeaderValue)>)> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    let status = response
        .parse(head)
        .context("failed parsing upstream handshake")?;
    if !matches!(status, httparse::Status::Complete(_)) {
        return Err(anyhow!("incomplete upstream handshake"));
    }

    let status = StatusCode::from_u16(
        response
            .code
            .ok_or_else(|| anyhow!("missing upstream status code"))?,
    )?;
    let mut parsed_headers = Vec::new();
    for header in response.headers.iter() {
        let name = HeaderName::from_bytes(header.name.as_bytes())?;
        let value = HeaderValue::from_bytes(header.value)?;
        parsed_headers.push((name, value));
    }
    Ok((status, parsed_headers))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn build_websocket_upstream_headers(
    original: &HeaderMap,
    route: &RouteDecision,
    upstream_host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
    original_host: &str,
) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    for (name, value) in original {
        if name == HOST || name.as_str().eq_ignore_ascii_case("proxy-connection") {
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
        let name = HeaderName::from_bytes(name.as_bytes())?;
        let value = HeaderValue::from_str(value)?;
        headers.insert(name, value);
    }

    headers.insert(HOST, HeaderValue::from_str(upstream_host)?);
    apply_forwarding_headers(&mut headers, original_host, remote_addr, scheme)?;

    Ok(headers)
}

fn serialize_http_request(
    method: &Method,
    url: &Url,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Vec<u8>> {
    let path_and_query = match url.query() {
        Some(query) => format!("{}?{}", url.path(), query),
        None => url.path().to_string(),
    };
    let mut request = format!("{} {} HTTP/1.1\r\n", method, path_and_query).into_bytes();

    for (name, value) in headers {
        request.extend_from_slice(name.as_str().as_bytes());
        request.extend_from_slice(b": ");
        request.extend_from_slice(value.as_bytes());
        request.extend_from_slice(b"\r\n");
    }

    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(body);
    Ok(request)
}

fn upstream_host_header(url: &Url) -> Result<String> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("upstream URL missing host"))?;
    match url.port() {
        Some(port)
            if !matches!(
                (url.scheme(), port),
                ("http", 80) | ("https", 443) | ("ws", 80) | ("wss", 443)
            ) =>
        {
            Ok(format!("{host}:{port}"))
        }
        _ => Ok(host.to_string()),
    }
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
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open certificate {}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse certificate pem")
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open private key {}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .context("failed to parse private key pem")?
        .ok_or_else(|| anyhow!("no private key found in {}", path.display()))
}

fn reload_fingerprint(config_path: &Path) -> Result<String> {
    let config = GatewayConfig::load(config_path)?;
    let mut context = md5::Context::new();
    context.consume(
        serde_json::to_vec(&config).context("failed serializing reload config fingerprint")?,
    );

    for path in watched_script_paths(&config) {
        context.consume(path.display().to_string().as_bytes());
        match std::fs::read(&path) {
            Ok(bytes) => context.consume(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                context.consume(b"missing");
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed reading script {}", path.display()));
            }
        }
    }

    Ok(format!("{:x}", context.compute()))
}

pub(crate) fn watched_script_paths(config: &GatewayConfig) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if config.script.enabled {
        let cwd = config
            .script
            .cwd
            .clone()
            .unwrap_or_else(|| config.root_dir.clone());

        if !config.script.entry.as_os_str().is_empty() {
            let entry = absolutize_script_path(&cwd, &config.script.entry);
            if is_script_file(&entry) {
                paths.push(entry);
            }
        }
    }

    if config.plugins.enabled && config.plugins.auto_load_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&config.plugins.auto_load_dir) {
            let extension_set: HashSet<String> = config
                .plugins
                .extensions
                .iter()
                .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
                .collect();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path
                        .extension()
                        .and_then(|value| value.to_str())
                        .map(|value| extension_set.contains(&value.to_ascii_lowercase()))
                        .unwrap_or(false)
                {
                    paths.push(path);
                    for sidecar in plugin_sidecar_paths(&entry.path()) {
                        if sidecar.exists() {
                            paths.push(sidecar);
                        }
                    }
                }
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

fn is_script_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "ts" | "js" | "mjs" | "cjs"
            )
        })
        .unwrap_or(false)
}

fn absolutize_script_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AdminConfig, HttpRateLimitConfig, RateLimitKey, ReverseProxyConfig,
        ReverseProxyRouteConfig, StaticSiteConfig, StreamAffinityConfig, WebDavConfig,
    };

    #[test]
    fn runtime_scope_key_contains_protocol_listener_and_upstream() {
        let key = runtime_scope_key("tcp", Some("tcp-affinity-demo"), "127.0.0.1:7001");
        assert_eq!(key, "tcp:tcp-affinity-demo:127.0.0.1:7001");
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
            status: None,
            content_type: None,
        };

        let candidates = normalize_candidates(&route);
        assert_eq!(
            candidates,
            vec!["127.0.0.1:7001".to_string(), "127.0.0.1:7002".to_string()]
        );
    }

    #[test]
    fn build_upstream_url_accepts_websocket_schemes() {
        let route = RouteDecision {
            upstream: "wss://chat.example.com/socket".to_string(),
            upstreams: Vec::new(),
            affinity_key: None,
            rewrite_path: None,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            status: None,
            content_type: None,
        };

        let uri: Uri = "/room?id=42".parse().expect("valid uri");
        let url = build_upstream_url(&route.upstream, &route, &uri).expect("valid websocket url");

        assert_eq!(url.scheme(), "wss");
        assert_eq!(url.as_str(), "wss://chat.example.com/room?id=42");
    }

    #[test]
    fn websocket_upgrade_detection_requires_upgrade_and_connection_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        headers.insert(
            "connection",
            HeaderValue::from_static("keep-alive, Upgrade"),
        );

        assert!(websocket_upgrade_requested(&headers));
    }

    #[test]
    fn reverse_proxy_route_matches_wildcard_host_and_strips_prefix() {
        let route = ReverseProxyRouteConfig {
            name: "api".to_string(),
            path_prefix: "/api".to_string(),
            hosts: vec!["*.example.com".to_string()],
            upstream: "http://127.0.0.1:8080".to_string(),
            upstreams: Vec::new(),
            strip_prefix: true,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
        };
        let uri: Uri = "/api/v1/users?q=1".parse().expect("valid uri");

        assert!(reverse_proxy_route_matches(
            &route,
            "edge.example.com:80",
            uri.path()
        ));
        let decision = reverse_proxy_route_decision(&route, &uri);
        assert_eq!(decision.rewrite_path.as_deref(), Some("/v1/users?q=1"));
    }

    #[test]
    fn configured_reverse_proxy_uses_longest_path_prefix() {
        let mut config = GatewayConfig::default();
        config.services.reverse_proxy = ReverseProxyConfig {
            routes: vec![
                ReverseProxyRouteConfig {
                    name: "root-api".to_string(),
                    path_prefix: "/api".to_string(),
                    hosts: Vec::new(),
                    upstream: "http://127.0.0.1:8080".to_string(),
                    upstreams: Vec::new(),
                    strip_prefix: false,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                },
                ReverseProxyRouteConfig {
                    name: "admin-api".to_string(),
                    path_prefix: "/api/admin".to_string(),
                    hosts: Vec::new(),
                    upstream: "http://127.0.0.1:9090".to_string(),
                    upstreams: Vec::new(),
                    strip_prefix: false,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                },
            ],
        };

        let uri: Uri = "/api/admin/users".parse().expect("valid uri");
        let decision =
            configured_reverse_proxy_route(&config, "example.com", &uri).expect("matched route");
        assert_eq!(decision.decision.upstream, "http://127.0.0.1:9090");
    }

    #[test]
    fn configured_domain_route_matches_host_and_enables_features() {
        let mut config = GatewayConfig::default();
        config.services.domain_routes.push(DomainRouteConfig {
            name: "app".to_string(),
            domains: vec!["example.com".to_string()],
            path_prefix: "/".to_string(),
            upstream: "http://127.0.0.1:9000".to_string(),
            upstreams: vec!["http://127.0.0.1:9001".to_string()],
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            compression: ResponseCompressionConfig {
                enabled: true,
                min_length: 128,
                content_types: vec!["application/json".to_string()],
            },
            cache: ResponseCacheConfig {
                enabled: true,
                ttl_secs: 5,
                statuses: vec![200],
                max_body_bytes: 4096,
            },
            ssl: crate::config::DomainTlsConfig::default(),
        });

        let uri: Uri = "/".parse().expect("valid uri");
        let route = configured_http_route(&config, "example.com", &uri).expect("matched route");
        assert_eq!(route.decision.upstream, "http://127.0.0.1:9000");
        assert!(route.compression.enabled);
        assert!(route.cache.enabled);
    }

    #[test]
    fn finalize_http_response_gzips_compressible_payloads() {
        let mut request_headers = HeaderMap::new();
        request_headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, br"));
        let response = GatewayHttpResponse::bytes(
            StatusCode::OK,
            "application/json",
            Bytes::from(vec![b'a'; 2048]),
            "http://127.0.0.1:9000",
        );
        let compression = ResponseCompressionConfig {
            enabled: true,
            min_length: 128,
            content_types: vec!["application/json".to_string()],
        };

        let response = finalize_http_response(&request_headers, &compression, response)
            .expect("finalize response");
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == CONTENT_ENCODING && value == "gzip"));
    }

    #[test]
    fn listener_specs_include_ftp_when_enabled() {
        let mut config = GatewayConfig::default();
        config.services.ftp.enabled = true;

        let keys = listener_specs(&config)
            .into_iter()
            .map(|spec| spec.key())
            .collect::<Vec<_>>();

        assert!(keys.iter().any(|key| key.starts_with("tcp:ftp:")));
    }

    #[test]
    fn http_rate_limit_key_can_use_header() {
        let config = HttpRateLimitConfig {
            enabled: true,
            key: RateLimitKey::Header("x-api-key".to_string()),
            requests: 1,
            window_ms: 1000,
            burst: 0,
            status: 429,
        };
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("abc"));
        let remote: SocketAddr = "127.0.0.1:12345".parse().expect("remote addr");

        let key = http_rate_limit_key(&config, "example.com", &headers, remote);
        assert_eq!(key.as_deref(), Some("header:x-api-key:abc"));
    }

    #[test]
    fn build_upstream_headers_appends_forwarding_chain() {
        let route = RouteDecision {
            upstream: "http://127.0.0.1:8080".to_string(),
            upstreams: Vec::new(),
            affinity_key: None,
            rewrite_path: None,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            status: None,
            content_type: None,
        };
        let mut original = HeaderMap::new();
        original.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.10, 198.51.100.11"),
        );
        original.insert(
            "forwarded",
            HeaderValue::from_static("for=198.51.100.10;proto=https"),
        );

        let headers = build_upstream_headers(
            &original,
            &route,
            "api.example.com",
            "203.0.113.20:443".parse().expect("remote addr"),
            "https",
        )
        .expect("headers");

        assert_eq!(
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok()),
            Some("198.51.100.10, 198.51.100.11, 203.0.113.20")
        );
        assert_eq!(
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok()),
            Some("203.0.113.20")
        );
        assert_eq!(
            headers
                .get("x-forwarded-host")
                .and_then(|value| value.to_str().ok()),
            Some("api.example.com")
        );
        assert_eq!(
            headers
                .get("x-forwarded-proto")
                .and_then(|value| value.to_str().ok()),
            Some("https")
        );
        assert_eq!(
            headers.get("forwarded").and_then(|value| value.to_str().ok()),
            Some(
                "for=198.51.100.10;proto=https, for=203.0.113.20;host=\"api.example.com\";proto=https"
            )
        );
    }

    #[test]
    fn http_rate_limit_blocks_after_limit() {
        let config = HttpRateLimitConfig {
            enabled: true,
            key: RateLimitKey::RemoteAddr,
            requests: 1,
            window_ms: 60_000,
            burst: 0,
            status: 429,
        };
        let store = DashMap::new();

        assert!(
            apply_http_rate_limit_to_store(&store, &config, "remote:127.0.0.1".to_string())
                .is_none()
        );
        let retry_after =
            apply_http_rate_limit_to_store(&store, &config, "remote:127.0.0.1".to_string())
                .expect("second request blocked");
        assert!(retry_after.parse::<u64>().expect("retry-after number") > 0);
    }

    #[test]
    fn webdav_path_mapping_rejects_traversal() {
        let config = WebDavConfig {
            enabled: true,
            path_prefix: "/dav".to_string(),
            root: PathBuf::from("/tmp/webdav-root"),
            allow_write: true,
        };

        let error = webdav_filesystem_path(&config, "/dav/%2e%2e/secret")
            .expect_err("traversal must be rejected");
        assert!(error.to_string().contains("escapes root"));
    }

    #[test]
    fn webdav_path_mapping_decodes_safe_paths() {
        let config = WebDavConfig {
            enabled: true,
            path_prefix: "/dav".to_string(),
            root: PathBuf::from("/tmp/webdav-root"),
            allow_write: true,
        };

        let target = webdav_filesystem_path(&config, "/dav/folder/a%20b.txt")
            .expect("path should map")
            .expect("path should match prefix");

        assert!(target.ends_with(Path::new("folder").join("a b.txt")));
    }

    #[tokio::test]
    async fn webdav_put_get_delete_roundtrip() {
        let root = std::env::temp_dir().join(format!("proxysss-webdav-test-{}", Uuid::new_v4()));
        let config = WebDavConfig {
            enabled: true,
            path_prefix: "/dav".to_string(),
            root: root.clone(),
            allow_write: true,
        };
        let uri: Uri = "/dav/hello.txt".parse().expect("valid uri");
        let headers = HeaderMap::new();

        let put = dispatch_webdav(
            &config,
            &Method::PUT,
            &uri,
            &headers,
            Bytes::from_static(b"hello webdav"),
        )
        .await
        .expect("put succeeds");
        assert_eq!(put.status, StatusCode::CREATED);

        let get = dispatch_webdav(&config, &Method::GET, &uri, &headers, Bytes::new())
            .await
            .expect("get succeeds");
        assert_eq!(get.status, StatusCode::OK);
        assert_eq!(get.body, Bytes::from_static(b"hello webdav"));

        let delete = dispatch_webdav(&config, &Method::DELETE, &uri, &headers, Bytes::new())
            .await
            .expect("delete succeeds");
        assert_eq!(delete.status, StatusCode::NO_CONTENT);

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn webdav_propfind_lists_collection() {
        let root =
            std::env::temp_dir().join(format!("proxysss-webdav-propfind-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create webdav root");
        tokio::fs::write(root.join("item.txt"), b"item")
            .await
            .expect("write child");

        let config = WebDavConfig {
            enabled: true,
            path_prefix: "/dav".to_string(),
            root: root.clone(),
            allow_write: true,
        };
        let uri: Uri = "/dav".parse().expect("valid uri");
        let response = dispatch_webdav(
            &config,
            &Method::from_bytes(b"PROPFIND").unwrap(),
            &uri,
            &HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("propfind succeeds");

        assert_eq!(response.status, StatusCode::from_u16(207).unwrap());
        let body = String::from_utf8(response.body.to_vec()).expect("utf8 body");
        assert!(body.contains("item.txt"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[test]
    fn static_path_mapping_rejects_traversal() {
        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/assets".to_string(),
            root: PathBuf::from("/tmp/static-root"),
            index_files: vec!["index.html".to_string()],
            autoindex: false,
        };

        let error = static_site_filesystem_path(&site, "/assets/%2e%2e/secret")
            .expect_err("traversal must be rejected");
        assert!(error.to_string().contains("escapes root"));
    }

    #[tokio::test]
    async fn static_site_serves_index_file() {
        let root = std::env::temp_dir().join(format!("proxysss-static-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create static root");
        tokio::fs::write(root.join("index.html"), b"<h1>ok</h1>")
            .await
            .expect("write index");

        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/assets".to_string(),
            root: root.clone(),
            index_files: vec!["index.html".to_string()],
            autoindex: false,
        };
        let uri: Uri = "/assets".parse().expect("valid uri");
        let response = dispatch_static_site(&site, &Method::GET, &uri)
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, Bytes::from_static(b"<h1>ok</h1>"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn static_site_autoindex_lists_directory() {
        let root =
            std::env::temp_dir().join(format!("proxysss-static-autoindex-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create static root");
        tokio::fs::write(root.join("item.txt"), b"item")
            .await
            .expect("write item");

        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/assets".to_string(),
            root: root.clone(),
            index_files: vec!["index.html".to_string()],
            autoindex: true,
        };
        let uri: Uri = "/assets".parse().expect("valid uri");
        let response = dispatch_static_site(&site, &Method::GET, &uri)
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::OK);
        let body = String::from_utf8(response.body.to_vec()).expect("utf8 body");
        assert!(body.contains("item.txt"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[test]
    fn watched_script_paths_include_main_script_and_plugins() {
        let root = std::env::temp_dir().join(format!("proxysss-watch-test-{}", Uuid::new_v4()));
        let plugins = root.join("plugins");
        std::fs::create_dir_all(&plugins).expect("create plugin dir");
        std::fs::write(root.join("gateway.ts"), "// gateway").expect("write gateway");
        std::fs::write(plugins.join("traffic-stats.ts"), "// plugin").expect("write plugin");
        std::fs::write(
            plugins.join("traffic-stats.plugin.yaml"),
            "enabled: true\npriority: 220\nconfig:\n  mode: sample\n",
        )
        .expect("write plugin sidecar");

        let config = GatewayConfig {
            root_dir: root.clone(),
            script: crate::config::ScriptConfig {
                enabled: true,
                entry: PathBuf::from("gateway.ts"),
                cwd: Some(root.clone()),
                ..crate::config::ScriptConfig::default()
            },
            plugins: crate::config::PluginsConfig {
                enabled: true,
                auto_load_dir: plugins.clone(),
                ..crate::config::PluginsConfig::default()
            },
            ..GatewayConfig::default()
        };

        let paths = watched_script_paths(&config);
        assert!(paths.contains(&root.join("gateway.ts")));
        assert!(paths.contains(&plugins.join("traffic-stats.ts")));
        assert!(paths.contains(&plugins.join("traffic-stats.plugin.yaml")));
    }

    #[test]
    fn auto_load_plugin_spec_reads_sidecar_metadata() {
        let root =
            std::env::temp_dir().join(format!("proxysss-plugin-spec-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let plugin = root.join("geo-headers.ts");
        std::fs::write(&plugin, "// plugin").expect("write plugin");
        std::fs::write(
            root.join("geo-headers.plugin.json"),
            r#"{"enabled":true,"priority":180,"config":{"mode":"geo_headers","header_prefix":"proxysss-"}}"#,
        )
        .expect("write sidecar");

        let spec = load_auto_plugin_spec(&plugin).expect("load sidecar metadata");
        assert_eq!(spec.name, "geo-headers");
        assert_eq!(spec.priority, Some(180));
        assert_eq!(spec.enabled, Some(true));
        assert_eq!(spec.config["mode"], "geo_headers");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn builtin_http_route_serves_welcome_on_root() {
        let route = builtin_http_route("/").expect("root route");
        assert_eq!(route.upstream, "proxysss://welcome");
        assert_eq!(
            builtin_http_route("/echo").map(|route| route.upstream),
            None
        );
    }

    #[test]
    fn welcome_page_stays_brand_focused() {
        let config = GatewayConfig::default();
        let html = render_welcome_html(&config);
        assert!(html.contains("<h1>proxysss</h1>"));
        assert!(html.contains("Supported Protocols"));
        assert!(html.contains("Benchmark Snapshot"));
        assert!(html.contains("https://github.com/neko233-com/proxysss#readme"));
        assert!(!html.contains("127.0.0.1:7777"));
        assert!(!html.contains("Open Admin Console"));
        assert!(html.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn reload_fingerprint_changes_when_main_script_changes() {
        let root =
            std::env::temp_dir().join(format!("proxysss-fingerprint-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let config_path = root.join("proxysss.yaml");
        let script_path = root.join("gateway.ts");

        std::fs::write(&script_path, "console.log('v1');").expect("write script v1");
        std::fs::write(
            &config_path,
            format!(
                "script:\n  enabled: true\n  cwd: {}\nplugins:\n  enabled: false\n",
                root.display().to_string().replace('\\', "/")
            ),
        )
        .expect("write config");

        let before = reload_fingerprint(&config_path).expect("fingerprint before");
        std::fs::write(&script_path, "console.log('v2');").expect("write script v2");
        let after = reload_fingerprint(&config_path).expect("fingerprint after");

        assert_eq!(before.len(), 32);
        assert_eq!(after.len(), 32);
        assert_ne!(before, after);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn build_dynamic_state_starts_embedded_script_engine() {
        let mut config = GatewayConfig::default();
        config.script.enabled = true;
        config.script.entry = PathBuf::from("gateway.ts");
        config.plugins.enabled = false;

        // The embedded engine starts even when the entry script is missing; the
        // gateway simply falls back to native/YAML routing for unmatched paths.
        let state = build_dynamic_state(config)
            .await
            .expect("dynamic state with embedded script engine");
        assert!(state.script.is_some());
    }

    #[test]
    fn listener_specs_reflect_hot_reloadable_binds() {
        let mut config = GatewayConfig::default();
        config.http.plain_bind = "127.0.0.1:7000".to_string();
        config.http.tls_bind.clear();
        config.http.h3_bind.clear();
        config.admin.bind = "127.0.0.1:7001".to_string();

        let keys = listener_specs(&config)
            .into_iter()
            .map(|spec| spec.key())
            .collect::<Vec<_>>();

        assert!(keys.contains(&"http:127.0.0.1:7000".to_string()));
        assert!(keys.contains(&"admin:127.0.0.1:7001".to_string()));
    }

    #[test]
    fn configured_stream_listener_route_uses_yaml_upstreams() {
        let mut config = GatewayConfig::default();
        config.tcp.listeners.push(TcpListenerConfig {
            name: "game-tcp".to_string(),
            bind: "0.0.0.0:7000".to_string(),
            upstream: "127.0.0.1:9000".to_string(),
            upstreams: vec!["127.0.0.1:9001".to_string()],
        });
        config.udp.listeners.push(UdpListenerConfig {
            name: "game-udp".to_string(),
            bind: "0.0.0.0:7001".to_string(),
            upstream: String::new(),
            upstreams: vec!["127.0.0.1:9100".to_string(), "127.0.0.1:9101".to_string()],
        });

        let tcp = configured_tcp_listener_route(&config, "game-tcp", Some("pid-1".to_string()))
            .expect("tcp route");
        assert_eq!(tcp.upstream, "127.0.0.1:9000");
        assert_eq!(tcp.upstreams[0], "127.0.0.1:9000");
        assert_eq!(tcp.affinity_key.as_deref(), Some("pid-1"));

        let udp = configured_udp_listener_route(&config, "game-udp", None).expect("udp route");
        assert_eq!(udp.upstream, "127.0.0.1:9100");
        assert_eq!(udp.upstreams.len(), 2);
    }

    #[test]
    fn monitoring_path_match_respects_enabled_flag() {
        let mut config = crate::config::MonitoringConfig {
            path: "/internal-metrics".to_string(),
            ..Default::default()
        };
        assert!(monitoring_path_matches(&config, "/internal-metrics"));
        assert!(!monitoring_path_matches(&config, "/metrics"));
        config.enabled = false;
        assert!(!monitoring_path_matches(&config, "/internal-metrics"));
    }
}
