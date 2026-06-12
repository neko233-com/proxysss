use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashSet};
use std::convert::Infallible;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Cursor};
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use brotli::CompressorWriter;
use bytes::{Buf, Bytes, BytesMut};
use dashmap::DashMap;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::TryStreamExt;
use h3::server::Connection as H3Connection;
use hmac::{Hmac, Mac};
use http::header::{
    ACCEPT_ENCODING, AUTHORIZATION, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE,
    COOKIE, HOST, LOCATION, SET_COOKIE, VARY,
};
use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri, Version,
};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use instant_acme::{
    Account, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder, OrderStatus, RetryPolicy,
};
use quinn::crypto::rustls::QuicServerConfig;
use rcgen::{CertificateParams, CustomExtension, DistinguishedName, KeyPair};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::UnixTime;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ClientConfig;
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::io::{
    copy_bidirectional, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
    BufReader as TokioBufReader,
};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use url::Url;
use uuid::Uuid;
use zstd::stream::encode_all as zstd_encode_all;

use crate::acme::{acme_challenge_fqdn, DnsProvider};
use crate::config::{
    on_demand_domain_allowed, AcmeChallengeType, ActiveHealthConfig, ActiveHealthOverrideConfig,
    AdminConfig, CacheBehavior, CompressionAlgorithm, DomainRouteConfig, DomainTlsMode,
    FileCloudConfig, FtpUserPolicy, GatewayConfig, HttpAccessControlConfig, HttpAffinityConfig,
    HttpRateLimitConfig, LoadBalanceAlgorithm, MonitoringFormat, OnDemandTlsConfig,
    RateLimitAlgorithm, RateLimitKey, ResponseCacheConfig, ResponseCompressionConfig,
    ReverseProxyRouteConfig, StaticSiteConfig, StreamAffinityConfig, StreamRateLimitConfig,
    StreamRouteConfig, TcpListenerConfig, TlsCertificateConfig, TlsMode, UdpListenerConfig,
    WebDavConfig,
};
use crate::install;
use crate::script::{HttpContext, RouteDecision, ScriptPluginSpec, ScriptRuntime, StreamContext};
use crate::security::{
    self, admin_loopback_only_allows, ip_access_is_denied, stream_access_is_denied,
    validate_domain_route_mutation, validate_reverse_proxy_route_mutation,
    validate_tcp_listener_mutation, validate_udp_listener_mutation, AdminAuthGuard, DdosGuard,
    DynamicBlacklist,
};
use crate::stream_routes::{parse_tls_client_hello_sni, StreamRouteTable};

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
    stream_rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    http_connection_limits: Arc<DashMap<String, u32>>,
    http_cache: Arc<DashMap<String, CachedHttpEntry>>,
    acme_http_challenges: Arc<DashMap<String, String>>,
    acme_tls_alpn_certs: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_certs: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_trigger: tokio::sync::mpsc::UnboundedSender<String>,
    on_demand_issue_counts: Arc<DashMap<String, u32>>,
    ddos_guard: DdosGuard,
    dynamic_blacklist: DynamicBlacklist,
    ftp_session_users: Arc<DashMap<SocketAddr, String>>,
    admin_auth_guard: AdminAuthGuard,
}

struct DynamicState {
    config: GatewayConfig,
    http_client: reqwest::Client,
    script: Option<Arc<ScriptRuntime>>,
}

pub(crate) struct GatewayHttpResponse {
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
    stream_body: Option<GatewayBody>,
    upstream: String,
}

type GatewayBody = UnsyncBoxBody<Bytes, anyhow::Error>;
type GatewayResponse = Response<GatewayBody>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompressionEncoding {
    Zstd,
    Brotli,
    Gzip,
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
    manually_disabled: bool,
    manual_reason: Option<String>,
    manual_changed_at_unix_ms: Option<u64>,
    active_probe_kind: Option<String>,
    active_probe_failure_count: u32,
    active_probe_success_count: u32,
    active_probe_healthy: Option<bool>,
    active_probe_status: Option<u16>,
    active_probe_error: Option<String>,
    active_probe_checked_at_unix_ms: Option<u64>,
    active_probe_rtt_ms: Option<u64>,
}

#[derive(Clone)]
struct RateLimitBucket {
    window_start: Instant,
    count: u32,
    tokens: f64,
    last_refill: Instant,
}

#[derive(Clone)]
struct CachedHttpEntry {
    expires_at_unix_ms: u64,
    stale_until_unix_ms: u64,
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
    upstream: String,
}

struct HttpCacheRevalidateRequest<'a> {
    host: &'a str,
    uri: &'a Uri,
    headers: &'a HeaderMap,
    remote_addr: SocketAddr,
    scheme: &'a str,
}

#[derive(Serialize, Deserialize)]
struct DiskCachedHttpEntry {
    expires_at_unix_ms: u64,
    #[serde(default)]
    stale_until_unix_ms: u64,
    status: u16,
    headers: Vec<(String, String)>,
    body_base64: String,
    upstream: String,
}

enum CacheLookup {
    Fresh(GatewayHttpResponse),
    Stale(GatewayHttpResponse),
    StaleIfError(GatewayHttpResponse),
}

#[derive(Clone)]
struct HttpRouteConfig {
    runtime_scope: Option<String>,
    decision: RouteDecision,
    compression: ResponseCompressionConfig,
    cache: ResponseCacheConfig,
    rate_limit: HttpRateLimitConfig,
}

#[derive(Clone)]
struct ResolvedActiveHealthConfig {
    enabled: bool,
    path: String,
    timeout_ms: u64,
    expected_statuses: Vec<u16>,
    failure_threshold: u32,
    success_threshold: u32,
    jitter_percent: u8,
    alert_webhooks: Vec<String>,
}

#[derive(Debug)]
struct SniResolver {
    default: Arc<CertifiedKey>,
    by_name: BTreeMap<String, Arc<CertifiedKey>>,
    acme_tls_alpn_by_name: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_certs: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand: OnDemandTlsConfig,
    on_demand_trigger: tokio::sync::mpsc::UnboundedSender<String>,
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
    blocked_requests_total: AtomicU64,
    ddos_bans_total: AtomicU64,
    process_metrics: Mutex<ProcessMetricsSampler>,
}

#[derive(Default)]
struct ProcessMetricsSampler {
    previous: Option<ProcessMetricsSample>,
}

#[derive(Clone, Copy)]
struct ProcessMetricsSample {
    wall: Instant,
    cpu_time_secs: f64,
}

struct ProcessMetricsSnapshot {
    pid: u32,
    cpu_percent: Option<f64>,
    memory_bytes: Option<u64>,
    memory_percent: Option<f64>,
}

struct UpstreamLease {
    runtime: Arc<DashMap<String, UpstreamRuntimeState>>,
    key: String,
}

struct HttpRateLimitLease {
    store: Arc<DashMap<String, u32>>,
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
        let Some(server_name) = client_hello.server_name() else {
            return Some(self.default.clone());
        };
        let server_name = server_name.to_ascii_lowercase();
        let is_acme_tls_alpn = client_hello
            .alpn()
            .map(|protocols| {
                protocols
                    .into_iter()
                    .any(|protocol| protocol == b"acme-tls/1")
            })
            .unwrap_or(false);
        if is_acme_tls_alpn {
            if let Some(certified) = self.acme_tls_alpn_by_name.get(&server_name) {
                return Some(certified.clone());
            }
        }
        if let Some(certified) = self.by_name.get(&server_name) {
            return Some(certified.clone());
        }
        if let Some(certified) = self.on_demand_certs.get(&server_name) {
            return Some(certified.clone());
        }

        if on_demand_domain_allowed(&self.on_demand, &server_name) {
            let _ = self.on_demand_trigger.send(server_name.clone());
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

impl Drop for HttpRateLimitLease {
    fn drop(&mut self) {
        if let Some(mut entry) = self.store.get_mut(&self.key) {
            if *entry <= 1 {
                drop(entry);
                self.store.remove(&self.key);
            } else {
                *entry = entry.saturating_sub(1);
            }
        }
    }
}

impl Gateway {
    pub async fn from_config(config_path: PathBuf, config: GatewayConfig) -> Result<Arc<Self>> {
        let acme_http_challenges = Arc::new(DashMap::new());
        let acme_tls_alpn_certs = Arc::new(DashMap::new());

        if config.uses_managed_dns01()
            && (!config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists())
        {
            issue_managed_acme_certificate(
                &config.http.tls,
                &acme_http_challenges,
                &acme_tls_alpn_certs,
            )
            .await
            .context("failed to issue initial managed DNS-01 certificate")?;
        }

        prepare_tls_material(&config)?;

        let dynamic = Arc::new(build_dynamic_state(config.clone()).await?);
        let (on_demand_trigger, on_demand_rx) = tokio::sync::mpsc::unbounded_channel();
        let dynamic_blacklist =
            DynamicBlacklist::load_from_disk(&config.security.dynamic_blacklist);

        let gateway = Arc::new(Self {
            config_path,
            bootstrap_config: config,
            dynamic: Arc::new(RwLock::new(dynamic)),
            stats: Arc::new(GatewayStats::default()),
            sticky_affinity: Arc::new(DashMap::new()),
            round_robin_state: Arc::new(DashMap::new()),
            upstream_runtime: Arc::new(DashMap::new()),
            http_rate_limits: Arc::new(DashMap::new()),
            stream_rate_limits: Arc::new(DashMap::new()),
            http_connection_limits: Arc::new(DashMap::new()),
            http_cache: Arc::new(DashMap::new()),
            acme_http_challenges,
            acme_tls_alpn_certs,
            on_demand_certs: Arc::new(DashMap::new()),
            on_demand_trigger,
            on_demand_issue_counts: Arc::new(DashMap::new()),
            ddos_guard: DdosGuard::default(),
            dynamic_blacklist,
            ftp_session_users: Arc::new(DashMap::new()),
            admin_auth_guard: AdminAuthGuard::default(),
        });
        gateway.spawn_on_demand_tls_worker(on_demand_rx);
        gateway.load_persisted_manual_upstream_state(&gateway.bootstrap_config)?;
        Ok(gateway)
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut tasks = JoinSet::new();

        if self.bootstrap_config.runtime.hot_reload.enabled {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_hot_reload_loop().await });
        }

        match self.bootstrap_config.http.tls.mode {
            TlsMode::AcmeManaged => {
                let gateway = self.clone();
                tasks.spawn(async move { gateway.run_managed_acme_renew_loop().await });
            }
            TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal => {
                let gateway = self.clone();
                tasks.spawn(async move { gateway.run_acme_renew_loop().await });
            }
            TlsMode::SelfSigned | TlsMode::Manual => {}
        }

        let gateway = self.clone();
        tasks.spawn(async move { gateway.run_listener_supervisor().await });

        let gateway = self.clone();
        tasks.spawn(async move { gateway.run_active_health_loop().await });

        if self.bootstrap_config.http.tls.on_demand.enabled {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_on_demand_tls_cleanup_loop().await });
        }

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

    async fn run_active_health_loop(self: Arc<Self>) -> Result<()> {
        loop {
            let state = self.current_state().await;
            let config = state.config.clone();
            let health = config.load_balance.active_health.clone();
            let client = state.http_client.clone();
            drop(state);

            let sleep_for = if health.enabled {
                self.run_active_health_pass(&config, &client).await;
                Duration::from_secs(health.interval_secs.max(1))
            } else {
                Duration::from_secs(1)
            };

            tokio::time::sleep(sleep_for).await;
        }
    }

    async fn run_acme_renew_loop(self: Arc<Self>) -> Result<()> {
        loop {
            let tls = {
                let state = self.current_state().await;
                state.config.http.tls.clone()
            };
            let renew_every = Duration::from_secs(tls.acme.renew_interval_hours.max(1) * 3600);
            tokio::time::sleep(renew_every).await;
            let tls = {
                let state = self.current_state().await;
                state.config.http.tls.clone()
            };
            let renew_result =
                tokio::task::spawn_blocking(move || run_acme_command(&tls, true)).await;

            match renew_result {
                Ok(Ok(())) => tracing::info!("acme renewal succeeded"),
                Ok(Err(error)) => tracing::warn!(?error, "acme renewal failed"),
                Err(error) => tracing::warn!(?error, "acme renewal task join failed"),
            }
        }
    }

    async fn run_managed_acme_renew_loop(self: Arc<Self>) -> Result<()> {
        loop {
            let tls = {
                let state = self.current_state().await;
                state.config.http.tls.clone()
            };
            match issue_managed_acme_certificate(
                &tls,
                &self.acme_http_challenges,
                &self.acme_tls_alpn_certs,
            )
            .await
            {
                Ok(()) => {
                    if let Err(error) = self.reload_from_disk().await {
                        tracing::warn!(?error, "managed acme issued certificate but reload failed");
                    } else {
                        tracing::info!("managed acme certificate refreshed");
                    }
                }
                Err(error) => tracing::warn!(?error, "managed acme renewal failed"),
            }

            let tls = {
                let state = self.current_state().await;
                state.config.http.tls.clone()
            };
            let renew_every = Duration::from_secs(tls.acme.renew_interval_hours.max(1) * 3600);
            tokio::time::sleep(renew_every).await;
        }
    }

    fn spawn_on_demand_tls_worker(
        self: &Arc<Self>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<String>,
    ) {
        let gateway = self.clone();
        tokio::spawn(async move {
            while let Some(domain) = rx.recv().await {
                if gateway.on_demand_certs.contains_key(&domain) {
                    continue;
                }
                let state = gateway.current_state().await;
                let on_demand = state.config.http.tls.on_demand.clone();
                if gateway.on_demand_certs.len() >= on_demand.max_active_certs {
                    tracing::warn!(%domain, "on-demand tls cert pool full");
                    continue;
                }
                let hour_key = format!(
                    "{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        / 3600
                );
                let mut issued = gateway.on_demand_issue_counts.entry(hour_key).or_insert(0);
                if *issued >= on_demand.max_issues_per_hour {
                    tracing::warn!(%domain, "on-demand tls hourly rate limit reached");
                    continue;
                }
                *issued = issued.saturating_add(1);
                drop(issued);

                if !on_demand.ask_url.trim().is_empty() {
                    let ask_url = on_demand.ask_url.replace("{domain}", &domain);
                    let client = state.http_client.clone();
                    match client.get(&ask_url).send().await {
                        Ok(response) if response.status().is_success() => {}
                        Ok(response) => {
                            tracing::info!(%domain, status = %response.status(), "on-demand tls ask denied");
                            continue;
                        }
                        Err(error) => {
                            tracing::warn!(?error, %domain, "on-demand tls ask request failed");
                            continue;
                        }
                    }
                }

                let tls = state.config.http.tls.clone();
                match issue_on_demand_managed_certificate(
                    &tls,
                    &domain,
                    &gateway.acme_http_challenges,
                    &gateway.acme_tls_alpn_certs,
                    &gateway.on_demand_certs,
                )
                .await
                {
                    Ok(()) => tracing::info!(%domain, "on-demand tls certificate issued"),
                    Err(error) => tracing::warn!(?error, %domain, "on-demand tls issuance failed"),
                }
            }
        });
    }

    async fn run_on_demand_tls_cleanup_loop(self: Arc<Self>) -> Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            let max = self
                .current_state()
                .await
                .config
                .http
                .tls
                .on_demand
                .max_active_certs;
            if self.on_demand_certs.len() > max {
                let overflow = self.on_demand_certs.len().saturating_sub(max);
                for key in self
                    .on_demand_certs
                    .iter()
                    .take(overflow)
                    .map(|entry| entry.key().clone())
                    .collect::<Vec<_>>()
                {
                    self.on_demand_certs.remove(&key);
                }
            }
        }
    }

    fn is_stream_connection_blocked(
        &self,
        config: &GatewayConfig,
        remote_addr: SocketAddr,
    ) -> bool {
        if self.dynamic_blacklist.is_blocked(remote_addr.ip()) {
            return true;
        }
        if self
            .ddos_guard
            .check_and_record(remote_addr.ip(), &config.security.ddos)
            .is_some()
        {
            self.stats.ddos_bans_total.fetch_add(1, Ordering::Relaxed);
            self.stats
                .blocked_requests_total
                .fetch_add(1, Ordering::Relaxed);
            self.dynamic_blacklist
                .add(remote_addr.ip(), config.security.ddos.ban_secs.max(1));
            return true;
        }
        stream_access_is_denied(&config.services.access_control.stream, remote_addr.ip()).is_some()
    }

    fn is_http_connection_blocked(&self, config: &GatewayConfig, remote_addr: SocketAddr) -> bool {
        if self.dynamic_blacklist.is_blocked(remote_addr.ip()) {
            return true;
        }
        if self
            .ddos_guard
            .check_and_record(remote_addr.ip(), &config.security.ddos)
            .is_some()
        {
            self.stats.ddos_bans_total.fetch_add(1, Ordering::Relaxed);
            self.dynamic_blacklist
                .add(remote_addr.ip(), config.security.ddos.ban_secs.max(1));
            return true;
        }
        false
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
                    async move {
                        gateway
                            .handle_admin_request(request, remote_addr, AdminTransport::Loopback)
                            .await
                    }
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
        transport: AdminTransport,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let headers = request.headers().clone();
        let body = if method == Method::GET || method == Method::HEAD {
            Bytes::new()
        } else {
            match request.body_mut().collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid body: {error}")}),
                    ));
                }
            }
        };
        self.serve_admin_api(method, path, headers, body, remote_addr, transport)
            .await
    }

    async fn serve_admin_api(
        &self,
        method: Method,
        path: String,
        headers: HeaderMap,
        body: Bytes,
        remote_addr: SocketAddr,
        transport: AdminTransport,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        self.stats
            .admin_requests_total
            .fetch_add(1, Ordering::Relaxed);

        if path == "/healthz" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "service": "proxysss", "remote_addr": remote_addr.to_string()}),
            ));
        }

        let state = self.current_state().await;

        if let Some(response) = check_admin_transport_access(&state.config, &transport, remote_addr)
        {
            return Ok(response);
        }

        if method == Method::GET && (path == "/" || path == "/index.html") {
            return Ok(html_response(
                StatusCode::OK,
                render_admin_console_html(&state.config),
            ));
        }

        let auth_key = AdminAuthGuard::key_for(remote_addr);
        if self
            .admin_auth_guard
            .is_locked(&auth_key, &state.config.admin.auth_rate_limit)
        {
            return Ok(text_response(
                StatusCode::TOO_MANY_REQUESTS,
                "admin authentication temporarily locked",
            ));
        }

        if method == Method::POST && path == "/v1/login" {
            let payload = match serde_json::from_slice::<AdminLoginRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid login payload: {error}")}),
                    ));
                }
            };
            if payload.username != state.config.admin.username
                || payload.password != state.config.admin.password
            {
                self.stats
                    .admin_auth_fail_total
                    .fetch_add(1, Ordering::Relaxed);
                self.admin_auth_guard
                    .record_failure(&auth_key, &state.config.admin.auth_rate_limit);
                return Ok(json_response(
                    StatusCode::UNAUTHORIZED,
                    serde_json::json!({"ok": false, "error": "invalid credentials"}),
                ));
            }
            self.admin_auth_guard.clear_failures(&auth_key);
            let Some((token, expires_at)) = issue_admin_session_token(&state.config.admin) else {
                return Ok(json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({"ok": false, "error": "failed to issue admin session"}),
                ));
            };
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "token_type": "Bearer",
                    "access_token": token,
                    "expires_at": expires_at,
                    "expires_in": ADMIN_SESSION_TTL_SECS,
                    "username": state.config.admin.username,
                }),
            ));
        }

        if !is_authorized(headers.get(AUTHORIZATION), &state.config.admin) {
            self.stats
                .admin_auth_fail_total
                .fetch_add(1, Ordering::Relaxed);
            self.admin_auth_guard
                .record_failure(&auth_key, &state.config.admin.auth_rate_limit);
            return Ok(text_response(StatusCode::UNAUTHORIZED, "unauthorized"));
        }

        self.admin_auth_guard.clear_failures(&auth_key);

        if admin_request_is_write(&method, &path) {
            if let Some(response) = check_admin_mutation_access(&state.config, &transport) {
                return Ok(response);
            }
        }

        if method == Method::GET && path == "/v1/stats" {
            return Ok(json_response(StatusCode::OK, self.stats.snapshot_json()));
        }

        if method == Method::GET && path == "/v1/upstreams" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "items": self.upstream_runtime_snapshot(&state.config)}),
            ));
        }

        if method == Method::POST && path == "/v1/upstreams/disable" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<UpstreamToggleRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid upstream toggle payload: {error}")}),
                    ));
                }
            };
            self.set_manual_upstream_state(&state.config, &payload.key, true, payload.reason);
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "key": payload.key}),
            ));
        }

        if method == Method::POST && path == "/v1/upstreams/enable" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<UpstreamToggleRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid upstream toggle payload: {error}")}),
                    ));
                }
            };
            self.set_manual_upstream_state(&state.config, &payload.key, false, None);
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "key": payload.key}),
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

        if method == Method::GET && path == "/v1/tls/summary" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "tls": tls_admin_summary(&state.config)}),
            ));
        }

        if method == Method::GET && path == "/v1/tls/dns-providers" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "providers": crate::acme::dns_providers_json(),
                    "zero_external_deps": true,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/domain-routes" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.services.domain_routes,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/reverse-proxy-routes" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.services.reverse_proxy.routes,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/tcp-listeners" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.tcp.listeners,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/udp-listeners" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.udp.listeners,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/stream-routes" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.tcp.stream_routes,
                }),
            ));
        }

        if method == Method::GET && path == "/v1/filecloud/summary" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "filecloud": filecloud_admin_summary(&state.config),
                }),
            ));
        }

        if method == Method::POST && path == "/v1/domain-routes/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }

            let body = body.clone();

            let route = match serde_json::from_slice::<DomainRouteConfig>(&body) {
                Ok(route) => route,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid domain route payload: {error}")}),
                    ));
                }
            };

            if let Err(error) = validate_domain_route_mutation(&route, &state.config.security) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }

            match self
                .persist_domain_route_and_reload(&state.config, route)
                .await
            {
                Ok(result) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": result.action,
                            "name": result.route.name,
                            "domains": result.route.domains,
                            "path_prefix": result.route.path_prefix,
                            "upstream": result.route.upstream,
                            "upstreams": result.route.upstreams,
                        }),
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

        if method == Method::POST && path == "/v1/reverse-proxy-routes/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }

            let body = body.clone();

            let route = match serde_json::from_slice::<ReverseProxyRouteConfig>(&body) {
                Ok(route) => route,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid reverse proxy payload: {error}")}),
                    ));
                }
            };

            if let Err(error) =
                validate_reverse_proxy_route_mutation(&route, &state.config.security)
            {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }

            match self
                .persist_reverse_proxy_route_and_reload(&state.config, route)
                .await
            {
                Ok(result) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": result.action,
                            "name": result.route.name,
                            "hosts": result.route.hosts,
                            "path_prefix": result.route.path_prefix,
                            "upstream": result.route.upstream,
                            "upstreams": result.route.upstreams,
                        }),
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

        if method == Method::POST && path == "/v1/tcp-listeners/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }

            let body = body.clone();

            let listener = match serde_json::from_slice::<TcpListenerConfig>(&body) {
                Ok(listener) => listener,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid tcp listener payload: {error}")}),
                    ));
                }
            };

            if let Err(error) = validate_tcp_listener_mutation(&listener, &state.config.security) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }

            match self
                .persist_tcp_listener_and_reload(&state.config, listener)
                .await
            {
                Ok(result) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": result.action,
                            "name": result.listener.name,
                            "bind": result.listener.bind,
                            "upstream": result.listener.upstream,
                            "upstreams": result.listener.upstreams,
                        }),
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

        if method == Method::POST && path == "/v1/udp-listeners/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }

            let body = body.clone();

            let listener = match serde_json::from_slice::<UdpListenerConfig>(&body) {
                Ok(listener) => listener,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid udp listener payload: {error}")}),
                    ));
                }
            };

            if let Err(error) = validate_udp_listener_mutation(&listener, &state.config.security) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }

            match self
                .persist_udp_listener_and_reload(&state.config, listener)
                .await
            {
                Ok(result) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": result.action,
                            "name": result.listener.name,
                            "bind": result.listener.bind,
                            "upstream": result.listener.upstream,
                            "upstreams": result.listener.upstreams,
                        }),
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

        if method == Method::POST && path == "/v1/stream-routes/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let route = match serde_json::from_slice::<StreamRouteConfig>(&body) {
                Ok(route) => route,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid stream route payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&route.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_stream_route_and_reload(&state.config, route)
                .await
            {
                Ok(result) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": result.action,
                            "name": result.route.name,
                            "domains": result.route.domains,
                            "listen": result.route.listen,
                            "upstream": result.route.upstream,
                            "protocol": result.route.protocol,
                        }),
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

        if method == Method::GET && path == "/v1/security/blacklist" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": self.dynamic_blacklist.list_active(),
                }),
            ));
        }

        if method == Method::POST && path == "/v1/security/blacklist/add" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<BlacklistMutationRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid blacklist payload: {error}")}),
                    ));
                }
            };
            let ip = match payload.ip.parse::<IpAddr>() {
                Ok(ip) => ip,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid ip: {error}")}),
                    ));
                }
            };
            let ban_secs = payload
                .ban_secs
                .unwrap_or(state.config.security.ddos.ban_secs.max(1));
            self.dynamic_blacklist.add(ip, ban_secs);
            self.ddos_guard.ban_ip(ip, ban_secs);
            let _ = self
                .dynamic_blacklist
                .persist(&state.config.security.dynamic_blacklist);
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "ip": ip.to_string(), "ban_secs": ban_secs}),
            ));
        }

        if method == Method::POST && path == "/v1/security/blacklist/remove" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<BlacklistMutationRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid blacklist payload: {error}")}),
                    ));
                }
            };
            let ip = match payload.ip.parse::<IpAddr>() {
                Ok(ip) => ip,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid ip: {error}")}),
                    ));
                }
            };
            self.dynamic_blacklist.remove(ip);
            self.ddos_guard.unban_ip(ip);
            let _ = self
                .dynamic_blacklist
                .persist(&state.config.security.dynamic_blacklist);
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"ok": true, "ip": ip.to_string()}),
            ));
        }

        if method == Method::POST && path == "/v1/domain-routes/delete" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<NamedDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid delete payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&payload.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_domain_route_delete_and_reload(&state.config, &payload.name)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "name": payload.name}),
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

        if method == Method::POST && path == "/v1/reverse-proxy-routes/delete" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<NamedDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid delete payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&payload.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_reverse_proxy_route_delete_and_reload(&state.config, &payload.name)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "name": payload.name}),
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

        if method == Method::POST && path == "/v1/tls/auto-https/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<AutoHttpsUpsertRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid tls payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_auto_https_and_reload(&state.config, payload)
                .await
            {
                Ok(domains) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "domains": domains, "mode": "acme_managed"}),
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

        if method == Method::POST && path == "/v1/tls/wildcard-dns/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<WildcardTlsUpsertRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid wildcard tls payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_wildcard_tls_and_reload(&state.config, payload)
                .await
            {
                Ok(domains) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "domains": domains, "mode": "acme_managed", "challenge": "dns01"}),
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

        if method == Method::POST && path == "/v1/tls/on-demand/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<OnDemandTlsUpsertRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid on-demand tls payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_on_demand_tls_and_reload(&state.config, payload)
                .await
            {
                Ok(summary) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "on_demand": summary}),
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

        if method == Method::POST && path == "/v1/tls/issue-now" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            match self.trigger_managed_tls_issue().await {
                Ok(summary) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "issued": summary}),
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

        if method == Method::GET && path == "/v1/tls/sni-certificates" {
            return Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "ok": true,
                    "items": state.config.http.tls.certificates.iter().map(|cert| {
                        serde_json::json!({
                            "domains": cert.domains,
                            "cert_path": cert.cert_path.display().to_string(),
                            "key_path": cert.key_path.display().to_string(),
                            "cert_exists": cert.cert_path.exists(),
                            "key_exists": cert.key_path.exists(),
                        })
                    }).collect::<Vec<_>>(),
                }),
            ));
        }

        if method == Method::POST && path == "/v1/tls/sni-certificates/upsert" {
            let body = body.clone();
            let payload = match serde_json::from_slice::<SniCertificateUpsertRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid sni certificate payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_sni_certificate_and_reload(&state.config, payload)
                .await
            {
                Ok(cert) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({
                            "ok": true,
                            "action": cert.action,
                            "domains": cert.certificate.domains,
                            "cert_path": cert.certificate.cert_path.display().to_string(),
                            "key_path": cert.certificate.key_path.display().to_string(),
                        }),
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

        if method == Method::POST && path == "/v1/tls/sni-certificates/delete" {
            let body = body.clone();
            let payload = match serde_json::from_slice::<SniCertificateDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid sni certificate delete payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_sni_certificate_delete_and_reload(&state.config, payload)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true}),
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

        if method == Method::POST && path == "/v1/filecloud/upsert" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<FileCloudUpsertRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid filecloud payload: {error}")}),
                    ));
                }
            };
            match self
                .persist_filecloud_and_reload(&state.config, payload)
                .await
            {
                Ok(summary) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "filecloud": summary}),
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

        if method == Method::POST && path == "/v1/tcp-listeners/delete" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<NamedDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid delete payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&payload.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_tcp_listener_delete_and_reload(&state.config, &payload.name)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "name": payload.name}),
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

        if method == Method::POST && path == "/v1/udp-listeners/delete" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<NamedDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid delete payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&payload.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_udp_listener_delete_and_reload(&state.config, &payload.name)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "name": payload.name}),
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

        if method == Method::POST && path == "/v1/stream-routes/delete" {
            if !state.config.admin.enable_write_ops {
                return Ok(text_response(
                    StatusCode::FORBIDDEN,
                    "write operations disabled",
                ));
            }
            let body = body.clone();
            let payload = match serde_json::from_slice::<NamedDeleteRequest>(&body) {
                Ok(payload) => payload,
                Err(error) => {
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        serde_json::json!({"ok": false, "error": format!("invalid delete payload: {error}")}),
                    ));
                }
            };
            if let Err(error) = security::validate_route_name(&payload.name) {
                return Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    serde_json::json!({"ok": false, "error": error.to_string()}),
                ));
            }
            match self
                .persist_stream_route_delete_and_reload(&state.config, &payload.name)
                .await
            {
                Ok(()) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "name": payload.name}),
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

            let body = body.clone();

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

            let body = body.clone();

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
        self.load_persisted_manual_upstream_state(&new_config)?;

        for warning in new_config.warnings() {
            tracing::warn!(warning, "configuration warning");
        }

        tracing::info!(path = %self.config_path.display(), "configuration reloaded");
        Ok(())
    }

    async fn persist_domain_route_and_reload(
        &self,
        current_config: &GatewayConfig,
        route: DomainRouteConfig,
    ) -> Result<DomainRouteUpsertResult> {
        let mut candidate = current_config.clone();
        let action =
            upsert_domain_route_config(&mut candidate.services.domain_routes, route.clone());
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_domain_route(&original, &route)?;

        security::atomic_write(&self.config_path, &updated)
            .with_context(|| format!("failed to write {}", self.config_path.display()))?;

        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "updated config was written but reload failed; original file was restored",
            ));
        }

        Ok(DomainRouteUpsertResult { action, route })
    }

    async fn persist_domain_route_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        name: &str,
    ) -> Result<()> {
        let mut candidate = current_config.clone();
        let removed = candidate
            .services
            .domain_routes
            .iter()
            .any(|route| route.name == name);
        if !removed {
            return Err(anyhow!("domain route {name} not found"));
        }
        candidate
            .services
            .domain_routes
            .retain(|route| route.name != name);
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_domain_route(&original, name)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(
                error.context("delete was written but reload failed; original file was restored")
            );
        }
        Ok(())
    }

    async fn persist_reverse_proxy_route_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        name: &str,
    ) -> Result<()> {
        let mut candidate = current_config.clone();
        if !candidate
            .services
            .reverse_proxy
            .routes
            .iter()
            .any(|route| route.name == name)
        {
            return Err(anyhow!("reverse proxy route {name} not found"));
        }
        candidate
            .services
            .reverse_proxy
            .routes
            .retain(|route| route.name != name);
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_reverse_proxy_route(&original, name)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(
                error.context("delete was written but reload failed; original file was restored")
            );
        }
        Ok(())
    }

    async fn persist_auto_https_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: AutoHttpsUpsertRequest,
    ) -> Result<Vec<String>> {
        if payload.domains.is_empty() {
            return Err(anyhow!("domains cannot be empty"));
        }
        if payload.email.trim().is_empty() {
            return Err(anyhow!("email is required for managed ACME"));
        }
        security::validate_domains(&payload.domains)?;

        let domains = payload.domains.clone();
        let mut candidate = current_config.clone();
        candidate.http.tls.auto_https.enabled = true;
        candidate.http.tls.auto_https.domains = domains.clone();
        candidate.http.tls.auto_https.email = payload.email.clone();
        candidate.http.tls.auto_https.production = payload.production;
        candidate.http.tls.mode = TlsMode::AcmeManaged;
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_auto_https(&original, &payload)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error
                .context("tls update was written but reload failed; original file was restored"));
        }
        Ok(domains)
    }

    async fn persist_wildcard_tls_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: WildcardTlsUpsertRequest,
    ) -> Result<Vec<String>> {
        security::validate_domains(&payload.domains)?;
        if payload.email.trim().is_empty() {
            return Err(anyhow!("email is required for wildcard ACME"));
        }
        if payload.dns_provider.trim().is_empty() {
            return Err(anyhow!("dns_provider is required for wildcard DNS-01"));
        }
        if !crate::acme::is_builtin_dns_provider(&payload.dns_provider) {
            return Err(anyhow!(
                "dns_provider '{}' is not supported; built-in providers: {}",
                payload.dns_provider,
                crate::acme::list_builtin_dns_provider_ids().join(", ")
            ));
        }
        let normalized_provider = crate::acme::normalize_provider_id(&payload.dns_provider);
        if normalized_provider != "manual" && payload.credentials.is_empty() {
            return Err(anyhow!(
                "credentials are required for dns provider {normalized_provider}"
            ));
        }

        let domains = payload.domains.clone();
        let mut candidate = current_config.clone();
        candidate.http.tls.mode = TlsMode::AcmeManaged;
        candidate.http.tls.acme.challenge = AcmeChallengeType::Dns01;
        candidate.http.tls.acme.email = payload.email.clone();
        candidate.http.tls.acme.domains = domains.clone();
        candidate.http.tls.acme.dns.provider =
            crate::acme::normalize_provider_id(&payload.dns_provider);
        candidate.http.tls.acme.dns.credentials = payload.credentials.clone();
        candidate.http.tls.generate_self_signed_if_missing = false;
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_wildcard_tls(&original, &payload)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "wildcard tls update was written but reload failed; original file was restored",
            ));
        }
        Ok(domains)
    }

    async fn persist_on_demand_tls_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: OnDemandTlsUpsertRequest,
    ) -> Result<serde_json::Value> {
        let mut candidate = current_config.clone();
        candidate.http.tls.on_demand.enabled = payload.enabled;
        candidate.http.tls.on_demand.allow = payload.allow.clone();
        if let Some(value) = payload.max_active_certs {
            candidate.http.tls.on_demand.max_active_certs = value;
        }
        if let Some(value) = payload.max_issues_per_hour {
            candidate.http.tls.on_demand.max_issues_per_hour = value;
        }
        if let Some(ref value) = payload.ask_url {
            candidate.http.tls.on_demand.ask_url = value.clone();
        }
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_on_demand_tls(&original, &payload)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "on-demand tls update was written but reload failed; original file was restored",
            ));
        }
        Ok(serde_json::json!({
            "enabled": payload.enabled,
            "allow": payload.allow,
        }))
    }

    async fn trigger_managed_tls_issue(&self) -> Result<serde_json::Value> {
        let state = self.current_state().await;
        if state.config.http.tls.mode != TlsMode::AcmeManaged {
            return Err(anyhow!(
                "tls.mode must be acme_managed to issue certificates (current: {:?})",
                state.config.http.tls.mode
            ));
        }
        issue_managed_acme_certificate(
            &state.config.http.tls,
            &self.acme_http_challenges,
            &self.acme_tls_alpn_certs,
        )
        .await
        .context("managed ACME certificate issuance failed")?;
        let tls = self.current_state().await.config.http.tls.clone();
        Ok(serde_json::json!({
            "cert_exists": tls.cert_path.exists(),
            "key_exists": tls.key_path.exists(),
            "cert_path": tls.cert_path.display().to_string(),
            "key_path": tls.key_path.display().to_string(),
        }))
    }

    async fn persist_sni_certificate_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: SniCertificateUpsertRequest,
    ) -> Result<SniCertificateUpsertResult> {
        if payload.domains.is_empty() || payload.domains.iter().any(|item| item.trim().is_empty()) {
            return Err(anyhow!(
                "domains must contain at least one non-empty hostname"
            ));
        }
        let (cert_path, key_path) =
            resolve_sni_certificate_material(&current_config.root_dir, &payload)?;

        let certificate = TlsCertificateConfig {
            domains: payload
                .domains
                .into_iter()
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect(),
            cert_path,
            key_path,
        };

        let mut candidate = current_config.clone();
        let action = upsert_sni_certificate_config(
            &mut candidate.http.tls.certificates,
            certificate.clone(),
        );
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_sni_certificate(&original, &certificate)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "sni certificate update was written but reload failed; original file was restored",
            ));
        }
        Ok(SniCertificateUpsertResult {
            action,
            certificate,
        })
    }

    async fn persist_sni_certificate_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: SniCertificateDeleteRequest,
    ) -> Result<()> {
        if payload.cert_path.as_deref().unwrap_or("").trim().is_empty()
            && payload.domain.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(anyhow!("cert_path or domain is required"));
        }
        let mut candidate = current_config.clone();
        let before = candidate.http.tls.certificates.len();
        candidate
            .http
            .tls
            .certificates
            .retain(|cert| !sni_certificate_matches_delete(cert, &payload));
        if candidate.http.tls.certificates.len() == before {
            return Err(anyhow!("sni certificate entry not found"));
        }
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_sni_certificate(&original, &payload)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "sni certificate delete was written but reload failed; original file was restored",
            ));
        }
        Ok(())
    }

    async fn persist_filecloud_and_reload(
        &self,
        current_config: &GatewayConfig,
        payload: FileCloudUpsertRequest,
    ) -> Result<serde_json::Value> {
        let password = if payload.password.trim().is_empty() {
            current_config.services.filecloud.password.clone()
        } else {
            payload.password.clone()
        };
        let filecloud = FileCloudConfig {
            enabled: payload.enabled,
            path_prefix: payload.path_prefix,
            root: payload.root,
            password,
            title: payload.title,
            allow_upload: payload.allow_upload,
            allow_delete: payload.allow_delete,
            allow_mkdir: payload.allow_mkdir,
            allow_move: payload.allow_move,
            max_upload_bytes: payload
                .max_upload_bytes
                .unwrap_or(current_config.services.filecloud.max_upload_bytes),
            cdn_cache_secs: payload
                .cdn_cache_secs
                .unwrap_or(current_config.services.filecloud.cdn_cache_secs),
            session_ttl_secs: payload
                .session_ttl_secs
                .unwrap_or(current_config.services.filecloud.session_ttl_secs),
            require_auth_for_download: payload.require_auth_for_download,
        };

        let mut candidate = current_config.clone();
        candidate.services.filecloud = filecloud.clone();
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_filecloud(&original, &filecloud)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "filecloud update was written but reload failed; original file was restored",
            ));
        }
        Ok(filecloud_admin_summary(&candidate))
    }

    async fn persist_tcp_listener_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        name: &str,
    ) -> Result<()> {
        let mut candidate = current_config.clone();
        if !candidate.tcp.listeners.iter().any(|item| item.name == name) {
            return Err(anyhow!("tcp listener {name} not found"));
        }
        candidate.tcp.listeners.retain(|item| item.name != name);
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_tcp_listener(&original, name)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(
                error.context("delete was written but reload failed; original file was restored")
            );
        }
        Ok(())
    }

    async fn persist_udp_listener_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        name: &str,
    ) -> Result<()> {
        let mut candidate = current_config.clone();
        if !candidate.udp.listeners.iter().any(|item| item.name == name) {
            return Err(anyhow!("udp listener {name} not found"));
        }
        candidate.udp.listeners.retain(|item| item.name != name);
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_udp_listener(&original, name)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(
                error.context("delete was written but reload failed; original file was restored")
            );
        }
        Ok(())
    }

    async fn persist_stream_route_delete_and_reload(
        &self,
        current_config: &GatewayConfig,
        name: &str,
    ) -> Result<()> {
        let mut candidate = current_config.clone();
        if !candidate
            .tcp
            .stream_routes
            .iter()
            .any(|item| item.name == name)
        {
            return Err(anyhow!("stream route {name} not found"));
        }
        candidate.tcp.stream_routes.retain(|item| item.name != name);
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_deleted_stream_route(&original, name)?;

        security::atomic_write(&self.config_path, &updated)?;
        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(
                error.context("delete was written but reload failed; original file was restored")
            );
        }
        Ok(())
    }

    async fn persist_reverse_proxy_route_and_reload(
        &self,
        current_config: &GatewayConfig,
        route: ReverseProxyRouteConfig,
    ) -> Result<ReverseProxyRouteUpsertResult> {
        let mut candidate = current_config.clone();
        let action = upsert_reverse_proxy_route_config(
            &mut candidate.services.reverse_proxy.routes,
            route.clone(),
        );
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_reverse_proxy_route(&original, &route)?;

        security::atomic_write(&self.config_path, &updated)
            .with_context(|| format!("failed to write {}", self.config_path.display()))?;

        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "updated config was written but reload failed; original file was restored",
            ));
        }

        Ok(ReverseProxyRouteUpsertResult { action, route })
    }

    async fn persist_tcp_listener_and_reload(
        &self,
        current_config: &GatewayConfig,
        listener: TcpListenerConfig,
    ) -> Result<TcpListenerUpsertResult> {
        let mut candidate = current_config.clone();
        let action = upsert_tcp_listener_config(&mut candidate.tcp.listeners, listener.clone());
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_tcp_listener(&original, &listener)?;

        security::atomic_write(&self.config_path, &updated)
            .with_context(|| format!("failed to write {}", self.config_path.display()))?;

        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "updated config was written but reload failed; original file was restored",
            ));
        }

        Ok(TcpListenerUpsertResult { action, listener })
    }

    async fn persist_udp_listener_and_reload(
        &self,
        current_config: &GatewayConfig,
        listener: UdpListenerConfig,
    ) -> Result<UdpListenerUpsertResult> {
        let mut candidate = current_config.clone();
        let action = upsert_udp_listener_config(&mut candidate.udp.listeners, listener.clone());
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_udp_listener(&original, &listener)?;

        security::atomic_write(&self.config_path, &updated)
            .with_context(|| format!("failed to write {}", self.config_path.display()))?;

        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "updated config was written but reload failed; original file was restored",
            ));
        }

        Ok(UdpListenerUpsertResult { action, listener })
    }

    async fn persist_stream_route_and_reload(
        &self,
        current_config: &GatewayConfig,
        route: StreamRouteConfig,
    ) -> Result<StreamRouteUpsertResult> {
        let mut candidate = current_config.clone();
        let action = upsert_stream_route_config(&mut candidate.tcp.stream_routes, route.clone());
        candidate.validate()?;

        let original = fs::read_to_string(&self.config_path)
            .with_context(|| format!("failed to read {}", self.config_path.display()))?;
        let updated = render_config_with_upserted_stream_route(&original, &route)?;

        security::atomic_write(&self.config_path, &updated)
            .with_context(|| format!("failed to write {}", self.config_path.display()))?;

        if let Err(error) = self.reload_from_disk().await {
            let _ = security::atomic_write(&self.config_path, &original);
            return Err(error.context(
                "updated config was written but reload failed; original file was restored",
            ));
        }

        Ok(StreamRouteUpsertResult { action, route })
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
            self.acme_tls_alpn_certs.clone(),
            self.on_demand_certs.clone(),
            self.on_demand_trigger.clone(),
            vec![b"acme-tls/1".to_vec(), b"h2".to_vec(), b"http/1.1".to_vec()],
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
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(build_rustls_server_config(
                &state.config,
                self.acme_tls_alpn_certs.clone(),
                self.on_demand_certs.clone(),
                self.on_demand_trigger.clone(),
                vec![b"h3".to_vec()],
            )?)?,
        ));
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
            let listener_bind = listener_config.bind.clone();
            let listener_default_upstream = listener_config.upstream.clone();
            let stream_rate_limit = self
                .current_state()
                .await
                .config
                .services
                .rate_limit
                .stream
                .clone();
            if !apply_stream_rate_limit(&self.stream_rate_limits, &stream_rate_limit, remote_addr) {
                tracing::info!(
                    %remote_addr,
                    listener = %listener_name,
                    "tcp connection rejected by stream rate limit"
                );
                self.stats
                    .blocked_requests_total
                    .fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let block_config = self.current_state().await.config.clone();
            if self.is_stream_connection_blocked(&block_config, remote_addr) {
                tracing::info!(%remote_addr, listener = %listener_name, "tcp connection blocked by security policy");
                self.stats
                    .blocked_requests_total
                    .fetch_add(1, Ordering::Relaxed);
                continue;
            }
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
                    if listener_name == "ftp"
                        && state.config.services.ftp.enabled
                        && state.config.services.ftp.native_control
                    {
                        gateway
                            .handle_ftp_control_session(
                                inbound,
                                &state.config.services.ftp,
                                remote_addr,
                            )
                            .await?;
                        return Ok(());
                    }

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

                    let route = if listener_name.starts_with("stream|") {
                        let stream_table =
                            StreamRouteTable::from_config(&state.config.tcp.stream_routes);
                        let sni = parse_tls_client_hello_sni(&first_payload);
                        let resolved = stream_table
                            .resolve_upstream(
                                &listener_bind,
                                sni.as_deref(),
                                &listener_default_upstream,
                            )
                            .ok_or_else(|| {
                                anyhow!(
                                    "no stream route matched for listener {} (sni={:?})",
                                    listener_name,
                                    sni
                                )
                            })?;
                        if let Some(denied) = stream_access_is_denied(
                            &resolved.route.access_control,
                            remote_addr.ip(),
                        ) {
                            return Err(anyhow!("stream access denied for {denied}"));
                        }
                        if !resolved.protocol.is_empty() {
                            tracing::info!(
                                protocol = %resolved.protocol,
                                route = %resolved.route.name,
                                sni = ?sni,
                                %remote_addr,
                                "domain stream route selected"
                            );
                        }
                        RouteDecision {
                            upstream: resolved.upstream.to_string(),
                            upstreams: resolved.route.upstreams.clone(),
                            upstream_weights: resolved.route.upstream_weights.clone(),
                            affinity_key: player_id.clone(),
                            rewrite_path: None,
                            set_headers: BTreeMap::new(),
                            strip_headers: Vec::new(),
                            status: None,
                            content_type: None,
                        }
                    } else if listener_name == "ftp" && state.config.services.ftp.enabled {
                        RouteDecision {
                            upstream: state.config.services.ftp.upstream.clone(),
                            upstreams: Vec::new(),
                            upstream_weights: BTreeMap::new(),
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

    async fn handle_ftp_control_session(
        &self,
        inbound: TcpStream,
        config: &crate::config::FtpConfig,
        remote_addr: SocketAddr,
    ) -> Result<()> {
        let ftp_acl = crate::config::HttpAccessControlConfig {
            enabled: !config.allow.is_empty() || !config.deny.is_empty(),
            allow: config.allow.clone(),
            deny: config.deny.clone(),
            status: 421,
        };
        if crate::security::ip_access_is_denied(&ftp_acl, remote_addr.ip()).is_some() {
            let (client_read, mut client_write) = inbound.into_split();
            let _ = client_read;
            client_write
                .write_all(b"421 Access denied by gateway policy.\r\n")
                .await
                .ok();
            return Ok(());
        }

        let upstream = TcpStream::connect(&config.upstream)
            .await
            .with_context(|| format!("failed to connect ftp upstream {}", config.upstream))?;
        let upstream_control_ip = upstream
            .peer_addr()
            .context("failed to resolve ftp upstream peer")?
            .ip();
        let local_control_ip = inbound
            .local_addr()
            .context("failed to resolve local ftp control address")?
            .ip();
        let public_ip = if config.public_ip.trim().is_empty() {
            local_control_ip
        } else {
            config
                .public_ip
                .parse::<IpAddr>()
                .with_context(|| format!("invalid services.ftp.public_ip {}", config.public_ip))?
        };

        let (client_read, mut client_write) = inbound.into_split();
        let (server_read, mut server_write) = upstream.into_split();
        let mut client_reader = TokioBufReader::new(client_read);
        let mut server_reader = TokioBufReader::new(server_read);
        let session_users = self.ftp_session_users.clone();

        loop {
            let mut client_line = Vec::new();
            let mut server_line = Vec::new();

            tokio::select! {
                read = client_reader.read_until(b'\n', &mut client_line) => {
                    let read = read.context("failed reading ftp client command")?;
                    if read == 0 {
                        break;
                    }
                    let command_line = String::from_utf8_lossy(&client_line).into_owned();
                    if let Some(verb) = parse_ftp_command_verb(&command_line) {
                        if verb == "USER" {
                            if let Some(user) = command_line.split_whitespace().nth(1) {
                                session_users.insert(remote_addr, user.to_string());
                            }
                        }
                        let active_user = session_users
                            .get(&remote_addr)
                            .map(|entry| entry.clone())
                            .unwrap_or_default();
                        if config.log_commands {
                            tracing::info!(
                                %remote_addr,
                                command = %verb,
                                user = %active_user,
                                "ftp control command"
                            );
                        }
                        if !ftp_command_allowed_for_user(config, &verb, &active_user) {
                            tracing::warn!(
                                %remote_addr,
                                command = %verb,
                                user = %active_user,
                                "ftp command denied by policy"
                            );
                            client_write
                                .write_all(b"502 Command not allowed by gateway policy.\r\n")
                                .await
                                .context("failed writing ftp policy rejection")?;
                            continue;
                        }
                        if ftp_transfer_verb(&verb)
                            && !ftp_transfer_allowed_for_user(config, &verb, &active_user)
                        {
                            tracing::warn!(
                                %remote_addr,
                                command = %verb,
                                user = %active_user,
                                "ftp transfer denied by policy"
                            );
                            client_write
                                .write_all(b"550 Transfer not allowed by gateway policy.\r\n")
                                .await
                                .context("failed writing ftp transfer rejection")?;
                            continue;
                        }
                        if ftp_transfer_verb(&verb) && config.log_transfers {
                            tracing::info!(
                                %remote_addr,
                                command = %verb,
                                user = %active_user,
                                "ftp transfer hook"
                            );
                        }
                    }
                    let outbound_line = rewrite_ftp_active_command(
                        &client_line,
                        config,
                        local_control_ip,
                        public_ip,
                        remote_addr,
                    )
                    .await?
                    .unwrap_or(command_line);
                    server_write
                        .write_all(outbound_line.as_bytes())
                        .await
                        .context("failed forwarding ftp client command")?;
                }
                read = server_reader.read_until(b'\n', &mut server_line) => {
                    let read = read.context("failed reading ftp upstream reply")?;
                    if read == 0 {
                        break;
                    }

                    let maybe_reply = rewrite_ftp_passive_reply(
                        &server_line,
                        config,
                        local_control_ip,
                        public_ip,
                        upstream_control_ip,
                        remote_addr,
                    ).await?;

                    if let Some(reply) = maybe_reply {
                        client_write
                            .write_all(reply.as_bytes())
                            .await
                            .context("failed writing rewritten ftp passive reply")?;
                    } else {
                        client_write
                            .write_all(&server_line)
                            .await
                            .context("failed forwarding ftp upstream reply")?;
                    }
                }
            }
        }

        session_users.remove(&remote_addr);
        Ok(())
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
    ) -> Result<GatewayResponse, Infallible> {
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
                headers.clone(),
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
        let response = decorate_error_response(&state.config, &headers, response);
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

        if !security::request_uri_is_safe(&uri) {
            return Ok(GatewayHttpResponse::bytes(
                StatusCode::BAD_REQUEST,
                "text/plain; charset=utf-8",
                Bytes::from_static(b"invalid request path"),
                "proxysss://security",
            ));
        }

        if let Some(status) =
            security::reject_ambiguous_http1_request(&headers, &state.config.security)
        {
            return Ok(GatewayHttpResponse::bytes(
                status,
                "text/plain; charset=utf-8",
                Bytes::from_static(b"ambiguous http/1 request"),
                "proxysss://security",
            ));
        }

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

        if let Some(token) = uri.path().strip_prefix("/.well-known/acme-challenge/") {
            if let Some(value) = self.acme_http_challenges.get(token) {
                return Ok(GatewayHttpResponse::bytes(
                    StatusCode::OK,
                    "text/plain; charset=utf-8",
                    Bytes::from(value.value().clone()),
                    "proxysss://acme-http01",
                ));
            }
        }

        if let Some(internal_path) = map_admin_gateway_path(&state.config.admin, uri.path()) {
            if state.config.admin.enabled {
                let transport = if scheme == "https" {
                    AdminTransport::GatewayHttps { host: host.clone() }
                } else {
                    AdminTransport::GatewayHttp
                };
                let response = self
                    .serve_admin_api(method, internal_path, headers, body, remote_addr, transport)
                    .await
                    .expect("admin handler is infallible");
                let (parts, response_body) = response.into_parts();
                let body_bytes = response_body
                    .collect()
                    .await
                    .map(|collected| collected.to_bytes())
                    .unwrap_or_default();
                let headers_out = parts
                    .headers
                    .into_iter()
                    .filter_map(|(name, value)| name.map(|name| (name, value)))
                    .collect::<Vec<_>>();
                return Ok(GatewayHttpResponse {
                    status: parts.status,
                    headers: headers_out,
                    body: body_bytes,
                    stream_body: None,
                    upstream: "proxysss://admin-https".to_string(),
                });
            }
        }

        if scheme == "http" && should_redirect_http_to_https(&state.config, &host, &uri) {
            let target = format!(
                "https://{}{}",
                strip_default_port(&host, 80),
                uri.path_and_query()
                    .map(|value| value.as_str())
                    .unwrap_or("/")
            );
            return Ok(GatewayHttpResponse::redirect_with_status(
                StatusCode::PERMANENT_REDIRECT,
                target,
                "proxysss://auto-https-redirect",
            ));
        }

        if monitoring_path_matches(&state.config.monitoring, uri.path()) {
            return Ok(match state.config.monitoring.format {
                MonitoringFormat::Json => json_gateway_response(
                    StatusCode::OK,
                    self.stats.snapshot_json(),
                    "proxysss://metrics",
                ),
                MonitoringFormat::Prometheus => GatewayHttpResponse::bytes(
                    StatusCode::OK,
                    "text/plain; version=0.0.4; charset=utf-8",
                    Bytes::from(self.stats.snapshot_prometheus()),
                    "proxysss://metrics",
                ),
            });
        }

        if self.is_http_connection_blocked(&state.config, remote_addr) {
            self.stats
                .blocked_requests_total
                .fetch_add(1, Ordering::Relaxed);
            return Ok(GatewayHttpResponse::bytes(
                StatusCode::TOO_MANY_REQUESTS,
                "text/plain; charset=utf-8",
                Bytes::from_static(b"connection blocked by security policy"),
                "proxysss://security",
            ));
        }

        if let Some(response) =
            self.apply_http_access_control(&state.config.services.access_control.http, remote_addr)
        {
            self.stats
                .blocked_requests_total
                .fetch_add(1, Ordering::Relaxed);
            return Ok(response);
        }

        if state.config.services.filecloud.enabled
            && crate::filecloud::path::path_matches(
                &state.config.services.filecloud.path_prefix,
                uri.path(),
            )
        {
            let _rate_limit_lease = match self.apply_http_rate_limit(
                &state.config.services.rate_limit.http,
                &host,
                &headers,
                remote_addr,
            ) {
                Ok(lease) => lease,
                Err(response) => return Ok(response),
            };
            let response = crate::filecloud::dispatch_filecloud(
                &state.config.services.filecloud,
                &method,
                uri.path(),
                uri.query(),
                &headers,
                body,
            )
            .await?;
            return finalize_http_response(
                &headers,
                &state.config.services.response_policy.compression,
                response,
            );
        }

        if state.config.services.webdav.enabled
            && webdav_path_matches(&state.config.services.webdav.path_prefix, uri.path())
        {
            let _rate_limit_lease = match self.apply_http_rate_limit(
                &state.config.services.rate_limit.http,
                &host,
                &headers,
                remote_addr,
            ) {
                Ok(lease) => lease,
                Err(response) => return Ok(response),
            };
            let response =
                dispatch_webdav(&state.config.services.webdav, &method, &uri, &headers, body)
                    .await?;
            return finalize_http_response(
                &headers,
                &state.config.services.response_policy.compression,
                response,
            );
        }

        if let Some(site) = state
            .config
            .services
            .static_sites
            .iter()
            .find(|site| static_site_path_matches(site, uri.path()))
        {
            let _rate_limit_lease = match self.apply_http_rate_limit(
                &state.config.services.rate_limit.http,
                &host,
                &headers,
                remote_addr,
            ) {
                Ok(lease) => lease,
                Err(response) => return Ok(response),
            };
            let response = dispatch_static_site(site, &method, &uri).await?;
            return finalize_http_response(
                &headers,
                &state.config.services.response_policy.compression,
                response,
            );
        }

        let route = if let Some(route) = configured_http_route(&state.config, &host, &uri) {
            route
        } else if let Some(script) = &state.script {
            HttpRouteConfig {
                runtime_scope: Some("script".to_string()),
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
                compression: state.config.services.response_policy.compression.clone(),
                cache: state.config.services.response_policy.cache.clone(),
                rate_limit: state.config.services.rate_limit.http.clone(),
            }
        } else if let Some(route) = builtin_http_route(uri.path()) {
            HttpRouteConfig {
                runtime_scope: Some("builtin".to_string()),
                decision: route,
                compression: state.config.services.response_policy.compression.clone(),
                cache: state.config.services.response_policy.cache.clone(),
                rate_limit: state.config.services.rate_limit.http.clone(),
            }
        } else {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "no built-in or YAML route matched; enable script.enabled to use TypeScript routing",
            ));
        };

        let _rate_limit_lease =
            match self.apply_http_rate_limit(&route.rate_limit, &host, &headers, remote_addr) {
                Ok(lease) => lease,
                Err(response) => return Ok(response),
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

        if method.as_str() == "PURGE" {
            return Ok(self.purge_http_cache(&state.config, &route.cache, &host, &uri));
        }

        let cache_key = cache_lookup_key(&route.cache, &method, &host, &uri, &headers);
        if let Some(cache_key) = cache_key.as_deref() {
            match self.lookup_cached_http_response(&state.config, &route.cache, cache_key) {
                Some(CacheLookup::Fresh(mut response)) => {
                    let _ = apply_cache_response_headers(&route.cache, &mut response, "HIT");
                    return finalize_http_response(&headers, &route.compression, response);
                }
                Some(CacheLookup::Stale(mut response)) => {
                    let gateway = self.clone();
                    let route_for_refresh = route.clone();
                    let cache_key_owned = cache_key.to_string();
                    let host_owned = host.to_string();
                    let uri_owned = uri.clone();
                    let headers_owned = headers.clone();
                    let remote = remote_addr;
                    let scheme_owned = scheme.to_string();
                    tokio::spawn(async move {
                        let revalidate_request = HttpCacheRevalidateRequest {
                            host: &host_owned,
                            uri: &uri_owned,
                            headers: &headers_owned,
                            remote_addr: remote,
                            scheme: &scheme_owned,
                        };
                        if let Err(error) = gateway
                            .revalidate_cached_http_response(
                                &route_for_refresh,
                                &cache_key_owned,
                                &revalidate_request,
                            )
                            .await
                        {
                            tracing::debug!(?error, key = %cache_key_owned, "cache background revalidation failed");
                        }
                    });
                    let _ = apply_cache_response_headers(&route.cache, &mut response, "STALE");
                    return finalize_http_response(&headers, &route.compression, response);
                }
                Some(CacheLookup::StaleIfError(mut response)) => {
                    let _ = apply_cache_response_headers(&route.cache, &mut response, "STALE");
                    return finalize_http_response(&headers, &route.compression, response);
                }
                None => {}
            }
        }

        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route.decision,
            "http",
            route.runtime_scope.as_deref(),
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
            let _lease =
                self.acquire_upstream_lease("http", route.runtime_scope.as_deref(), upstream);

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
                    self.on_upstream_failure(
                        &state.config,
                        "http",
                        route.runtime_scope.as_deref(),
                        upstream,
                    );
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
            if status.is_server_error() && attempt + 1 < max_attempts {
                self.on_upstream_failure(
                    &state.config,
                    "http",
                    route.runtime_scope.as_deref(),
                    upstream,
                );
                last_error = Some(anyhow!(
                    "upstream {upstream} returned server error {}",
                    status.as_u16()
                ));
                continue;
            }

            self.on_upstream_success("http", route.runtime_scope.as_deref(), upstream);
            let is_sse = response_headers
                .iter()
                .find(|(name, _)| name == CONTENT_TYPE)
                .and_then(|(_, value)| value.to_str().ok())
                .map(|value| value.starts_with("text/event-stream"))
                .unwrap_or(false);
            let (body, stream_body) = if version == "HTTP/3" || cache_key.is_some() || !is_sse {
                let response_body = match upstream_response.bytes().await {
                    Ok(body_bytes) => body_bytes,
                    Err(error) => {
                        self.on_upstream_failure(
                            &state.config,
                            "http",
                            route.runtime_scope.as_deref(),
                            upstream,
                        );
                        last_error =
                            Some(anyhow!("failed reading upstream response body: {error}"));
                        continue;
                    }
                };
                (response_body, None)
            } else {
                (
                    Bytes::new(),
                    Some(streaming_body(upstream_response.bytes_stream())),
                )
            };
            let response = GatewayHttpResponse {
                status,
                headers: response_headers,
                body,
                stream_body,
                upstream: upstream.clone(),
            };
            if let Some(cache_key) = cache_key.as_deref() {
                self.store_cached_http_response(&state.config, cache_key, &route.cache, &response);
            }
            let mut response = response;
            if cache_key.is_some() && route.cache.enabled {
                let _ = apply_cache_response_headers(&route.cache, &mut response, "MISS");
            }
            return finalize_http_response(&headers, &route.compression, response);
        }

        if let Some(cache_key) = cache_key.as_deref() {
            if let Some(
                CacheLookup::StaleIfError(mut response) | CacheLookup::Stale(mut response),
            ) = self.lookup_cached_http_response(&state.config, &route.cache, cache_key)
            {
                let _ = apply_cache_response_headers(&route.cache, &mut response, "STALE");
                return finalize_http_response(&headers, &route.compression, response);
            }
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
                stream_body: None,
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
            stream_body: None,
            upstream,
        })
    }

    async fn current_state(&self) -> Arc<DynamicState> {
        self.dynamic.read().await.clone()
    }

    fn lookup_cached_http_response(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        key: &str,
    ) -> Option<CacheLookup> {
        let storage_key = cache_storage_key(cache, key);
        let now = current_unix_millis();

        let entry = if let Some(entry) = self.http_cache.get(&storage_key) {
            entry.clone()
        } else {
            let entry = self.load_disk_cached_http_response(config, cache, &storage_key)?;
            self.http_cache.insert(storage_key.clone(), entry.clone());
            entry
        };

        if entry.stale_until_unix_ms > 0 && now >= entry.stale_until_unix_ms {
            drop(entry);
            self.http_cache.remove(&storage_key);
            self.remove_disk_cached_http_response(config, cache, &storage_key);
            return None;
        }

        let response = GatewayHttpResponse {
            status: entry.status,
            headers: entry.headers.clone(),
            body: entry.body.clone(),
            stream_body: None,
            upstream: entry.upstream.clone(),
        };

        if now < entry.expires_at_unix_ms {
            return Some(CacheLookup::Fresh(response));
        }

        if cache.stale_while_revalidate_secs > 0 {
            return Some(CacheLookup::Stale(response));
        }

        if cache.stale_if_error_secs > 0 && now < entry.stale_until_unix_ms {
            return Some(CacheLookup::StaleIfError(response));
        }

        self.http_cache.remove(&storage_key);
        self.remove_disk_cached_http_response(config, cache, &storage_key);
        None
    }

    fn store_cached_http_response(
        &self,
        gateway_config: &GatewayConfig,
        key: &str,
        cache_config: &ResponseCacheConfig,
        response: &GatewayHttpResponse,
    ) {
        if !cache_config.enabled
            || cache_config.behavior == CacheBehavior::Bypass
            || cache_config.behavior == CacheBehavior::NoCache
            || response.body.len() > cache_config.max_body_bytes
            || !cache_config
                .statuses
                .iter()
                .any(|status| *status == response.status.as_u16())
            || response.headers.iter().any(|(name, _)| name == SET_COOKIE)
            || (cache_config.behavior == CacheBehavior::RespectOrigin
                && cache_control_prevents_storage(&response.headers))
        {
            return;
        }

        let storage_key = cache_storage_key(cache_config, key);
        let now = current_unix_millis();
        let edge_ttl_secs = effective_edge_ttl_secs(cache_config, &response.headers);
        let fresh_until = now.saturating_add(edge_ttl_secs.saturating_mul(1000));
        let mut stale_until = if cache_config.stale_while_revalidate_secs > 0 {
            fresh_until.saturating_add(
                cache_config
                    .stale_while_revalidate_secs
                    .saturating_mul(1000),
            )
        } else {
            fresh_until
        };
        if cache_config.stale_if_error_secs > 0 {
            stale_until =
                stale_until.saturating_add(cache_config.stale_if_error_secs.saturating_mul(1000));
        }
        let entry = CachedHttpEntry {
            expires_at_unix_ms: fresh_until,
            stale_until_unix_ms: stale_until,
            status: response.status,
            headers: response.headers.clone(),
            body: response.body.clone(),
            upstream: response.upstream.clone(),
        };
        self.http_cache.insert(storage_key.clone(), entry.clone());
        self.persist_disk_cached_http_response(gateway_config, cache_config, &storage_key, &entry);
        self.evict_cache_zone_if_needed(gateway_config, cache_config);
    }

    async fn revalidate_cached_http_response(
        &self,
        route: &HttpRouteConfig,
        cache_key: &str,
        request: &HttpCacheRevalidateRequest<'_>,
    ) -> Result<()> {
        let state = self.current_state().await;
        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route.decision,
            "http",
            route.runtime_scope.as_deref(),
            route.decision.affinity_key.as_deref(),
            Some(&request.remote_addr.to_string()),
        );
        let upstream = upstream_plan
            .first()
            .cloned()
            .unwrap_or_else(|| route.decision.upstream.clone());
        let upstream_url = build_upstream_url(&upstream, &route.decision, request.uri)?;
        let upstream_headers = build_upstream_headers(
            request.headers,
            &route.decision,
            request.host,
            request.remote_addr,
            request.scheme,
        )?;
        let response = state
            .http_client
            .get(upstream_url)
            .headers(upstream_headers)
            .send()
            .await
            .context("cache revalidation upstream request failed")?;
        let status = response.status();
        let response_headers = response
            .headers()
            .iter()
            .filter(|(name, _)| !is_hop_header(name.as_str()))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<Vec<_>>();
        let response_body = response
            .bytes()
            .await
            .context("cache revalidation body read failed")?;
        let cached = GatewayHttpResponse {
            status,
            headers: response_headers,
            body: response_body,
            stream_body: None,
            upstream,
        };
        self.store_cached_http_response(&state.config, cache_key, &route.cache, &cached);
        Ok(())
    }

    fn purge_http_cache(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        host: &str,
        uri: &Uri,
    ) -> GatewayHttpResponse {
        if !cache.enabled || !cache.allow_purge {
            return GatewayHttpResponse::error(
                StatusCode::FORBIDDEN,
                "cache purge is disabled for this route",
            );
        }

        let key = format!(
            "GET:{}:{}",
            host.to_ascii_lowercase(),
            uri.path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/")
        );
        if self.purge_cached_http_response(config, cache, &key) {
            GatewayHttpResponse::bytes(
                StatusCode::OK,
                "application/json",
                Bytes::from(
                    serde_json::json!({"ok": true, "zone": cache.zone, "key": key}).to_string(),
                ),
                "proxysss://cache-purge",
            )
        } else {
            GatewayHttpResponse::bytes(
                StatusCode::NOT_FOUND,
                "application/json",
                Bytes::from(
                    serde_json::json!({"ok": false, "zone": cache.zone, "key": key}).to_string(),
                ),
                "proxysss://cache-purge",
            )
        }
    }

    fn purge_cached_http_response(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        key: &str,
    ) -> bool {
        let storage_key = cache_storage_key(cache, key);
        let removed = self.http_cache.remove(&storage_key).is_some();
        let removed_disk = self.remove_disk_cached_http_response(config, cache, &storage_key);
        removed || removed_disk
    }

    fn evict_cache_zone_if_needed(&self, config: &GatewayConfig, cache: &ResponseCacheConfig) {
        let max_entries = cache_zone_max_entries(config, &cache.zone);
        let prefix = format!("{}:", cache.zone);
        let now = current_unix_millis();

        let stale_keys = self
            .http_cache
            .iter()
            .filter(|entry| {
                entry.key().starts_with(&prefix) && entry.value().stale_until_unix_ms <= now
            })
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for stale_key in stale_keys {
            self.http_cache.remove(&stale_key);
            self.remove_disk_cached_http_response(config, cache, &stale_key);
        }

        let zone_keys = self
            .http_cache
            .iter()
            .filter(|entry| entry.key().starts_with(&prefix))
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        if zone_keys.len() < max_entries {
            return;
        }

        let overflow = zone_keys
            .len()
            .saturating_sub(max_entries)
            .saturating_add(1);
        for key in zone_keys.into_iter().take(overflow) {
            self.http_cache.remove(&key);
            self.remove_disk_cached_http_response(config, cache, &key);
        }
    }

    fn load_disk_cached_http_response(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        storage_key: &str,
    ) -> Option<CachedHttpEntry> {
        let path = cache_disk_path(config, cache, storage_key)?;
        let payload = fs::read(&path).ok()?;
        let disk: DiskCachedHttpEntry = serde_json::from_slice(&payload).ok()?;
        let body = base64::engine::general_purpose::STANDARD
            .decode(disk.body_base64)
            .ok()?;
        let status = StatusCode::from_u16(disk.status).ok()?;
        let headers = disk
            .headers
            .into_iter()
            .filter_map(|(name, value)| {
                Some((
                    HeaderName::from_bytes(name.as_bytes()).ok()?,
                    security::sanitize_header_value(&value).ok()?,
                ))
            })
            .collect::<Vec<_>>();

        let stale_until = if disk.stale_until_unix_ms > 0 {
            disk.stale_until_unix_ms
        } else {
            disk.expires_at_unix_ms
        };

        Some(CachedHttpEntry {
            expires_at_unix_ms: disk.expires_at_unix_ms,
            stale_until_unix_ms: stale_until,
            status,
            headers,
            body: Bytes::from(body),
            upstream: disk.upstream,
        })
    }

    fn persist_disk_cached_http_response(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        storage_key: &str,
        entry: &CachedHttpEntry,
    ) {
        let Some(path) = cache_disk_path(config, cache, storage_key) else {
            return;
        };
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let disk = DiskCachedHttpEntry {
            expires_at_unix_ms: entry.expires_at_unix_ms,
            stale_until_unix_ms: entry.stale_until_unix_ms,
            status: entry.status.as_u16(),
            headers: entry
                .headers
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
                })
                .collect(),
            body_base64: base64::engine::general_purpose::STANDARD.encode(&entry.body),
            upstream: entry.upstream.clone(),
        };
        if let Ok(bytes) = serde_json::to_vec(&disk) {
            let _ = fs::write(path, bytes);
        }
    }

    fn remove_disk_cached_http_response(
        &self,
        config: &GatewayConfig,
        cache: &ResponseCacheConfig,
        storage_key: &str,
    ) -> bool {
        let Some(path) = cache_disk_path(config, cache, storage_key) else {
            return false;
        };
        fs::remove_file(path).is_ok()
    }

    fn apply_http_rate_limit(
        &self,
        config: &HttpRateLimitConfig,
        host: &str,
        headers: &HeaderMap,
        remote_addr: SocketAddr,
    ) -> std::result::Result<Option<HttpRateLimitLease>, GatewayHttpResponse> {
        if !config.enabled {
            return Ok(None);
        }

        let Some(key) = http_rate_limit_key(config, host, headers, remote_addr) else {
            return Ok(None);
        };
        if let Some(retry_after) =
            apply_http_rate_limit_to_store(&self.http_rate_limits, config, key.clone())
        {
            return Err(rate_limit_rejection_response(config, &retry_after));
        }

        if config.max_connections == 0 {
            return Ok(None);
        }

        let mut entry = self.http_connection_limits.entry(key.clone()).or_insert(0);
        if *entry >= config.max_connections {
            return Err(rate_limit_rejection_response(config, "1"));
        }
        *entry = entry.saturating_add(1);
        drop(entry);

        Ok(Some(HttpRateLimitLease {
            store: self.http_connection_limits.clone(),
            key,
        }))
    }

    fn apply_http_access_control(
        &self,
        config: &HttpAccessControlConfig,
        remote_addr: SocketAddr,
    ) -> Option<GatewayHttpResponse> {
        let denied = http_access_is_denied(config, remote_addr.ip())?;
        let status = StatusCode::from_u16(config.status).unwrap_or(StatusCode::FORBIDDEN);
        Some(GatewayHttpResponse::bytes(
            status,
            "text/plain; charset=utf-8",
            Bytes::from(format!("access denied for {}", denied)),
            "proxysss://access-control",
        ))
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
            LoadBalanceAlgorithm::Weighted => {
                self.select_weighted_plan(&scope_base, candidates, &route.upstream_weights)
            }
        }
    }

    fn select_weighted_plan(
        &self,
        scope_base: &str,
        candidates: Vec<String>,
        weights: &BTreeMap<String, u32>,
    ) -> Vec<String> {
        if candidates.len() <= 1 {
            return candidates;
        }

        let weighted = candidates
            .iter()
            .map(|candidate| {
                let weight = weights.get(candidate).copied().unwrap_or(1).max(1);
                (candidate.clone(), weight)
            })
            .collect::<Vec<_>>();
        let total: u32 = weighted.iter().map(|(_, weight)| weight).sum();
        if total == 0 {
            return candidates;
        }

        let counter = {
            let mut entry = self
                .round_robin_state
                .entry(format!("weighted:{scope_base}"))
                .or_insert(0);
            let current = *entry;
            *entry = entry.wrapping_add(1);
            current
        };

        let mut pick = (counter as u32) % total;
        let mut primary_idx = 0usize;
        for (index, (_, weight)) in weighted.iter().enumerate() {
            if pick < *weight {
                primary_idx = index;
                break;
            }
            pick -= weight;
        }

        let mut plan = vec![weighted[primary_idx].0.clone()];
        for (index, (candidate, _)) in weighted.iter().enumerate() {
            if index != primary_idx {
                plan.push(candidate.clone());
            }
        }
        plan
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
        let now = Instant::now();
        let mut available = Vec::new();

        for candidate in &candidates {
            let key = runtime_scope_key(protocol, listener, candidate);
            let healthy = match self.upstream_runtime.get(&key) {
                Some(state) => {
                    if state.manually_disabled {
                        false
                    } else {
                        let passive_healthy = match state.quarantined_until {
                            Some(until) if config.load_balance.passive_health.enabled => {
                                until <= now
                            }
                            None => true,
                            Some(_) => true,
                        };
                        let active_healthy = if config.load_balance.active_health.enabled {
                            state.active_probe_healthy.unwrap_or(true)
                        } else {
                            true
                        };
                        passive_healthy && active_healthy
                    }
                }
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
        if !entry.manually_disabled {
            entry.quarantined_until = None;
        }
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

    fn upstream_runtime_snapshot(&self, config: &GatewayConfig) -> Vec<serde_json::Value> {
        let now = Instant::now();
        let mut result = self
            .upstream_runtime
            .iter()
            .map(|entry| {
                let (protocol, listener, upstream) = split_runtime_scope_key(entry.key());
                let route_names =
                    route_names_for_runtime_scope(config, protocol, listener, upstream);
                let value = entry.value();
                let remaining = value
                    .quarantined_until
                    .map(|until| until.saturating_duration_since(now).as_secs())
                    .unwrap_or(0);
                let passive_healthy = value
                    .quarantined_until
                    .map(|until| until <= now)
                    .unwrap_or(true);
                let active_healthy = value.active_probe_healthy.unwrap_or(true);
                serde_json::json!({
                    "key": entry.key(),
                    "protocol": protocol,
                    "listener": listener,
                    "upstream": upstream,
                    "route_names": route_names,
                    "consecutive_failures": value.consecutive_failures,
                    "active_connections": value.active_connections,
                    "manually_disabled": value.manually_disabled,
                    "manual_reason": value.manual_reason,
                    "manual_changed_at_unix_ms": value.manual_changed_at_unix_ms,
                    "active_probe_kind": value.active_probe_kind,
                    "quarantine_remaining_secs": remaining,
                    "passive_healthy": passive_healthy,
                    "active_healthy": value.active_probe_healthy,
                    "active_probe_status": value.active_probe_status,
                    "active_probe_error": value.active_probe_error,
                    "active_probe_checked_at_unix_ms": value.active_probe_checked_at_unix_ms,
                    "active_probe_rtt_ms": value.active_probe_rtt_ms,
                    "healthy": passive_healthy && active_healthy,
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

    async fn run_active_health_pass(&self, config: &GatewayConfig, client: &reqwest::Client) {
        let targets = collect_active_health_targets(config);
        if targets.is_empty() {
            return;
        }

        for target in targets {
            if !target.settings.enabled {
                continue;
            }

            if let Some(jitter) = health_check_jitter_delay(&target) {
                tokio::time::sleep(jitter).await;
            }

            let key = runtime_scope_key(
                &target.protocol,
                target.listener.as_deref(),
                &target.upstream,
            );
            let (healthy, status, error, rtt_ms) = match target.kind {
                ActiveHealthKind::Http => {
                    probe_http_upstream(client, &target.upstream, &target.settings).await
                }
                ActiveHealthKind::Tcp => {
                    probe_tcp_upstream(&target.upstream, &target.settings).await
                }
            };

            let mut alert_payload = None;
            let mut entry = self.upstream_runtime.entry(key.clone()).or_default();
            entry.active_probe_kind = Some(target.kind.as_str().to_string());
            entry.active_probe_status = status;
            entry.active_probe_error = error;
            entry.active_probe_checked_at_unix_ms = Some(current_unix_millis());
            entry.active_probe_rtt_ms = rtt_ms;
            let previous_health = entry.active_probe_healthy;

            if healthy {
                entry.active_probe_success_count =
                    entry.active_probe_success_count.saturating_add(1);
                entry.active_probe_failure_count = 0;
                if entry.active_probe_success_count >= target.settings.success_threshold {
                    entry.active_probe_healthy = Some(true);
                }
            } else {
                entry.active_probe_failure_count =
                    entry.active_probe_failure_count.saturating_add(1);
                entry.active_probe_success_count = 0;
                if entry.active_probe_failure_count >= target.settings.failure_threshold {
                    entry.active_probe_healthy = Some(false);
                }
            }

            if previous_health != entry.active_probe_healthy {
                alert_payload = Some(serde_json::json!({
                    "key": key,
                    "protocol": target.protocol,
                    "listener": target.listener,
                    "upstream": target.upstream,
                    "healthy": entry.active_probe_healthy,
                    "probe_kind": entry.active_probe_kind,
                    "status": entry.active_probe_status,
                    "error": entry.active_probe_error,
                    "checked_at_unix_ms": entry.active_probe_checked_at_unix_ms,
                }));
            }
            drop(entry);

            if let Some(payload) = alert_payload {
                dispatch_active_health_alerts(client, &target.settings.alert_webhooks, payload)
                    .await;
            }
        }
    }
}

fn split_runtime_scope_key(key: &str) -> (&str, &str, &str) {
    let mut parts = key.splitn(3, ':');
    let protocol = parts.next().unwrap_or("unknown");
    let listener = parts.next().unwrap_or("default");
    let upstream = parts.next().unwrap_or("");
    (protocol, listener, upstream)
}

fn route_names_for_runtime_scope(
    config: &GatewayConfig,
    protocol: &str,
    listener: &str,
    upstream: &str,
) -> Vec<String> {
    let mut names = BTreeSet::new();

    match protocol {
        "http" => {
            if listener != "default" && !listener.is_empty() {
                names.insert(listener.to_string());
                return names.into_iter().collect();
            }
            for route in &config.services.reverse_proxy.routes {
                if normalize_candidates(&RouteDecision {
                    upstream: route.upstream.clone(),
                    upstreams: route.upstreams.clone(),
                    upstream_weights: route.upstream_weights.clone(),
                    affinity_key: None,
                    rewrite_path: None,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                    status: None,
                    content_type: None,
                })
                .iter()
                .any(|candidate| candidate == upstream)
                {
                    names.insert(route.name.clone());
                }
            }
            for route in &config.services.domain_routes {
                if normalize_candidates(&RouteDecision {
                    upstream: route.upstream.clone(),
                    upstreams: route.upstreams.clone(),
                    upstream_weights: route.upstream_weights.clone(),
                    affinity_key: None,
                    rewrite_path: None,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                    status: None,
                    content_type: None,
                })
                .iter()
                .any(|candidate| candidate == upstream)
                {
                    names.insert(route.name.clone());
                }
            }
        }
        "tcp" => {
            if listener == "ftp" {
                names.insert("ftp".to_string());
            } else if !listener.is_empty() && listener != "default" {
                names.insert(listener.to_string());
            }
        }
        "udp" if !listener.is_empty() && listener != "default" => {
            names.insert(listener.to_string());
        }
        _ => {}
    }

    names.into_iter().collect()
}

#[derive(Clone)]
struct ActiveHealthTarget {
    protocol: String,
    listener: Option<String>,
    upstream: String,
    kind: ActiveHealthKind,
    settings: ResolvedActiveHealthConfig,
}

#[derive(Clone, Copy)]
enum ActiveHealthKind {
    Http,
    Tcp,
}

impl ActiveHealthKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Tcp => "tcp",
        }
    }
}

fn collect_active_health_targets(config: &GatewayConfig) -> Vec<ActiveHealthTarget> {
    let mut upstreams = BTreeMap::<String, ActiveHealthTarget>::new();

    if config.load_balance.active_health.enabled && config.load_balance.active_health.http_enabled {
        for route in &config.services.reverse_proxy.routes {
            for upstream in normalize_candidates(&RouteDecision {
                upstream: route.upstream.clone(),
                upstreams: route.upstreams.clone(),
                upstream_weights: route.upstream_weights.clone(),
                affinity_key: None,
                rewrite_path: None,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                status: None,
                content_type: None,
            }) {
                let key = runtime_scope_key("http", Some(&route.name), &upstream);
                upstreams.entry(key).or_insert(ActiveHealthTarget {
                    protocol: "http".to_string(),
                    listener: Some(route.name.clone()),
                    upstream,
                    kind: ActiveHealthKind::Http,
                    settings: resolve_active_health_config(
                        &config.load_balance.active_health,
                        &route.active_health,
                    ),
                });
            }
        }

        for route in &config.services.domain_routes {
            for upstream in normalize_candidates(&RouteDecision {
                upstream: route.upstream.clone(),
                upstreams: route.upstreams.clone(),
                upstream_weights: route.upstream_weights.clone(),
                affinity_key: None,
                rewrite_path: None,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                status: None,
                content_type: None,
            }) {
                let key = runtime_scope_key("http", Some(&route.name), &upstream);
                upstreams.entry(key).or_insert(ActiveHealthTarget {
                    protocol: "http".to_string(),
                    listener: Some(route.name.clone()),
                    upstream,
                    kind: ActiveHealthKind::Http,
                    settings: resolve_active_health_config(
                        &config.load_balance.active_health,
                        &route.active_health,
                    ),
                });
            }
        }
    }

    if config.load_balance.active_health.enabled && config.load_balance.active_health.tcp_enabled {
        for listener in &config.tcp.listeners {
            for upstream in normalize_candidates(&RouteDecision {
                upstream: listener.upstream.clone(),
                upstreams: listener.upstreams.clone(),
                upstream_weights: listener.upstream_weights.clone(),
                affinity_key: None,
                rewrite_path: None,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                status: None,
                content_type: None,
            }) {
                let key = runtime_scope_key("tcp", Some(&listener.name), &upstream);
                upstreams.entry(key).or_insert(ActiveHealthTarget {
                    protocol: "tcp".to_string(),
                    listener: Some(listener.name.clone()),
                    upstream,
                    kind: ActiveHealthKind::Tcp,
                    settings: resolve_global_active_health_config(
                        &config.load_balance.active_health,
                    ),
                });
            }
        }

        if config.services.ftp.enabled {
            let upstream = config.services.ftp.upstream.clone();
            let key = runtime_scope_key("tcp", Some("ftp"), &upstream);
            upstreams.entry(key).or_insert(ActiveHealthTarget {
                protocol: "tcp".to_string(),
                listener: Some("ftp".to_string()),
                upstream,
                kind: ActiveHealthKind::Tcp,
                settings: resolve_global_active_health_config(&config.load_balance.active_health),
            });
        }
    }

    upstreams.into_values().collect()
}

async fn probe_http_upstream(
    client: &reqwest::Client,
    upstream: &str,
    settings: &ResolvedActiveHealthConfig,
) -> (bool, Option<u16>, Option<String>, Option<u64>) {
    let started_at = Instant::now();

    let target_url = match build_health_check_url(upstream, &settings.path) {
        Ok(url) => url,
        Err(error) => return (false, None, Some(error.to_string()), None),
    };

    let request = client
        .get(target_url)
        .timeout(Duration::from_millis(settings.timeout_ms.max(100)));
    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let healthy = settings.expected_statuses.contains(&status);
            let error = if healthy {
                None
            } else {
                Some(format!("unexpected status {status}"))
            };
            (
                healthy,
                Some(status),
                error,
                Some(
                    started_at
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX),
                ),
            )
        }
        Err(error) => (
            false,
            None,
            Some(error.to_string()),
            Some(
                started_at
                    .elapsed()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX),
            ),
        ),
    }
}

async fn probe_tcp_upstream(
    upstream: &str,
    settings: &ResolvedActiveHealthConfig,
) -> (bool, Option<u16>, Option<String>, Option<u64>) {
    let started_at = Instant::now();
    let timeout = Duration::from_millis(settings.timeout_ms.max(100));
    let target = match tcp_probe_target(upstream) {
        Ok(target) => target,
        Err(error) => return (false, None, Some(error.to_string()), None),
    };

    match tokio::time::timeout(timeout, TcpStream::connect(&target)).await {
        Ok(Ok(stream)) => {
            drop(stream);
            (
                true,
                None,
                None,
                Some(
                    started_at
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX),
                ),
            )
        }
        Ok(Err(error)) => (
            false,
            None,
            Some(error.to_string()),
            Some(
                started_at
                    .elapsed()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX),
            ),
        ),
        Err(_) => (
            false,
            None,
            Some(format!(
                "tcp connect timeout after {}ms",
                timeout.as_millis()
            )),
            Some(
                started_at
                    .elapsed()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX),
            ),
        ),
    }
}

fn tcp_probe_target(upstream: &str) -> Result<String> {
    if upstream.starts_with("http://")
        || upstream.starts_with("https://")
        || upstream.starts_with("ws://")
        || upstream.starts_with("wss://")
    {
        let url = Url::parse(upstream)
            .with_context(|| format!("invalid tcp probe upstream url {upstream}"))?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("upstream URL missing host"))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("upstream URL missing port"))?;
        Ok(format!("{host}:{port}"))
    } else {
        Ok(upstream.to_string())
    }
}

fn health_check_jitter_delay(target: &ActiveHealthTarget) -> Option<Duration> {
    if target.settings.jitter_percent == 0 {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    target.protocol.hash(&mut hasher);
    target.listener.hash(&mut hasher);
    target.upstream.hash(&mut hasher);
    let bucket = hasher.finish() % 10_000;
    let max_ms = target
        .settings
        .timeout_ms
        .saturating_mul(target.settings.jitter_percent as u64)
        / 100;
    Some(Duration::from_millis(
        (bucket.saturating_mul(max_ms.max(1)) / 10_000).max(1),
    ))
}

async fn dispatch_active_health_alerts(
    client: &reqwest::Client,
    webhooks: &[String],
    payload: serde_json::Value,
) {
    for webhook in webhooks {
        let result = client.post(webhook).json(&payload).send().await;
        if let Err(error) = result {
            tracing::warn!(?error, webhook = %webhook, "active health alert webhook failed");
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedManualUpstreamState {
    items: BTreeMap<String, PersistedManualUpstreamEntry>,
}

#[derive(Serialize, Deserialize)]
struct PersistedManualUpstreamEntry {
    manually_disabled: bool,
    manual_reason: Option<String>,
    manual_changed_at_unix_ms: Option<u64>,
}

impl Gateway {
    fn set_manual_upstream_state(
        &self,
        config: &GatewayConfig,
        key: &str,
        disabled: bool,
        reason: Option<String>,
    ) {
        let mut entry = self.upstream_runtime.entry(key.to_string()).or_default();
        entry.manually_disabled = disabled;
        entry.manual_reason = if disabled { reason } else { None };
        entry.manual_changed_at_unix_ms = Some(current_unix_millis());
        if !disabled {
            entry.quarantined_until = None;
        }
        drop(entry);
        let _ = self.persist_manual_upstream_state(config);
    }

    fn persist_manual_upstream_state(&self, config: &GatewayConfig) -> Result<()> {
        if !config.runtime.maintenance_state.enabled {
            return Ok(());
        }

        let items = self
            .upstream_runtime
            .iter()
            .filter(|entry| entry.value().manually_disabled)
            .map(|entry| {
                (
                    entry.key().clone(),
                    PersistedManualUpstreamEntry {
                        manually_disabled: entry.value().manually_disabled,
                        manual_reason: entry.value().manual_reason.clone(),
                        manual_changed_at_unix_ms: entry.value().manual_changed_at_unix_ms,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        let payload = PersistedManualUpstreamState { items };
        if let Some(parent) = config.runtime.maintenance_state.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(
            &config.runtime.maintenance_state.path,
            serde_json::to_vec_pretty(&payload).context("failed to serialize maintenance state")?,
        )
        .with_context(|| {
            format!(
                "failed to write {}",
                config.runtime.maintenance_state.path.display()
            )
        })
    }

    fn load_persisted_manual_upstream_state(&self, config: &GatewayConfig) -> Result<()> {
        if !config.runtime.maintenance_state.enabled
            || !config.runtime.maintenance_state.path.exists()
        {
            return Ok(());
        }

        let bytes = fs::read(&config.runtime.maintenance_state.path).with_context(|| {
            format!(
                "failed to read {}",
                config.runtime.maintenance_state.path.display()
            )
        })?;
        let payload: PersistedManualUpstreamState =
            serde_json::from_slice(&bytes).with_context(|| {
                format!(
                    "failed to decode {}",
                    config.runtime.maintenance_state.path.display()
                )
            })?;

        for (key, item) in payload.items {
            if item.manually_disabled {
                let mut entry = self.upstream_runtime.entry(key).or_default();
                entry.manually_disabled = true;
                entry.manual_reason = item.manual_reason;
                entry.manual_changed_at_unix_ms = item.manual_changed_at_unix_ms;
            }
        }
        Ok(())
    }
}

fn build_health_check_url(upstream: &str, path: &str) -> Result<Url> {
    let normalized = if upstream.starts_with("http://")
        || upstream.starts_with("https://")
        || upstream.starts_with("ws://")
        || upstream.starts_with("wss://")
    {
        upstream.to_string()
    } else {
        format!("http://{upstream}")
    };
    let mut url = Url::parse(&normalized)
        .with_context(|| format!("invalid health check upstream url {upstream}"))?;
    let mapped_scheme = match url.scheme() {
        "ws" => "http",
        "wss" => "https",
        other => other,
    }
    .to_string();
    if mapped_scheme != url.scheme() {
        url.set_scheme(&mapped_scheme)
            .map_err(|_| anyhow!("failed to map upstream scheme for {upstream}"))?;
    }
    url.set_path(path);
    url.set_query(None);
    Ok(url)
}

impl GatewayHttpResponse {
    pub(crate) fn error(status: StatusCode, message: impl Into<String>) -> Self {
        let body = Bytes::from(message.into());
        Self {
            status,
            headers: vec![(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            body,
            stream_body: None,
            upstream: "-".to_string(),
        }
    }

    pub(crate) fn html(body: impl Into<String>, upstream: impl Into<String>) -> Self {
        Self::html_with_status(StatusCode::OK, body, upstream)
    }

    fn html_with_status(
        status: StatusCode,
        body: impl Into<String>,
        upstream: impl Into<String>,
    ) -> Self {
        Self {
            status,
            headers: vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            )],
            body: Bytes::from(body.into()),
            stream_body: None,
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
            stream_body: None,
            upstream: upstream.into(),
        }
    }

    pub(crate) fn bytes(
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
            stream_body: None,
            upstream: upstream.into(),
        }
    }

    pub(crate) fn json<T: Serialize>(
        status: StatusCode,
        value: &T,
        upstream: impl Into<String>,
    ) -> Result<Self> {
        let body = serde_json::to_vec(value).context("failed to encode json response")?;
        Ok(Self {
            status,
            headers: vec![(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            body: Bytes::from(body),
            stream_body: None,
            upstream: upstream.into(),
        })
    }

    pub(crate) fn push_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.push((name, value));
    }

    #[cfg(test)]
    pub(crate) fn headers(&self) -> &[(HeaderName, HeaderValue)] {
        &self.headers
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> StatusCode {
        self.status
    }

    #[cfg(test)]
    pub(crate) fn body(&self) -> &Bytes {
        &self.body
    }

    fn into_hyper(self) -> GatewayResponse {
        let mut builder = Response::builder().status(self.status);
        for (name, value) in self.headers {
            builder = builder.header(name, value);
        }
        let body = self.stream_body.unwrap_or_else(|| full_body(self.body));
        builder.body(body).unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(full_body(Bytes::from_static(b"response build failure")))
                .expect("static response build should never fail")
        })
    }
}

fn full_body(body: Bytes) -> GatewayBody {
    Full::new(body)
        .map_err(|never| match never {})
        .boxed_unsync()
}

fn streaming_body<S>(stream: S) -> GatewayBody
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let stream = stream
        .map_ok(Frame::data)
        .map_err(|error| anyhow!("upstream response stream failed: {error}"));
    StreamBody::new(stream).boxed_unsync()
}

impl GatewayStats {
    fn snapshot_json(&self) -> serde_json::Value {
        let process = self.process_snapshot();
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
            "blocked_requests_total": self.blocked_requests_total.load(Ordering::Relaxed),
            "ddos_bans_total": self.ddos_bans_total.load(Ordering::Relaxed),
            "process": {
                "pid": process.pid,
                "cpu_percent": process.cpu_percent,
                "memory_bytes": process.memory_bytes,
                "memory_mb": process.memory_bytes.map(|value| value as f64 / 1024.0 / 1024.0),
                "memory_percent": process.memory_percent,
            },
        })
    }

    fn snapshot_prometheus(&self) -> String {
        let process = self.process_snapshot();
        let metrics = [
            (
                "proxysss_http_requests_total",
                "Total HTTP requests handled by the gateway",
                self.http_requests.load(Ordering::Relaxed),
            ),
            (
                "proxysss_http_errors_total",
                "Total HTTP error responses emitted by the gateway",
                self.http_errors.load(Ordering::Relaxed),
            ),
            (
                "proxysss_tcp_sessions_total",
                "Total TCP stream sessions accepted",
                self.tcp_sessions_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_udp_packets_total",
                "Total UDP datagrams proxied",
                self.udp_packets_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_udp_bytes_total",
                "Total UDP payload bytes proxied",
                self.udp_bytes_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_reload_success_total",
                "Successful configuration reload operations",
                self.reload_success_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_reload_failure_total",
                "Failed configuration reload operations",
                self.reload_failure_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_admin_requests_total",
                "Admin API requests served",
                self.admin_requests_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_admin_auth_fail_total",
                "Admin API authentication failures",
                self.admin_auth_fail_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_script_fail_total",
                "Embedded script execution failures",
                self.script_fail_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_blocked_requests_total",
                "Requests or connections blocked by security policy",
                self.blocked_requests_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_ddos_bans_total",
                "Connections temporarily banned by DDoS mitigation",
                self.ddos_bans_total.load(Ordering::Relaxed),
            ),
        ];

        let mut lines = Vec::new();
        for (name, help, value) in metrics {
            lines.push(format!("# HELP {name} {help}"));
            lines.push(format!("# TYPE {name} counter"));
            lines.push(format!("{name} {value}"));
        }
        lines.push("# HELP proxysss_tcp_sessions_active Active TCP stream sessions".to_string());
        lines.push("# TYPE proxysss_tcp_sessions_active gauge".to_string());
        lines.push(format!(
            "proxysss_tcp_sessions_active {}",
            self.tcp_sessions_active.load(Ordering::Relaxed)
        ));
        if let Some(cpu_percent) = process.cpu_percent {
            lines.push(
                "# HELP proxysss_process_cpu_percent Current process CPU usage percentage"
                    .to_string(),
            );
            lines.push("# TYPE proxysss_process_cpu_percent gauge".to_string());
            lines.push(format!("proxysss_process_cpu_percent {cpu_percent:.3}"));
        }
        if let Some(memory_bytes) = process.memory_bytes {
            lines.push(
                "# HELP proxysss_process_memory_bytes Current process resident memory in bytes"
                    .to_string(),
            );
            lines.push("# TYPE proxysss_process_memory_bytes gauge".to_string());
            lines.push(format!("proxysss_process_memory_bytes {memory_bytes}"));
        }
        if let Some(memory_percent) = process.memory_percent {
            lines.push(
                "# HELP proxysss_process_memory_percent Current process resident memory percentage"
                    .to_string(),
            );
            lines.push("# TYPE proxysss_process_memory_percent gauge".to_string());
            lines.push(format!(
                "proxysss_process_memory_percent {memory_percent:.3}"
            ));
        }
        lines.push(String::new());
        lines.join("\n")
    }

    fn process_snapshot(&self) -> ProcessMetricsSnapshot {
        let mut sampler = self
            .process_metrics
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sampler.snapshot()
    }
}

impl ProcessMetricsSampler {
    fn snapshot(&mut self) -> ProcessMetricsSnapshot {
        let sample = current_process_cpu_sample();
        let cpu_percent = sample.and_then(|current| {
            let previous = self.previous;
            self.previous = Some(current);
            previous.and_then(|previous| {
                let wall_secs = current.wall.duration_since(previous.wall).as_secs_f64();
                if wall_secs <= 0.0 {
                    return None;
                }
                let cpu_delta = (current.cpu_time_secs - previous.cpu_time_secs).max(0.0);
                Some((cpu_delta / wall_secs / available_parallelism() as f64) * 100.0)
            })
        });

        ProcessMetricsSnapshot {
            pid: std::process::id(),
            cpu_percent,
            memory_bytes: current_process_memory_bytes(),
            memory_percent: current_process_memory_percent(),
        }
    }
}

fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .max(1)
}

#[cfg(unix)]
fn current_process_cpu_sample() -> Option<ProcessMetricsSample> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    let user = usage.ru_utime.tv_sec as f64 + usage.ru_utime.tv_usec as f64 / 1_000_000.0;
    let system = usage.ru_stime.tv_sec as f64 + usage.ru_stime.tv_usec as f64 / 1_000_000.0;
    Some(ProcessMetricsSample {
        wall: Instant::now(),
        cpu_time_secs: user + system,
    })
}

#[cfg(windows)]
fn current_process_cpu_sample() -> Option<ProcessMetricsSample> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileTime {
        low_date_time: u32,
        high_date_time: u32,
    }

    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessTimes(
            process: *mut std::ffi::c_void,
            creation_time: *mut FileTime,
            exit_time: *mut FileTime,
            kernel_time: *mut FileTime,
            user_time: *mut FileTime,
        ) -> i32;
    }

    fn filetime_to_u64(value: FileTime) -> u64 {
        ((value.high_date_time as u64) << 32) | value.low_date_time as u64
    }

    let mut creation = FileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        return None;
    }

    let cpu_100ns = filetime_to_u64(kernel).saturating_add(filetime_to_u64(user));
    Some(ProcessMetricsSample {
        wall: Instant::now(),
        cpu_time_secs: cpu_100ns as f64 / 10_000_000.0,
    })
}

#[cfg(not(any(unix, windows)))]
fn current_process_cpu_sample() -> Option<ProcessMetricsSample> {
    None
}

#[cfg(unix)]
fn current_process_memory_bytes() -> Option<u64> {
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return None;
    }
    Some(resident_pages.saturating_mul(page_size as u64))
}

#[cfg(windows)]
fn current_process_memory_bytes() -> Option<u64> {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    Some(counters.working_set_size as u64)
}

#[cfg(not(any(unix, windows)))]
fn current_process_memory_bytes() -> Option<u64> {
    None
}

fn current_process_memory_percent() -> Option<f64> {
    let memory = current_process_memory_bytes()? as f64;
    let total = total_system_memory_bytes()? as f64;
    if total <= 0.0 {
        return None;
    }
    Some(memory / total * 100.0)
}

#[cfg(unix)]
fn total_system_memory_bytes() -> Option<u64> {
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(windows)]
fn total_system_memory_bytes() -> Option<u64> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }

    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 || status.total_phys == 0 {
        return None;
    }
    Some(status.total_phys)
}

#[cfg(not(any(unix, windows)))]
fn total_system_memory_bytes() -> Option<u64> {
    None
}

fn decorate_error_response(
    config: &GatewayConfig,
    request_headers: &HeaderMap,
    response: GatewayHttpResponse,
) -> GatewayHttpResponse {
    if response.stream_body.is_some() {
        return response;
    }

    if !response.status.is_client_error() && !response.status.is_server_error() {
        return response;
    }

    if let Some(page) = config
        .http
        .error_pages
        .pages
        .iter()
        .find(|page| page.status == response.status.as_u16())
    {
        if let Some(body) = load_configured_error_page(
            page,
            response.status,
            &response.body,
            config.http.error_pages.show_details,
        ) {
            let mut replacement = response;
            replacement.headers = vec![(
                CONTENT_TYPE,
                HeaderValue::from_str(&page.content_type)
                    .unwrap_or_else(|_| HeaderValue::from_static("text/html; charset=utf-8")),
            )];
            replacement.body = Bytes::from(body);
            return replacement;
        }
    }

    if wants_html_response(request_headers) {
        let detail = if config.http.error_pages.show_details {
            String::from_utf8_lossy(&response.body).to_string()
        } else {
            String::new()
        };
        return GatewayHttpResponse::html_with_status(
            response.status,
            render_default_error_html(response.status, &detail),
            response.upstream,
        );
    }

    response
}

fn load_configured_error_page(
    page: &crate::config::HttpErrorPageConfig,
    status: StatusCode,
    body: &[u8],
    show_details: bool,
) -> Option<String> {
    let mut content = if !page.body.trim().is_empty() {
        page.body.clone()
    } else if !page.file_path.as_os_str().is_empty() {
        fs::read_to_string(&page.file_path).ok()?
    } else {
        return None;
    };

    let detail = if show_details {
        String::from_utf8_lossy(body).to_string()
    } else {
        String::new()
    };
    content = content.replace("{{status}}", &status.as_u16().to_string());
    content = content.replace("{{reason}}", status.canonical_reason().unwrap_or("Error"));
    content = content.replace("{{detail}}", &detail);
    Some(content)
}

fn wants_html_response(headers: &HeaderMap) -> bool {
    headers
        .get(http::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|accept| {
            accept.contains("text/html")
                || accept.contains("application/xhtml+xml")
                || accept.contains("*/*")
        })
        .unwrap_or(true)
}

fn render_default_error_html(status: StatusCode, detail: &str) -> String {
    let title = format!(
        "{} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Error")
    );
    let detail_block = if detail.trim().is_empty() {
        "".to_string()
    } else {
        format!("<pre>{}</pre>", html_escape(detail))
    };

    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" /><title>{title}</title><style>body{{margin:0;min-height:100vh;display:grid;place-items:center;font-family:Avenir Next,PingFang SC,Microsoft YaHei,sans-serif;background:radial-gradient(circle at top left,rgba(89,208,255,.18),transparent 30%),linear-gradient(160deg,#07111a,#0c1823);color:#eef6ff}}.card{{width:min(760px,calc(100vw - 28px));padding:28px;border-radius:28px;background:rgba(10,20,34,.88);border:1px solid rgba(146,191,255,.14);box-shadow:0 24px 70px rgba(0,0,0,.24)}}.eyebrow{{font-size:12px;letter-spacing:.18em;text-transform:uppercase;color:#7ef4b0}}h1{{margin:12px 0 8px;font-size:clamp(44px,8vw,88px);line-height:.92;letter-spacing:-.05em}}p{{margin:0;color:#9ab0c2;font-size:16px;line-height:1.6}}.actions{{display:flex;gap:12px;flex-wrap:wrap;margin-top:22px}}a{{display:inline-flex;align-items:center;text-decoration:none;padding:12px 16px;border-radius:999px;font-weight:800}}.primary{{background:linear-gradient(135deg,#56d7ff,#77f3bf);color:#04111a}}.ghost{{background:rgba(255,255,255,.06);color:#eef6ff}}pre{{margin-top:18px;padding:16px;border-radius:18px;background:rgba(2,8,18,.76);overflow:auto;color:#d7e9ff}}</style></head><body><main class=\"card\"><div class=\"eyebrow\">gateway response</div><h1>{}</h1><p>{}</p><div class=\"actions\"><a class=\"primary\" href=\"/\">Back to proxysss</a><a class=\"ghost\" href=\"/docs.html\">Open docs</a></div>{}</main></body></html>",
        html_escape(&title),
        html_escape(status.canonical_reason().unwrap_or("The gateway returned an error response.")),
        detail_block,
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

    ["yaml", "yml"]
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
            upstream_weights: BTreeMap::new(),
        }));
    }

    let stream_table = StreamRouteTable::from_config(&config.tcp.stream_routes);
    for bind in stream_table.by_bind.keys() {
        let routes = stream_table.routes_for_bind(bind).unwrap_or(&[]);
        let default_upstream = routes
            .first()
            .map(|route| route.upstream.clone())
            .unwrap_or_default();
        specs.push(ListenerSpec::Tcp(TcpListenerConfig {
            name: format!("stream|{bind}"),
            bind: bind.to_string(),
            upstream: default_upstream,
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
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
        TlsMode::AcmeManaged => {
            if config.http.tls.acme.challenge == AcmeChallengeType::Dns01 {
                if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                    return Err(anyhow!(
                        "tls.mode=acme_managed with challenge=dns01 still missing cert/key after issuance attempt: {} {}",
                        config.http.tls.cert_path.display(),
                        config.http.tls.key_path.display()
                    ));
                }
            } else if config.http.tls.generate_self_signed_if_missing {
                install::ensure_cert_pair(
                    &config.http.tls.cert_path,
                    &config.http.tls.key_path,
                    &config.http.tls.server_name,
                    false,
                )?;
            }
            if !config.http.tls.cert_path.exists() || !config.http.tls.key_path.exists() {
                return Err(anyhow!(
                    "tls.mode=acme_managed requires bootstrap cert/key or generate_self_signed_if_missing=true: {} {}",
                    config.http.tls.cert_path.display(),
                    config.http.tls.key_path.display()
                ));
            }
        }
        TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal => {
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
    acme_tls_alpn_by_name: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_certs: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_trigger: tokio::sync::mpsc::UnboundedSender<String>,
    alpn_protocols: Vec<Vec<u8>>,
) -> Result<rustls::ServerConfig> {
    let mut server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(build_tls_resolver(
            config,
            acme_tls_alpn_by_name,
            on_demand_certs,
            on_demand_trigger,
        )?);
    server_config.alpn_protocols = alpn_protocols;
    Ok(server_config)
}

fn build_tls_resolver(
    config: &GatewayConfig,
    acme_tls_alpn_by_name: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_certs: Arc<DashMap<String, Arc<CertifiedKey>>>,
    on_demand_trigger: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<Arc<dyn ResolvesServerCert>> {
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

    Ok(Arc::new(SniResolver {
        default,
        by_name,
        acme_tls_alpn_by_name,
        on_demand_certs,
        on_demand: config.http.tls.on_demand.clone(),
        on_demand_trigger,
    }))
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

fn build_acme_tls_alpn_certified_key(domain: &str, digest: &[u8]) -> Result<CertifiedKey> {
    if digest.len() != 32 {
        return Err(anyhow!(
            "acme tls-alpn digest must be 32 bytes, got {}",
            digest.len()
        ));
    }

    let mut params = CertificateParams::new(vec![domain.to_string()])
        .context("failed to initialize acme tls-alpn certificate params")?;
    params.distinguished_name = DistinguishedName::new();
    params
        .custom_extensions
        .push(CustomExtension::from_oid_content(
            &[1, 3, 6, 1, 5, 5, 7, 1, 31],
            acme_identifier_extension_content(digest),
        ));

    let key_pair = KeyPair::generate().context("failed generating acme tls-alpn key pair")?;
    let certificate = params
        .self_signed(&key_pair)
        .context("failed generating acme tls-alpn certificate")?;
    let certs = load_certs_from_pem(&certificate.pem())?;
    let key = load_private_key_from_pem(&key_pair.serialize_pem())?;
    let provider = rustls::crypto::ring::default_provider();
    let signing_key = provider
        .key_provider
        .load_private_key(key)
        .map_err(|error| anyhow!("failed loading in-memory acme tls-alpn private key: {error}"))?;
    Ok(CertifiedKey::new(certs, signing_key))
}

fn acme_identifier_extension_content(digest: &[u8]) -> Vec<u8> {
    let mut der = Vec::with_capacity(2 + digest.len());
    der.push(0x04);
    der.push(u8::try_from(digest.len()).unwrap_or(32));
    der.extend_from_slice(digest);
    der
}

async fn issue_managed_acme_certificate(
    tls: &crate::config::TlsConfig,
    challenges: &DashMap<String, String>,
    tls_alpn_certs: &DashMap<String, Arc<CertifiedKey>>,
) -> Result<()> {
    let cache_dir = tls.acme.cache_dir.clone();
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create acme cache dir {}", cache_dir.display()))?;
    let credentials_path = cache_dir.join("account.json");
    let directory_url = if tls.acme.directory_production {
        LetsEncrypt::Production.url().to_string()
    } else {
        LetsEncrypt::Staging.url().to_string()
    };

    let account = if credentials_path.exists() {
        let bytes = fs::read(&credentials_path)
            .with_context(|| format!("failed to read {}", credentials_path.display()))?;
        let credentials: instant_acme::AccountCredentials = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to decode {}", credentials_path.display()))?;
        Account::builder()
            .context("failed to build managed acme account client")?
            .from_credentials(credentials)
            .await
            .context("failed to restore managed acme account")?
    } else {
        let contact = format!("mailto:{}", tls.acme.email);
        let contacts = [contact.as_str()];
        let (account, credentials) = Account::builder()
            .context("failed to build managed acme account client")?
            .create(
                &NewAccount {
                    contact: &contacts,
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                directory_url,
                None,
            )
            .await
            .context("failed to create managed acme account")?;
        fs::write(
            &credentials_path,
            serde_json::to_vec_pretty(&credentials)
                .context("failed to serialize acme credentials")?,
        )
        .with_context(|| format!("failed to write {}", credentials_path.display()))?;
        account
    };

    let identifiers = tls
        .acme
        .domains
        .iter()
        .map(|domain| Identifier::Dns(domain.clone()))
        .collect::<Vec<_>>();
    let mut order = account
        .new_order(&NewOrder::new(&identifiers))
        .await
        .context("failed to create managed acme order")?;

    let mut inserted_tokens = Vec::new();
    let mut inserted_tls_domains = Vec::new();
    let mut inserted_dns_records = Vec::new();
    let dns_provider = if tls.acme.challenge == AcmeChallengeType::Dns01 {
        Some(DnsProvider::create(
            &tls.acme.dns.provider,
            tls.acme.dns.credentials.clone(),
        )?)
    } else {
        None
    };
    let result = async {
        let mut authorizations = order.authorizations();
        while let Some(authz) = authorizations.next().await {
            let mut authz = authz.context("failed to fetch acme authorization")?;
            let identifier = authz.identifier().to_string().to_ascii_lowercase();
            let mut challenge = match tls.acme.challenge {
                AcmeChallengeType::Http01 => authz
                    .challenge(ChallengeType::Http01)
                    .ok_or_else(|| anyhow!("acme server did not offer http-01 challenge"))?,
                AcmeChallengeType::TlsAlpn01 => authz
                    .challenge(ChallengeType::TlsAlpn01)
                    .ok_or_else(|| anyhow!("acme server did not offer tls-alpn-01 challenge"))?,
                AcmeChallengeType::Dns01 => authz
                    .challenge(ChallengeType::Dns01)
                    .ok_or_else(|| anyhow!("acme server did not offer dns-01 challenge"))?,
            };

            match tls.acme.challenge {
                AcmeChallengeType::Http01 => {
                    let token = challenge.token.clone();
                    let key_authorization = challenge.key_authorization().as_str().to_string();
                    challenges.insert(token.clone(), key_authorization);
                    inserted_tokens.push(token);
                }
                AcmeChallengeType::TlsAlpn01 => {
                    let certified = build_acme_tls_alpn_certified_key(
                        &identifier,
                        challenge.key_authorization().digest().as_ref(),
                    )?;
                    tls_alpn_certs.insert(identifier.clone(), Arc::new(certified));
                    inserted_tls_domains.push(identifier);
                }
                AcmeChallengeType::Dns01 => {
                    let provider = dns_provider
                        .as_ref()
                        .ok_or_else(|| anyhow!("managed dns-01 provider is not configured"))?;
                    let fqdn = acme_challenge_fqdn(&identifier);
                    let txt_value = challenge.key_authorization().dns_value();
                    let handle = provider
                        .upsert_txt_record(&fqdn, &txt_value)
                        .await
                        .with_context(|| {
                            format!("failed to publish dns-01 txt record for {fqdn}")
                        })?;
                    inserted_dns_records.push((provider.id().to_string(), handle));
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            challenge
                .set_ready()
                .await
                .context("failed to notify acme server that challenge is ready")?;
        }

        let retry = RetryPolicy::default().timeout(Duration::from_secs(120));
        let status = order
            .poll_ready(&retry)
            .await
            .context("timed out waiting for acme order readiness")?;
        if status != OrderStatus::Ready {
            return Err(anyhow!("acme order ended in unexpected state {status:?}"));
        }

        let private_key_pem = order
            .finalize()
            .await
            .context("failed to finalize managed acme order")?;
        let certificate_pem = order
            .poll_certificate(&retry)
            .await
            .context("failed to retrieve managed acme certificate")?;

        if let Some(parent) = tls.cert_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        if let Some(parent) = tls.key_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&tls.cert_path, certificate_pem)
            .with_context(|| format!("failed to write {}", tls.cert_path.display()))?;
        fs::write(&tls.key_path, private_key_pem)
            .with_context(|| format!("failed to write {}", tls.key_path.display()))?;

        Ok::<_, anyhow::Error>(())
    }
    .await;

    for token in inserted_tokens {
        challenges.remove(&token);
    }
    for domain in inserted_tls_domains {
        tls_alpn_certs.remove(&domain);
    }
    if let Some(provider) = dns_provider.as_ref() {
        for (_provider_id, handle) in inserted_dns_records {
            if let Err(error) = provider.delete_txt_record(&handle).await {
                tracing::warn!(?error, record = %handle.name, "failed to clean up dns-01 txt record");
            }
        }
    }

    result
}

async fn issue_on_demand_managed_certificate(
    tls: &crate::config::TlsConfig,
    domain: &str,
    challenges: &DashMap<String, String>,
    tls_alpn_certs: &DashMap<String, Arc<CertifiedKey>>,
    on_demand_certs: &DashMap<String, Arc<CertifiedKey>>,
) -> Result<()> {
    if on_demand_certs.contains_key(domain) {
        return Ok(());
    }
    let mut tls = tls.clone();
    tls.acme.domains = vec![domain.to_string()];
    let cache_dir = tls.acme.cache_dir.join("on-demand");
    fs::create_dir_all(&cache_dir).with_context(|| {
        format!(
            "failed to create on-demand cache dir {}",
            cache_dir.display()
        )
    })?;
    let cert_path = cache_dir.join(format!("{domain}.crt.pem"));
    let key_path = cache_dir.join(format!("{domain}.key.pem"));
    tls.cert_path = cert_path;
    tls.key_path = key_path;
    issue_managed_acme_certificate(&tls, challenges, tls_alpn_certs).await?;
    let certified = Arc::new(build_certified_key(&tls.cert_path, &tls.key_path)?);
    on_demand_certs.insert(domain.to_ascii_lowercase(), certified);
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
        if tls.mode != TlsMode::AcmeDnsExternal {
            issue.arg("--standalone");
        }
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

    if tls.mode == TlsMode::AcmeDnsExternal {
        for (name, value) in &acme.dns.credentials {
            issue.env(name.trim(), value);
        }
        issue.arg("--dns").arg(acme.dns.provider.trim());
    } else {
        match acme.challenge {
            AcmeChallengeType::TlsAlpn01 => {
                issue.arg("--alpn");
            }
            AcmeChallengeType::Http01 => {
                issue.arg("--standalone");
            }
            AcmeChallengeType::Dns01 => {
                for (name, value) in &acme.dns.credentials {
                    issue.env(name.trim(), value);
                }
                issue.arg("--dns").arg(acme.dns.provider.trim());
            }
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

    headers.insert(
        HOST,
        HeaderValue::from_str(host).context("invalid host header")?,
    );
    apply_forwarding_headers(&mut headers, host, remote_addr, scheme)?;

    Ok(headers)
}

fn finalize_http_response(
    request_headers: &HeaderMap,
    compression: &ResponseCompressionConfig,
    mut response: GatewayHttpResponse,
) -> Result<GatewayHttpResponse> {
    apply_streaming_response_headers(&mut response)?;
    if !compression.enabled || !response_allows_compression(&response) {
        return Ok(response);
    }
    let Some(encoding) = select_compression_encoding(request_headers, compression) else {
        return Ok(response);
    };
    if response.body.len() < compression.min_length
        || !content_type_matches(&response, &compression.content_types)
    {
        return Ok(response);
    }

    let (compressed, encoding_header) = match encoding {
        CompressionEncoding::Zstd => (
            zstd_bytes(&response.body)?,
            HeaderValue::from_static("zstd"),
        ),
        CompressionEncoding::Brotli => (
            brotli_bytes(&response.body)?,
            HeaderValue::from_static("br"),
        ),
        CompressionEncoding::Gzip => (
            gzip_bytes(&response.body)?,
            HeaderValue::from_static("gzip"),
        ),
    };
    response.body = Bytes::from(compressed);
    response.headers.retain(|(name, _)| name != CONTENT_LENGTH);
    response.headers.push((CONTENT_ENCODING, encoding_header));
    append_or_insert_header(&mut response.headers, VARY, "accept-encoding")?;
    response.headers.push((
        CONTENT_LENGTH,
        HeaderValue::from_str(&response.body.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    Ok(response)
}

fn apply_streaming_response_headers(response: &mut GatewayHttpResponse) -> Result<()> {
    if response.stream_body.is_none() && !response_content_type_is(response, "text/event-stream") {
        return Ok(());
    }

    append_header_token_once(&mut response.headers, CACHE_CONTROL, "no-cache")?;
    append_header_token_once(&mut response.headers, CACHE_CONTROL, "no-transform")?;
    response
        .headers
        .retain(|(name, _)| name != CONTENT_LENGTH && name != CONTENT_ENCODING);
    set_header(
        &mut response.headers,
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    Ok(())
}

fn gzip_bytes(body: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, body).context("failed to gzip response body")?;
    encoder.finish().context("failed to finalize gzip response")
}

fn brotli_bytes(body: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = CompressorWriter::new(Vec::new(), 4096, 5, 22);
    std::io::Write::write_all(&mut encoder, body)
        .context("failed to brotli-compress response body")?;
    Ok(encoder.into_inner())
}

fn zstd_bytes(body: &[u8]) -> Result<Vec<u8>> {
    zstd_encode_all(body, 3).context("failed to zstd-compress response body")
}

fn select_compression_encoding(
    headers: &HeaderMap,
    compression: &ResponseCompressionConfig,
) -> Option<CompressionEncoding> {
    let accepted = headers.get(ACCEPT_ENCODING)?.to_str().ok()?;
    let allow_zstd = compression.algorithms.contains(&CompressionAlgorithm::Zstd);
    let allow_br = compression
        .algorithms
        .contains(&CompressionAlgorithm::Brotli);
    let allow_gzip = compression.algorithms.contains(&CompressionAlgorithm::Gzip);
    if !allow_zstd && !allow_br && !allow_gzip {
        return None;
    }

    let mut zstd_q: Option<f32> = None;
    let mut br_q: Option<f32> = None;
    let mut gzip_q: Option<f32> = None;
    let mut wildcard_q: Option<f32> = None;

    for item in accepted.split(',') {
        let mut segments = item.split(';');
        let encoding = segments.next()?.trim().to_ascii_lowercase();
        if encoding.is_empty() {
            continue;
        }
        let mut q = 1.0f32;
        for param in segments {
            let mut kv = param.trim().splitn(2, '=');
            let Some(k) = kv.next() else {
                continue;
            };
            let Some(v) = kv.next() else {
                continue;
            };
            if k.trim().eq_ignore_ascii_case("q") {
                if let Ok(parsed) = v.trim().parse::<f32>() {
                    q = parsed.clamp(0.0, 1.0);
                }
            }
        }
        if q <= 0.0 {
            continue;
        }

        match encoding.as_str() {
            "zstd" => zstd_q = Some(zstd_q.map_or(q, |existing| existing.max(q))),
            "br" => br_q = Some(br_q.map_or(q, |existing| existing.max(q))),
            "gzip" => gzip_q = Some(gzip_q.map_or(q, |existing| existing.max(q))),
            "*" => wildcard_q = Some(wildcard_q.map_or(q, |existing| existing.max(q))),
            _ => {}
        }
    }

    let zstd = if allow_zstd {
        zstd_q.or(wildcard_q).unwrap_or(0.0)
    } else {
        0.0
    };
    let br = if allow_br {
        br_q.or(wildcard_q).unwrap_or(0.0)
    } else {
        0.0
    };
    let gzip = if allow_gzip {
        gzip_q.or(wildcard_q).unwrap_or(0.0)
    } else {
        0.0
    };
    if zstd <= 0.0 && br <= 0.0 && gzip <= 0.0 {
        return None;
    }

    if zstd >= br && zstd >= gzip {
        Some(CompressionEncoding::Zstd)
    } else if br >= gzip {
        Some(CompressionEncoding::Brotli)
    } else {
        Some(CompressionEncoding::Gzip)
    }
}

fn response_allows_compression(response: &GatewayHttpResponse) -> bool {
    !response.body.is_empty()
        && response.stream_body.is_none()
        && response.status != StatusCode::NO_CONTENT
        && response.status != StatusCode::NOT_MODIFIED
        && !response_content_type_is(response, "text/event-stream")
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

fn response_content_type_is(response: &GatewayHttpResponse, expected: &str) -> bool {
    response
        .headers
        .iter()
        .find(|(name, _)| name == CONTENT_TYPE)
        .and_then(|(_, value)| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or("").trim())
        .is_some_and(|content_type| content_type.eq_ignore_ascii_case(expected))
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

fn append_header_token_once(
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
        if existing_value
            .split(',')
            .any(|token| token.trim().eq_ignore_ascii_case(value))
        {
            return Ok(());
        }
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

fn set_header(headers: &mut Vec<(HeaderName, HeaderValue)>, name: HeaderName, value: HeaderValue) {
    headers.retain(|(header_name, _)| *header_name != name);
    headers.push((name, value));
}

fn cache_lookup_key(
    config: &ResponseCacheConfig,
    method: &Method,
    host: &str,
    uri: &Uri,
    headers: &HeaderMap,
) -> Option<String> {
    if !config.enabled
        || config.behavior == CacheBehavior::Bypass
        || config.behavior == CacheBehavior::NoCache
        || *method != Method::GET
        || headers.contains_key(AUTHORIZATION)
        || headers.contains_key(COOKIE)
    {
        return None;
    }

    let mut parts = vec![
        method.as_str().to_string(),
        host.to_ascii_lowercase(),
        uri.path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/")
            .to_string(),
    ];
    for header_name in &config.vary_headers {
        let value = headers
            .get(header_name.as_str())
            .and_then(|item| item.to_str().ok())
            .unwrap_or("");
        parts.push(format!("{header_name}={value}"));
    }
    let key = parts.join(":");
    if config.key_prefix.is_empty() {
        Some(key)
    } else {
        Some(format!("{}:{}", config.key_prefix, key))
    }
}

fn cache_storage_key(config: &ResponseCacheConfig, key: &str) -> String {
    format!("{}:{key}", config.zone)
}

fn cache_zone_max_entries(config: &GatewayConfig, zone: &str) -> usize {
    config
        .services
        .cache_zones
        .iter()
        .find(|item| item.name == zone)
        .map(|item| item.max_entries)
        .unwrap_or(4096)
}

fn cache_disk_path(
    config: &GatewayConfig,
    cache: &ResponseCacheConfig,
    storage_key: &str,
) -> Option<PathBuf> {
    let zone = config
        .services
        .cache_zones
        .iter()
        .find(|item| item.name == cache.zone)?;
    if zone.disk_path.as_os_str().is_empty() {
        return None;
    }
    let digest = md5::compute(storage_key.as_bytes());
    Some(zone.disk_path.join(format!("{:x}.json", digest)))
}

fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

async fn rewrite_ftp_passive_reply(
    raw_reply: &[u8],
    config: &crate::config::FtpConfig,
    bind_ip: IpAddr,
    public_ip: IpAddr,
    upstream_control_ip: IpAddr,
    remote_addr: SocketAddr,
) -> Result<Option<String>> {
    let reply = String::from_utf8_lossy(raw_reply);

    if let Some(upstream_addr) = parse_ftp_pasv_addr(&reply) {
        let Some((listener, port)) =
            bind_ftp_passive_listener(bind_ip, config.passive_port_start, config.passive_port_end)
                .await?
        else {
            tracing::warn!(%remote_addr, "ftp passive listener pool exhausted");
            return Ok(Some(
                "425 Can't open passive data connection.\r\n".to_string(),
            ));
        };
        tracing::info!(
            %remote_addr,
            %upstream_addr,
            channel = "passive",
            "ftp data channel bridge starting"
        );
        spawn_ftp_passive_bridge(listener, upstream_addr, remote_addr);

        let IpAddr::V4(public_v4) = public_ip else {
            tracing::warn!(%remote_addr, "ftp PASV rewrite requires IPv4 public_ip/local bind");
            return Ok(Some(
                "425 Can't expose passive IPv6 address via PASV.\r\n".to_string(),
            ));
        };
        let octets = public_v4.octets();
        let p1 = port / 256;
        let p2 = port % 256;
        return Ok(Some(format!(
            "227 Entering Passive Mode ({},{},{},{},{},{}).\r\n",
            octets[0], octets[1], octets[2], octets[3], p1, p2
        )));
    }

    if let Some(upstream_port) = parse_ftp_epsv_port(&reply) {
        let Some((listener, port)) =
            bind_ftp_passive_listener(bind_ip, config.passive_port_start, config.passive_port_end)
                .await?
        else {
            tracing::warn!(%remote_addr, "ftp EPSV listener pool exhausted");
            return Ok(Some(
                "425 Can't open passive data connection.\r\n".to_string(),
            ));
        };
        let upstream_addr = SocketAddr::new(upstream_control_ip, upstream_port);
        tracing::info!(
            %remote_addr,
            %upstream_addr,
            channel = "passive-epsv",
            "ftp data channel bridge starting"
        );
        spawn_ftp_passive_bridge(listener, upstream_addr, remote_addr);
        return Ok(Some(format!(
            "229 Entering Extended Passive Mode (|||{}|)\r\n",
            port
        )));
    }

    Ok(None)
}

async fn rewrite_ftp_active_command(
    raw_command: &[u8],
    config: &crate::config::FtpConfig,
    bind_ip: IpAddr,
    public_ip: IpAddr,
    remote_addr: SocketAddr,
) -> Result<Option<String>> {
    let command = String::from_utf8_lossy(raw_command);

    if let Some(target_addr) = parse_ftp_port_command(&command) {
        let Some((listener, port)) =
            bind_ftp_passive_listener(bind_ip, config.passive_port_start, config.passive_port_end)
                .await?
        else {
            return Err(anyhow!("ftp active listener pool exhausted"));
        };
        tracing::info!(
            %remote_addr,
            %target_addr,
            channel = "active-port",
            "ftp data channel bridge starting"
        );
        spawn_ftp_active_bridge(listener, target_addr, remote_addr);

        let IpAddr::V4(ip) = public_ip else {
            return Err(anyhow!(
                "ftp PORT rewrite requires an IPv4 public_ip/local bind"
            ));
        };
        let octets = ip.octets();
        let p1 = port / 256;
        let p2 = port % 256;
        return Ok(Some(format!(
            "PORT {},{},{},{},{},{}\r\n",
            octets[0], octets[1], octets[2], octets[3], p1, p2
        )));
    }

    if let Some((af, target_addr)) = parse_ftp_eprt_command(&command) {
        let Some((listener, port)) =
            bind_ftp_passive_listener(bind_ip, config.passive_port_start, config.passive_port_end)
                .await?
        else {
            return Err(anyhow!("ftp active listener pool exhausted"));
        };
        tracing::info!(
            %remote_addr,
            %target_addr,
            channel = "active-eprt",
            "ftp data channel bridge starting"
        );
        spawn_ftp_active_bridge(listener, target_addr, remote_addr);

        let advertised_addr = match (af, public_ip) {
            (1, IpAddr::V4(ip)) => ip.to_string(),
            (2, IpAddr::V6(ip)) => ip.to_string(),
            (1, IpAddr::V6(_)) => {
                return Err(anyhow!(
                    "ftp EPRT IPv4 rewrite requires an IPv4 public_ip/local bind"
                ))
            }
            (2, IpAddr::V4(_)) => {
                return Err(anyhow!(
                    "ftp EPRT IPv6 rewrite requires an IPv6 public_ip/local bind"
                ))
            }
            _ => return Ok(None),
        };

        return Ok(Some(format!("EPRT |{af}|{advertised_addr}|{port}|\r\n")));
    }

    Ok(None)
}

fn parse_ftp_command_verb(line: &str) -> Option<String> {
    line.split_whitespace()
        .next()
        .map(|verb| verb.to_ascii_uppercase())
}

fn ftp_transfer_verb(verb: &str) -> bool {
    matches!(
        verb,
        "RETR" | "STOR" | "STOU" | "APPE" | "LIST" | "NLST" | "MLSD" | "MLST"
    )
}

fn ftp_user_policy<'a>(
    config: &'a crate::config::FtpConfig,
    user: &str,
) -> Option<&'a FtpUserPolicy> {
    if user.is_empty() {
        return None;
    }
    config
        .user_policies
        .iter()
        .find(|policy| policy.user.eq_ignore_ascii_case(user))
}

fn ftp_command_allowed_for_user(config: &crate::config::FtpConfig, verb: &str, user: &str) -> bool {
    if let Some(policy) = ftp_user_policy(config, user) {
        if !policy.command_allow.is_empty()
            && !policy
                .command_allow
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(verb))
        {
            return false;
        }
        if policy
            .command_deny
            .iter()
            .any(|denied| denied.eq_ignore_ascii_case(verb))
        {
            return false;
        }
    }
    ftp_command_allowed(config, verb)
}

fn ftp_transfer_allowed_for_user(
    config: &crate::config::FtpConfig,
    verb: &str,
    user: &str,
) -> bool {
    if let Some(policy) = ftp_user_policy(config, user) {
        if !policy.transfer_allow.is_empty()
            && !policy
                .transfer_allow
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(verb))
        {
            return false;
        }
        if policy
            .transfer_deny
            .iter()
            .any(|denied| denied.eq_ignore_ascii_case(verb))
        {
            return false;
        }
    }
    if !config.transfer_allow.is_empty()
        && !config
            .transfer_allow
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(verb))
    {
        return false;
    }
    !config
        .transfer_deny
        .iter()
        .any(|denied| denied.eq_ignore_ascii_case(verb))
}

fn ftp_command_allowed(config: &crate::config::FtpConfig, verb: &str) -> bool {
    if !config.command_allow.is_empty()
        && !config
            .command_allow
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(verb))
    {
        return false;
    }
    !config
        .command_deny
        .iter()
        .any(|denied| denied.eq_ignore_ascii_case(verb))
}

fn parse_ftp_pasv_addr(reply: &str) -> Option<SocketAddr> {
    let start = reply.find('(')?;
    let end = reply[start + 1..].find(')')? + start + 1;
    let numbers = reply[start + 1..end]
        .split(',')
        .map(|part| part.trim().parse::<u16>().ok())
        .collect::<Option<Vec<_>>>()?;
    if numbers.len() != 6 {
        return None;
    }
    let ip = IpAddr::V4(std::net::Ipv4Addr::new(
        numbers[0].try_into().ok()?,
        numbers[1].try_into().ok()?,
        numbers[2].try_into().ok()?,
        numbers[3].try_into().ok()?,
    ));
    let port = numbers[4].saturating_mul(256).saturating_add(numbers[5]);
    Some(SocketAddr::new(ip, port))
}

fn parse_ftp_port_command(command: &str) -> Option<SocketAddr> {
    let payload = command.trim().strip_prefix("PORT ")?;
    let numbers = payload
        .split(',')
        .map(|part| part.trim().parse::<u16>().ok())
        .collect::<Option<Vec<_>>>()?;
    if numbers.len() != 6 {
        return None;
    }
    let ip = IpAddr::V4(std::net::Ipv4Addr::new(
        numbers[0].try_into().ok()?,
        numbers[1].try_into().ok()?,
        numbers[2].try_into().ok()?,
        numbers[3].try_into().ok()?,
    ));
    let port = numbers[4].saturating_mul(256).saturating_add(numbers[5]);
    Some(SocketAddr::new(ip, port))
}

fn parse_ftp_eprt_command(command: &str) -> Option<(u8, SocketAddr)> {
    let payload = command.trim().strip_prefix("EPRT ")?;
    let delimiter = payload.chars().next()?;
    let segments = payload[1..].split(delimiter).collect::<Vec<_>>();
    if segments.len() < 3 {
        return None;
    }
    let af = segments[0].trim().parse::<u8>().ok()?;
    let ip = segments[1].trim().parse::<IpAddr>().ok()?;
    let port = segments[2].trim().parse::<u16>().ok()?;
    Some((af, SocketAddr::new(ip, port)))
}

fn parse_ftp_epsv_port(reply: &str) -> Option<u16> {
    let start = reply.find('(')?;
    let end = reply[start + 1..].find(')')? + start + 1;
    let payload = &reply[start + 1..end];
    let digits = payload
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u16>().ok()
    }
}

async fn bind_ftp_passive_listener(
    bind_ip: IpAddr,
    start: u16,
    end: u16,
) -> Result<Option<(TcpListener, u16)>> {
    let candidate_ip = match bind_ip {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
        ip => ip,
    };

    for port in start..=end {
        match TcpListener::bind(SocketAddr::new(candidate_ip, port)).await {
            Ok(listener) => return Ok(Some((listener, port))),
            Err(_) => continue,
        }
    }

    Ok(None)
}

fn spawn_ftp_passive_bridge(
    listener: TcpListener,
    upstream_addr: SocketAddr,
    remote_addr: SocketAddr,
) {
    tokio::spawn(async move {
        let result = async {
            let (mut downstream_data, _) = listener
                .accept()
                .await
                .context("failed accepting ftp passive data connection")?;
            let mut upstream_data = TcpStream::connect(upstream_addr).await.with_context(|| {
                format!("failed connecting ftp passive upstream {upstream_addr}")
            })?;
            copy_bidirectional(&mut downstream_data, &mut upstream_data)
                .await
                .context("ftp passive data relay failed")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;

        match result {
            Ok(()) => {
                tracing::info!(%remote_addr, %upstream_addr, channel = "passive", "ftp data channel bridge closed")
            }
            Err(error) => {
                tracing::warn!(?error, %remote_addr, %upstream_addr, channel = "passive", "ftp passive bridge failed")
            }
        }
    });
}

fn spawn_ftp_active_bridge(
    listener: TcpListener,
    client_target: SocketAddr,
    remote_addr: SocketAddr,
) {
    tokio::spawn(async move {
        let result = async {
            let (mut server_data, _) = listener
                .accept()
                .await
                .context("failed accepting ftp active data connection from upstream")?;
            let mut client_data = TcpStream::connect(client_target).await.with_context(|| {
                format!("failed connecting ftp active client target {client_target}")
            })?;
            copy_bidirectional(&mut server_data, &mut client_data)
                .await
                .context("ftp active data relay failed")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;

        match result {
            Ok(()) => {
                tracing::info!(%remote_addr, %client_target, channel = "active", "ftp data channel bridge closed")
            }
            Err(error) => {
                tracing::warn!(?error, %remote_addr, %client_target, channel = "active", "ftp active bridge failed")
            }
        }
    });
}

fn effective_edge_ttl_secs(
    cache: &ResponseCacheConfig,
    headers: &[(HeaderName, HeaderValue)],
) -> u64 {
    if cache.behavior == CacheBehavior::Override {
        return cache.ttl_secs.max(1);
    }
    parse_cache_control_max_age(headers)
        .unwrap_or(cache.ttl_secs)
        .max(1)
}

fn parse_cache_control_max_age(headers: &[(HeaderName, HeaderValue)]) -> Option<u64> {
    let value = headers
        .iter()
        .find(|(name, _)| name == CACHE_CONTROL)
        .and_then(|(_, value)| value.to_str().ok())?;
    for part in value.split(',') {
        let part = part.trim().to_ascii_lowercase();
        if let Some(max_age) = part.strip_prefix("max-age=") {
            return max_age.parse().ok();
        }
        if part == "no-store" || part == "no-cache" || part == "private" {
            return Some(0);
        }
    }
    None
}

fn apply_cache_response_headers(
    cache: &ResponseCacheConfig,
    response: &mut GatewayHttpResponse,
    status: &str,
) -> Result<()> {
    append_or_insert_header(
        &mut response.headers,
        HeaderName::from_static("x-cache"),
        status,
    )?;
    if cache.emit_cdn_cache_control {
        let edge_ttl = cache.ttl_secs.max(1);
        append_or_insert_header(
            &mut response.headers,
            HeaderName::from_static("cdn-cache-control"),
            &format!("max-age={edge_ttl}"),
        )?;
    }
    if cache.browser_ttl_secs > 0 {
        response.headers.retain(|(name, _)| name != CACHE_CONTROL);
        append_or_insert_header(
            &mut response.headers,
            CACHE_CONTROL,
            &format!("public, max-age={}", cache.browser_ttl_secs),
        )?;
    } else if cache.behavior == CacheBehavior::NoCache {
        append_or_insert_header(&mut response.headers, CACHE_CONTROL, "no-cache")?;
    }
    Ok(())
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum AdminTransport {
    Loopback,
    GatewayHttp,
    GatewayHttps { host: String },
}

fn admin_request_is_write(method: &Method, path: &str) -> bool {
    path != "/healthz" && *method == Method::POST
}

fn map_admin_gateway_path(admin: &AdminConfig, request_path: &str) -> Option<String> {
    if !admin.https.enabled {
        return None;
    }
    let prefix = crate::config::normalize_admin_https_path_prefix(&admin.https.path_prefix);
    if request_path == prefix.as_str() {
        return Some("/".to_string());
    }
    let nested = format!("{prefix}/");
    if request_path == nested.as_str() {
        return Some("/".to_string());
    }
    let rest = request_path.strip_prefix(&nested)?;
    Some(format!("/{rest}"))
}

fn admin_https_host_allowed(admin: &AdminConfig, host: &str) -> bool {
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    if admin.https.hosts.is_empty() {
        return true;
    }
    admin
        .https
        .hosts
        .iter()
        .any(|pattern| crate::config::domain_matches_pattern(&host, pattern))
}

fn check_admin_transport_access(
    config: &GatewayConfig,
    transport: &AdminTransport,
    remote_addr: SocketAddr,
) -> Option<Response<Full<Bytes>>> {
    match transport {
        AdminTransport::Loopback => {
            if config.admin.loopback_only
                && !admin_loopback_only_allows(remote_addr, &config.admin.bind)
            {
                return Some(text_response(
                    StatusCode::FORBIDDEN,
                    "admin API is restricted to loopback clients",
                ));
            }
        }
        AdminTransport::GatewayHttp => {
            return Some(text_response(
                StatusCode::FORBIDDEN,
                "admin API on the public gateway requires HTTPS; configure TLS first, then call the HTTPS admin path",
            ));
        }
        AdminTransport::GatewayHttps { host } => {
            if !config.admin.https.enabled {
                return Some(text_response(
                    StatusCode::FORBIDDEN,
                    "admin https API is disabled",
                ));
            }
            if !admin_https_host_allowed(&config.admin, host) {
                return Some(text_response(
                    StatusCode::FORBIDDEN,
                    "admin host is not allowed for HTTPS admin API",
                ));
            }
        }
    }
    None
}

fn check_admin_mutation_access(
    config: &GatewayConfig,
    transport: &AdminTransport,
) -> Option<Response<Full<Bytes>>> {
    if !config.admin.enable_write_ops {
        return Some(text_response(
            StatusCode::FORBIDDEN,
            "write operations disabled",
        ));
    }
    match transport {
        AdminTransport::Loopback => {}
        AdminTransport::GatewayHttp => {
            return Some(text_response(
                StatusCode::FORBIDDEN,
                "admin write operations require HTTPS on the public gateway",
            ));
        }
        AdminTransport::GatewayHttps { .. } => {
            if !gateway_tls_material_ready(config) {
                return Some(text_response(
                    StatusCode::FORBIDDEN,
                    "TLS certificate material must exist before admin write operations over HTTPS; bootstrap ACME/TLS on loopback first",
                ));
            }
        }
    }
    None
}

fn is_authorized(header: Option<&HeaderValue>, admin: &AdminConfig) -> bool {
    let Some(header) = header else {
        return false;
    };

    let Ok(value) = header.to_str() else {
        return false;
    };

    if let Some(token) = value.strip_prefix("Bearer ") {
        return (!admin.bearer_token.is_empty() && token == admin.bearer_token)
            || verify_admin_session_token(token, admin);
    }

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
struct AdminLoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdminSessionClaims {
    sub: String,
    exp: u64,
    iat: u64,
    scope: String,
}

type HmacSha256 = Hmac<Sha256>;

const ADMIN_SESSION_TTL_SECS: u64 = 12 * 60 * 60;

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn admin_session_signing_key(admin: &AdminConfig) -> Vec<u8> {
    format!(
        "proxysss-admin-session:{}:{}:{}",
        admin.username, admin.password, admin.bearer_token
    )
    .into_bytes()
}

fn sign_admin_session_payload(payload: &str, admin: &AdminConfig) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(&admin_session_signing_key(admin)).ok()?;
    mac.update(payload.as_bytes());
    Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn issue_admin_session_token(admin: &AdminConfig) -> Option<(String, u64)> {
    let now = now_unix_secs();
    let expires_at = now.saturating_add(ADMIN_SESSION_TTL_SECS);
    let claims = AdminSessionClaims {
        sub: admin.username.clone(),
        exp: expires_at,
        iat: now,
        scope: "admin".to_string(),
    };
    let payload = serde_json::to_vec(&claims).ok()?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    let signature = sign_admin_session_payload(&payload, admin)?;
    Some((format!("{payload}.{signature}"), expires_at))
}

fn verify_admin_session_token(token: &str, admin: &AdminConfig) -> bool {
    let Some((payload, signature)) = token.split_once('.') else {
        return false;
    };
    let Some(expected) = sign_admin_session_payload(payload, admin) else {
        return false;
    };
    if signature != expected {
        return false;
    }
    let Ok(decoded) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) else {
        return false;
    };
    let Ok(claims) = serde_json::from_slice::<AdminSessionClaims>(&decoded) else {
        return false;
    };
    claims.sub == admin.username && claims.scope == "admin" && claims.exp > now_unix_secs()
}

#[derive(Debug, Deserialize)]
struct PluginUnloadRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct NamedDeleteRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AutoHttpsUpsertRequest {
    domains: Vec<String>,
    email: String,
    #[serde(default)]
    production: bool,
}

#[derive(Debug, Deserialize)]
struct WildcardTlsUpsertRequest {
    domains: Vec<String>,
    email: String,
    dns_provider: String,
    #[serde(default)]
    credentials: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct OnDemandTlsUpsertRequest {
    enabled: bool,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    max_active_certs: Option<usize>,
    #[serde(default)]
    max_issues_per_hour: Option<u32>,
    #[serde(default)]
    ask_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SniCertificateUpsertRequest {
    domains: Vec<String>,
    #[serde(default)]
    cert_path: Option<PathBuf>,
    #[serde(default)]
    key_path: Option<PathBuf>,
    #[serde(default)]
    cert_pem: Option<String>,
    #[serde(default)]
    key_pem: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SniCertificateDeleteRequest {
    #[serde(default)]
    cert_path: Option<String>,
    #[serde(default)]
    domain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileCloudUpsertRequest {
    enabled: bool,
    path_prefix: String,
    root: PathBuf,
    #[serde(default)]
    password: String,
    title: String,
    #[serde(default = "default_true")]
    allow_upload: bool,
    #[serde(default = "default_true")]
    allow_delete: bool,
    #[serde(default = "default_true")]
    allow_mkdir: bool,
    #[serde(default = "default_true")]
    allow_move: bool,
    #[serde(default)]
    max_upload_bytes: Option<u64>,
    #[serde(default)]
    cdn_cache_secs: Option<u64>,
    #[serde(default)]
    session_ttl_secs: Option<u64>,
    #[serde(default)]
    require_auth_for_download: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct UpstreamToggleRequest {
    key: String,
    #[serde(default)]
    reason: Option<String>,
}

struct DomainRouteUpsertResult {
    action: &'static str,
    route: DomainRouteConfig,
}

struct ReverseProxyRouteUpsertResult {
    action: &'static str,
    route: ReverseProxyRouteConfig,
}

struct TcpListenerUpsertResult {
    action: &'static str,
    listener: TcpListenerConfig,
}

struct UdpListenerUpsertResult {
    action: &'static str,
    listener: UdpListenerConfig,
}

struct StreamRouteUpsertResult {
    action: &'static str,
    route: StreamRouteConfig,
}

struct SniCertificateUpsertResult {
    action: &'static str,
    certificate: TlsCertificateConfig,
}

fn gateway_tls_material_ready(config: &GatewayConfig) -> bool {
    (config.http.tls.cert_path.exists() && config.http.tls.key_path.exists())
        || config
            .http
            .tls
            .certificates
            .iter()
            .any(|cert| cert.cert_path.exists() && cert.key_path.exists())
}

#[derive(Debug, Deserialize)]
struct BlacklistMutationRequest {
    ip: String,
    #[serde(default)]
    ban_secs: Option<u64>,
}

fn tls_admin_summary(config: &GatewayConfig) -> serde_json::Value {
    let tls = &config.http.tls;
    let mode = serde_json::to_value(tls.mode).unwrap_or(serde_json::Value::Null);
    let challenge = serde_json::to_value(tls.acme.challenge).unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "mode": mode,
        "challenge": challenge,
        "server_name": tls.server_name,
        "cert_path": tls.cert_path.display().to_string(),
        "key_path": tls.key_path.display().to_string(),
        "cert_exists": tls.cert_path.exists(),
        "key_exists": tls.key_path.exists(),
        "auto_https": {
            "enabled": tls.auto_https.enabled,
            "domains": tls.auto_https.domains,
            "email": tls.auto_https.email,
            "production": tls.auto_https.production,
        },
        "acme": {
            "email": tls.acme.email,
            "domains": tls.acme.domains,
            "directory_production": tls.acme.directory_production,
            "dns_provider": tls.acme.dns.provider,
            "dns_credentials_configured": !tls.acme.dns.credentials.is_empty(),
        },
        "domain_routes_count": config.services.domain_routes.len(),
        "on_demand": {
            "enabled": tls.on_demand.enabled,
            "allow": tls.on_demand.allow,
            "max_active_certs": tls.on_demand.max_active_certs,
            "max_issues_per_hour": tls.on_demand.max_issues_per_hour,
            "ask_url": tls.on_demand.ask_url,
        },
        "sni_certificates": tls.certificates.iter().map(|cert| {
            serde_json::json!({
                "domains": cert.domains,
                "cert_path": cert.cert_path.display().to_string(),
                "key_path": cert.key_path.display().to_string(),
                "cert_exists": cert.cert_path.exists(),
                "key_exists": cert.key_path.exists(),
            })
        }).collect::<Vec<_>>(),
        "write_ops_enabled": config.admin.enable_write_ops,
        "expose_config": config.admin.expose_config,
        "managed_acme_zero_external_deps": true,
        "https_api": {
            "enabled": config.admin.https.enabled,
            "path_prefix": crate::config::normalize_admin_https_path_prefix(&config.admin.https.path_prefix),
            "hosts": config.admin.https.hosts,
            "tls_ready": gateway_tls_material_ready(config),
            "loopback_bind": config.admin.bind,
            "agent_note": "Bootstrap TLS/ACME on loopback admin; after cert material exists, drive automation over HTTPS at path_prefix/v1/*",
        },
    })
}

fn filecloud_admin_summary(config: &GatewayConfig) -> serde_json::Value {
    let fc = &config.services.filecloud;
    serde_json::json!({
        "enabled": fc.enabled,
        "path_prefix": fc.path_prefix,
        "root": fc.root.display().to_string(),
        "title": fc.title,
        "password_configured": !fc.password.trim().is_empty(),
        "allow_upload": fc.allow_upload,
        "allow_delete": fc.allow_delete,
        "allow_mkdir": fc.allow_mkdir,
        "allow_move": fc.allow_move,
        "max_upload_bytes": fc.max_upload_bytes,
        "cdn_cache_secs": fc.cdn_cache_secs,
        "session_ttl_secs": fc.session_ttl_secs,
        "require_auth_for_download": fc.require_auth_for_download,
        "ui_url": if fc.enabled { fc.path_prefix.clone() } else { String::new() },
        "write_ops_enabled": config.admin.enable_write_ops,
    })
}

fn sanitize_config(config: &GatewayConfig) -> serde_json::Value {
    let mut value = serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({}));

    if let Some(admin) = value.get_mut("admin").and_then(|item| item.as_object_mut()) {
        admin.insert(
            "password".to_string(),
            serde_json::Value::String("***".to_string()),
        );
        admin.insert(
            "bearer_token".to_string(),
            serde_json::Value::String("***".to_string()),
        );
    }
    if let Some(credentials) = value
        .get_mut("http")
        .and_then(|item| item.get_mut("tls"))
        .and_then(|item| item.get_mut("acme"))
        .and_then(|item| item.get_mut("dns"))
        .and_then(|item| item.get_mut("credentials"))
        .and_then(|item| item.as_object_mut())
    {
        for value in credentials.values_mut() {
            *value = serde_json::Value::String("***".to_string());
        }
    }
    if let Some(password) = value
        .get_mut("services")
        .and_then(|item| item.get_mut("filecloud"))
        .and_then(|item| item.get_mut("password"))
    {
        *password = serde_json::Value::String("***".to_string());
    }

    value
}

fn upsert_domain_route_config(
    routes: &mut Vec<DomainRouteConfig>,
    route: DomainRouteConfig,
) -> &'static str {
    if let Some(existing) = routes.iter_mut().find(|item| item.name == route.name) {
        *existing = route;
        "updated"
    } else {
        routes.push(route);
        "created"
    }
}

fn upsert_reverse_proxy_route_config(
    routes: &mut Vec<ReverseProxyRouteConfig>,
    route: ReverseProxyRouteConfig,
) -> &'static str {
    if let Some(existing) = routes.iter_mut().find(|item| item.name == route.name) {
        *existing = route;
        "updated"
    } else {
        routes.push(route);
        "created"
    }
}

fn upsert_tcp_listener_config(
    listeners: &mut Vec<TcpListenerConfig>,
    listener: TcpListenerConfig,
) -> &'static str {
    if let Some(existing) = listeners.iter_mut().find(|item| item.name == listener.name) {
        *existing = listener;
        "updated"
    } else {
        listeners.push(listener);
        "created"
    }
}

fn upsert_udp_listener_config(
    listeners: &mut Vec<UdpListenerConfig>,
    listener: UdpListenerConfig,
) -> &'static str {
    if let Some(existing) = listeners.iter_mut().find(|item| item.name == listener.name) {
        *existing = listener;
        "updated"
    } else {
        listeners.push(listener);
        "created"
    }
}

fn upsert_stream_route_config(
    routes: &mut Vec<StreamRouteConfig>,
    route: StreamRouteConfig,
) -> &'static str {
    if let Some(existing) = routes.iter_mut().find(|item| item.name == route.name) {
        *existing = route;
        "updated"
    } else {
        routes.push(route);
        "created"
    }
}

fn render_config_with_upserted_domain_route(
    original: &str,
    route: &DomainRouteConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;

    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;

    let services_key = serde_yaml::Value::String("services".to_string());
    if !root.contains_key(&services_key) {
        root.insert(
            services_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let services = root
        .get_mut(&services_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("services must be a mapping"))?;

    let routes_key = serde_yaml::Value::String("domain_routes".to_string());
    if !services.contains_key(&routes_key) {
        services.insert(routes_key.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    let routes = services
        .get_mut(&routes_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("services.domain_routes must be a sequence"))?;

    let route_value = serde_yaml::to_value(route).context("failed to serialize domain route")?;
    if let Some(existing) = routes.iter_mut().find(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == route.name)
            .unwrap_or(false)
    }) {
        *existing = route_value;
    } else {
        routes.push(route_value);
    }

    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_upserted_reverse_proxy_route(
    original: &str,
    route: &ReverseProxyRouteConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;

    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;

    let services_key = serde_yaml::Value::String("services".to_string());
    if !root.contains_key(&services_key) {
        root.insert(
            services_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let services = root
        .get_mut(&services_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("services must be a mapping"))?;

    let reverse_key = serde_yaml::Value::String("reverse_proxy".to_string());
    if !services.contains_key(&reverse_key) {
        services.insert(
            reverse_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let reverse_proxy = services
        .get_mut(&reverse_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("services.reverse_proxy must be a mapping"))?;

    let routes_key = serde_yaml::Value::String("routes".to_string());
    if !reverse_proxy.contains_key(&routes_key) {
        reverse_proxy.insert(routes_key.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    let routes = reverse_proxy
        .get_mut(&routes_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("services.reverse_proxy.routes must be a sequence"))?;

    let route_value =
        serde_yaml::to_value(route).context("failed to serialize reverse proxy route")?;
    if let Some(existing) = routes.iter_mut().find(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == route.name)
            .unwrap_or(false)
    }) {
        *existing = route_value;
    } else {
        routes.push(route_value);
    }

    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_domain_route(original: &str, name: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let routes = value
        .get_mut("services")
        .and_then(|item| item.get_mut("domain_routes"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("services.domain_routes must be a sequence"))?;
    let before = routes.len();
    routes.retain(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value != name)
            .unwrap_or(true)
    });
    if routes.len() == before {
        return Err(anyhow!("domain route {name} not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_reverse_proxy_route(original: &str, name: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let routes = value
        .get_mut("services")
        .and_then(|item| item.get_mut("reverse_proxy"))
        .and_then(|item| item.get_mut("routes"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("services.reverse_proxy.routes must be a sequence"))?;
    let before = routes.len();
    routes.retain(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value != name)
            .unwrap_or(true)
    });
    if routes.len() == before {
        return Err(anyhow!("reverse proxy route {name} not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_auto_https(
    original: &str,
    payload: &AutoHttpsUpsertRequest,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;
    let http_key = serde_yaml::Value::String("http".to_string());
    if !root.contains_key(&http_key) {
        root.insert(
            http_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let http = root
        .get_mut(&http_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http must be a mapping"))?;
    let tls_key = serde_yaml::Value::String("tls".to_string());
    if !http.contains_key(&tls_key) {
        http.insert(
            tls_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tls = http
        .get_mut(&tls_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http.tls must be a mapping"))?;
    tls.insert(
        serde_yaml::Value::String("mode".to_string()),
        serde_yaml::Value::String("acme_managed".to_string()),
    );
    let auto_https = serde_yaml::Mapping::from_iter([
        (
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(true),
        ),
        (
            serde_yaml::Value::String("domains".to_string()),
            serde_yaml::Value::Sequence(
                payload
                    .domains
                    .iter()
                    .map(|domain| serde_yaml::Value::String(domain.clone()))
                    .collect(),
            ),
        ),
        (
            serde_yaml::Value::String("email".to_string()),
            serde_yaml::Value::String(payload.email.clone()),
        ),
        (
            serde_yaml::Value::String("production".to_string()),
            serde_yaml::Value::Bool(payload.production),
        ),
    ]);
    tls.insert(
        serde_yaml::Value::String("auto_https".to_string()),
        serde_yaml::Value::Mapping(auto_https),
    );
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_wildcard_tls(
    original: &str,
    payload: &WildcardTlsUpsertRequest,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;
    let http_key = serde_yaml::Value::String("http".to_string());
    if !root.contains_key(&http_key) {
        root.insert(
            http_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let http = root
        .get_mut(&http_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http must be a mapping"))?;
    let tls_key = serde_yaml::Value::String("tls".to_string());
    if !http.contains_key(&tls_key) {
        http.insert(
            tls_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tls = http
        .get_mut(&tls_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http.tls must be a mapping"))?;
    tls.insert(
        serde_yaml::Value::String("mode".to_string()),
        serde_yaml::Value::String("acme_managed".to_string()),
    );
    tls.insert(
        serde_yaml::Value::String("generate_self_signed_if_missing".to_string()),
        serde_yaml::Value::Bool(false),
    );
    let credentials = serde_yaml::Mapping::from_iter(
        payload
            .credentials
            .iter()
            .map(|(key, value)| {
                (
                    serde_yaml::Value::String(key.clone()),
                    serde_yaml::Value::String(value.clone()),
                )
            })
            .collect::<Vec<_>>(),
    );
    let acme = serde_yaml::Mapping::from_iter([
        (
            serde_yaml::Value::String("email".to_string()),
            serde_yaml::Value::String(payload.email.clone()),
        ),
        (
            serde_yaml::Value::String("challenge".to_string()),
            serde_yaml::Value::String("dns01".to_string()),
        ),
        (
            serde_yaml::Value::String("domains".to_string()),
            serde_yaml::Value::Sequence(
                payload
                    .domains
                    .iter()
                    .map(|domain| serde_yaml::Value::String(domain.clone()))
                    .collect(),
            ),
        ),
        (
            serde_yaml::Value::String("directory_production".to_string()),
            serde_yaml::Value::Bool(true),
        ),
        (
            serde_yaml::Value::String("dns".to_string()),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter([
                (
                    serde_yaml::Value::String("provider".to_string()),
                    serde_yaml::Value::String(crate::acme::normalize_provider_id(
                        &payload.dns_provider,
                    )),
                ),
                (
                    serde_yaml::Value::String("credentials".to_string()),
                    serde_yaml::Value::Mapping(credentials),
                ),
            ])),
        ),
    ]);
    tls.insert(
        serde_yaml::Value::String("acme".to_string()),
        serde_yaml::Value::Mapping(acme),
    );
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_on_demand_tls(
    original: &str,
    payload: &OnDemandTlsUpsertRequest,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;
    let http_key = serde_yaml::Value::String("http".to_string());
    if !root.contains_key(&http_key) {
        root.insert(
            http_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let http = root
        .get_mut(&http_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http must be a mapping"))?;
    let tls_key = serde_yaml::Value::String("tls".to_string());
    if !http.contains_key(&tls_key) {
        http.insert(
            tls_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tls = http
        .get_mut(&tls_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http.tls must be a mapping"))?;
    let mut on_demand = serde_yaml::Mapping::from_iter([
        (
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(payload.enabled),
        ),
        (
            serde_yaml::Value::String("allow".to_string()),
            serde_yaml::Value::Sequence(
                payload
                    .allow
                    .iter()
                    .map(|item| serde_yaml::Value::String(item.clone()))
                    .collect(),
            ),
        ),
    ]);
    if let Some(value) = payload.max_active_certs {
        on_demand.insert(
            serde_yaml::Value::String("max_active_certs".to_string()),
            serde_yaml::Value::Number(value.into()),
        );
    }
    if let Some(value) = payload.max_issues_per_hour {
        on_demand.insert(
            serde_yaml::Value::String("max_issues_per_hour".to_string()),
            serde_yaml::Value::Number(value.into()),
        );
    }
    if let Some(value) = &payload.ask_url {
        on_demand.insert(
            serde_yaml::Value::String("ask_url".to_string()),
            serde_yaml::Value::String(value.clone()),
        );
    }
    tls.insert(
        serde_yaml::Value::String("on_demand".to_string()),
        serde_yaml::Value::Mapping(on_demand),
    );
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_filecloud(original: &str, filecloud: &FileCloudConfig) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;
    let services_key = serde_yaml::Value::String("services".to_string());
    if !root.contains_key(&services_key) {
        root.insert(
            services_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let services = root
        .get_mut(&services_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("services must be a mapping"))?;
    services.insert(
        serde_yaml::Value::String("filecloud".to_string()),
        serde_yaml::to_value(filecloud).context("failed to serialize filecloud config")?,
    );
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn sni_certificate_slug(domains: &[String]) -> String {
    domains
        .first()
        .map(|domain| {
            domain
                .trim()
                .trim_start_matches("*.")
                .replace('.', "-")
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
                .collect::<String>()
        })
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| "sni-cert".to_string())
}

fn resolve_sni_certificate_material(
    root_dir: &Path,
    payload: &SniCertificateUpsertRequest,
) -> Result<(PathBuf, PathBuf)> {
    if let (Some(cert_pem), Some(key_pem)) = (&payload.cert_pem, &payload.key_pem) {
        if cert_pem.trim().is_empty() || key_pem.trim().is_empty() {
            return Err(anyhow!(
                "cert_pem and key_pem cannot be empty when provided"
            ));
        }
        let dir = root_dir.join("certs").join("sni");
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
        let slug = sni_certificate_slug(&payload.domains);
        let cert_path = dir.join(format!("{slug}.crt"));
        let key_path = dir.join(format!("{slug}.key"));
        security::atomic_write(&cert_path, cert_pem.trim())?;
        security::atomic_write(&key_path, key_pem.trim())?;
        return Ok((cert_path, key_path));
    }

    let cert_path = payload
        .cert_path
        .clone()
        .ok_or_else(|| anyhow!("cert_path is required unless cert_pem/key_pem are provided"))?;
    let key_path = payload
        .key_path
        .clone()
        .ok_or_else(|| anyhow!("key_path is required unless cert_pem/key_pem are provided"))?;
    if !cert_path.exists() {
        return Err(anyhow!("cert_path does not exist: {}", cert_path.display()));
    }
    if !key_path.exists() {
        return Err(anyhow!("key_path does not exist: {}", key_path.display()));
    }
    Ok((cert_path, key_path))
}

fn sni_certificates_share_identity(
    left: &TlsCertificateConfig,
    right: &TlsCertificateConfig,
) -> bool {
    left.cert_path == right.cert_path
        || left
            .domains
            .iter()
            .any(|domain| right.domains.iter().any(|other| other == domain))
}

fn upsert_sni_certificate_config(
    certificates: &mut Vec<TlsCertificateConfig>,
    certificate: TlsCertificateConfig,
) -> &'static str {
    if let Some(existing) = certificates
        .iter_mut()
        .find(|item| sni_certificates_share_identity(item, &certificate))
    {
        *existing = certificate;
        "updated"
    } else {
        certificates.push(certificate);
        "created"
    }
}

fn sni_certificate_matches_delete(
    cert: &TlsCertificateConfig,
    payload: &SniCertificateDeleteRequest,
) -> bool {
    if let Some(cert_path) = payload.cert_path.as_deref() {
        if cert.cert_path.display().to_string() == cert_path {
            return true;
        }
    }
    if let Some(domain) = payload.domain.as_deref() {
        let domain = domain.trim().to_ascii_lowercase();
        if cert
            .domains
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&domain))
        {
            return true;
        }
    }
    false
}

fn render_config_with_upserted_sni_certificate(
    original: &str,
    certificate: &TlsCertificateConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;
    let http_key = serde_yaml::Value::String("http".to_string());
    if !root.contains_key(&http_key) {
        root.insert(
            http_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let http = root
        .get_mut(&http_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http must be a mapping"))?;
    let tls_key = serde_yaml::Value::String("tls".to_string());
    if !http.contains_key(&tls_key) {
        http.insert(
            tls_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tls = http
        .get_mut(&tls_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("http.tls must be a mapping"))?;
    let certs_key = serde_yaml::Value::String("certificates".to_string());
    if !tls.contains_key(&certs_key) {
        tls.insert(certs_key.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    let certificates = tls
        .get_mut(&certs_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("http.tls.certificates must be a sequence"))?;
    let cert_value =
        serde_yaml::to_value(certificate).context("failed to serialize sni certificate")?;
    if let Some(existing) = certificates.iter_mut().find(|item| {
        item.get("cert_path")
            .and_then(|value| value.as_str())
            .map(|path| path == certificate.cert_path.display().to_string())
            .unwrap_or(false)
            || item
                .get("domains")
                .and_then(|value| value.as_sequence())
                .map(|domains| {
                    domains
                        .iter()
                        .filter_map(|value| value.as_str())
                        .any(|domain| {
                            certificate
                                .domains
                                .iter()
                                .any(|configured| configured == domain)
                        })
                })
                .unwrap_or(false)
    }) {
        *existing = cert_value;
    } else {
        certificates.push(cert_value);
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_sni_certificate(
    original: &str,
    payload: &SniCertificateDeleteRequest,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let certificates = value
        .get_mut("http")
        .and_then(|item| item.get_mut("tls"))
        .and_then(|item| item.get_mut("certificates"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("http.tls.certificates must be a sequence"))?;
    let before = certificates.len();
    certificates.retain(|item| {
        let cert_path = item
            .get("cert_path")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let domains = item
            .get("domains")
            .and_then(|value| value.as_sequence())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let pseudo = TlsCertificateConfig {
            domains: domains.into_iter().map(str::to_string).collect(),
            cert_path: PathBuf::from(cert_path),
            key_path: PathBuf::new(),
        };
        !sni_certificate_matches_delete(&pseudo, payload)
    });
    if certificates.len() == before {
        return Err(anyhow!("sni certificate entry not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_tcp_listener(original: &str, name: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let listeners = value
        .get_mut("tcp")
        .and_then(|item| item.get_mut("listeners"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("tcp.listeners must be a sequence"))?;
    let before = listeners.len();
    listeners.retain(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value != name)
            .unwrap_or(true)
    });
    if listeners.len() == before {
        return Err(anyhow!("tcp listener {name} not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_udp_listener(original: &str, name: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let listeners = value
        .get_mut("udp")
        .and_then(|item| item.get_mut("listeners"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("udp.listeners must be a sequence"))?;
    let before = listeners.len();
    listeners.retain(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value != name)
            .unwrap_or(true)
    });
    if listeners.len() == before {
        return Err(anyhow!("udp listener {name} not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_deleted_stream_route(original: &str, name: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let routes = value
        .get_mut("tcp")
        .and_then(|item| item.get_mut("stream_routes"))
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("tcp.stream_routes must be a sequence"))?;
    let before = routes.len();
    routes.retain(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value != name)
            .unwrap_or(true)
    });
    if routes.len() == before {
        return Err(anyhow!("stream route {name} not found"));
    }
    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_upserted_tcp_listener(
    original: &str,
    listener: &TcpListenerConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;

    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;

    let tcp_key = serde_yaml::Value::String("tcp".to_string());
    if !root.contains_key(&tcp_key) {
        root.insert(
            tcp_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tcp = root
        .get_mut(&tcp_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("tcp must be a mapping"))?;

    let listeners_key = serde_yaml::Value::String("listeners".to_string());
    if !tcp.contains_key(&listeners_key) {
        tcp.insert(
            listeners_key.clone(),
            serde_yaml::Value::Sequence(Vec::new()),
        );
    }
    let listeners = tcp
        .get_mut(&listeners_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("tcp.listeners must be a sequence"))?;

    let listener_value =
        serde_yaml::to_value(listener).context("failed to serialize tcp listener")?;
    if let Some(existing) = listeners.iter_mut().find(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == listener.name)
            .unwrap_or(false)
    }) {
        *existing = listener_value;
    } else {
        listeners.push(listener_value);
    }

    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_upserted_udp_listener(
    original: &str,
    listener: &UdpListenerConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;

    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;

    let udp_key = serde_yaml::Value::String("udp".to_string());
    if !root.contains_key(&udp_key) {
        root.insert(
            udp_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let udp = root
        .get_mut(&udp_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("udp must be a mapping"))?;

    let listeners_key = serde_yaml::Value::String("listeners".to_string());
    if !udp.contains_key(&listeners_key) {
        udp.insert(
            listeners_key.clone(),
            serde_yaml::Value::Sequence(Vec::new()),
        );
    }
    let listeners = udp
        .get_mut(&listeners_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("udp.listeners must be a sequence"))?;

    let listener_value =
        serde_yaml::to_value(listener).context("failed to serialize udp listener")?;
    if let Some(existing) = listeners.iter_mut().find(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == listener.name)
            .unwrap_or(false)
    }) {
        *existing = listener_value;
    } else {
        listeners.push(listener_value);
    }

    serde_yaml::to_string(&value).context("failed to render updated YAML config")
}

fn render_config_with_upserted_stream_route(
    original: &str,
    route: &StreamRouteConfig,
) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;

    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("top-level YAML config must be a mapping"))?;

    let tcp_key = serde_yaml::Value::String("tcp".to_string());
    if !root.contains_key(&tcp_key) {
        root.insert(
            tcp_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let tcp = root
        .get_mut(&tcp_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow!("tcp must be a mapping"))?;

    let routes_key = serde_yaml::Value::String("stream_routes".to_string());
    if !tcp.contains_key(&routes_key) {
        tcp.insert(routes_key.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    let routes = tcp
        .get_mut(&routes_key)
        .and_then(|item| item.as_sequence_mut())
        .ok_or_else(|| anyhow!("tcp.stream_routes must be a sequence"))?;

    let route_value = serde_yaml::to_value(route).context("failed to serialize stream route")?;
    if let Some(existing) = routes.iter_mut().find(|item| {
        item.get("name")
            .and_then(|value| value.as_str())
            .map(|value| value == route.name)
            .unwrap_or(false)
    }) {
        *existing = route_value;
    } else {
        routes.push(route_value);
    }

    serde_yaml::to_string(&value).context("failed to render updated YAML config")
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

fn http_access_is_denied(config: &HttpAccessControlConfig, ip: IpAddr) -> Option<String> {
    ip_access_is_denied(config, ip)
}

fn http_rate_limit_key(
    config: &HttpRateLimitConfig,
    host: &str,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
) -> Option<String> {
    let scope = match &config.key {
        RateLimitKey::RemoteAddr => format!("remote:{}", remote_addr.ip()),
        RateLimitKey::Host => format!("host:{}", host.to_ascii_lowercase()),
        RateLimitKey::Header(name) => headers
            .get(name.as_str())
            .and_then(|value| value.to_str().ok())
            .map(|value| format!("header:{}:{}", name.to_ascii_lowercase(), value))?,
    };
    Some(format!("{}:{scope}", config.zone))
}

fn rate_limit_rejection_response(
    config: &HttpRateLimitConfig,
    retry_after: &str,
) -> GatewayHttpResponse {
    let status = StatusCode::from_u16(config.status).unwrap_or(StatusCode::TOO_MANY_REQUESTS);
    let mut response = GatewayHttpResponse::bytes(
        status,
        "text/plain; charset=utf-8",
        Bytes::from_static(b"rate limit exceeded"),
        "proxysss://rate-limit",
    );
    response.headers.push((
        HeaderName::from_static("retry-after"),
        HeaderValue::from_str(retry_after).unwrap_or_else(|_| HeaderValue::from_static("1")),
    ));
    response
}

fn apply_http_rate_limit_to_store(
    store: &DashMap<String, RateLimitBucket>,
    config: &HttpRateLimitConfig,
    key: String,
) -> Option<String> {
    let now = Instant::now();
    let window = Duration::from_millis(config.window_ms.max(100));
    let limit = config.requests.saturating_add(config.burst).max(1);

    let initial_tokens = match config.algorithm {
        RateLimitAlgorithm::TokenBucket => limit as f64,
        RateLimitAlgorithm::LeakyBucket => 0.0,
        RateLimitAlgorithm::FixedWindow => limit as f64,
    };
    let mut bucket = store.entry(key).or_insert(RateLimitBucket {
        window_start: now,
        count: 0,
        tokens: initial_tokens,
        last_refill: now,
    });

    match config.algorithm {
        RateLimitAlgorithm::FixedWindow => {
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
        RateLimitAlgorithm::TokenBucket => {
            let elapsed = now.duration_since(bucket.last_refill);
            let refill_per_ms = config.requests as f64 / window.as_millis().max(1) as f64;
            let refill = elapsed.as_millis() as f64 * refill_per_ms;
            bucket.tokens = (bucket.tokens + refill).min(limit as f64);
            bucket.last_refill = now;

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                return None;
            }

            let deficit = 1.0 - bucket.tokens;
            let wait_ms = (deficit / refill_per_ms).ceil() as u64;
            Some(wait_ms.max(1).to_string())
        }
        RateLimitAlgorithm::LeakyBucket => {
            let leak_per_ms = config.requests as f64 / window.as_millis().max(1) as f64;
            let elapsed = now.duration_since(bucket.last_refill);
            bucket.tokens = (bucket.tokens - elapsed.as_millis() as f64 * leak_per_ms).max(0.0);
            bucket.last_refill = now;

            if bucket.tokens + 1.0 <= limit as f64 {
                bucket.tokens += 1.0;
                return None;
            }

            let overflow = bucket.tokens + 1.0 - limit as f64;
            let wait_ms = (overflow / leak_per_ms).ceil() as u64;
            Some(wait_ms.max(1).to_string())
        }
    }
}

fn stream_rate_limit_key(config: &StreamRateLimitConfig, remote_addr: SocketAddr) -> String {
    format!("{}:remote:{}", config.zone, remote_addr.ip())
}

fn apply_stream_rate_limit(
    store: &DashMap<String, RateLimitBucket>,
    config: &StreamRateLimitConfig,
    remote_addr: SocketAddr,
) -> bool {
    if !config.enabled {
        return true;
    }
    let key = stream_rate_limit_key(config, remote_addr);
    let limit_config = HttpRateLimitConfig {
        enabled: true,
        zone: config.zone.clone(),
        algorithm: config.algorithm,
        key: RateLimitKey::RemoteAddr,
        requests: config.connections,
        window_ms: config.window_ms,
        burst: config.burst,
        max_connections: 0,
        status: 429,
    };
    apply_http_rate_limit_to_store(store, &limit_config, key).is_none()
}

fn builtin_http_route(path: &str) -> Option<RouteDecision> {
    let upstream = match path {
        "/" | "/index.html" => "proxysss://welcome",
        "/docs" | "/docs.html" => "proxysss://docs",
        "/healthz" => "proxysss://healthz",
        "/admin" => "proxysss://admin",
        path if path.starts_with("/static/") => {
            return Some(RouteDecision {
                upstream: format!("proxysss://static/{}", path.trim_start_matches("/static/")),
                upstreams: Vec::new(),
                upstream_weights: BTreeMap::new(),
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
        upstream_weights: BTreeMap::new(),
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
        upstream_weights: BTreeMap::new(),
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
    configured_ai_proxy_route(config, host, uri)
        .or_else(|| configured_domain_route(config, host, uri))
        .or_else(|| configured_reverse_proxy_route(config, host, uri))
}

fn configured_ai_proxy_route(
    config: &GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig> {
    if !config.services.ai_proxy.enabled {
        return None;
    }
    let route = config
        .services
        .ai_proxy
        .routes
        .iter()
        .filter(|route| crate::ai_proxy::route_matches(route, host, uri.path()))
        .max_by_key(|route| route.path_prefix.len())?;
    Some(HttpRouteConfig {
        runtime_scope: Some(format!("ai:{}", route.name)),
        decision: crate::ai_proxy::build_route_decision(
            route,
            uri,
            &config.services.ai_proxy.header_prefix,
        ),
        compression: config.services.response_policy.compression.clone(),
        cache: ResponseCacheConfig {
            enabled: false,
            ..Default::default()
        },
        rate_limit: config.services.rate_limit.http.clone(),
    })
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
        .map(|route| domain_route_config(config, route, uri))
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
            runtime_scope: Some(route.name.clone()),
            decision: reverse_proxy_route_decision(route, uri),
            compression: merge_compression_policy(
                &config.services.response_policy.compression,
                &route.compression,
            ),
            cache: merge_cache_policy(&config.services.response_policy.cache, &route.cache),
            rate_limit: merge_rate_limit_policy(
                &config.services.rate_limit.http,
                &route.rate_limit,
            ),
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
    if prefix == "/" {
        return path.starts_with('/');
    }
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

fn strip_default_port(host: &str, default_port: u16) -> String {
    match host.rsplit_once(':') {
        Some((name, port)) if port == default_port.to_string() => name.to_string(),
        _ => host.to_string(),
    }
}

fn should_redirect_http_to_https(config: &GatewayConfig, host: &str, uri: &Uri) -> bool {
    if uri.path().starts_with("/.well-known/acme-challenge/") {
        return false;
    }
    if config.http.tls_bind.trim().is_empty() {
        return false;
    }
    let normalized_host = strip_default_port(host, 80).to_ascii_lowercase();

    if matches!(
        config.http.tls.mode,
        TlsMode::AcmeManaged | TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal
    ) && config
        .http
        .tls
        .acme
        .domains
        .iter()
        .any(|domain| host_matches(domain, &normalized_host))
    {
        return true;
    }

    if config.http.tls.certificates.iter().any(|cert| {
        cert.domains
            .iter()
            .any(|domain| host_matches(domain, &normalized_host))
    }) {
        return true;
    }

    config.services.domain_routes.iter().any(|route| {
        domain_route_matches(route, &normalized_host, uri.path())
            && route.ssl.effective_mode() != DomainTlsMode::Disabled
    })
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
        upstream_weights: route.upstream_weights.clone(),
        affinity_key: None,
        rewrite_path,
        set_headers: route.set_headers.clone(),
        strip_headers: route.strip_headers.clone(),
        status: None,
        content_type: None,
    }
}

fn domain_route_config(
    config: &GatewayConfig,
    route: &DomainRouteConfig,
    uri: &Uri,
) -> HttpRouteConfig {
    HttpRouteConfig {
        runtime_scope: Some(route.name.clone()),
        decision: RouteDecision {
            upstream: route.upstream.clone(),
            upstreams: route.upstreams.clone(),
            upstream_weights: route.upstream_weights.clone(),
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
        compression: merge_compression_policy(
            &config.services.response_policy.compression,
            &route.compression,
        ),
        cache: merge_cache_policy(&config.services.response_policy.cache, &route.cache),
        rate_limit: merge_rate_limit_policy(&config.services.rate_limit.http, &route.rate_limit),
    }
}

fn resolve_active_health_config(
    base: &ActiveHealthConfig,
    override_config: &ActiveHealthOverrideConfig,
) -> ResolvedActiveHealthConfig {
    ResolvedActiveHealthConfig {
        enabled: override_config
            .enabled
            .unwrap_or(base.enabled && base.http_enabled),
        path: override_config
            .path
            .clone()
            .unwrap_or_else(|| base.path.clone()),
        timeout_ms: override_config.timeout_ms.unwrap_or(base.timeout_ms),
        expected_statuses: override_config
            .expected_statuses
            .clone()
            .unwrap_or_else(|| base.expected_statuses.clone()),
        failure_threshold: override_config
            .failure_threshold
            .unwrap_or(base.failure_threshold),
        success_threshold: override_config
            .success_threshold
            .unwrap_or(base.success_threshold),
        jitter_percent: override_config
            .jitter_percent
            .unwrap_or(base.jitter_percent),
        alert_webhooks: if override_config.alert_webhooks.is_empty() {
            base.alert_webhooks.clone()
        } else {
            override_config.alert_webhooks.clone()
        },
    }
}

fn resolve_global_active_health_config(base: &ActiveHealthConfig) -> ResolvedActiveHealthConfig {
    ResolvedActiveHealthConfig {
        enabled: base.enabled,
        path: base.path.clone(),
        timeout_ms: base.timeout_ms,
        expected_statuses: base.expected_statuses.clone(),
        failure_threshold: base.failure_threshold,
        success_threshold: base.success_threshold,
        jitter_percent: base.jitter_percent,
        alert_webhooks: base.alert_webhooks.clone(),
    }
}

fn merge_compression_policy(
    base: &ResponseCompressionConfig,
    override_policy: &ResponseCompressionConfig,
) -> ResponseCompressionConfig {
    if override_policy.enabled {
        override_policy.clone()
    } else {
        base.clone()
    }
}

fn merge_cache_policy(
    base: &ResponseCacheConfig,
    override_policy: &ResponseCacheConfig,
) -> ResponseCacheConfig {
    if override_policy.enabled {
        override_policy.clone()
    } else {
        base.clone()
    }
}

fn merge_rate_limit_policy(
    base: &HttpRateLimitConfig,
    override_policy: &HttpRateLimitConfig,
) -> HttpRateLimitConfig {
    if override_policy.enabled {
        override_policy.clone()
    } else {
        base.clone()
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
    if prefix == "/" {
        return path.starts_with('/');
    }
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
        "proxysss://docs" => GatewayHttpResponse::html(render_docs_html(config), "proxysss://docs"),
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
    let mut components = relative_path.components();
    let Some(first) = components.next() else {
        return Err(anyhow!("static path cannot be empty"));
    };
    if !matches!(first, std::path::Component::Normal(_))
        || components
            .clone()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(anyhow!(
            "static path cannot contain parent directory or absolute prefixes"
        ));
    }

    let root = root_dir
        .canonicalize()
        .unwrap_or_else(|_| root_dir.to_path_buf());
    let base = root.join(first.as_os_str());
    let canonical_base = base
        .canonicalize()
        .with_context(|| format!("failed to resolve static base {}", base.display()))?;
    let candidate = components.fold(canonical_base.clone(), |path, component| {
        path.join(component.as_os_str())
    });
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to resolve static path {}", candidate.display()))?;
    if !canonical.starts_with(&canonical_base) {
        return Err(anyhow!("static path escaped static base"));
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
    include_str!("../templates/welcome.html").replace("__VERSION__", env!("CARGO_PKG_VERSION"))
}
pub(crate) fn render_docs_html(_config: &GatewayConfig) -> String {
    let reverse_proxy = docs_template_reverse_proxy();
    let ai_proxy = docs_template_ai_proxy();
    let static_site = docs_template_static_site();
    let webdav = docs_template_webdav();
    let streams = docs_template_streams();
    let ftp = docs_template_ftp();
    let acme_dns = docs_template_acme_dns();
    let health = docs_template_health();
    let maintenance = docs_template_maintenance();
    let error_pages = docs_template_error_pages();

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>proxysss docs</title>
    <style>
        :root {{
            --bg: #08131d;
            --bg-2: #0d1d2d;
            --panel: rgba(11, 21, 36, 0.88);
            --line: rgba(140, 192, 255, 0.12);
            --text: #eef6ff;
            --muted: #93a8c4;
            --accent: #59d0ff;
            --accent-2: #7ef4b0;
            --gold: #f3d27c;
        }}
        * {{ box-sizing: border-box; }}
        body {{
            margin: 0;
            font-family: "Avenir Next", "PingFang SC", "Microsoft YaHei", sans-serif;
            color: var(--text);
            background:
                radial-gradient(circle at top left, rgba(89, 208, 255, 0.18), transparent 28%),
                linear-gradient(160deg, var(--bg), var(--bg-2));
        }}
        .shell {{ width: min(1400px, calc(100vw - 28px)); margin: 14px auto; display: grid; grid-template-columns: 300px minmax(0, 1fr); gap: 14px; }}
        .nav, .content {{ background: var(--panel); border: 1px solid var(--line); border-radius: 24px; box-shadow: 0 24px 70px rgba(0,0,0,.24); backdrop-filter: blur(16px); }}
        .nav {{ padding: 22px; position: sticky; top: 14px; align-self: start; display: grid; gap: 18px; }}
        .content {{ padding: 22px; display: grid; gap: 18px; }}
        h1, h2, h3, p {{ margin: 0; }}
        h1 {{ font-size: 34px; letter-spacing: -0.04em; }}
        h2 {{ font-size: 24px; margin-bottom: 10px; }}
        h3 {{ font-size: 17px; margin-bottom: 8px; }}
        .eyebrow {{ font-size: 11px; letter-spacing: .18em; text-transform: uppercase; color: var(--accent-2); }}
        .muted {{ color: var(--muted); }}
        nav a {{ display: block; padding: 10px 12px; margin-top: 6px; border-radius: 12px; color: var(--text); text-decoration: none; background: rgba(255,255,255,.03); border: 1px solid rgba(255,255,255,.04); }}
        section {{ padding: 18px; border-radius: 20px; background: rgba(255,255,255,.03); border: 1px solid rgba(255,255,255,.05); }}
        .cards {{ display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }}
        .card {{ padding: 14px; border-radius: 16px; background: rgba(255,255,255,.03); border: 1px solid rgba(255,255,255,.05); }}
        .list {{ display: grid; gap: 8px; color: var(--muted); }}
        .list strong {{ color: var(--text); }}
        pre {{ margin: 0; padding: 16px; border-radius: 18px; overflow: auto; background: rgba(3, 9, 18, .78); border: 1px solid rgba(255,255,255,.05); color: #d7e9ff; font-size: 12px; line-height: 1.5; }}
        .table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
        .table th, .table td {{ text-align: left; padding: 10px 12px; border-bottom: 1px solid rgba(255,255,255,.06); vertical-align: top; }}
        .table th {{ color: var(--muted); }}
        .pill {{ display: inline-flex; padding: 4px 8px; border-radius: 999px; font-size: 12px; font-weight: 700; background: rgba(255,255,255,.08); }}
        .pill.good {{ color: var(--accent-2); background: rgba(126,244,176,.14); }}
        .pill.warn {{ color: var(--gold); background: rgba(243,210,124,.14); }}
        .actions {{ display: flex; gap: 10px; flex-wrap: wrap; }}
        .actions a {{ display: inline-flex; align-items: center; text-decoration: none; padding: 12px 16px; border-radius: 999px; font-weight: 800; }}
        .primary {{ background: linear-gradient(135deg, var(--accent), var(--accent-2)); color: #04111a; }}
        .ghost {{ background: rgba(255,255,255,.06); color: var(--text); }}
        @media (max-width: 1080px) {{ .shell {{ grid-template-columns: 1fr; }} .nav {{ position: static; }} .cards {{ grid-template-columns: 1fr; }} }}
    </style>
</head>
<body>
    <main class="shell">
        <aside class="nav">
            <div>
                <div class="eyebrow">documentation</div>
                <h1>proxysss</h1>
                <p class="muted">Built-in manual, operational guide, config templates, and parity notes.</p>
            </div>
            <nav>
                <a href="#quickstart">Quick Start</a>
                <a href="#operations">Operations</a>
                <a href="#templates">Templates</a>
                <a href="#wildcard-ssl">Wildcard SSL</a>
                <a href="#parity">Nginx Parity</a>
                <a href="#gaps">Tracked Gaps</a>
            </nav>
            <div class="actions">
                <a class="primary" href="/">Back to Welcome</a>
                <a class="ghost" href="/admin">Open Admin</a>
            </div>
        </aside>

        <section class="content">
            <section id="quickstart">
                <div class="eyebrow">Quick Start</div>
                <h2>Default Behavior</h2>
                <div class="cards">
                    <article class="card"><strong>Public HTTP</strong><p class="muted">Port 80 with a built-in welcome page and optional automatic redirect to HTTPS for managed TLS domains.</p></article>
                    <article class="card"><strong>Public TLS</strong><p class="muted">Port 443 for HTTPS/HTTP2, optional HTTP3 on the same public edge, managed ACME, SNI certs, and WebSocket tunneling.</p></article>
                    <article class="card"><strong>Admin</strong><p class="muted">Port 7777 with stats, upstream health, maintenance mode toggles, and live runtime inspection.</p></article>
                </div>
            </section>

            <section id="operations">
                <div class="eyebrow">Operations</div>
                <h2>Built-in Control Plane</h2>
                <div class="list">
                    <div><strong>Health:</strong> active HTTP/TCP health probes plus passive quarantine and manual drain state.</div>
                    <div><strong>Reload:</strong> the main YAML config, scripts, plugins, and route-level health policy reload without a full process restart where supported.</div>
                    <div><strong>Route automation:</strong> token-authenticated `POST /v1/domain-routes/upsert` can persist new domain routes into the main YAML file and reload them in process.</div>
                    <div><strong>Maintenance:</strong> upstream disable/restore can be persisted on disk through runtime maintenance state.</div>
                    <div><strong>Error Pages:</strong> configurable status-page bodies/files plus polished built-in browser-facing 404/403/5xx pages.</div>
                    <div><strong>Runtime tuning:</strong> Ubuntu/Debian-first TCP tuning assistant via <code>proxysss tune tcp</code>.</div>
                </div>
            </section>

            <section id="templates">
                <div class="eyebrow">Templates</div>
                <h2>Configuration Templates</h2>
                <h3>HTTP Reverse Proxy</h3>
                <pre>{}</pre>
                <h3>AI API Reverse Proxy</h3>
                <pre>{}</pre>
                <h3>Static Site</h3>
                <pre>{}</pre>
                <h3>WebDAV</h3>
                <pre>{}</pre>
                <h3>TCP / UDP Streams</h3>
                <pre>{}</pre>
                <h3>FTP Native Control + Data Channels</h3>
                <pre>{}</pre>
                <h3 id="wildcard-ssl">Wildcard SSL with built-in DNS-01</h3>
                <p class="muted">Use <code>http.tls.mode: acme_managed</code> with <code>http.tls.acme.challenge: dns01</code> for wildcard certificates. Without cloud tokens, HTTP-01/TLS-ALPN-01 still works via <code>auto_https</code>. Built-in providers: <code>cloudflare</code>, <code>aliyun_cn</code>, <code>aliyun_intl</code>, <code>tencent</code>, <code>volcengine</code>, <code>aws</code>, <code>azure</code>, <code>google</code>.</p>
                <pre>{}</pre>
                <h3>Active Health, Maintenance Persistence, Alerts</h3>
                <pre>{}</pre>
                <h3>Custom Error Pages</h3>
                <pre>{}</pre>
                <h3>Maintenance Persistence</h3>
                <pre>{}</pre>
            </section>

            <section id="parity">
                <div class="eyebrow">Parity</div>
                <h2>Regular Gateway Behavior</h2>
                <table class="table">
                    <thead><tr><th>Surface</th><th>Status</th><th>Notes</th></tr></thead>
                    <tbody>
                        <tr><td>HTTP reverse proxy</td><td><span class="pill good">supported</span></td><td>host/path matching, upstream pools, strip prefix, header set/strip, retry, health, maintenance drain</td></tr>
                        <tr><td>AI API reverse proxy</td><td><span class="pill good">supported</span></td><td>native New API, sub2api, and OpenAI-compatible routes through services.ai_proxy</td></tr>
                        <tr><td>HTTPS / HTTP2 / HTTP3</td><td><span class="pill good">supported</span></td><td>self-signed, manual SNI, managed ACME HTTP-01/TLS-ALPN-01/DNS-01, WebSocket, automatic redirect for managed domains</td></tr>
                        <tr><td>Static files</td><td><span class="pill good">supported</span></td><td>index files, autoindex, default welcome, custom browser error pages</td></tr>
                        <tr><td>WebDAV</td><td><span class="pill good">supported</span></td><td>OPTIONS/PROPFIND/GET/HEAD/PUT/DELETE/MKCOL/COPY/MOVE</td></tr>
                        <tr><td>TCP / UDP streams</td><td><span class="pill good">supported</span></td><td>YAML listeners with upstream pools and runtime health</td></tr>
                        <tr><td>FTP</td><td><span class="pill good">supported</span></td><td>nginx ftp module directive parity: bind/proxy_pass, passive range, pasv_address, allow/deny, command and transfer policies, per-user rules, lifecycle logs</td></tr>
                    </tbody>
                </table>
            </section>

            <section id="gaps">
                <div class="eyebrow">Tracked Gaps</div>
                <h2>Still Honest About What Remains</h2>
                <div class="list">
                    <div><strong>On-demand TLS:</strong> not yet policy-gated first-hit certificate issuance.</div>
                    <div><strong>Auto HTTPS boundary:</strong> built-in managed ACME covers HTTP-01, TLS-ALPN-01, and DNS-01 wildcard issuance through provider strategies; legacy <code>acme_dns_external</code> remains for acme.sh-only providers.</div>
                </div>
            </section>
        </section>
    </main>
</body>
</html>"##,
        reverse_proxy,
        ai_proxy,
        static_site,
        webdav,
        streams,
        ftp,
        acme_dns,
        health,
        error_pages,
        maintenance,
    )
}

fn docs_template_reverse_proxy() -> &'static str {
    "http:\n  plain_bind: 0.0.0.0:80\n  tls_bind: 0.0.0.0:443\nservices:\n  access_control:\n    http:\n      enabled: true\n      blacklist: [203.0.113.10, 198.51.100.0/24]\n  domain_routes:\n    - name: example-site\n      domains: [example.com, www.example.com]\n      path_prefix: /\n      upstream: http://127.0.0.1:9000\n      compression:\n        enabled: true\n    - name: neko233-store\n      domains: [neko233.store]\n      path_prefix: /\n      upstream: http://127.0.0.1:9000\n      upstreams:\n        - http://127.0.0.1:9001\n      cache:\n        enabled: true\n        ttl_secs: 30\n      rate_limit:\n        enabled: true\n        requests: 120\n        window_ms: 60000\n        burst: 30\n      active_health:\n        path: /healthz\n        failure_threshold: 2\n        success_threshold: 2\n"
}

fn docs_template_ai_proxy() -> &'static str {
    "services:\n  ai_proxy:\n    enabled: true\n    header_prefix: proxysss-\n    routes:\n      - name: new-api\n        provider: new-api\n        match_host: ai.example.com\n        path_prefix: /v1\n        upstream: http://127.0.0.1:3000\n        rewrite_base_path: /v1\n      - name: sub2api\n        provider: sub2api\n        match_host: sub2api.example.com\n        path_prefix: /\n        upstream: http://127.0.0.1:3001\n        rewrite_base_path: /v1\n"
}

fn docs_template_static_site() -> &'static str {
    "services:\n  static_sites:\n    - name: public\n      path_prefix: /assets\n      root: ./public\n      index_files: [index.html, index.htm]\n      autoindex: false\n"
}

fn docs_template_webdav() -> &'static str {
    "services:\n  webdav:\n    enabled: true\n    path_prefix: /dav\n    root: ./webdav\n    allow_write: true\n"
}

fn docs_template_streams() -> &'static str {
    "tcp:\n  listeners:\n    - name: game-tcp\n      bind: 0.0.0.0:7000\n      upstreams: [127.0.0.1:9000, 127.0.0.1:9001]\nudp:\n  listeners:\n    - name: realtime\n      bind: 0.0.0.0:7001\n      upstreams: [127.0.0.1:9100, 127.0.0.1:9101]\n"
}

fn docs_template_ftp() -> &'static str {
    "services:\n  ftp:\n    enabled: true\n    bind: 0.0.0.0:21\n    upstream: 127.0.0.1:2121\n    native_control: true\n    public_ip: 203.0.113.10\n    passive_port_start: 50000\n    passive_port_end: 50100\n    log_commands: true\n    log_transfers: true\n    allow: [198.51.100.0/24]\n    deny: [203.0.113.9]\n    command_deny: [SITE, STAT]\n    transfer_allow: [RETR, STOR]\n    user_policies:\n      - user: readonly\n        transfer_allow: [RETR]\n        transfer_deny: [STOR, DELE]\n"
}

fn docs_template_acme_dns() -> &'static str {
    "http:\n  tls:\n    mode: acme_managed\n    cert_path: certs/proxysss-cert.pem\n    key_path: certs/proxysss-key.pem\n    generate_self_signed_if_missing: false\n    server_name: example.com\n    acme:\n      email: admin@example.com\n      challenge: dns01\n      domains: [example.com, \"*.example.com\"]\n      directory_production: true\n      renew_interval_hours: 12\n      dns:\n        provider: cloudflare\n        credentials:\n          api_token: your-cloudflare-api-token\n"
}

fn docs_template_health() -> &'static str {
    "load_balance:\n  active_health:\n    enabled: true\n    http_enabled: true\n    tcp_enabled: true\n    interval_secs: 10\n    timeout_ms: 2000\n    path: /healthz\n    expected_statuses: [200, 204]\n    failure_threshold: 2\n    success_threshold: 2\n    jitter_percent: 20\n    alert_webhooks:\n      - https://ops.example.com/webhooks/proxysss\n"
}

fn docs_template_error_pages() -> &'static str {
    "http:\n  error_pages:\n    enabled: true\n    show_details: false\n    pages:\n      - status: 404\n        content_type: text/html; charset=utf-8\n        body: |\n          <html><body><h1>{{status}} {{reason}}</h1><p>The requested route does not exist.</p></body></html>\n"
}

fn docs_template_maintenance() -> &'static str {
    "runtime:\n  maintenance_state:\n    enabled: true\n    path: ./runtime/maintenance-state.json\n"
}

fn render_admin_console_html(config: &GatewayConfig) -> String {
    let html = r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>proxysss admin</title>
    <style>
        :root {
            --bg: #090f1c;
            --panel: #111827;
            --panel-2: #0f172a;
            --line: rgba(148, 163, 184, 0.18);
            --text: #e5e7eb;
            --muted: #94a3b8;
            --accent: #3b82f6;
            --accent-2: #22c55e;
            --warn: #f59e0b;
            --bad: #fb7185;
            --good: #4ade80;
            --soft: rgba(148, 163, 184, 0.08);
            --shadow: 0 24px 70px rgba(0, 0, 0, 0.32);
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            min-height: 100vh;
            font-family: Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
            color: var(--text);
            background:
                radial-gradient(circle at 15% 0%, rgba(59, 130, 246, 0.18), transparent 30%),
                radial-gradient(circle at 92% 8%, rgba(34, 197, 94, 0.12), transparent 26%),
                var(--bg);
        }
        .login-screen {
            min-height: 100vh;
            display: grid;
            place-items: center;
            padding: 32px 16px;
            background:
                radial-gradient(circle at 24% 12%, rgba(59, 130, 246, 0.22), transparent 34%),
                radial-gradient(circle at 78% 10%, rgba(34, 197, 94, 0.12), transparent 28%),
                linear-gradient(180deg, #0b1220 0%, #090f1c 100%);
        }
        .login-panel {
            width: min(420px, 100%);
            display: grid;
            gap: 18px;
            padding: 34px;
            border-radius: 18px;
            background: rgba(15, 23, 42, 0.92);
            border: 1px solid var(--line);
            box-shadow: var(--shadow);
            backdrop-filter: blur(18px);
        }
        .login-logo {
            width: 44px;
            height: 44px;
            display: grid;
            place-items: center;
            border-radius: 12px;
            background: linear-gradient(135deg, var(--accent), #2563eb);
            color: #fff;
            font-weight: 900;
            font-size: 22px;
        }
        .login-title {
            display: grid;
            gap: 8px;
        }
        .login-title h1 { font-size: 28px; letter-spacing: 0; }
        .login-form {
            display: grid;
            gap: 14px;
        }
        .login-error {
            min-height: 20px;
            color: var(--bad);
            font-size: 13px;
        }
        .login-screen.hidden, .app-hidden { display: none !important; }
        .admin-top {
            min-height: 58px;
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 16px;
            padding: 0 24px;
            background: rgba(15, 23, 42, 0.92);
            border-bottom: 1px solid var(--line);
            position: sticky;
            top: 0;
            z-index: 10;
        }
        .top-actions {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .language-select {
            width: auto;
            min-width: 112px;
            padding: 9px 10px;
        }
        .shell {
            width: 100%;
            min-height: calc(100vh - 58px);
            margin: 0;
            display: grid;
            grid-template-columns: 252px minmax(0, 1fr);
            gap: 0;
        }
        .sidebar, .content {
            background: var(--panel);
            border: 1px solid var(--line);
            box-shadow: none;
        }
        .sidebar {
            padding: 16px;
            display: grid;
            align-content: start;
            gap: 12px;
            border-width: 0 1px 0 0;
            border-radius: 0;
        }
        .brand {
            display: grid;
            gap: 6px;
        }
        .eyebrow {
            font-size: 11px;
            letter-spacing: 0.10em;
            text-transform: uppercase;
            color: var(--accent);
            font-weight: 800;
        }
        h1, h2, h3, p { margin: 0; }
        h1 { font-size: 22px; letter-spacing: 0; }
        h2 { font-size: 24px; letter-spacing: 0; }
        h3 { font-size: 16px; }
        .muted { color: var(--muted); }
        .login-card, .meta-card {
            padding: 12px;
            border-radius: 10px;
            background: var(--soft);
            border: 1px solid var(--line);
            display: grid;
            gap: 10px;
        }
        label {
            font-size: 12px;
            color: var(--muted);
        }
        input {
            width: 100%;
            border: 1px solid rgba(148, 163, 184, 0.22);
            background: rgba(15, 23, 42, 0.86);
            color: var(--text);
            padding: 11px 12px;
            border-radius: 8px;
            outline: none;
        }
        input:focus, select:focus, textarea:focus { border-color: var(--accent); box-shadow: 0 0 0 3px rgba(59,130,246,.18); }
        select {
            width: 100%;
            border: 1px solid rgba(148, 163, 184, 0.22);
            background: rgba(15, 23, 42, 0.86);
            color: var(--text);
            padding: 11px 12px;
            border-radius: 8px;
            outline: none;
        }
        .button-row { display: flex; gap: 10px; flex-wrap: wrap; }
        button {
            border: 1px solid transparent;
            border-radius: 8px;
            padding: 10px 13px;
            font-weight: 700;
            cursor: pointer;
        }
        .primary { background: linear-gradient(135deg, var(--accent), #2563eb); color: #fff; }
        .ghost { background: rgba(148, 163, 184, 0.08); color: var(--text); border-color: var(--line); }
        .danger { background: rgba(251, 113, 133, 0.14); color: var(--bad); border-color: rgba(251, 113, 133, 0.25); }
        .success { background: rgba(74, 222, 128, 0.12); color: var(--good); border-color: rgba(74, 222, 128, 0.25); }
        .content {
            padding: 18px;
            display: grid;
            gap: 14px;
            min-width: 0;
            border: 0;
            border-radius: 0;
            background: transparent;
        }
        .topbar {
            display: flex;
            justify-content: space-between;
            align-items: flex-end;
            gap: 16px;
        }
        .status-dot {
            display: inline-flex;
            align-items: center;
            gap: 8px;
            font-size: 13px;
            color: var(--muted);
        }
        .status-dot::before {
            content: "";
            width: 8px;
            height: 8px;
            border-radius: 999px;
            background: var(--warn);
            box-shadow: none;
        }
        .status-dot.ok::before { background: var(--good); box-shadow: none; }
        .status-dot.bad::before { background: var(--bad); box-shadow: none; }
        .cards {
            display: grid;
            grid-template-columns: repeat(7, minmax(0, 1fr));
            gap: 12px;
        }
        .card {
            padding: 14px;
            border-radius: 10px;
            background: var(--panel-2);
            border: 1px solid var(--line);
            min-width: 0;
            box-shadow: 0 14px 34px rgba(0,0,0,.18);
        }
        .card strong {
            display: block;
            font-size: 24px;
            line-height: 1;
            margin-top: 8px;
            letter-spacing: -0.04em;
        }
        .surface {
            padding: 14px;
            border-radius: 10px;
            background: var(--panel-2);
            border: 1px solid var(--line);
            min-width: 0;
            box-shadow: 0 14px 34px rgba(0,0,0,.18);
        }
        .surface-head {
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 12px;
            margin-bottom: 12px;
        }
        .surface-title { display: grid; gap: 4px; }
        .surface-actions { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
        .compact-controls {
            display: grid;
            grid-template-columns: minmax(180px, 280px) 150px 140px;
            gap: 8px;
            align-items: end;
        }
        .compact-controls input, .compact-controls select { padding: 9px 10px; }
        .filters {
            display: grid;
            grid-template-columns: repeat(4, minmax(0, 1fr));
            gap: 12px;
        }
        .filter {
            display: grid;
            gap: 8px;
        }
        .group-grid {
            display: grid;
            grid-template-columns: repeat(3, minmax(0, 1fr));
            gap: 12px;
        }
        .group-card {
            padding: 14px;
            border-radius: 10px;
            background: var(--soft);
            border: 1px solid var(--line);
            display: grid;
            gap: 8px;
        }
        .group-card strong {
            font-size: 18px;
            letter-spacing: -0.03em;
        }
        .group-meta {
            display: flex;
            gap: 8px;
            flex-wrap: wrap;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 12px;
        }
        th, td {
            text-align: left;
            padding: 9px 10px;
            border-bottom: 1px solid var(--line);
            vertical-align: top;
        }
        th { color: var(--muted); font-weight: 600; }
        td code {
            font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
            color: #bfdbfe;
            word-break: break-all;
        }
        .pill {
            display: inline-flex;
            align-items: center;
            padding: 5px 9px;
            border-radius: 999px;
            font-size: 12px;
            font-weight: 700;
            background: rgba(148, 163, 184, 0.12);
        }
        .pill.good { background: rgba(74, 222, 128, 0.12); color: var(--good); }
        .pill.bad { background: rgba(251, 113, 133, 0.14); color: var(--bad); }
        .pill.warn { background: rgba(245, 158, 11, 0.14); color: var(--warn); }
        .table-actions {
            display: flex;
            gap: 8px;
            flex-wrap: wrap;
        }
        .table-actions button {
            padding: 7px 9px;
            font-size: 12px;
            border-radius: 10px;
        }
        .raw-grid {
            display: grid;
            grid-template-columns: repeat(2, minmax(0, 1fr));
            gap: 12px;
        }
        pre {
            margin: 0;
            min-height: 220px;
            padding: 14px;
            border-radius: 10px;
            overflow: auto;
            background: #020617;
            border: 1px solid rgba(148, 163, 184, 0.16);
            color: #dbeafe;
            font-size: 12px;
        }
        .empty {
            padding: 28px;
            border-radius: 10px;
            border: 1px dashed rgba(148, 163, 184, 0.25);
            text-align: center;
            color: var(--muted);
        }
        .nav-card .button-row { display: grid; grid-template-columns: 1fr; gap: 8px; }
        .nav-btn { justify-content: flex-start; }
        .nav-btn.active { background: rgba(59, 130, 246, 0.16); color: #93c5fd; border: 1px solid rgba(96, 165, 250, 0.30); }
        .view-hidden { display: none !important; }
        .tls-panel { font-size: 16px; }
        .tls-panel label { font-size: 14px; }
        .tls-panel input, .tls-panel select, .tls-panel textarea {
            font-size: 16px;
            padding: 14px 16px;
        }
        .tls-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; }
        .tls-grid .full { grid-column: 1 / -1; }
        textarea {
            width: 100%;
            min-height: 96px;
            border: 1px solid rgba(148, 163, 184, 0.22);
            background: rgba(15, 23, 42, 0.86);
            color: var(--text);
            padding: 14px 16px;
            border-radius: 8px;
            outline: none;
            resize: vertical;
            font-family: inherit;
        }
        .hint { font-size: 14px; color: var(--muted); line-height: 1.5; }
        details.meta-card summary { cursor: pointer; font-weight: 800; }
        .meta-list { display: grid; gap: 6px; margin-top: 10px; font-size: 12px; }
        @media (max-width: 1180px) {
            .shell { grid-template-columns: 1fr; }
            .sidebar { border-right: 0; border-bottom: 1px solid var(--line); }
            .cards { grid-template-columns: repeat(4, minmax(0, 1fr)); }
        }
        @media (max-width: 860px) {
            .cards { grid-template-columns: repeat(2, minmax(0, 1fr)); }
            .compact-controls { grid-template-columns: 1fr; }
            .filters { grid-template-columns: repeat(2, minmax(0, 1fr)); }
            .group-grid { grid-template-columns: 1fr; }
            .raw-grid { grid-template-columns: 1fr; }
        }
        @media (max-width: 640px) {
            .shell { width: 100%; margin: 0; }
            .admin-top { padding: 0 14px; }
            .cards { grid-template-columns: 1fr; }
            .filters { grid-template-columns: 1fr; }
            th:nth-child(2), td:nth-child(2), th:nth-child(6), td:nth-child(6), th:nth-child(7), td:nth-child(7), th:nth-child(10), td:nth-child(10), th:nth-child(11), td:nth-child(11) { display: none; }
        }
    </style>
</head>
<body>
    <section id="login-screen" class="login-screen">
        <form id="login-form" class="login-panel">
            <div class="login-logo">p</div>
            <div class="login-title">
                <div class="eyebrow" data-i18n="adminConsole">Admin Console</div>
                <h1 data-i18n="loginTitle">登录 proxysss</h1>
                <p class="muted" data-i18n="loginSubtitle">进入网关管理后台。</p>
            </div>
            <div class="login-form">
                <div class="filter">
                    <label for="login-username" data-i18n="username">Username</label>
                    <input id="login-username" autocomplete="username" value="__ADMIN_USER__" autofocus />
                </div>
                <div class="filter">
                    <label for="login-password" data-i18n="password">Password</label>
                    <input id="login-password" type="password" autocomplete="current-password" />
                </div>
                <button id="login-submit" class="primary" type="submit" data-i18n="login">登录</button>
                <div id="login-error" class="login-error"></div>
            </div>
        </form>
    </section>

    <header id="admin-top" class="admin-top app-hidden">
        <div><strong>proxysss</strong> <span class="muted" data-i18n="adminConsole">Admin Console</span></div>
        <div class="top-actions">
            <select id="language-select" class="language-select ghost" aria-label="Language">
                <option value="auto">Auto</option>
                <option value="zh-CN">中文</option>
                <option value="en-US">English</option>
            </select>
            <button id="logout" class="ghost" type="button" data-i18n="logout">退出登录</button>
        </div>
    </header>

    <main id="admin-app" class="shell app-hidden">
        <aside class="sidebar">
            <div class="brand">
                <div class="eyebrow" data-i18n="adminConsole">admin console</div>
                <h1>proxysss</h1>
                <p class="muted" data-i18n="brandSubtitle">Reverse proxy health, upstream state, and runtime stats.</p>
            </div>

            <section class="meta-card nav-card">
                <h3 data-i18n="consoleViews">Console Views</h3>
                <div class="button-row">
                    <button class="ghost nav-btn active" data-view="dashboard" data-i18n="dashboard">Dashboard</button>
                    <button class="ghost nav-btn" data-view="tls">TLS / ACME</button>
                    <button class="ghost nav-btn" data-view="domains" data-i18n="domainRoutes">Domain Routes</button>
                    <button class="ghost nav-btn" data-view="reverse" data-i18n="reverseProxy">Reverse Proxy</button>
                    <button class="ghost nav-btn" data-view="listeners" data-i18n="listeners">Listeners</button>
                    <button class="ghost nav-btn" data-view="filecloud">FileCloud</button>
                    <button class="ghost nav-btn" data-view="security" data-i18n="security">Security</button>
                </div>
            </section>

            <section class="meta-card">
                <h3 data-i18n="controls">Controls</h3>
                <div class="button-row">
                    <button id="load" class="primary" data-i18n="refreshDashboard">Refresh Dashboard</button>
                    <button id="toggle-auto" class="ghost">Auto Refresh: Off</button>
                </div>
            </section>

            <details class="meta-card">
                <summary data-i18n="apiEndpoints">API Endpoints</summary>
                <div class="meta-list muted">
                    <span>Stats: <strong>/v1/stats</strong></span>
                    <span>Upstreams: <strong>/v1/upstreams</strong></span>
                    <span>Config: <strong>/v1/config</strong></span>
                    <span>Routes: <strong>/v1/domain-routes/upsert</strong></span>
                    <span>Auth: <strong>Bearer token</strong></span>
                    <span>__ADMIN_HTTPS_HINT__</span>
                </div>
            </details>
        </aside>

        <section class="content" id="view-dashboard">
            <div class="topbar">
                <div>
                    <div class="eyebrow" data-i18n="runtimeOverview">Runtime Overview</div>
                    <h2 data-i18n="reverseProxyHealth">Reverse Proxy Health</h2>
                </div>
                <div id="load-state" class="status-dot" data-i18n="waitingRefresh">Waiting for refresh</div>
            </div>

            <section class="cards">
                <article class="card">
                    <span class="muted" data-i18n="httpRequests">HTTP Requests</span>
                    <strong id="card-http-requests">0</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="httpErrors">HTTP Errors</span>
                    <strong id="card-http-errors">0</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="healthyUpstreams">Healthy Upstreams</span>
                    <strong id="card-healthy">0</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="degradedUpstreams">Degraded Upstreams</span>
                    <strong id="card-degraded">0</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="processCpu">Process CPU</span>
                    <strong id="card-process-cpu">warming</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="processMemory">Process Memory</span>
                    <strong id="card-process-memory">0 MB</strong>
                </article>
                <article class="card">
                    <span class="muted" data-i18n="memoryPercent">Memory %</span>
                    <strong id="card-process-memory-percent">0%</strong>
                </article>
            </section>

            <section class="surface">
                <div class="surface-head">
                    <div class="surface-title">
                        <h3 data-i18n="groupedView">Grouped View</h3>
                    </div>
                    <div class="surface-actions">
                        <div class="compact-controls">
                            <input id="search" placeholder="Search route / upstream / listener" data-i18n-placeholder="searchPlaceholder" />
                            <select id="health-filter">
                                <option value="all" data-i18n="allHealth">All health</option>
                                <option value="healthy" data-i18n="healthy">Healthy</option>
                                <option value="degraded" data-i18n="degraded">Degraded</option>
                                <option value="manual" data-i18n="manualOffline">Manual offline</option>
                            </select>
                            <select id="group-by">
                                <option value="route" data-i18n="groupRoute">Group: route</option>
                                <option value="listener" data-i18n="groupListener">Group: listener</option>
                                <option value="protocol" data-i18n="groupProtocol">Group: protocol</option>
                                <option value="none" data-i18n="flat">Flat</option>
                            </select>
                        </div>
                        <span class="muted" id="group-count">0 groups</span>
                    </div>
                </div>
                <div id="group-view-wrap" class="empty" data-i18n="refreshGroupsHint">Refresh the dashboard to build aggregated health groups.</div>
            </section>

            <section class="surface">
                <div class="surface-head">
                    <div>
                        <h3 data-i18n="upstreamHealthTable">Upstream Health Table</h3>
                        <p class="muted" data-i18n="upstreamTableHint">Health, target, probe, and drain controls.</p>
                    </div>
                    <span class="muted" id="upstream-count">0 upstreams</span>
                </div>
                <div id="upstream-table-wrap" class="empty" data-i18n="refreshUpstreamsHint">Refresh the dashboard to load upstream health data.</div>
            </section>

            <section class="raw-grid">
                <article class="surface">
                    <div class="surface-head">
                        <h3>Stats JSON</h3>
                        <span class="muted">/v1/stats</span>
                    </div>
                    <pre id="stats-json">{}</pre>
                </article>
                <article class="surface">
                    <div class="surface-head">
                        <h3>Upstreams JSON</h3>
                        <span class="muted">/v1/upstreams</span>
                    </div>
                    <pre id="upstreams-json">[]</pre>
                </article>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-tls">
            <div class="topbar">
                <div>
                    <div class="eyebrow">Built-in ACME</div>
                    <h2>TLS / Certificates</h2>
                    <p class="hint">Fully embedded — no acme.sh, certbot, or cloud CLI. HTTP-01/TLS-ALPN-01 for normal domains; DNS-01 for wildcard via built-in cloud providers or manual TXT.</p>
                </div>
                <div id="tls-state" class="status-dot">Loading TLS summary</div>
            </div>

            <section class="cards">
                <article class="card"><span class="muted">TLS Mode</span><strong id="tls-mode">-</strong></article>
                <article class="card"><span class="muted">Challenge</span><strong id="tls-challenge">-</strong></article>
                <article class="card"><span class="muted">Cert Ready</span><strong id="tls-cert-ready">-</strong></article>
                <article class="card"><span class="muted">Domain Routes</span><strong id="tls-route-count">0</strong></article>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>Current TLS Summary</h3><p class="muted">Live runtime state from <code>/v1/tls/summary</code></p></div></div>
                <pre id="tls-summary-json">{}</pre>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>Auto HTTPS (HTTP-01 / TLS-ALPN-01)</h3><p class="muted">No DNS API key required.</p></div></div>
                <div class="tls-grid">
                    <div class="filter full"><label for="auto-domains">Domains (one per line)</label><textarea id="auto-domains" placeholder="example.com&#10;www.example.com"></textarea></div>
                    <div class="filter"><label for="auto-email">ACME Email</label><input id="auto-email" placeholder="admin@example.com" /></div>
                    <div class="filter"><label for="auto-production">Production</label><select id="auto-production"><option value="true">Let's Encrypt Production</option><option value="false">Staging</option></select></div>
                    <div class="filter full"><button id="save-auto-https" class="primary">Save Auto HTTPS</button><span class="hint" id="auto-https-result"></span></div>
                </div>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>Wildcard DNS-01</h3><p class="muted">Built-in Cloudflare / Aliyun / Tencent / Volcengine / AWS / Azure / Google, or <code>manual</code> without API key.</p></div></div>
                <div class="tls-grid">
                    <div class="filter full"><label for="wildcard-domains">Domains (include <code>*.example.com</code>)</label><textarea id="wildcard-domains" placeholder="example.com&#10;*.example.com"></textarea></div>
                    <div class="filter"><label for="wildcard-email">ACME Email</label><input id="wildcard-email" placeholder="admin@example.com" /></div>
                    <div class="filter"><label for="wildcard-provider">DNS Provider</label><select id="wildcard-provider"></select></div>
                    <div class="filter full" id="dns-credentials"></div>
                    <div class="filter full"><button id="save-wildcard-tls" class="primary">Save Wildcard DNS-01</button><span class="hint" id="wildcard-tls-result"></span></div>
                </div>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>On-Demand TLS</h3><p class="muted">First-hit certificate issuance for allowed host patterns.</p></div></div>
                <div class="tls-grid">
                    <div class="filter"><label for="on-demand-enabled">Enabled</label><select id="on-demand-enabled"><option value="false">false</option><option value="true">true</option></select></div>
                    <div class="filter full"><label for="on-demand-allow">Allow patterns (one per line)</label><textarea id="on-demand-allow" placeholder="*.example.com"></textarea></div>
                    <div class="filter full"><button id="save-on-demand" class="primary">Save On-Demand TLS</button><button id="issue-tls-now" class="ghost">Issue Certificate Now</button><span class="hint" id="on-demand-result"></span></div>
                </div>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>SNI Manual Certificates</h3><p class="muted">Per-host TLS material under <code>http.tls.certificates</code>. Upload PEM inline or reference existing files.</p></div></div>
                <div id="sni-certs-wrap" class="empty">Loading SNI certificates...</div>
                <div class="tls-grid">
                    <div class="filter full"><label for="sni-domains">Domains (one per line)</label><textarea id="sni-domains" placeholder="api.example.com&#10;*.internal.example.com"></textarea></div>
                    <div class="filter"><label for="sni-cert-path">Cert Path (optional if PEM below)</label><input id="sni-cert-path" placeholder="./certs/sni/api.crt" /></div>
                    <div class="filter"><label for="sni-key-path">Key Path (optional if PEM below)</label><input id="sni-key-path" placeholder="./certs/sni/api.key" /></div>
                    <div class="filter full"><label for="sni-cert-pem">Cert PEM (optional upload)</label><textarea id="sni-cert-pem" placeholder="-----BEGIN CERTIFICATE-----"></textarea></div>
                    <div class="filter full"><label for="sni-key-pem">Key PEM (optional upload)</label><textarea id="sni-key-pem" placeholder="-----BEGIN PRIVATE KEY-----"></textarea></div>
                    <div class="filter full"><button id="save-sni-cert" class="primary">Save SNI Certificate</button><span class="hint" id="sni-cert-result"></span></div>
                </div>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-domains">
            <div class="topbar">
                <div>
                    <div class="eyebrow">Host Routing</div>
                    <h2>Domain Routes</h2>
                    <p class="hint">Persisted into <code>proxysss.yaml</code> and hot-reloaded. Requires <code>admin.enable_write_ops=true</code>.</p>
                </div>
                <div id="domain-state" class="status-dot">Loading routes</div>
            </div>

            <section class="surface">
                <div class="surface-head"><div><h3>Active Domain Routes</h3></div><span class="muted" id="domain-route-count">0 routes</span></div>
                <div id="domain-routes-wrap" class="empty">Refresh to load domain routes.</div>
            </section>

            <section class="surface">
                <div class="surface-head"><div><h3>Upsert Domain Route</h3></div></div>
                <div class="tls-grid">
                    <div class="filter"><label for="route-name">Route Name</label><input id="route-name" placeholder="api" /></div>
                    <div class="filter"><label for="route-upstream">Upstream</label><input id="route-upstream" placeholder="http://127.0.0.1:8080" /></div>
                    <div class="filter"><label for="route-path-prefix">Path Prefix</label><input id="route-path-prefix" value="/" /></div>
                    <div class="filter full"><label for="route-domains">Hostnames (comma separated)</label><input id="route-domains" placeholder="api.example.com, *.api.example.com" /></div>
                    <div class="filter full"><label for="route-upstreams">Extra Upstreams (one per line, optional)</label><textarea id="route-upstreams" placeholder="http://10.0.0.13:8080"></textarea></div>
                    <div class="filter full"><label><input id="route-strip-prefix" type="checkbox" /> Strip path prefix before upstream</label></div>
                    <div class="filter full"><button id="save-domain-route" class="primary">Save Domain Route</button><span class="hint" id="domain-route-result"></span><span class="hint">Advanced fields (weights, cache, ssl): use JSON API — see docs/AGENT-API.md</span></div>
                </div>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-reverse">
            <div class="topbar"><div><div class="eyebrow">Path Routing</div><h2>Reverse Proxy Routes</h2></div><div id="reverse-state" class="status-dot">Loading</div></div>
            <section class="surface">
                <div class="surface-head"><h3>Active Routes</h3><span class="muted" id="reverse-count">0 routes</span></div>
                <div id="reverse-routes-wrap" class="empty">Loading...</div>
            </section>
            <section class="surface">
                <div class="surface-head"><h3>Upsert Route</h3></div>
                <div class="tls-grid">
                    <div class="filter"><label for="reverse-name">Name</label><input id="reverse-name" placeholder="api" /></div>
                    <div class="filter"><label for="reverse-prefix">Path Prefix</label><input id="reverse-prefix" value="/api" /></div>
                    <div class="filter"><label for="reverse-upstream">Upstream</label><input id="reverse-upstream" placeholder="http://127.0.0.1:8080" /></div>
                    <div class="filter full"><label for="reverse-hosts">Hosts (comma separated, optional)</label><input id="reverse-hosts" placeholder="api.example.com" /></div>
                    <div class="filter full"><label for="reverse-upstreams">Extra Upstreams (one per line, optional)</label><textarea id="reverse-upstreams" placeholder="http://127.0.0.1:9001"></textarea></div>
                    <div class="filter full"><label><input id="reverse-strip-prefix" type="checkbox" checked /> Strip path prefix</label></div>
                    <div class="filter full"><button id="save-reverse-route" class="primary">Save Route</button><span class="hint" id="reverse-route-result"></span></div>
                </div>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-listeners">
            <div class="topbar"><div><div class="eyebrow">Stream Layer</div><h2>TCP / UDP / Stream Routes</h2></div><div id="listeners-state" class="status-dot">Loading</div></div>
            <section class="surface">
                <div class="surface-head"><h3>TCP Listeners</h3></div>
                <div id="tcp-listeners-wrap" class="empty">Loading...</div>
                <div class="tls-grid" style="margin-top:14px">
                    <div class="filter"><label for="tcp-name">Name</label><input id="tcp-name" placeholder="game" /></div>
                    <div class="filter"><label for="tcp-bind">Bind</label><input id="tcp-bind" placeholder="0.0.0.0:7000" /></div>
                    <div class="filter"><label for="tcp-upstream">Upstream</label><input id="tcp-upstream" placeholder="127.0.0.1:9000" /></div>
                    <div class="filter full"><button id="save-tcp-listener" class="primary">Save TCP Listener</button><span class="hint" id="tcp-listener-result"></span></div>
                </div>
            </section>
            <section class="surface">
                <div class="surface-head"><h3>UDP Listeners</h3></div>
                <div id="udp-listeners-wrap" class="empty">Loading...</div>
                <div class="tls-grid" style="margin-top:14px">
                    <div class="filter"><label for="udp-name">Name</label><input id="udp-name" placeholder="voice" /></div>
                    <div class="filter"><label for="udp-bind">Bind</label><input id="udp-bind" placeholder="0.0.0.0:7001" /></div>
                    <div class="filter"><label for="udp-upstream">Upstream</label><input id="udp-upstream" placeholder="127.0.0.1:9001" /></div>
                    <div class="filter full"><button id="save-udp-listener" class="primary">Save UDP Listener</button><span class="hint" id="udp-listener-result"></span></div>
                </div>
            </section>
            <section class="surface">
                <div class="surface-head"><h3>Stream Routes (SNI)</h3></div>
                <div id="stream-routes-wrap" class="empty">Loading...</div>
                <div class="tls-grid" style="margin-top:14px">
                    <div class="filter"><label for="stream-name">Name</label><input id="stream-name" placeholder="redis-prod" /></div>
                    <div class="filter"><label for="stream-listen">Listen</label><input id="stream-listen" placeholder="6379" /></div>
                    <div class="filter"><label for="stream-upstream">Upstream</label><input id="stream-upstream" placeholder="127.0.0.1:6379" /></div>
                    <div class="filter full"><label for="stream-domains">Domains (comma separated)</label><input id="stream-domains" placeholder="redis.example.com" /></div>
                    <div class="filter full"><button id="save-stream-route" class="primary">Save Stream Route</button><span class="hint" id="stream-route-result"></span></div>
                </div>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-filecloud">
            <div class="topbar"><div><div class="eyebrow">FileCloud</div><h2>Shared Folder</h2><p class="hint">Password-protected interactive file space — not nginx WebDAV. <a id="filecloud-open-link" href='#' target="_blank" rel="noopener" style="display:none">Open FileCloud UI</a></p></div><div id="filecloud-state" class="status-dot">Loading</div></div>
            <section class="surface">
                <div class="surface-head"><h3>Current FileCloud</h3></div>
                <pre id="filecloud-summary-json">{}</pre>
            </section>
            <section class="surface">
                <div class="surface-head"><h3>Configure FileCloud</h3></div>
                <div class="tls-grid">
                    <div class="filter"><label for="fc-enabled">Enabled</label><select id="fc-enabled"><option value="false">false</option><option value="true">true</option></select></div>
                    <div class="filter"><label for="fc-prefix">Path Prefix</label><input id="fc-prefix" value="/filecloud" /></div>
                    <div class="filter"><label for="fc-root">Root Directory</label><input id="fc-root" placeholder="./filecloud-data" /></div>
                    <div class="filter"><label for="fc-title">Title</label><input id="fc-title" placeholder="Shared Files" /></div>
                    <div class="filter"><label for="fc-password">Password (blank keeps existing)</label><input id="fc-password" type="password" placeholder="new password" /></div>
                    <div class="filter"><label for="fc-max-upload">Max Upload Bytes</label><input id="fc-max-upload" type="number" placeholder="536870912" /></div>
                    <div class="filter"><label for="fc-cdn-cache">CDN Cache Secs</label><input id="fc-cdn-cache" type="number" placeholder="86400" /></div>
                    <div class="filter full"><label><input id="fc-allow-upload" type="checkbox" checked /> Allow upload</label> <label><input id="fc-allow-delete" type="checkbox" checked /> Delete</label> <label><input id="fc-allow-mkdir" type="checkbox" checked /> Mkdir</label> <label><input id="fc-allow-move" type="checkbox" checked /> Move</label></div>
                    <div class="filter full"><button id="save-filecloud" class="primary">Save FileCloud</button><span class="hint" id="filecloud-result"></span></div>
                </div>
            </section>
        </section>

        <section class="content view-hidden tls-panel" id="view-security">
            <div class="topbar"><div><div class="eyebrow">Security</div><h2>Dynamic IP Blacklist</h2><p class="hint">Runtime bans persisted when <code>security.dynamic_blacklist.enabled=true</code>.</p></div><div id="security-state" class="status-dot">Loading</div></div>
            <section class="surface">
                <div class="surface-head"><h3>Active Bans</h3></div>
                <div id="blacklist-wrap" class="empty">Loading...</div>
            </section>
            <section class="surface">
                <div class="surface-head"><h3>Add Ban</h3></div>
                <div class="tls-grid">
                    <div class="filter"><label for="blacklist-ip">IP</label><input id="blacklist-ip" placeholder="203.0.113.5" /></div>
                    <div class="filter"><label for="blacklist-ban-secs">Ban Seconds</label><input id="blacklist-ban-secs" type="number" placeholder="3600" /></div>
                    <div class="filter full"><button id="save-blacklist-add" class="primary">Add Ban</button><span class="hint" id="blacklist-result"></span></div>
                </div>
            </section>
        </section>
    </main>

    <script>
        const stateNode = document.getElementById('load-state');
        const statsJson = document.getElementById('stats-json');
        const upstreamsJson = document.getElementById('upstreams-json');
        const upstreamWrap = document.getElementById('upstream-table-wrap');
        const groupWrap = document.getElementById('group-view-wrap');
        const upstreamCount = document.getElementById('upstream-count');
        const groupCount = document.getElementById('group-count');
        const autoToggle = document.getElementById('toggle-auto');
        const searchInput = document.getElementById('search');
        const healthFilter = document.getElementById('health-filter');
        const groupBy = document.getElementById('group-by');
        const sessionKey = 'proxysss.admin.auth';
        const loginScreen = document.getElementById('login-screen');
        const adminTop = document.getElementById('admin-top');
        const adminApp = document.getElementById('admin-app');
        const loginForm = document.getElementById('login-form');
        const loginError = document.getElementById('login-error');
        const languageSelect = document.getElementById('language-select');
        const i18n = {
            'en-US': {
                adminConsole: 'Admin Console',
                loginTitle: 'Sign in to proxysss',
                loginSubtitle: 'Enter the gateway admin console.',
                username: 'Username',
                password: 'Password',
                login: 'Sign in',
                logout: 'Sign out',
                brandSubtitle: 'Reverse proxy health, upstream state, and runtime stats.',
                consoleViews: 'Console Views',
                dashboard: 'Dashboard',
                domainRoutes: 'Domain Routes',
                reverseProxy: 'Reverse Proxy',
                listeners: 'Listeners',
                security: 'Security',
                controls: 'Controls',
                refreshDashboard: 'Refresh Dashboard',
                autoRefreshOff: 'Auto Refresh: Off',
                autoRefreshOn: 'Auto Refresh: On',
                apiEndpoints: 'API Endpoints',
                runtimeOverview: 'Runtime Overview',
                reverseProxyHealth: 'Reverse Proxy Health',
                waitingRefresh: 'Waiting for refresh',
                refreshingDashboard: 'Refreshing dashboard',
                dashboardFresh: 'Dashboard fresh',
                refreshFailed: 'Refresh failed',
                httpRequests: 'HTTP Requests',
                httpErrors: 'HTTP Errors',
                healthyUpstreams: 'Healthy Upstreams',
                degradedUpstreams: 'Degraded Upstreams',
                processCpu: 'Process CPU',
                processMemory: 'Process Memory',
                memoryPercent: 'Memory %',
                groupedView: 'Grouped View',
                searchPlaceholder: 'Search route / upstream / listener',
                allHealth: 'All health',
                healthy: 'Healthy',
                degraded: 'Degraded',
                manualOffline: 'Manual offline',
                groupRoute: 'Group: route',
                groupListener: 'Group: listener',
                groupProtocol: 'Group: protocol',
                flat: 'Flat',
                refreshGroupsHint: 'Refresh the dashboard to build aggregated health groups.',
                noUpstreamsMatch: 'No upstreams match the current filters.',
                upstreamHealthTable: 'Upstream Health Table',
                upstreamTableHint: 'Health, target, probe, and drain controls.',
                refreshUpstreamsHint: 'Refresh the dashboard to load upstream health data.',
                noUpstreamState: 'No upstream runtime state recorded yet. Send traffic or enable active health checks.',
                groups: 'groups',
                upstreams: 'upstreams',
                invalidCredentials: 'Invalid username or password',
                loginExpired: 'Session expired. Please sign in again.',
                takeOffline: 'Take Offline',
                restore: 'Restore',
                tableHealth: 'Health',
                tableTarget: 'Target',
                tableActive: 'Active',
                tableProbe: 'Probe',
                tableManual: 'Manual',
                tableAction: 'Action',
                tableLastCheck: 'Last Check',
                warming: 'warming',
                unknown: 'unknown',
                pass: 'pass',
                fail: 'fail',
                quarantined: 'quarantined',
                unmapped: 'unmapped',
                active: 'active',
                passive: 'passive',
                rtt: 'rtt',
                failureShort: 'fail',
                quarantine: 'quarantine',
            },
            'zh-CN': {
                adminConsole: '管理控制台',
                loginTitle: '登录 proxysss',
                loginSubtitle: '进入网关管理后台。',
                username: '用户名',
                password: '密码',
                login: '登录',
                logout: '退出登录',
                brandSubtitle: '反向代理健康、上游状态、运行指标。',
                consoleViews: '控制台视图',
                dashboard: '仪表盘',
                domainRoutes: '域名路由',
                reverseProxy: '反向代理',
                listeners: '监听器',
                security: '安全',
                controls: '操作',
                refreshDashboard: '刷新仪表盘',
                autoRefreshOff: '自动刷新：关',
                autoRefreshOn: '自动刷新：开',
                apiEndpoints: 'API 端点',
                runtimeOverview: '运行概览',
                reverseProxyHealth: '反向代理健康',
                waitingRefresh: '等待刷新',
                refreshingDashboard: '正在刷新',
                dashboardFresh: '仪表盘已刷新',
                refreshFailed: '刷新失败',
                httpRequests: 'HTTP 请求',
                httpErrors: 'HTTP 错误',
                healthyUpstreams: '健康上游',
                degradedUpstreams: '异常上游',
                processCpu: '进程 CPU',
                processMemory: '进程内存',
                memoryPercent: '内存占比',
                groupedView: '分组视图',
                searchPlaceholder: '搜索路由 / 上游 / 监听器',
                allHealth: '全部健康状态',
                healthy: '健康',
                degraded: '异常',
                manualOffline: '手动下线',
                groupRoute: '分组：路由',
                groupListener: '分组：监听器',
                groupProtocol: '分组：协议',
                flat: '平铺',
                refreshGroupsHint: '刷新仪表盘后显示聚合健康分组。',
                noUpstreamsMatch: '没有上游匹配当前筛选。',
                upstreamHealthTable: '上游健康表',
                upstreamTableHint: '健康、目标、探测和摘流操作。',
                refreshUpstreamsHint: '刷新仪表盘后加载上游健康数据。',
                noUpstreamState: '暂无上游运行状态。请先产生流量或开启主动健康检查。',
                groups: '组',
                upstreams: '上游',
                invalidCredentials: '用户名或密码错误',
                loginExpired: '登录已失效，请重新登录',
                takeOffline: '下线',
                restore: '恢复',
                tableHealth: '健康',
                tableTarget: '目标',
                tableActive: '活跃',
                tableProbe: '探测',
                tableManual: '手动',
                tableAction: '操作',
                tableLastCheck: '最后检查',
                warming: '预热中',
                unknown: '未知',
                pass: '通过',
                fail: '失败',
                quarantined: '隔离',
                unmapped: '未映射',
                active: '主动',
                passive: '被动',
                rtt: '延迟',
                failureShort: '失败',
                quarantine: '隔离',
            },
        };
        let currentLocale = 'en-US';
        let authValue = '';
        let autoRefresh = false;
        let autoTimer = null;
        let latestUpstreams = [];
        let latestStats = null;

        function authHeader() {
            return authValue;
        }

        function detectLocale() {
            const saved = localStorage.getItem('proxysss.admin.locale') || 'auto';
            languageSelect.value = saved;
            if (saved !== 'auto') return saved;
            const languages = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || 'en-US'];
            return languages.some(value => String(value).toLowerCase().startsWith('zh')) ? 'zh-CN' : 'en-US';
        }

        function t(key) {
            return (i18n[currentLocale] && i18n[currentLocale][key]) || i18n['en-US'][key] || key;
        }

        function applyLocale() {
            currentLocale = detectLocale();
            document.documentElement.lang = currentLocale;
            document.querySelectorAll('[data-i18n]').forEach(node => {
                node.textContent = t(node.dataset.i18n);
            });
            document.querySelectorAll('[data-i18n-placeholder]').forEach(node => {
                node.placeholder = t(node.dataset.i18nPlaceholder);
            });
            autoToggle.textContent = autoRefresh ? t('autoRefreshOn') : t('autoRefreshOff');
            if (stateNode.dataset.stateKey) stateNode.textContent = t(stateNode.dataset.stateKey);
        }

        function persistAuth(user, token, expiresAt) {
            authValue = 'Bearer ' + token;
            sessionStorage.setItem(sessionKey, JSON.stringify({ user, auth: authValue, expiresAt }));
        }

        function clearAuth() {
            authValue = '';
            sessionStorage.removeItem(sessionKey);
        }

        function showLogin(message) {
            loginScreen.classList.remove('hidden');
            adminTop.classList.add('app-hidden');
            adminApp.classList.add('app-hidden');
            if (message) loginError.textContent = message;
        }

        function showApp() {
            loginScreen.classList.add('hidden');
            adminTop.classList.remove('app-hidden');
            adminApp.classList.remove('app-hidden');
            loginError.textContent = '';
        }

        async function loginWithPassword(user, pass) {
            const response = await fetch('/v1/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username: user, password: pass }),
            });
            const data = await response.json().catch(() => ({}));
            if (!response.ok) {
                throw new Error(data.error || t('invalidCredentials'));
            }
            persistAuth(data.username || user, data.access_token, data.expires_at);
            return data;
        }

        async function verifyAuth() {
            const response = await fetch('/v1/stats', {
                headers: { Authorization: authHeader() },
            });
            if (!response.ok) {
                throw new Error(response.status === 401 ? t('invalidCredentials') : `/v1/stats -> ${response.status}`);
            }
            return response.json();
        }

        function setState(message, kind, key) {
            stateNode.textContent = key ? t(key) : message;
            stateNode.dataset.stateKey = key || '';
            stateNode.className = 'status-dot ' + (kind || '');
        }

        function formatNumber(value) {
            return new Intl.NumberFormat().format(value || 0);
        }

        function formatPercent(value) {
            if (value === null || value === undefined || Number.isNaN(Number(value))) return t('warming');
            return Number(value).toFixed(1) + '%';
        }

        function formatMegabytes(value) {
            if (value === null || value === undefined || Number.isNaN(Number(value))) return t('unknown');
            return Number(value).toFixed(1) + ' MB';
        }

        function formatTimestamp(value) {
            if (!value) return 'never';
            try {
                return new Date(value).toLocaleString();
            } catch {
                return 'invalid';
            }
        }

        function renderSummary(stats, upstreams) {
            const healthy = upstreams.filter(item => item.healthy).length;
            const degraded = upstreams.length - healthy;
            document.getElementById('card-http-requests').textContent = formatNumber(stats.http_requests);
            document.getElementById('card-http-errors').textContent = formatNumber(stats.http_errors);
            document.getElementById('card-healthy').textContent = formatNumber(healthy);
            document.getElementById('card-degraded').textContent = formatNumber(degraded);
            const process = stats.process || {};
            document.getElementById('card-process-cpu').textContent = formatPercent(process.cpu_percent);
            document.getElementById('card-process-memory').textContent = formatMegabytes(process.memory_mb);
            document.getElementById('card-process-memory-percent').textContent = formatPercent(process.memory_percent);
            upstreamCount.textContent = `${upstreams.length} ${t('upstreams')}`;
        }

        function healthPill(item) {
            if (item.manually_disabled) return `<span class="pill warn">${t('manualOffline')}</span>`;
            if (item.healthy) return `<span class="pill good">${t('healthy')}</span>`;
            if (item.active_healthy === null || item.active_healthy === undefined) return `<span class="pill warn">${t('warming')}</span>`;
            return `<span class="pill bad">${t('degraded')}</span>`;
        }

        function probePill(item) {
            if (item.active_healthy === true) return `<span class="pill good">${t('pass')}</span>`;
            if (item.active_healthy === false) return `<span class="pill bad">${t('fail')}</span>`;
            return `<span class="pill warn">${t('unknown')}</span>`;
        }

        function routeLabel(item) {
            return (item.route_names && item.route_names.length ? item.route_names.join(', ') : t('unmapped'));
        }

        function filteredUpstreams() {
            const query = searchInput.value.trim().toLowerCase();
            const filter = healthFilter.value;

            return latestUpstreams.filter(item => {
                if (filter === 'healthy' && !item.healthy) return false;
                if (filter === 'degraded' && item.healthy) return false;
                if (filter === 'manual' && !item.manually_disabled) return false;

                if (!query) return true;
                const haystack = [
                    item.protocol,
                    item.listener,
                    item.upstream,
                    routeLabel(item),
                    item.key,
                ].join(' ').toLowerCase();
                return haystack.includes(query);
            });
        }

        function groupKey(item) {
            switch (groupBy.value) {
                case 'protocol': return item.protocol || 'unknown';
                case 'listener': return item.listener || 'default';
                case 'none': return item.key;
                case 'route':
                default:
                    return (item.route_names && item.route_names.length ? item.route_names[0] : `${item.protocol}:${item.listener}`);
            }
        }

        function renderGroups(upstreams) {
            if (!upstreams.length) {
                groupWrap.className = 'empty';
                groupWrap.textContent = t('noUpstreamsMatch');
                groupCount.textContent = `0 ${t('groups')}`;
                return;
            }

            const grouped = new Map();
            for (const item of upstreams) {
                const key = groupKey(item);
                if (!grouped.has(key)) grouped.set(key, []);
                grouped.get(key).push(item);
            }

            groupCount.textContent = `${grouped.size} ${t('groups')}`;
            groupWrap.className = 'group-grid';
            groupWrap.innerHTML = Array.from(grouped.entries()).map(([key, items]) => {
                const healthy = items.filter(item => item.healthy).length;
                const manual = items.filter(item => item.manually_disabled).length;
                const protocols = Array.from(new Set(items.map(item => item.protocol))).join(', ');
                return `
                    <article class="group-card">
                        <strong>${key}</strong>
                        <div class="group-meta">
                            <span class="pill ${healthy === items.length ? 'good' : healthy === 0 ? 'bad' : 'warn'}">${healthy}/${items.length} ${t('healthy')}</span>
                            ${manual ? `<span class="pill warn">${manual} ${t('manualOffline')}</span>` : ''}
                            <span class="pill">${protocols}</span>
                        </div>
                        <div class="muted">${items.map(item => item.upstream).join('<br/>')}</div>
                    </article>`;
            }).join('');
        }

        async function toggleUpstream(action, key) {
            const reason = action === 'disable' ? window.prompt('Reason for taking this upstream offline', 'manual drain') : null;
            const response = await fetch(`/v1/upstreams/${action}`, {
                method: 'POST',
                headers: {
                    Authorization: authHeader(),
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ key, reason }),
            });
            if (!response.ok) {
                throw new Error(`/v1/upstreams/${action} -> ${response.status} ${response.statusText}`);
            }
            await refreshDashboard();
        }

        function renderUpstreams(upstreams) {
            if (!upstreams.length) {
                upstreamWrap.className = 'empty';
                upstreamWrap.textContent = t('noUpstreamState');
                return;
            }

            const rows = upstreams.map(item => {
                const status = item.active_probe_status ?? '-';
                const error = item.active_probe_error ? `<div class="muted">${item.active_probe_error}</div>` : '';
                const quarantine = item.quarantine_remaining_secs > 0 ? `${item.quarantine_remaining_secs}s` : '-';
                const rtt = item.active_probe_rtt_ms ?? '-';
                const routeNames = routeLabel(item);
                const targetMeta = `${item.protocol} / ${item.listener}`;
                const probeMeta = [
                    `${t('active')} ${probePill(item)}`,
                    `${t('passive')} ${item.passive_healthy ? `<span class="pill good">${t('pass')}</span>` : `<span class="pill bad">${t('quarantined')}</span>`}`,
                    `http ${status}`,
                    `${t('rtt')} ${rtt}ms`,
                    `${t('failureShort')} ${item.consecutive_failures}`,
                    quarantine !== '-' ? `${t('quarantine')} ${quarantine}` : '',
                ].filter(Boolean).join(' ');
                const actionButton = item.manually_disabled
                    ? `<button class="success" data-action="enable" data-key="${item.key}">${t('restore')}</button>`
                    : `<button class="danger" data-action="disable" data-key="${item.key}">${t('takeOffline')}</button>`;
                return `
                    <tr>
                        <td>${healthPill(item)}</td>
                        <td><code>${item.upstream}</code><div class="muted">${routeNames}</div><div class="muted">${targetMeta}</div>${error}</td>
                        <td>${item.active_connections}</td>
                        <td><div class="group-meta">${probeMeta}</div><div class="muted">${item.active_probe_kind ?? '-'}</div></td>
                        <td>${item.manual_reason ?? '-'}</td>
                        <td><div class="table-actions">${actionButton}</div></td>
                        <td>${formatTimestamp(item.active_probe_checked_at_unix_ms)}</td>
                    </tr>`;
            }).join('');

            upstreamWrap.className = '';
            upstreamWrap.innerHTML = `
                <table>
                    <thead>
                        <tr>
                            <th>${t('tableHealth')}</th>
                            <th>${t('tableTarget')}</th>
                            <th>${t('tableActive')}</th>
                            <th>${t('tableProbe')}</th>
                            <th>${t('tableManual')}</th>
                            <th>${t('tableAction')}</th>
                            <th>${t('tableLastCheck')}</th>
                        </tr>
                    </thead>
                    <tbody>${rows}</tbody>
                </table>`;
        }

        async function loadJson(path) {
            const response = await fetch(path, {
                headers: { Authorization: authHeader() },
            });
            if (response.status === 401) {
                clearAuth();
                showLogin(t('loginExpired'));
            }
            if (!response.ok) {
                throw new Error(`${path} -> ${response.status} ${response.statusText}`);
            }
            return response.json();
        }

        async function refreshDashboard() {
            setState('', '', 'refreshingDashboard');
            try {
                const [stats, upstreamsPayload] = await Promise.all([
                    loadJson('/v1/stats'),
                    loadJson('/v1/upstreams'),
                ]);
                latestStats = stats;
                latestUpstreams = upstreamsPayload.items || [];
                const upstreams = filteredUpstreams();
                statsJson.textContent = JSON.stringify(stats, null, 2);
                upstreamsJson.textContent = JSON.stringify(upstreamsPayload, null, 2);
                renderSummary(stats, latestUpstreams);
                renderGroups(upstreams);
                renderUpstreams(upstreams);
                setState('', upstreams.every(item => item.healthy) ? 'ok' : 'bad', 'dashboardFresh');
            } catch (error) {
                setState('', 'bad', 'refreshFailed');
                statsJson.textContent = String(error);
                upstreamsJson.textContent = String(error);
                upstreamWrap.className = 'empty';
                upstreamWrap.textContent = String(error);
                groupWrap.className = 'empty';
                groupWrap.textContent = String(error);
            }
        }

        document.getElementById('load').addEventListener('click', refreshDashboard);
        loginForm.addEventListener('submit', async (event) => {
            event.preventDefault();
            loginError.textContent = '';
            const user = document.getElementById('login-username').value.trim();
            const pass = document.getElementById('login-password').value;
            try {
                await loginWithPassword(user, pass);
                const stats = await verifyAuth();
                showApp();
                document.getElementById('login-password').value = '';
                statsJson.textContent = JSON.stringify(stats, null, 2);
                await refreshDashboard();
            } catch (error) {
                clearAuth();
                showLogin(String(error.message || error));
            }
        });
        document.getElementById('logout').addEventListener('click', () => {
            clearAuth();
            clearInterval(autoTimer);
            autoRefresh = false;
            autoToggle.textContent = t('autoRefreshOff');
            showLogin('');
            document.getElementById('login-password').value = '';
        });
        languageSelect.addEventListener('change', () => {
            localStorage.setItem('proxysss.admin.locale', languageSelect.value);
            applyLocale();
            if (latestUpstreams.length) {
                const upstreams = filteredUpstreams();
                if (latestStats) renderSummary(latestStats, latestUpstreams);
                renderGroups(upstreams);
                renderUpstreams(upstreams);
            }
        });
        [searchInput, healthFilter, groupBy].forEach(node => {
            node.addEventListener('input', refreshDashboard);
            node.addEventListener('change', refreshDashboard);
        });
        upstreamWrap.addEventListener('click', async (event) => {
            const button = event.target.closest('button[data-action]');
            if (!button) return;
            try {
                await toggleUpstream(button.dataset.action, button.dataset.key);
            } catch (error) {
                setState(String(error), 'bad');
            }
        });
        autoToggle.addEventListener('click', () => {
            autoRefresh = !autoRefresh;
            autoToggle.textContent = autoRefresh ? t('autoRefreshOn') : t('autoRefreshOff');
            clearInterval(autoTimer);
            if (autoRefresh) {
                autoTimer = setInterval(refreshDashboard, 5000);
                refreshDashboard();
            }
        });

        const views = {
            dashboard: document.getElementById('view-dashboard'),
            tls: document.getElementById('view-tls'),
            domains: document.getElementById('view-domains'),
            reverse: document.getElementById('view-reverse'),
            listeners: document.getElementById('view-listeners'),
            filecloud: document.getElementById('view-filecloud'),
            security: document.getElementById('view-security'),
        };
        let dnsProviders = [];

        function switchView(name) {
            Object.entries(views).forEach(([key, node]) => {
                if (!node) return;
                node.classList.toggle('view-hidden', key !== name);
            });
            document.querySelectorAll('.nav-btn').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.view === name);
            });
            if (name === 'tls') refreshTlsPanel();
            if (name === 'domains') refreshDomainRoutes();
            if (name === 'reverse') refreshReverseRoutes();
            if (name === 'listeners') refreshListenersPanel();
            if (name === 'filecloud') refreshFileCloudPanel();
            if (name === 'security') refreshSecurityPanel();
        }

        document.querySelectorAll('.nav-btn').forEach(btn => {
            btn.addEventListener('click', () => switchView(btn.dataset.view));
        });

        function parseDomainLines(value) {
            return value.split(/[\n,]+/).map(item => item.trim()).filter(Boolean);
        }

        function renderCredentialFields(providerId) {
            const container = document.getElementById('dns-credentials');
            container.innerHTML = '';
            const provider = dnsProviders.find(item => item.id === providerId);
            if (!provider || provider.id === 'manual') {
                container.innerHTML = '<p class="hint">manual 模式无需 API Key：proxysss 会输出 TXT 记录并通过公网 DNS 轮询验证。</p>';
                return;
            }
            for (const spec of provider.credential_keys || []) {
                const optional = spec.endsWith('?');
                const keys = spec.replace(/\?$/, '').split('+');
                for (const key of keys) {
                    const id = `cred-${key}`;
                    container.insertAdjacentHTML('beforeend', `
                        <div class="filter">
                            <label for="${id}">${key}${optional ? ' (optional)' : ''}</label>
                            <input id="${id}" data-cred-key="${key}" placeholder="${key}" />
                        </div>`);
                }
            }
        }

        function collectCredentials() {
            const credentials = {};
            document.querySelectorAll('#dns-credentials [data-cred-key]').forEach(input => {
                if (input.value.trim()) credentials[input.dataset.credKey] = input.value.trim();
            });
            return credentials;
        }

        async function postJson(path, payload) {
            const response = await fetch(path, {
                method: 'POST',
                headers: {
                    Authorization: authHeader(),
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(payload),
            });
            const data = await response.json().catch(() => ({}));
            if (!response.ok) {
                throw new Error(data.error || `${path} -> ${response.status}`);
            }
            return data;
        }

        async function refreshTlsPanel() {
            const tlsState = document.getElementById('tls-state');
            try {
                const [summaryPayload, providerPayloadRaw] = await Promise.all([
                    loadJson('/v1/tls/summary'),
                    dnsProviders.length ? Promise.resolve({ providers: dnsProviders }) : loadJson('/v1/tls/dns-providers'),
                ]);
                const tls = summaryPayload.tls || {};
                const providerPayload = providerPayloadRaw.providers ? providerPayloadRaw : { providers: providerPayloadRaw.providers || [] };
                dnsProviders = providerPayload.providers || providerPayloadRaw.providers || dnsProviders;
                document.getElementById('tls-summary-json').textContent = JSON.stringify(tls, null, 2);
                document.getElementById('tls-mode').textContent = tls.mode || '-';
                document.getElementById('tls-challenge').textContent = tls.challenge || '-';
                document.getElementById('tls-cert-ready').textContent = (tls.cert_exists && tls.key_exists) ? 'yes' : 'no';
                document.getElementById('tls-route-count').textContent = tls.domain_routes_count || 0;
                if (tls.auto_https?.domains?.length) {
                    document.getElementById('auto-domains').value = tls.auto_https.domains.join('\n');
                    document.getElementById('auto-email').value = tls.auto_https.email || '';
                }
                if (tls.acme?.domains?.length) {
                    document.getElementById('wildcard-domains').value = tls.acme.domains.join('\n');
                    document.getElementById('wildcard-email').value = tls.acme.email || '';
                }
                const providerSelect = document.getElementById('wildcard-provider');
                if (!providerSelect.options.length) {
                    providerSelect.innerHTML = dnsProviders.map(item => `<option value="${item.id}">${item.display_name} (${item.id})</option>`).join('');
                    providerSelect.addEventListener('change', () => renderCredentialFields(providerSelect.value));
                }
                if (tls.acme?.dns_provider) providerSelect.value = tls.acme.dns_provider;
                renderCredentialFields(providerSelect.value);
                if (tls.on_demand) {
                    document.getElementById('on-demand-enabled').value = tls.on_demand.enabled ? 'true' : 'false';
                    document.getElementById('on-demand-allow').value = (tls.on_demand.allow || []).join('\n');
                }
                await refreshSniCertificates();
                tlsState.textContent = tls.write_ops_enabled ? 'Write ops enabled' : 'Read-only (enable admin.enable_write_ops)';
                tlsState.className = 'status-dot ' + (tls.write_ops_enabled ? 'ok' : 'warn');
            } catch (error) {
                tlsState.textContent = String(error);
                tlsState.className = 'status-dot bad';
            }
        }

        async function refreshSniCertificates() {
            const wrap = document.getElementById('sni-certs-wrap');
            try {
                const payload = await loadJson('/v1/tls/sni-certificates');
                const items = payload.items || [];
                if (!items.length) {
                    wrap.className = 'empty';
                    wrap.textContent = 'No SNI certificates configured yet.';
                    return;
                }
                wrap.className = '';
                wrap.innerHTML = `<table><thead><tr><th>Domains</th><th>Cert</th><th>Key</th><th>Ready</th><th>Action</th></tr></thead><tbody>${items.map(item => `
                    <tr>
                        <td>${(item.domains || []).join('<br/>')}</td>
                        <td><code>${item.cert_path}</code></td>
                        <td><code>${item.key_path}</code></td>
                        <td>${item.cert_exists && item.key_exists ? 'yes' : 'no'}</td>
                        <td><button class="danger" data-delete-sni-domain="${(item.domains || [])[0] || ''}" data-delete-sni-cert="${item.cert_path}">Delete</button></td>
                    </tr>`).join('')}</tbody></table>`;
                wrap.querySelectorAll('[data-delete-sni-cert]').forEach(button => {
                    button.onclick = async () => {
                        if (!confirm(`Delete SNI certificate ${button.dataset.deleteSniCert}?`)) return;
                        await postJson('/v1/tls/sni-certificates/delete', {
                            cert_path: button.dataset.deleteSniCert,
                            domain: button.dataset.deleteSniDomain || undefined,
                        });
                        await refreshSniCertificates();
                    };
                });
            } catch (error) {
                wrap.className = 'empty';
                wrap.textContent = String(error);
            }
        }

        async function refreshSecurityPanel() {
            const state = document.getElementById('security-state');
            const wrap = document.getElementById('blacklist-wrap');
            try {
                const payload = await loadJson('/v1/security/blacklist');
                const items = payload.items || [];
                if (!items.length) {
                    wrap.className = 'empty';
                    wrap.textContent = 'No active IP bans.';
                } else {
                    wrap.className = '';
                    wrap.innerHTML = `<table><thead><tr><th>IP</th><th>Action</th></tr></thead><tbody>${items.map(item => {
                        const ip = typeof item === 'string' ? item : item.ip;
                        return `<tr><td><code>${ip}</code></td><td><button class="danger" data-remove-blacklist="${ip}">Remove</button></td></tr>`;
                    }).join('')}</tbody></table>`;
                    wrap.querySelectorAll('[data-remove-blacklist]').forEach(button => {
                        button.onclick = async () => {
                            await postJson('/v1/security/blacklist/remove', { ip: button.dataset.removeBlacklist });
                            await refreshSecurityPanel();
                        };
                    });
                }
                state.textContent = `${items.length} active bans`;
                state.className = 'status-dot ok';
            } catch (error) {
                state.textContent = String(error);
                state.className = 'status-dot bad';
            }
        }

        async function refreshDomainRoutes() {
            const domainState = document.getElementById('domain-state');
            const wrap = document.getElementById('domain-routes-wrap');
            try {
                const payload = await loadJson('/v1/domain-routes');
                const items = payload.items || [];
                document.getElementById('domain-route-count').textContent = `${items.length} routes`;
                if (!items.length) {
                    wrap.className = 'empty';
                    wrap.textContent = 'No domain routes configured yet.';
                } else {
                    wrap.className = '';
                    wrap.innerHTML = `<table><thead><tr><th>Name</th><th>Domains</th><th>Prefix</th><th>Upstream</th><th>Action</th></tr></thead><tbody>${items.map(item => `
                        <tr>
                            <td><code>${item.name}</code></td>
                            <td>${(item.domains || []).join('<br/>')}</td>
                            <td><code>${item.path_prefix || '/'}</code></td>
                            <td><code>${item.upstream || ''}</code></td>
                            <td><button class="danger" data-delete-domain="${item.name}">Delete</button></td>
                        </tr>`).join('')}</tbody></table>`;
                }
                domainState.textContent = 'Routes loaded';
                domainState.className = 'status-dot ok';
            } catch (error) {
                domainState.textContent = String(error);
                domainState.className = 'status-dot bad';
                wrap.className = 'empty';
                wrap.textContent = String(error);
            }
        }

        document.getElementById('save-auto-https').addEventListener('click', async () => {
            const result = document.getElementById('auto-https-result');
            try {
                const data = await postJson('/v1/tls/auto-https/upsert', {
                    domains: parseDomainLines(document.getElementById('auto-domains').value),
                    email: document.getElementById('auto-email').value.trim(),
                    production: document.getElementById('auto-production').value === 'true',
                });
                result.textContent = `Saved (${data.mode}) for ${(data.domains || []).join(', ')}`;
                await refreshTlsPanel();
            } catch (error) {
                result.textContent = String(error);
            }
        });

        document.getElementById('save-wildcard-tls').addEventListener('click', async () => {
            const result = document.getElementById('wildcard-tls-result');
            try {
                const data = await postJson('/v1/tls/wildcard-dns/upsert', {
                    domains: parseDomainLines(document.getElementById('wildcard-domains').value),
                    email: document.getElementById('wildcard-email').value.trim(),
                    dns_provider: document.getElementById('wildcard-provider').value,
                    credentials: collectCredentials(),
                });
                result.textContent = `Saved DNS-01 (${data.challenge}) for ${(data.domains || []).join(', ')}`;
                await refreshTlsPanel();
            } catch (error) {
                result.textContent = String(error);
            }
        });

        document.getElementById('save-domain-route').addEventListener('click', async () => {
            const result = document.getElementById('domain-route-result');
            try {
                const domains = document.getElementById('route-domains').value.split(',').map(item => item.trim()).filter(Boolean);
                const upstreams = document.getElementById('route-upstreams').value.split('\n').map(item => item.trim()).filter(Boolean);
                const payload = {
                    name: document.getElementById('route-name').value.trim(),
                    upstream: document.getElementById('route-upstream').value.trim(),
                    path_prefix: document.getElementById('route-path-prefix').value.trim() || '/',
                    domains,
                    strip_prefix: document.getElementById('route-strip-prefix').checked,
                };
                if (upstreams.length) payload.upstreams = upstreams;
                const data = await postJson('/v1/domain-routes/upsert', payload);
                result.textContent = `${data.action} route ${data.name}`;
                await refreshDomainRoutes();
            } catch (error) {
                result.textContent = String(error);
            }
        });

        document.getElementById('domain-routes-wrap').addEventListener('click', async (event) => {
            const button = event.target.closest('[data-delete-domain]');
            if (!button) return;
            if (!confirm(`Delete domain route ${button.dataset.deleteDomain}?`)) return;
            try {
                await postJson('/v1/domain-routes/delete', { name: button.dataset.deleteDomain });
                await refreshDomainRoutes();
            } catch (error) {
                document.getElementById('domain-route-result').textContent = String(error);
            }
        });

        async function refreshReverseRoutes() {
            const state = document.getElementById('reverse-state');
            const wrap = document.getElementById('reverse-routes-wrap');
            try {
                const payload = await loadJson('/v1/reverse-proxy-routes');
                const items = payload.items || [];
                document.getElementById('reverse-count').textContent = `${items.length} routes`;
                if (!items.length) {
                    wrap.className = 'empty';
                    wrap.textContent = 'No reverse proxy routes yet.';
                } else {
                    wrap.className = '';
                    wrap.innerHTML = `<table><thead><tr><th>Name</th><th>Prefix</th><th>Upstream</th><th>Hosts</th><th>Action</th></tr></thead><tbody>${items.map(item => `
                        <tr><td><code>${item.name}</code></td><td><code>${item.path_prefix || '/'}</code></td><td><code>${item.upstream}</code></td><td>${(item.hosts || []).join('<br/>') || '-'}</td>
                        <td><button class="danger" data-delete-reverse="${item.name}">Delete</button></td></tr>`).join('')}</tbody></table>`;
                    wrap.querySelectorAll('[data-delete-reverse]').forEach(button => {
                        button.onclick = async () => {
                            if (!confirm(`Delete reverse route ${button.dataset.deleteReverse}?`)) return;
                            await postJson('/v1/reverse-proxy-routes/delete', { name: button.dataset.deleteReverse });
                            await refreshReverseRoutes();
                        };
                    });
                }
                state.textContent = 'Routes loaded';
                state.className = 'status-dot ok';
            } catch (error) {
                state.textContent = String(error);
                state.className = 'status-dot bad';
            }
        }

        function renderListenerTable(items, wrap, deletePath, attr) {
            if (!items.length) {
                wrap.className = 'empty';
                wrap.textContent = 'No entries configured yet.';
                return;
            }
            wrap.className = '';
            wrap.innerHTML = `<table><thead><tr><th>Name</th><th>Bind/Listen</th><th>Upstream</th><th>Action</th></tr></thead><tbody>${items.map(item => `
                <tr><td><code>${item.name}</code></td><td><code>${item.bind || item.listen}</code></td><td><code>${item.upstream}</code></td>
                <td><button class="danger" data-${attr}="${item.name}">Delete</button></td></tr>`).join('')}</tbody></table>`;
            wrap.querySelectorAll(`[data-${attr}]`).forEach(button => {
                button.onclick = async () => {
                    if (!confirm(`Delete ${button.dataset[attr]}?`)) return;
                    await postJson(deletePath, { name: button.dataset[attr] });
                    await refreshListenersPanel();
                };
            });
        }

        async function refreshListenersPanel() {
            const state = document.getElementById('listeners-state');
            try {
                const [tcp, udp, stream] = await Promise.all([
                    loadJson('/v1/tcp-listeners'),
                    loadJson('/v1/udp-listeners'),
                    loadJson('/v1/stream-routes'),
                ]);
                renderListenerTable(tcp.items || [], document.getElementById('tcp-listeners-wrap'), '/v1/tcp-listeners/delete', 'delete-tcp');
                renderListenerTable(udp.items || [], document.getElementById('udp-listeners-wrap'), '/v1/udp-listeners/delete', 'delete-udp');
                const streamWrap = document.getElementById('stream-routes-wrap');
                const streamItems = stream.items || [];
                if (!streamItems.length) {
                    streamWrap.className = 'empty';
                    streamWrap.textContent = 'No stream routes yet.';
                } else {
                    streamWrap.className = '';
                    streamWrap.innerHTML = `<table><thead><tr><th>Name</th><th>Listen</th><th>Upstream</th><th>Domains</th><th>Action</th></tr></thead><tbody>${streamItems.map(item => `
                        <tr><td><code>${item.name}</code></td><td><code>${item.listen}</code></td><td><code>${item.upstream}</code></td><td>${(item.domains || []).join('<br/>')}</td>
                        <td><button class="danger" data-delete-stream="${item.name}">Delete</button></td></tr>`).join('')}</tbody></table>`;
                    streamWrap.querySelectorAll('[data-delete-stream]').forEach(button => {
                        button.onclick = async () => {
                            if (!confirm(`Delete stream route ${button.dataset.deleteStream}?`)) return;
                            await postJson('/v1/stream-routes/delete', { name: button.dataset.deleteStream });
                            await refreshListenersPanel();
                        };
                    });
                }
                state.textContent = 'Listeners loaded';
                state.className = 'status-dot ok';
            } catch (error) {
                state.textContent = String(error);
                state.className = 'status-dot bad';
            }
        }

        async function refreshFileCloudPanel() {
            const state = document.getElementById('filecloud-state');
            try {
                const payload = await loadJson('/v1/filecloud/summary');
                const fc = payload.filecloud || {};
                document.getElementById('filecloud-summary-json').textContent = JSON.stringify(fc, null, 2);
                document.getElementById('fc-enabled').value = fc.enabled ? 'true' : 'false';
                document.getElementById('fc-prefix').value = fc.path_prefix || '/filecloud';
                document.getElementById('fc-root').value = fc.root || '';
                document.getElementById('fc-title').value = fc.title || 'FileCloud';
                document.getElementById('fc-max-upload').value = fc.max_upload_bytes || '';
                document.getElementById('fc-cdn-cache').value = fc.cdn_cache_secs || '';
                document.getElementById('fc-allow-upload').checked = fc.allow_upload !== false;
                document.getElementById('fc-allow-delete').checked = fc.allow_delete !== false;
                document.getElementById('fc-allow-mkdir').checked = fc.allow_mkdir !== false;
                document.getElementById('fc-allow-move').checked = fc.allow_move !== false;
                const openLink = document.getElementById('filecloud-open-link');
                if (fc.enabled && fc.ui_url) {
                    openLink.href = fc.ui_url;
                    openLink.style.display = '';
                    state.textContent = `Enabled — ${fc.ui_url}`;
                } else {
                    openLink.style.display = 'none';
                    state.textContent = 'Disabled';
                }
                state.className = 'status-dot ' + (fc.enabled ? 'ok' : '');
            } catch (error) {
                state.textContent = String(error);
                state.className = 'status-dot bad';
            }
        }

        document.getElementById('save-on-demand').addEventListener('click', async () => {
            const result = document.getElementById('on-demand-result');
            try {
                await postJson('/v1/tls/on-demand/upsert', {
                    enabled: document.getElementById('on-demand-enabled').value === 'true',
                    allow: parseDomainLines(document.getElementById('on-demand-allow').value),
                });
                result.textContent = 'On-demand TLS saved';
                await refreshTlsPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('issue-tls-now').addEventListener('click', async () => {
            const result = document.getElementById('on-demand-result');
            try {
                const data = await postJson('/v1/tls/issue-now', {});
                result.textContent = `Issued: cert=${data.issued?.cert_exists} key=${data.issued?.key_exists}`;
                await refreshTlsPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-reverse-route').addEventListener('click', async () => {
            const result = document.getElementById('reverse-route-result');
            try {
                const hosts = document.getElementById('reverse-hosts').value.split(',').map(v => v.trim()).filter(Boolean);
                const upstreams = document.getElementById('reverse-upstreams').value.split('\n').map(v => v.trim()).filter(Boolean);
                const payload = {
                    name: document.getElementById('reverse-name').value.trim(),
                    path_prefix: document.getElementById('reverse-prefix').value.trim() || '/',
                    upstream: document.getElementById('reverse-upstream').value.trim(),
                    hosts,
                    strip_prefix: document.getElementById('reverse-strip-prefix').checked,
                };
                if (upstreams.length) payload.upstreams = upstreams;
                const data = await postJson('/v1/reverse-proxy-routes/upsert', payload);
                result.textContent = `${data.action} ${data.name}`;
                await refreshReverseRoutes();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-tcp-listener').addEventListener('click', async () => {
            const result = document.getElementById('tcp-listener-result');
            try {
                await postJson('/v1/tcp-listeners/upsert', {
                    name: document.getElementById('tcp-name').value.trim(),
                    bind: document.getElementById('tcp-bind').value.trim(),
                    upstream: document.getElementById('tcp-upstream').value.trim(),
                });
                result.textContent = 'TCP listener saved';
                await refreshListenersPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-udp-listener').addEventListener('click', async () => {
            const result = document.getElementById('udp-listener-result');
            try {
                await postJson('/v1/udp-listeners/upsert', {
                    name: document.getElementById('udp-name').value.trim(),
                    bind: document.getElementById('udp-bind').value.trim(),
                    upstream: document.getElementById('udp-upstream').value.trim(),
                });
                result.textContent = 'UDP listener saved';
                await refreshListenersPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-stream-route').addEventListener('click', async () => {
            const result = document.getElementById('stream-route-result');
            try {
                await postJson('/v1/stream-routes/upsert', {
                    name: document.getElementById('stream-name').value.trim(),
                    listen: document.getElementById('stream-listen').value.trim(),
                    upstream: document.getElementById('stream-upstream').value.trim(),
                    domains: document.getElementById('stream-domains').value.split(',').map(v => v.trim()).filter(Boolean),
                });
                result.textContent = 'Stream route saved';
                await refreshListenersPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-filecloud').addEventListener('click', async () => {
            const result = document.getElementById('filecloud-result');
            try {
                await postJson('/v1/filecloud/upsert', {
                    enabled: document.getElementById('fc-enabled').value === 'true',
                    path_prefix: document.getElementById('fc-prefix').value.trim(),
                    root: document.getElementById('fc-root').value.trim(),
                    password: document.getElementById('fc-password').value,
                    title: document.getElementById('fc-title').value.trim(),
                    allow_upload: document.getElementById('fc-allow-upload').checked,
                    allow_delete: document.getElementById('fc-allow-delete').checked,
                    allow_mkdir: document.getElementById('fc-allow-mkdir').checked,
                    allow_move: document.getElementById('fc-allow-move').checked,
                    max_upload_bytes: Number(document.getElementById('fc-max-upload').value) || undefined,
                    cdn_cache_secs: Number(document.getElementById('fc-cdn-cache').value) || undefined,
                });
                result.textContent = 'FileCloud saved';
                document.getElementById('fc-password').value = '';
                await refreshFileCloudPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-sni-cert').addEventListener('click', async () => {
            const result = document.getElementById('sni-cert-result');
            try {
                const payload = {
                    domains: parseDomainLines(document.getElementById('sni-domains').value),
                    cert_path: document.getElementById('sni-cert-path').value.trim() || undefined,
                    key_path: document.getElementById('sni-key-path').value.trim() || undefined,
                    cert_pem: document.getElementById('sni-cert-pem').value.trim() || undefined,
                    key_pem: document.getElementById('sni-key-pem').value.trim() || undefined,
                };
                const data = await postJson('/v1/tls/sni-certificates/upsert', payload);
                result.textContent = `${data.action} SNI cert for ${(data.domains || []).join(', ')}`;
                document.getElementById('sni-cert-pem').value = '';
                document.getElementById('sni-key-pem').value = '';
                await refreshSniCertificates();
                await refreshTlsPanel();
            } catch (error) { result.textContent = String(error); }
        });

        document.getElementById('save-blacklist-add').addEventListener('click', async () => {
            const result = document.getElementById('blacklist-result');
            try {
                const banSecs = Number(document.getElementById('blacklist-ban-secs').value);
                await postJson('/v1/security/blacklist/add', {
                    ip: document.getElementById('blacklist-ip').value.trim(),
                    ban_secs: banSecs > 0 ? banSecs : undefined,
                });
                result.textContent = 'Ban added';
                await refreshSecurityPanel();
            } catch (error) { result.textContent = String(error); }
        });

        applyLocale();

        (async function bootAdmin() {
            try {
                const saved = JSON.parse(sessionStorage.getItem(sessionKey) || 'null');
                if (!saved || !saved.auth) {
                    showLogin('');
                    return;
                }
                if (saved.expiresAt && Number(saved.expiresAt) <= Math.floor(Date.now() / 1000)) {
                    clearAuth();
                    showLogin('');
                    return;
                }
                authValue = saved.auth;
                if (saved.user) document.getElementById('login-username').value = saved.user;
                await verifyAuth();
                showApp();
                await refreshDashboard();
            } catch {
                clearAuth();
                showLogin('');
            }
        })();
    </script>
</body>
</html>"#;

    html.replace("__ADMIN_USER__", &config.admin.username)
        .replace(
            "__ADMIN_HTTPS_HINT__",
            &if config.admin.https.enabled {
                format!(
                    "HTTPS agent API: <strong>https://&lt;host&gt;{}/v1/*</strong> (TLS required; writes after cert material exists)",
                    crate::config::normalize_admin_https_path_prefix(&config.admin.https.path_prefix)
                )
            } else {
                "Enable <code>admin.https.enabled</code> after TLS bootstrap for secure remote agent automation".to_string()
            },
        )
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

    let host = if original_host.trim().is_empty() {
        upstream_host
    } else {
        original_host
    };
    headers.insert(HOST, HeaderValue::from_str(host)?);
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

fn load_certs_from_pem(pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(Cursor::new(pem.as_bytes()));
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse in-memory certificate pem")
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open private key {}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .context("failed to parse private key pem")?
        .ok_or_else(|| anyhow!("no private key found in {}", path.display()))
}

fn load_private_key_from_pem(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(Cursor::new(pem.as_bytes()));
    rustls_pemfile::private_key(&mut reader)
        .context("failed to parse in-memory private key pem")?
        .ok_or_else(|| anyhow!("no private key found in in-memory pem"))
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
    fn is_authorized_accepts_bearer_token() {
        let admin = AdminConfig {
            bearer_token: "cluster-secret".to_string(),
            ..AdminConfig::default()
        };
        let header = HeaderValue::from_static("Bearer cluster-secret");

        assert!(is_authorized(Some(&header), &admin));
    }

    #[test]
    fn map_admin_gateway_path_strips_configured_prefix() {
        let mut admin = AdminConfig::default();
        admin.https.enabled = true;
        admin.https.path_prefix = "/_proxysss/admin".to_string();
        assert_eq!(
            map_admin_gateway_path(&admin, "/_proxysss/admin/v1/stats").as_deref(),
            Some("/v1/stats")
        );
        assert_eq!(
            map_admin_gateway_path(&admin, "/_proxysss/admin").as_deref(),
            Some("/")
        );
        assert!(map_admin_gateway_path(&admin, "/v1/stats").is_none());
    }

    #[test]
    fn render_config_with_upserted_sni_certificate_appends_entry() {
        let updated = render_config_with_upserted_sni_certificate(
            "http:\n  tls:\n    certificates: []\n",
            &TlsCertificateConfig {
                domains: vec!["api.example.com".to_string()],
                cert_path: PathBuf::from("certs/api.crt"),
                key_path: PathBuf::from("certs/api.key"),
            },
        )
        .expect("render sni cert");
        assert!(updated.contains("api.example.com"));
        assert!(updated.contains("certs/api.crt"));
    }

    #[test]
    fn admin_https_writes_require_tls_material() {
        let mut config = GatewayConfig::default();
        config.admin.enable_write_ops = true;
        config.admin.https.enabled = true;
        let denied = check_admin_mutation_access(
            &config,
            &AdminTransport::GatewayHttps {
                host: "ops.example.com".to_string(),
            },
        );
        assert!(denied.is_some());

        let cert_dir = std::env::temp_dir().join(format!(
            "proxysss-admin-tls-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&cert_dir).expect("cert dir");
        let cert_path = cert_dir.join("cert.pem");
        let key_path = cert_dir.join("key.pem");
        std::fs::write(&cert_path, b"cert").expect("cert");
        std::fs::write(&key_path, b"key").expect("key");
        config.http.tls.cert_path = cert_path;
        config.http.tls.key_path = key_path;
        assert!(check_admin_mutation_access(
            &config,
            &AdminTransport::GatewayHttps {
                host: "ops.example.com".to_string(),
            },
        )
        .is_none());
    }

    #[test]
    fn admin_gateway_http_is_rejected() {
        let config = GatewayConfig::default();
        let denied = check_admin_transport_access(
            &config,
            &AdminTransport::GatewayHttp,
            "127.0.0.1:7777".parse().expect("addr"),
        );
        assert!(denied.is_some());
    }

    #[test]
    fn render_config_with_upserted_domain_route_appends_route() {
        let updated = render_config_with_upserted_domain_route(
            "http:\n  plain_bind: 0.0.0.0:80\nservices:\n  domain_routes: []\n",
            &DomainRouteConfig {
                name: "app".to_string(),
                domains: vec!["example.com".to_string()],
                path_prefix: "/".to_string(),
                upstream: "http://127.0.0.1:9000".to_string(),
                upstreams: vec!["http://127.0.0.1:9001".to_string()],
                upstream_weights: BTreeMap::new(),
                strip_prefix: false,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                compression: ResponseCompressionConfig::default(),
                cache: ResponseCacheConfig::default(),
                rate_limit: HttpRateLimitConfig::default(),
                active_health: ActiveHealthOverrideConfig::default(),
                ssl: crate::config::DomainTlsConfig::default(),
            },
        )
        .expect("render updated config");

        assert!(updated.contains("name: app"));
        assert!(updated.contains("- example.com"));
        assert!(updated.contains("- http://127.0.0.1:9001"));
    }

    #[test]
    fn render_config_with_upserted_domain_route_replaces_by_name() {
        let updated = render_config_with_upserted_domain_route(
            "services:\n  domain_routes:\n    - name: app\n      domains: [old.example.com]\n      path_prefix: /\n      upstream: http://127.0.0.1:8000\n",
            &DomainRouteConfig {
                name: "app".to_string(),
                domains: vec!["new.example.com".to_string()],
                path_prefix: "/api".to_string(),
                upstream: "http://127.0.0.1:9000".to_string(),
                upstreams: Vec::new(),
                upstream_weights: BTreeMap::new(),
                strip_prefix: true,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                compression: ResponseCompressionConfig::default(),
                cache: ResponseCacheConfig::default(),
                rate_limit: HttpRateLimitConfig::default(),
                active_health: ActiveHealthOverrideConfig::default(),
                ssl: crate::config::DomainTlsConfig::default(),
            },
        )
        .expect("render updated config");

        assert!(!updated.contains("old.example.com"));
        assert!(updated.contains("new.example.com"));
        assert!(updated.contains("path_prefix: /api"));
        assert!(updated.contains("strip_prefix: true"));
    }

    #[test]
    fn render_config_with_upserted_reverse_proxy_route_replaces_by_name() {
        let updated = render_config_with_upserted_reverse_proxy_route(
            "services:\n  reverse_proxy:\n    routes:\n      - name: api\n        path_prefix: /api\n        hosts: [api.old.example.com]\n        upstream: http://127.0.0.1:8000\n",
            &ReverseProxyRouteConfig {
                name: "api".to_string(),
                path_prefix: "/v2".to_string(),
                hosts: vec!["api.example.com".to_string()],
                upstream: "http://127.0.0.1:9000".to_string(),
                upstreams: vec!["http://127.0.0.1:9001".to_string()],
                upstream_weights: BTreeMap::new(),
                strip_prefix: true,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                compression: ResponseCompressionConfig::default(),
                cache: ResponseCacheConfig::default(),
                rate_limit: HttpRateLimitConfig::default(),
                active_health: ActiveHealthOverrideConfig::default(),
            },
        )
        .expect("render updated config");

        assert!(!updated.contains("api.old.example.com"));
        assert!(updated.contains("api.example.com"));
        assert!(updated.contains("path_prefix: /v2"));
    }

    #[test]
    fn render_config_with_upserted_tcp_listener_replaces_by_name() {
        let updated = render_config_with_upserted_tcp_listener(
            "tcp:\n  listeners:\n    - name: game\n      bind: 0.0.0.0:7000\n      upstream: 127.0.0.1:9000\n",
            &TcpListenerConfig {
                name: "game".to_string(),
                bind: "0.0.0.0:7001".to_string(),
                upstream: "127.0.0.1:9100".to_string(),
                upstreams: vec!["127.0.0.1:9101".to_string()],
                upstream_weights: BTreeMap::new(),
            },
        )
        .expect("render updated config");

        assert!(updated.contains("bind: 0.0.0.0:7001"));
        assert!(updated.contains("127.0.0.1:9101"));
    }

    #[test]
    fn render_config_with_upserted_udp_listener_replaces_by_name() {
        let updated = render_config_with_upserted_udp_listener(
            "udp:\n  listeners:\n    - name: realtime\n      bind: 0.0.0.0:8000\n      upstreams: [127.0.0.1:9200]\n",
            &UdpListenerConfig {
                name: "realtime".to_string(),
                bind: "0.0.0.0:8001".to_string(),
                upstream: String::new(),
                upstreams: vec!["127.0.0.1:9300".to_string()],
            upstream_weights: BTreeMap::new(),
            },
        )
        .expect("render updated config");

        assert!(updated.contains("bind: 0.0.0.0:8001"));
        assert!(updated.contains("127.0.0.1:9300"));
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
            upstream_weights: BTreeMap::new(),
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
            upstream_weights: BTreeMap::new(),
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
            upstream_weights: BTreeMap::new(),
            strip_prefix: true,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
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
                    upstream_weights: BTreeMap::new(),
                    strip_prefix: false,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                    compression: ResponseCompressionConfig::default(),
                    cache: ResponseCacheConfig::default(),
                    rate_limit: HttpRateLimitConfig::default(),
                    active_health: ActiveHealthOverrideConfig::default(),
                },
                ReverseProxyRouteConfig {
                    name: "admin-api".to_string(),
                    path_prefix: "/api/admin".to_string(),
                    hosts: Vec::new(),
                    upstream: "http://127.0.0.1:9090".to_string(),
                    upstreams: Vec::new(),
                    upstream_weights: BTreeMap::new(),
                    strip_prefix: false,
                    set_headers: BTreeMap::new(),
                    strip_headers: Vec::new(),
                    compression: ResponseCompressionConfig::default(),
                    cache: ResponseCacheConfig::default(),
                    rate_limit: HttpRateLimitConfig::default(),
                    active_health: ActiveHealthOverrideConfig::default(),
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
            upstream_weights: BTreeMap::new(),
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            compression: ResponseCompressionConfig {
                enabled: true,
                algorithms: crate::config::ResponseCompressionConfig::default().algorithms,
                min_length: 128,
                content_types: vec!["application/json".to_string()],
            },
            cache: ResponseCacheConfig {
                enabled: true,
                zone: "default".to_string(),
                ttl_secs: 5,
                statuses: vec![200],
                max_body_bytes: 4096,
                allow_purge: true,
                ..Default::default()
            },
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
            ssl: crate::config::DomainTlsConfig::default(),
        });

        let uri: Uri = "/".parse().expect("valid uri");
        let route = configured_http_route(&config, "example.com", &uri).expect("matched route");
        assert_eq!(route.decision.upstream, "http://127.0.0.1:9000");
        assert!(route.compression.enabled);
        assert!(route.cache.enabled);
    }

    #[test]
    fn configured_domain_route_uses_domain_as_primary_service_group() {
        let mut config = GatewayConfig::default();
        config.services.domain_routes.push(DomainRouteConfig {
            name: "example-site".to_string(),
            domains: vec!["example.com".to_string()],
            path_prefix: "/".to_string(),
            upstream: "http://127.0.0.1:9000".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
            ssl: crate::config::DomainTlsConfig::default(),
        });
        config.services.domain_routes.push(DomainRouteConfig {
            name: "store".to_string(),
            domains: vec!["neko233.store".to_string()],
            path_prefix: "/".to_string(),
            upstream: "http://127.0.0.1:9000".to_string(),
            upstreams: vec!["http://127.0.0.1:9001".to_string()],
            upstream_weights: BTreeMap::new(),
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
            ssl: crate::config::DomainTlsConfig::default(),
        });

        let uri: Uri = "/".parse().expect("valid uri");
        let example = configured_http_route(&config, "example.com", &uri).expect("example route");
        let store = configured_http_route(&config, "neko233.store", &uri).expect("store route");

        assert_eq!(example.runtime_scope.as_deref(), Some("example-site"));
        assert_eq!(example.decision.upstream, "http://127.0.0.1:9000");
        assert!(example.decision.upstreams.is_empty());

        assert_eq!(store.runtime_scope.as_deref(), Some("store"));
        assert_eq!(store.decision.upstream, "http://127.0.0.1:9000");
        assert_eq!(
            store.decision.upstreams,
            vec!["http://127.0.0.1:9001".to_string()]
        );
    }

    #[test]
    fn finalize_http_response_prefers_brotli_for_compressible_payloads() {
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
            algorithms: vec![CompressionAlgorithm::Brotli, CompressionAlgorithm::Gzip],
            min_length: 128,
            content_types: vec!["application/json".to_string()],
        };

        let response = finalize_http_response(&request_headers, &compression, response)
            .expect("finalize response");
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == CONTENT_ENCODING && value == "br"));
    }

    #[test]
    fn finalize_http_response_falls_back_to_gzip_when_brotli_disabled() {
        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("br;q=0, gzip;q=1"),
        );
        let response = GatewayHttpResponse::bytes(
            StatusCode::OK,
            "application/json",
            Bytes::from(vec![b'a'; 2048]),
            "http://127.0.0.1:9000",
        );
        let compression = ResponseCompressionConfig {
            enabled: true,
            algorithms: vec![CompressionAlgorithm::Gzip],
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
    fn finalize_http_response_marks_sse_as_unbuffered_and_untransformed() {
        let mut request_headers = HeaderMap::new();
        request_headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("br, gzip"));
        let mut response = GatewayHttpResponse::bytes(
            StatusCode::OK,
            "text/event-stream; charset=utf-8",
            Bytes::from_static(b"data: first\n\n"),
            "http://127.0.0.1:9000",
        );
        response.push_header(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response.push_header(
            HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        );
        let compression = ResponseCompressionConfig {
            enabled: true,
            algorithms: vec![CompressionAlgorithm::Brotli, CompressionAlgorithm::Gzip],
            min_length: 1,
            content_types: vec!["text/".to_string()],
        };

        let response = finalize_http_response(&request_headers, &compression, response)
            .expect("finalize response");
        assert!(response.headers.iter().any(|(name, value)| {
            name == CACHE_CONTROL
                && value
                    .to_str()
                    .expect("cache-control")
                    .contains("no-transform")
        }));
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| { name.as_str() == "x-accel-buffering" && value == "no" }));
        assert_eq!(
            response
                .headers
                .iter()
                .filter(|(name, _)| name.as_str() == "x-accel-buffering")
                .count(),
            1
        );
        assert_eq!(
            response
                .headers
                .iter()
                .filter(|(name, value)| {
                    name == CACHE_CONTROL
                        && value
                            .to_str()
                            .expect("cache-control")
                            .split(',')
                            .filter(|token| token.trim().eq_ignore_ascii_case("no-cache"))
                            .count()
                            == 1
                })
                .count(),
            1
        );
        assert!(!response
            .headers
            .iter()
            .any(|(name, _)| name == CONTENT_ENCODING));
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
            zone: "default".to_string(),
            algorithm: RateLimitAlgorithm::default(),
            key: RateLimitKey::Header("x-api-key".to_string()),
            requests: 1,
            window_ms: 1000,
            burst: 0,
            max_connections: 0,
            status: 429,
        };
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("abc"));
        let remote: SocketAddr = "127.0.0.1:12345".parse().expect("remote addr");

        let key = http_rate_limit_key(&config, "example.com", &headers, remote);
        assert_eq!(key.as_deref(), Some("default:header:x-api-key:abc"));
    }

    #[test]
    fn build_upstream_headers_appends_forwarding_chain() {
        let route = RouteDecision {
            upstream: "http://127.0.0.1:8080".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
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
    fn snapshot_prometheus_emits_counter_lines() {
        let stats = GatewayStats::default();
        stats.http_requests.store(42, Ordering::Relaxed);
        let _ = stats.snapshot_json();
        std::thread::sleep(Duration::from_millis(5));
        let payload = stats.snapshot_json();
        let body = stats.snapshot_prometheus();
        assert!(body.contains("proxysss_http_requests_total 42"));
        assert!(body.contains("# TYPE proxysss_http_requests_total counter"));
        assert!(body.contains("proxysss_process_memory_bytes"));
        assert_eq!(
            payload["process"]["pid"].as_u64(),
            Some(std::process::id() as u64)
        );
        assert!(payload["process"]["memory_mb"].as_f64().is_some());
    }

    #[test]
    fn weighted_plan_prefers_heavier_upstream() {
        let gateway = Gateway {
            config_path: PathBuf::from("proxysss.yaml"),
            bootstrap_config: GatewayConfig::default(),
            dynamic: Arc::new(RwLock::new(Arc::new(DynamicState {
                config: GatewayConfig::default(),
                http_client: reqwest::Client::new(),
                script: None,
            }))),
            stats: Arc::new(GatewayStats::default()),
            sticky_affinity: Arc::new(DashMap::new()),
            round_robin_state: Arc::new(DashMap::new()),
            upstream_runtime: Arc::new(DashMap::new()),
            http_rate_limits: Arc::new(DashMap::new()),
            stream_rate_limits: Arc::new(DashMap::new()),
            http_connection_limits: Arc::new(DashMap::new()),
            http_cache: Arc::new(DashMap::new()),
            acme_http_challenges: Arc::new(DashMap::new()),
            acme_tls_alpn_certs: Arc::new(DashMap::new()),
            on_demand_certs: Arc::new(DashMap::new()),
            on_demand_trigger: tokio::sync::mpsc::unbounded_channel().0,
            on_demand_issue_counts: Arc::new(DashMap::new()),
            ddos_guard: DdosGuard::default(),
            dynamic_blacklist: DynamicBlacklist::default(),
            ftp_session_users: Arc::new(DashMap::new()),
            admin_auth_guard: AdminAuthGuard::default(),
        };
        let mut weights = BTreeMap::new();
        weights.insert("http://127.0.0.1:9000".to_string(), 1);
        weights.insert("http://127.0.0.1:9001".to_string(), 9);
        let plan = gateway.select_weighted_plan(
            "http:test",
            vec![
                "http://127.0.0.1:9000".to_string(),
                "http://127.0.0.1:9001".to_string(),
            ],
            &weights,
        );
        assert_eq!(plan.len(), 2);
        let heavy_first = plan
            .iter()
            .filter(|item| **item == "http://127.0.0.1:9001")
            .count();
        assert!(heavy_first >= 1);
    }

    #[test]
    fn token_bucket_rate_limit_allows_burst_then_blocks() {
        let config = HttpRateLimitConfig {
            enabled: true,
            zone: "default".to_string(),
            algorithm: RateLimitAlgorithm::TokenBucket,
            key: RateLimitKey::RemoteAddr,
            requests: 1,
            window_ms: 60_000,
            burst: 1,
            max_connections: 0,
            status: 429,
        };
        let store = DashMap::new();
        let key = "remote:127.0.0.1".to_string();
        assert!(apply_http_rate_limit_to_store(&store, &config, key.clone()).is_none());
        assert!(apply_http_rate_limit_to_store(&store, &config, key.clone()).is_none());
        assert!(apply_http_rate_limit_to_store(&store, &config, key).is_some());
    }

    #[test]
    fn http_rate_limit_blocks_after_limit() {
        let config = HttpRateLimitConfig {
            enabled: true,
            zone: "default".to_string(),
            algorithm: RateLimitAlgorithm::default(),
            key: RateLimitKey::RemoteAddr,
            requests: 1,
            window_ms: 60_000,
            burst: 0,
            max_connections: 0,
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
    fn http_access_control_blocks_blacklisted_ip() {
        let config = HttpAccessControlConfig {
            enabled: true,
            allow: Vec::new(),
            deny: vec!["203.0.113.0/24".to_string()],
            status: 403,
        };

        assert_eq!(
            http_access_is_denied(&config, "203.0.113.20".parse().expect("ip")),
            Some("203.0.113.20".to_string())
        );
        assert_eq!(
            http_access_is_denied(&config, "198.51.100.20".parse().expect("ip")),
            None
        );
    }

    #[test]
    fn http_access_control_allowlist_blocks_unknown_ip() {
        let config = HttpAccessControlConfig {
            enabled: true,
            allow: vec!["2001:db8::/32".to_string()],
            deny: Vec::new(),
            status: 403,
        };

        assert_eq!(
            http_access_is_denied(&config, "2001:db8::1".parse().expect("ip")),
            None
        );
        assert_eq!(
            http_access_is_denied(&config, "2001:db9::1".parse().expect("ip")),
            Some("2001:db9::1".to_string())
        );
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
    async fn static_site_root_prefix_serves_child_files() {
        let root = std::env::temp_dir().join(format!(
            "proxysss-static-root-prefix-test-{}",
            Uuid::new_v4()
        ));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create static root");
        tokio::fs::write(root.join("test.txt"), b"hello")
            .await
            .expect("write file");

        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/".to_string(),
            root: root.clone(),
            index_files: vec!["index.html".to_string()],
            autoindex: true,
        };
        let uri: Uri = "/test.txt".parse().expect("valid uri");
        let response = dispatch_static_site(&site, &Method::GET, &uri)
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, Bytes::from_static(b"hello"));

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
            root.join("geo-headers.plugin.yaml"),
            "enabled: true\npriority: 180\nconfig:\n  mode: geo_headers\n  header_prefix: proxysss-\n",
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
    fn builtin_http_route_serves_docs_on_docs_html() {
        let route = builtin_http_route("/docs.html").expect("docs route");
        assert_eq!(route.upstream, "proxysss://docs");
    }

    #[test]
    fn welcome_page_stays_brand_focused() {
        let config = GatewayConfig::default();
        let html = render_welcome_html(&config);
        assert!(html.contains("Welcome to proxysss"));
        assert!(html.contains("<h1>Gateway ready.</h1>"));
        assert!(html.contains("animation:"));
        assert!(html.contains("@keyframes"));
        assert!(!html.contains("<script"));
        assert!(html.contains("/docs.html"));
        assert!(!html.contains("127.0.0.1:7777"));
        assert!(!html.contains("Open Admin Console"));
        assert!(html.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn admin_console_uses_login_screen_without_embedding_password() {
        let mut config = GatewayConfig::default();
        config.admin.username = "ops-admin".to_string();
        config.admin.password = "super-secret-password".to_string();

        let html = render_admin_console_html(&config);

        assert!(html.contains("id=\"login-screen\""));
        assert!(html.contains("id=\"admin-app\" class=\"shell app-hidden\""));
        assert!(html.contains("sessionStorage"));
        assert!(html.contains("value=\"ops-admin\""));
        assert!(html.contains("/v1/login"));
        assert!(html.contains("Bearer "));
        assert!(!html.contains("Quick Login"));
        assert!(!html.contains("super-secret-password"));
        assert!(!html.contains("__ADMIN_PASS__"));
        assert!(!html.contains("btoa(user + ':' + pass)"));
    }

    #[test]
    fn admin_session_token_authorizes_until_expiry() {
        let mut config = GatewayConfig::default();
        config.admin.username = "ops-admin".to_string();
        config.admin.password = "super-secret-password".to_string();
        config.admin.bearer_token = "static-token".to_string();

        let (token, expires_at) =
            issue_admin_session_token(&config.admin).expect("issue admin session token");

        assert!(expires_at > now_unix_secs());
        assert!(verify_admin_session_token(&token, &config.admin));
        assert!(is_authorized(
            Some(&HeaderValue::from_str(&format!("Bearer {token}")).expect("header")),
            &config.admin
        ));

        config.admin.password = "rotated-password".to_string();
        assert!(!verify_admin_session_token(&token, &config.admin));
    }

    #[test]
    fn default_error_page_links_back_to_docs() {
        let html = render_default_error_html(StatusCode::NOT_FOUND, "");
        assert!(html.contains("/docs.html"));
        assert!(html.contains("404"));
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
            upstream_weights: BTreeMap::new(),
        });
        config.udp.listeners.push(UdpListenerConfig {
            name: "game-udp".to_string(),
            bind: "0.0.0.0:7001".to_string(),
            upstream: String::new(),
            upstreams: vec!["127.0.0.1:9100".to_string(), "127.0.0.1:9101".to_string()],
            upstream_weights: BTreeMap::new(),
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

    #[test]
    fn cache_lookup_key_skips_bypass_behavior() {
        let cache = ResponseCacheConfig {
            enabled: true,
            behavior: CacheBehavior::Bypass,
            ..Default::default()
        };
        let uri: Uri = "/".parse().expect("uri");
        let headers = HeaderMap::new();
        assert!(cache_lookup_key(&cache, &Method::GET, "example.com", &uri, &headers).is_none());
    }

    #[test]
    fn effective_edge_ttl_override_ignores_origin_max_age() {
        let cache = ResponseCacheConfig {
            enabled: true,
            behavior: CacheBehavior::Override,
            ttl_secs: 3600,
            ..Default::default()
        };
        let headers = vec![(CACHE_CONTROL, HeaderValue::from_static("max-age=60"))];
        assert_eq!(effective_edge_ttl_secs(&cache, &headers), 3600);
    }

    #[test]
    fn ftp_transfer_policy_honors_deny_list() {
        let config = crate::config::FtpConfig {
            transfer_deny: vec!["STOR".to_string()],
            ..Default::default()
        };
        assert!(!ftp_transfer_allowed_for_user(&config, "STOR", "alice"));
        assert!(ftp_transfer_allowed_for_user(&config, "RETR", "alice"));
    }

    #[test]
    fn ftp_command_policy_honors_allow_and_deny_lists() {
        let config = crate::config::FtpConfig {
            command_deny: vec!["DELE".to_string()],
            ..Default::default()
        };
        assert!(!ftp_command_allowed(&config, "DELE"));
        assert!(ftp_command_allowed(&config, "LIST"));

        let config = crate::config::FtpConfig {
            command_allow: vec!["USER".to_string(), "PASS".to_string()],
            command_deny: vec!["DELE".to_string()],
            ..Default::default()
        };
        assert!(ftp_command_allowed(&config, "USER"));
        assert!(!ftp_command_allowed(&config, "LIST"));
    }

    #[test]
    fn cache_lookup_key_includes_vary_headers_and_prefix() {
        let mut cache = ResponseCacheConfig {
            enabled: true,
            vary_headers: vec!["Accept-Encoding".to_string()],
            key_prefix: "api".to_string(),
            ..Default::default()
        };
        let uri: Uri = "/v1/items".parse().expect("uri");
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));
        let key = cache_lookup_key(&cache, &Method::GET, "example.com", &uri, &headers)
            .expect("cache key");
        assert!(key.starts_with("api:"));
        assert!(key.contains("Accept-Encoding=gzip"));

        cache.vary_headers.clear();
        let plain = cache_lookup_key(&cache, &Method::GET, "example.com", &uri, &headers)
            .expect("cache key");
        assert!(plain.starts_with("api:GET:example.com:/v1/items"));
    }

    #[test]
    fn leaky_bucket_rate_limit_blocks_sustained_overflow() {
        let config = HttpRateLimitConfig {
            enabled: true,
            zone: "default".to_string(),
            algorithm: RateLimitAlgorithm::LeakyBucket,
            key: RateLimitKey::RemoteAddr,
            requests: 2,
            window_ms: 1_000,
            burst: 0,
            max_connections: 0,
            status: 429,
        };
        let store = DashMap::new();
        let key = "remote:127.0.0.1".to_string();
        assert!(apply_http_rate_limit_to_store(&store, &config, key.clone()).is_none());
        assert!(apply_http_rate_limit_to_store(&store, &config, key.clone()).is_none());
        assert!(apply_http_rate_limit_to_store(&store, &config, key).is_some());
    }

    #[test]
    fn stream_rate_limit_uses_shared_zone_store() {
        let config = StreamRateLimitConfig {
            enabled: true,
            zone: "stream".to_string(),
            algorithm: RateLimitAlgorithm::FixedWindow,
            connections: 1,
            window_ms: 60_000,
            burst: 0,
        };
        let store = DashMap::new();
        let addr = "127.0.0.1:4000".parse().expect("socket");
        assert!(apply_stream_rate_limit(&store, &config, addr));
        assert!(!apply_stream_rate_limit(&store, &config, addr));
    }
}
