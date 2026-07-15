use std::borrow::Cow;
use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::Infallible;
use std::fs;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Cursor, SeekFrom};
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use brotli::CompressorWriter;
use bytes::{Buf, Bytes, BytesMut};
use crossbeam_queue::ArrayQueue;
use dashmap::{DashMap, DashSet};
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::TryStreamExt;
use h3::server::Connection as H3Connection;
use hmac::{Hmac, Mac};
use http::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_RANGES, AUTHORIZATION, CACHE_CONTROL, CONNECTION,
    CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, COOKIE, HOST, LOCATION, RANGE,
    SET_COOKIE, TRANSFER_ENCODING, VARY,
};
use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri, Version,
};
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Body as HyperBody, SizeHint};
use hyper::body::{Frame, Incoming};
use hyper::server::conn::http2::Builder as Http2ServerBuilder;
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use instant_acme::{
    Account, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder, OrderStatus, RetryPolicy,
};
use memchr::memmem;
use quinn::crypto::rustls::QuicServerConfig;
use rcgen::{CertificateParams, CustomExtension, DistinguishedName, KeyPair};
use rustc_hash::FxHashMap;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::UnixTime;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ClientConfig;
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use socket2::{Domain, Protocol as SocketProtocol, Socket, Type};
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt,
    BufReader as TokioBufReader, ReadBuf,
};
use tokio::net::{TcpListener, TcpSocket, TcpStream, UdpSocket};
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::io::ReaderStream;
use url::Url;
use uuid::Uuid;
use zstd::stream::encode_all as zstd_encode_all;

#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::sync::OnceLock;

use crate::acme::{acme_challenge_fqdn, DnsProvider};
use crate::config::{
    on_demand_domain_allowed, AcmeChallengeType, ActiveHealthConfig, ActiveHealthOverrideConfig,
    AdminConfig, CacheBehavior, CompressionAlgorithm, DomainRouteConfig, DomainTlsMode,
    FileCloudConfig, FtpUserPolicy, GatewayConfig, HttpAccessControlConfig, HttpAffinityConfig,
    HttpRateLimitConfig, LoadBalanceAlgorithm, MonitoringFormat, OnDemandTlsConfig,
    RateLimitAlgorithm, RateLimitKey, ResponseCacheConfig, ResponseCompressionConfig,
    ReverseProxyRouteConfig, RuntimePerformanceTrafficProfile, StaticSiteConfig,
    StreamAffinityConfig, StreamRateLimitConfig, StreamRouteConfig, TcpListenerConfig,
    TlsCertificateConfig, TlsMode, UdpListenerConfig, WebDavConfig,
};
use crate::install;
use crate::linux_tune::{self, TcpTuneProfile};
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
    bootstrap_fast_lane: FastLaneState,
    dynamic: Arc<RwLock<Arc<DynamicState>>>,
    stats: Arc<GatewayStats>,
    sticky_affinity: Arc<DashMap<String, StickyEntry>>,
    round_robin_state: Arc<DashMap<String, u64>>,
    upstream_runtime: Arc<DashMap<String, UpstreamRuntimeState>>,
    http_rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    stream_rate_limits: Arc<DashMap<String, RateLimitBucket>>,
    http_connection_limits: Arc<DashMap<String, u32>>,
    http_cache: Arc<DashMap<String, CachedHttpEntry>>,
    raw_http_pools: Arc<DashMap<String, Arc<RawHttpUpstreamPool>>>,
    static_route_cache: Arc<DashMap<String, PathBuf>>,
    static_file_cache: Arc<DashMap<String, CachedStaticFile>>,
    static_file_cache_bytes: Arc<AtomicU64>,
    static_file_load_locks: Arc<DashMap<String, Arc<TokioMutex<()>>>>,
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
    fast_lane: FastLaneState,
    http_client: reqwest::Client,
    http_fast_client: HyperClient<HttpConnector, Full<Bytes>>,
    script: Option<Arc<ScriptRuntime>>,
}

#[derive(Clone, Debug, Default)]
struct FastLaneState {
    plain_http_static_sendfile: bool,
    hyper_static_success: bool,
    simple_http_proxy: bool,
    raw_sse_proxy: bool,
    raw_reverse_proxy: bool,
    raw_websocket_proxy: bool,
}

impl FastLaneState {
    fn compile(config: &GatewayConfig) -> Self {
        let performance_enabled = config.runtime.performance.enabled;
        let simple_http_proxy = performance_enabled && simple_http_proxy_fast_path_allowed(config);
        Self {
            plain_http_static_sendfile: performance_enabled
                && cfg!(target_os = "linux")
                && plain_static_fast_path_allowed(config),
            hyper_static_success: performance_enabled
                && hyper_static_success_fast_path_globally_allowed(config),
            simple_http_proxy,
            raw_sse_proxy: simple_http_proxy,
            raw_reverse_proxy: simple_http_proxy,
            raw_websocket_proxy: simple_http_proxy,
        }
    }
}

struct RawHttpUpstreamPool {
    host: String,
    port: u16,
    idle: ArrayQueue<TcpStream>,
}

impl RawHttpUpstreamPool {
    fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            idle: ArrayQueue::new(raw_http_pool_idle_capacity()),
        }
    }

    async fn checkout(&self) -> Result<TcpStream> {
        while let Some(stream) = self.idle.pop() {
            if raw_http_idle_stream_reusable(&stream) {
                return Ok(stream);
            }
        }

        let stream = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .with_context(|| {
                format!(
                    "failed connecting raw HTTP upstream {}:{}",
                    self.host, self.port
                )
            })?;
        let _ = stream.set_nodelay(true);
        tune_tcp_stream_for_gateway(&stream);
        Ok(stream)
    }

    fn checkin(&self, stream: TcpStream) {
        let _ = self.idle.push(stream);
    }
}

fn raw_http_idle_stream_reusable(stream: &TcpStream) -> bool {
    let mut probe = [0_u8; 1];
    matches!(
        stream.try_read(&mut probe),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    )
}

fn raw_http_pool_idle_capacity() -> usize {
    adaptive_data_plane_workers(1).saturating_mul(64).max(64)
}

/// Bounded, lock-free pool of reusable heap buffers for hot-path relay and
/// streaming loops. Caps steady-state allocation churn under very high
/// connection counts (target 100k-1M concurrent sockets) without unbounded
/// growth: the backing queue is bounded, so surplus buffers are freed on return
/// and the pool's resident memory stays predictable (no leak, no runaway).
struct ByteBufferPool {
    idle: ArrayQueue<Box<[u8]>>,
    buf_size: usize,
}

impl ByteBufferPool {
    fn new(capacity: usize, buf_size: usize) -> Self {
        Self {
            idle: ArrayQueue::new(capacity.max(1)),
            buf_size,
        }
    }

    fn acquire(&'static self) -> PooledBuffer {
        let buffer = self
            .idle
            .pop()
            .unwrap_or_else(|| vec![0_u8; self.buf_size].into_boxed_slice());
        PooledBuffer {
            buffer: Some(buffer),
            pool: self,
        }
    }
}

/// RAII handle that returns its buffer to the pool on drop. When the pool is
/// full the buffer is simply freed, so the pool never exceeds its bound.
struct PooledBuffer {
    buffer: Option<Box<[u8]>>,
    pool: &'static ByteBufferPool,
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            // Bounded queue: push fails when full -> buffer is freed here.
            let _ = self.pool.idle.push(buffer);
        }
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.buffer.as_deref().expect("pooled buffer present")
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.buffer.as_deref_mut().expect("pooled buffer present")
    }
}

const POOL_BUFFER_BYTES: usize = 64 * 1024;
#[cfg(test)]
const LATENCY_RELAY_BUFFER_BYTES: usize = 16 * 1024;
// A WebSocket game gateway must be able to keep 100k mostly-idle tunnels
// resident on an 8GiB host. Four KiB covers the normal game frame while
// avoiding a permanent 3.2GiB two-direction 16KiB allocation at that scale.
const WEBSOCKET_RELAY_BUFFER_BYTES: usize = 4 * 1024;
const HTTP2_STREAM_WINDOW_SIZE_BYTES: u32 = 4 * 1024 * 1024;
const HTTP2_CONNECTION_WINDOW_SIZE_BYTES: u32 = 16 * 1024 * 1024;
const HTTP2_MAX_SEND_BUF_BYTES: usize = 4 * 1024 * 1024;
const HTTP2_KEEP_ALIVE_INTERVAL_SECS: u64 = 15;
const HTTP2_KEEP_ALIVE_TIMEOUT_SECS: u64 = 20;
const HTTP2_MAX_CONCURRENT_STREAMS: u32 = 1024;
const HTTP2_MAX_FRAME_SIZE_BYTES: u32 = 64 * 1024;

static RELAY_BUFFER_POOL: OnceLock<ByteBufferPool> = OnceLock::new();
#[cfg(test)]
static LATENCY_RELAY_BUFFER_POOL: OnceLock<ByteBufferPool> = OnceLock::new();
static WEBSOCKET_RELAY_BUFFER_POOL: OnceLock<ByteBufferPool> = OnceLock::new();
static UDP_BUFFER_POOL: OnceLock<ByteBufferPool> = OnceLock::new();

fn pooled_buffer_capacity() -> usize {
    // Scales with cores but stays bounded so idle pool memory is capped at
    // capacity * 64 KiB. In-flight relay/UDP buffers are served from the pool
    // first and fall back to a one-off allocation only on a miss.
    adaptive_data_plane_workers(1)
        .saturating_mul(1024)
        .clamp(1024, 65_536)
}

fn relay_buffer_pool() -> &'static ByteBufferPool {
    RELAY_BUFFER_POOL
        .get_or_init(|| ByteBufferPool::new(pooled_buffer_capacity(), POOL_BUFFER_BYTES))
}

#[cfg(test)]
fn latency_relay_buffer_pool() -> &'static ByteBufferPool {
    LATENCY_RELAY_BUFFER_POOL.get_or_init(|| {
        // Long-lived game/WebSocket connections need a small bounded resident
        // footprint. Retaining 16KiB buffers per CPU shard handles reconnect
        // bursts while excess buffers are released when a surge drains.
        let capacity = adaptive_data_plane_workers(1)
            .saturating_mul(512)
            .clamp(512, 8_192);
        ByteBufferPool::new(capacity, LATENCY_RELAY_BUFFER_BYTES)
    })
}

fn websocket_relay_buffer_pool() -> &'static ByteBufferPool {
    WEBSOCKET_RELAY_BUFFER_POOL.get_or_init(|| {
        // Keep retained memory bounded during reconnect waves. Buffers are
        // borrowed per full-duplex direction and released once the tunnel
        // closes; 4KiB is deliberately tuned for game/WebSocket frames rather
        // than bulk transfer payloads.
        let capacity = adaptive_data_plane_workers(1)
            .saturating_mul(512)
            .clamp(512, 8_192);
        ByteBufferPool::new(capacity, WEBSOCKET_RELAY_BUFFER_BYTES)
    })
}

fn udp_buffer_pool() -> &'static ByteBufferPool {
    UDP_BUFFER_POOL.get_or_init(|| ByteBufferPool::new(pooled_buffer_capacity(), POOL_BUFFER_BYTES))
}

struct UpstreamHttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: UpstreamHttpBody,
}

enum UpstreamHttpBody {
    Reqwest(reqwest::Response),
    Hyper(Incoming),
}

impl DynamicState {
    async fn dispatch_upstream_http(
        &self,
        method: Method,
        upstream_target: UpstreamHttpTarget,
        upstream_headers: HeaderMap,
        body: Bytes,
    ) -> Result<UpstreamHttpResponse> {
        if let UpstreamHttpTarget::Hyper(upstream_uri) = upstream_target {
            let mut request = Request::builder()
                .method(method)
                .uri(upstream_uri)
                .body(Full::new(body))
                .context("failed building upstream request")?;
            *request.headers_mut() = upstream_headers;
            let response = self
                .http_fast_client
                .request(request)
                .await
                .context("upstream request failed")?;
            let (parts, body) = response.into_parts();
            return Ok(UpstreamHttpResponse {
                status: parts.status,
                headers: parts.headers,
                body: UpstreamHttpBody::Hyper(body),
            });
        }

        let UpstreamHttpTarget::Reqwest(upstream_url) = upstream_target else {
            unreachable!("hyper upstream target handled above")
        };
        let response = self
            .http_client
            .request(method, upstream_url)
            .headers(upstream_headers)
            .body(body)
            .send()
            .await
            .context("upstream request failed")?;
        let status = response.status();
        let headers = response.headers().clone();
        Ok(UpstreamHttpResponse {
            status,
            headers,
            body: UpstreamHttpBody::Reqwest(response),
        })
    }
}

enum UpstreamHttpTarget {
    Hyper(Uri),
    Reqwest(Url),
}

impl UpstreamHttpResponse {
    fn status(&self) -> StatusCode {
        self.status
    }

    async fn bytes(self) -> Result<Bytes> {
        match self.body {
            UpstreamHttpBody::Reqwest(response) => response
                .bytes()
                .await
                .context("failed reading upstream response body"),
            UpstreamHttpBody::Hyper(body) => body
                .collect()
                .await
                .map(|collected| collected.to_bytes())
                .context("failed reading upstream response body"),
        }
    }

    fn into_stream_body(self) -> GatewayBody {
        match self.body {
            UpstreamHttpBody::Reqwest(response) => streaming_body(response.bytes_stream()),
            UpstreamHttpBody::Hyper(body) => GatewayBody::Stream(
                body.map_err(|error| anyhow!("upstream response stream failed: {error}"))
                    .boxed_unsync(),
            ),
        }
    }
}

async fn collect_request_body_if_needed(
    method: &Method,
    headers: &HeaderMap,
    body: &mut Incoming,
) -> Result<Bytes, hyper::Error> {
    if request_body_declared_empty(method, headers) {
        return Ok(Bytes::new());
    }

    body.collect().await.map(|collected| collected.to_bytes())
}

fn request_body_declared_empty(method: &Method, headers: &HeaderMap) -> bool {
    if headers.contains_key(TRANSFER_ENCODING) {
        return false;
    }

    if let Some(length) = headers.get(CONTENT_LENGTH) {
        return length
            .to_str()
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(|value| value == 0)
            .unwrap_or(false);
    }

    method == Method::GET
        || method == Method::HEAD
        || method == Method::OPTIONS
        || method == Method::TRACE
}

/// Copy a full-duplex stream with two buffers borrowed from a bounded lock-free
/// pool. This mirrors Tokio's single-future `copy_bidirectional` state machine
/// instead of splitting each socket into two nested async pumps. The latter is
/// correct, but costs enough extra wakeups to show up at thousands of active
/// WebSocket game sessions.
///
/// EOF is propagated as a half-close, matching `copy_bidirectional`: the other
/// direction continues draining until it reaches EOF too.
async fn copy_bidirectional_with_pooled_buffers<A, B>(
    left: &mut A,
    right: &mut B,
    buffer_pool: &'static ByteBufferPool,
) -> std::io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin + ?Sized,
    B: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    copy_bidirectional_with_pooled_buffers_limit::<_, _, MAX_POOLED_RELAY_POLL_STEPS, false, false>(
        left,
        right,
        buffer_pool,
    )
    .await
}

/// A browser/client may tear down a WSS socket without sending TLS
/// `close_notify`. Rustls reports that as `UnexpectedEof` after the WebSocket
/// tunnel has otherwise completed. Treat that and normal peer-reset variants
/// as an expected session close so high reconnect churn does not flood the
/// production error log or obscure actual upstream failures.
fn is_expected_websocket_disconnect(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .map(|io_error| {
                matches!(
                    io_error.kind(),
                    std::io::ErrorKind::UnexpectedEof
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionAborted
                )
            })
            .unwrap_or(false)
    }) || error.chain().any(|cause| {
        cause
            .to_string()
            .contains("without sending TLS close_notify")
    })
}

async fn copy_bidirectional_with_pooled_buffers_limit<
    A,
    B,
    const MAX_POLL_STEPS: usize,
    const FLUSH_EACH_BATCH: bool,
    const FLUSH_REQUIRED: bool,
>(
    left: &mut A,
    right: &mut B,
    buffer_pool: &'static ByteBufferPool,
) -> std::io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin + ?Sized,
    B: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    let mut left_to_right = RelayTransferState::Running(PooledRelayCopyBuffer::<
        MAX_POLL_STEPS,
        FLUSH_EACH_BATCH,
        FLUSH_REQUIRED,
    >::new(buffer_pool.acquire()));
    let mut right_to_left = RelayTransferState::Running(PooledRelayCopyBuffer::<
        MAX_POLL_STEPS,
        FLUSH_EACH_BATCH,
        FLUSH_REQUIRED,
    >::new(buffer_pool.acquire()));

    std::future::poll_fn(|cx| {
        let left_result = poll_relay_transfer(cx, &mut left_to_right, left, right);
        let right_result = poll_relay_transfer(cx, &mut right_to_left, right, left);
        match (left_result, right_result) {
            (Poll::Ready(Ok(left_bytes)), Poll::Ready(Ok(right_bytes))) => {
                Poll::Ready(Ok((left_bytes, right_bytes)))
            }
            (Poll::Ready(Err(error)), _) | (_, Poll::Ready(Err(error))) => Poll::Ready(Err(error)),
            _ => Poll::Pending,
        }
    })
    .await
}

/// Keep each polling turn bounded so a continuously writable bulk connection
/// cannot monopolize the runtime that also services game/WebSocket sessions.
const MAX_POOLED_RELAY_POLL_STEPS: usize = 64;
// A normal WebSocket echo frame needs one read and one write. A budget of two
// forced a self-wake immediately after every frame, before the relay could poll
// the next read and naturally park on Pending. Three removes that redundant
// runnable task while still bounding a continuously readable peer to at most
// two frame batches per runtime turn.
const WEBSOCKET_RELAY_POLL_STEPS: usize = 3;

struct PooledRelayCopyBuffer<
    const MAX_POLL_STEPS: usize,
    const FLUSH_EACH_BATCH: bool,
    const FLUSH_REQUIRED: bool,
> {
    read_done: bool,
    need_flush: bool,
    pos: usize,
    cap: usize,
    copied: u64,
    buffer: PooledBuffer,
}

impl<const MAX_POLL_STEPS: usize, const FLUSH_EACH_BATCH: bool, const FLUSH_REQUIRED: bool>
    PooledRelayCopyBuffer<MAX_POLL_STEPS, FLUSH_EACH_BATCH, FLUSH_REQUIRED>
{
    fn new(buffer: PooledBuffer) -> Self {
        Self {
            read_done: false,
            need_flush: false,
            pos: 0,
            cap: 0,
            copied: 0,
            buffer,
        }
    }

    fn poll_fill<R>(
        &mut self,
        cx: &mut TaskContext<'_>,
        mut reader: Pin<&mut R>,
    ) -> Poll<std::io::Result<()>>
    where
        R: AsyncRead + ?Sized,
    {
        let mut read_buffer = ReadBuf::new(&mut self.buffer);
        read_buffer.set_filled(self.cap);
        match reader.as_mut().poll_read(cx, &mut read_buffer) {
            Poll::Ready(Ok(())) => {
                let filled = read_buffer.filled().len();
                self.read_done = self.cap == filled;
                self.cap = filled;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_write<R, W>(
        &mut self,
        cx: &mut TaskContext<'_>,
        mut reader: Pin<&mut R>,
        mut writer: Pin<&mut W>,
    ) -> Poll<std::io::Result<usize>>
    where
        R: AsyncRead + ?Sized,
        W: AsyncWrite + ?Sized,
    {
        match writer
            .as_mut()
            .poll_write(cx, &self.buffer[self.pos..self.cap])
        {
            Poll::Pending => {
                // Top up on a temporarily blocked write. This preserves large
                // writes for bulk peers without delaying a ready small frame.
                if !self.read_done && self.cap < self.buffer.len() {
                    match self.poll_fill(cx, reader.as_mut()) {
                        Poll::Ready(Ok(())) | Poll::Pending => {}
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    }
                }
                Poll::Pending
            }
            result => result,
        }
    }

    fn poll_copy<R, W>(
        &mut self,
        cx: &mut TaskContext<'_>,
        mut reader: Pin<&mut R>,
        mut writer: Pin<&mut W>,
    ) -> Poll<std::io::Result<u64>>
    where
        R: AsyncRead + ?Sized,
        W: AsyncWrite + ?Sized,
    {
        // Tokio's registered socket I/O already consumes cooperative budget on
        // every readiness poll. The explicit max-step limit below also bounds
        // non-socket AsyncRead/AsyncWrite implementations. Charging another
        // budget unit here duplicated scheduler bookkeeping for every relay
        // direction and every game frame.
        let mut steps = 0_usize;
        loop {
            if steps >= MAX_POLL_STEPS.max(1) {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            if self.cap < self.buffer.len() && !self.read_done {
                match self.poll_fill(cx, reader.as_mut()) {
                    Poll::Ready(Ok(())) => {
                        steps += 1;
                    }
                    Poll::Ready(Err(error)) => {
                        return Poll::Ready(Err(error));
                    }
                    Poll::Pending if self.pos == self.cap => {
                        if self.need_flush {
                            match writer.as_mut().poll_flush(cx) {
                                Poll::Ready(Ok(())) => {
                                    self.need_flush = false;
                                }
                                Poll::Ready(Err(error)) => {
                                    return Poll::Ready(Err(error));
                                }
                                Poll::Pending => return Poll::Pending,
                            }
                        }
                        return Poll::Pending;
                    }
                    Poll::Pending => {}
                }
            }

            while self.pos < self.cap {
                let written = match self.poll_write(cx, reader.as_mut(), writer.as_mut()) {
                    Poll::Ready(Ok(written)) => written,
                    Poll::Ready(Err(error)) => {
                        return Poll::Ready(Err(error));
                    }
                    Poll::Pending => return Poll::Pending,
                };
                steps += 1;
                if written == 0 {
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "relay writer accepted zero bytes",
                    )));
                }
                self.pos += written;
                self.copied = self.copied.saturating_add(written as u64);
                self.need_flush = FLUSH_REQUIRED;
            }

            debug_assert!(self.pos <= self.cap, "relay writer exceeded buffer length");
            self.pos = 0;
            self.cap = 0;
            if self.need_flush && FLUSH_EACH_BATCH {
                match writer.as_mut().poll_flush(cx) {
                    Poll::Ready(Ok(())) => {
                        self.need_flush = false;
                    }
                    Poll::Ready(Err(error)) => {
                        return Poll::Ready(Err(error));
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
            if self.read_done {
                if self.need_flush {
                    match writer.as_mut().poll_flush(cx) {
                        Poll::Ready(Ok(())) => {
                            self.need_flush = false;
                        }
                        Poll::Ready(Err(error)) => {
                            return Poll::Ready(Err(error));
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                return Poll::Ready(Ok(self.copied));
            }
        }
    }
}

enum RelayTransferState<
    const MAX_POLL_STEPS: usize,
    const FLUSH_EACH_BATCH: bool,
    const FLUSH_REQUIRED: bool,
> {
    Running(PooledRelayCopyBuffer<MAX_POLL_STEPS, FLUSH_EACH_BATCH, FLUSH_REQUIRED>),
    ShuttingDown(u64),
    Done(u64),
}

fn poll_relay_transfer<
    R,
    W,
    const MAX_POLL_STEPS: usize,
    const FLUSH_EACH_BATCH: bool,
    const FLUSH_REQUIRED: bool,
>(
    cx: &mut TaskContext<'_>,
    state: &mut RelayTransferState<MAX_POLL_STEPS, FLUSH_EACH_BATCH, FLUSH_REQUIRED>,
    reader: &mut R,
    writer: &mut W,
) -> Poll<std::io::Result<u64>>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    loop {
        match state {
            RelayTransferState::Running(buffer) => {
                let copied =
                    match buffer.poll_copy(cx, Pin::new(&mut *reader), Pin::new(&mut *writer)) {
                        Poll::Ready(Ok(copied)) => copied,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    };
                *state = RelayTransferState::ShuttingDown(copied);
            }
            RelayTransferState::ShuttingDown(copied) => {
                match Pin::new(&mut *writer).poll_shutdown(cx) {
                    Poll::Ready(Ok(())) => *state = RelayTransferState::Done(*copied),
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Pending => return Poll::Pending,
                }
            }
            RelayTransferState::Done(copied) => return Poll::Ready(Ok(*copied)),
        }
    }
}

async fn copy_tcp_bidirectional_parallel(
    mut left: TcpStream,
    mut right: TcpStream,
    buffer_pool: &'static ByteBufferPool,
) -> Result<(u64, u64)> {
    copy_bidirectional_with_pooled_buffers(&mut left, &mut right, buffer_pool)
        .await
        .context("tcp bidirectional copy failed")
}

async fn copy_tcp_bidirectional_adaptive(
    left: TcpStream,
    right: TcpStream,
    performance_enabled: bool,
    profile: TcpRelayProfile,
) -> Result<(u64, u64)> {
    #[cfg(target_os = "linux")]
    if LINUX_STREAM_REACTOR_ENABLED
        && performance_enabled
        && matches!(profile, TcpRelayProfile::RealtimeSmall)
    {
        match crate::stream_reactor::dispatch_with_completion(
            left.as_raw_fd(),
            right.as_raw_fd(),
            realtime_stream_reactor_workers(),
            realtime_stream_reactor_nice(),
        ) {
            Ok(completion) => {
                // The reactor owns duplicated descriptors now. Dropping the
                // Tokio registrations (without shutdown) leaves exactly one
                // descriptor per side and frees their runtime tasks/reactors.
                drop(left);
                drop(right);
                completion
                    .await
                    .context("Linux stream reactor stopped before TCP session close")?;
                return Ok((0, 0));
            }
            Err(error) => {
                tracing::debug!(
                    ?error,
                    "Linux stream reactor handoff unavailable; using async TCP relay"
                );
            }
        }
    }

    if performance_enabled
        && matches!(profile, TcpRelayProfile::Bulk)
        && tcp_splice_fast_path_enabled()
    {
        return copy_tcp_bidirectional_splice(left, right).await;
    }

    match profile {
        TcpRelayProfile::RealtimeSmall | TcpRelayProfile::Latency => {
            copy_tcp_bidirectional_latency(left, right).await
        }
        TcpRelayProfile::Bulk => {
            copy_tcp_bidirectional_parallel(left, right, relay_buffer_pool()).await
        }
    }
}

struct DirectTcpFastPath<'a> {
    upstream: String,
    listener_name: &'a str,
    protocol: &'a str,
    nodelay: bool,
    connect_timeout_ms: u64,
    first_payload: BytesMut,
    remote_addr: SocketAddr,
    worker_index: usize,
}

async fn relay_direct_tcp_fast_path(
    inbound: TcpStream,
    context: DirectTcpFastPath<'_>,
) -> Result<()> {
    let DirectTcpFastPath {
        upstream,
        listener_name,
        protocol,
        nodelay,
        connect_timeout_ms,
        first_payload,
        remote_addr,
        worker_index,
    } = context;

    let mut outbound = tokio::time::timeout(
        Duration::from_millis(connect_timeout_ms.max(1)),
        TcpStream::connect(&upstream),
    )
    .await
    .map_err(|_| {
        anyhow!("timed out connecting direct tcp upstream {upstream} after {connect_timeout_ms}ms")
    })?
    .with_context(|| format!("failed to connect direct tcp upstream {upstream}"))?;

    if nodelay {
        outbound
            .set_nodelay(true)
            .with_context(|| format!("failed setting TCP_NODELAY for upstream {upstream}"))?;
    }
    tune_tcp_stream_for_latency(&outbound);

    if !protocol.trim().is_empty() {
        tracing::debug!(
            protocol = %protocol,
            listener = %listener_name,
            upstream = %upstream,
            worker = worker_index,
            %remote_addr,
            "direct tcp fast path selected"
        );
    }

    let first_payload_len = first_payload.len();
    if !first_payload.is_empty() {
        outbound
            .write_all(&first_payload)
            .await
            .context("failed to forward peeked tcp payload")?;
    }

    copy_tcp_bidirectional_adaptive(
        inbound,
        outbound,
        true,
        tcp_relay_profile(protocol, first_payload_len),
    )
    .await
    .context("direct tcp fast path copy failed")?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpRelayProfile {
    RealtimeSmall,
    Latency,
    Bulk,
}

fn tcp_relay_profile(protocol: &str, first_payload_len: usize) -> TcpRelayProfile {
    let protocol = protocol.trim().to_ascii_lowercase();
    if first_payload_len >= 64 * 1024
        || protocol.contains("bulk")
        || protocol.contains("file")
        || protocol.contains("backup")
    {
        TcpRelayProfile::Bulk
    } else if first_payload_len <= 1024 {
        TcpRelayProfile::RealtimeSmall
    } else {
        TcpRelayProfile::Latency
    }
}

async fn copy_tcp_bidirectional_latency(left: TcpStream, right: TcpStream) -> Result<(u64, u64)> {
    let mut left = left;
    let mut right = right;
    tokio::io::copy_bidirectional(&mut left, &mut right)
        .await
        .context("tcp latency relay failed")
}

fn tcp_splice_fast_path_enabled() -> bool {
    tcp_splice_supported()
}

#[cfg(target_os = "linux")]
fn tcp_splice_supported() -> bool {
    let level = *RUNTIME_SOCKET_TUNE_LEVEL
        .get()
        .unwrap_or(&linux_tune::RuntimeSocketTuneLevel::PortableLinux);
    !matches!(level, linux_tune::RuntimeSocketTuneLevel::Disabled)
}

#[cfg(not(target_os = "linux"))]
fn tcp_splice_supported() -> bool {
    false
}

#[cfg(target_os = "linux")]
async fn copy_tcp_bidirectional_splice(left: TcpStream, right: TcpStream) -> Result<(u64, u64)> {
    copy_tcp_bidirectional_splice_with_mode(left, right, true).await
}

#[cfg(target_os = "linux")]
async fn copy_tcp_bidirectional_splice_with_mode(
    left: TcpStream,
    right: TcpStream,
    coalesce_more: bool,
) -> Result<(u64, u64)> {
    let left = Arc::new(left);
    let right = Arc::new(right);

    let left_to_right = tokio::spawn(splice_tcp_one_direction(
        left.clone(),
        right.clone(),
        coalesce_more,
    ));
    let right_to_left = tokio::spawn(splice_tcp_one_direction(right, left, coalesce_more));

    let (left_result, right_result) = tokio::join!(left_to_right, right_to_left);
    let left_bytes = left_result.context("left-to-right tcp splice task failed")??;
    let right_bytes = right_result.context("right-to-left tcp splice task failed")??;
    Ok((left_bytes, right_bytes))
}

#[cfg(not(target_os = "linux"))]
async fn copy_tcp_bidirectional_splice(left: TcpStream, right: TcpStream) -> Result<(u64, u64)> {
    copy_tcp_bidirectional_parallel(left, right, relay_buffer_pool()).await
}

#[cfg(target_os = "linux")]
struct TcpSplicePipe {
    read_fd: std::os::fd::RawFd,
    write_fd: std::os::fd::RawFd,
}

#[cfg(target_os = "linux")]
impl TcpSplicePipe {
    fn new() -> std::io::Result<Self> {
        let mut fds = [0; 2];
        let result = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }

        let pipe = Self {
            read_fd: fds[0],
            write_fd: fds[1],
        };
        pipe.set_capacity(1024 * 1024);
        Ok(pipe)
    }

    fn set_capacity(&self, bytes: usize) {
        unsafe {
            let _ = libc::fcntl(self.write_fd, libc::F_SETPIPE_SZ, bytes as libc::c_int);
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for TcpSplicePipe {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::close(self.read_fd);
            let _ = libc::close(self.write_fd);
        }
    }
}

#[cfg(target_os = "linux")]
async fn splice_tcp_one_direction(
    reader: Arc<TcpStream>,
    writer: Arc<TcpStream>,
    coalesce_more: bool,
) -> std::io::Result<u64> {
    let pipe = TcpSplicePipe::new()?;
    let reader_fd = reader.as_raw_fd();
    let writer_fd = writer.as_raw_fd();
    let mut copied = 0_u64;
    let splice_flags = libc::SPLICE_F_MOVE
        | libc::SPLICE_F_NONBLOCK
        | if coalesce_more {
            libc::SPLICE_F_MORE
        } else {
            0
        };

    loop {
        let read = reader
            .async_io(tokio::io::Interest::READABLE, || {
                splice_fd_to_fd(reader_fd, pipe.write_fd, 256 * 1024, splice_flags)
            })
            .await?;

        if read == 0 {
            shutdown_fd_write(writer_fd);
            return Ok(copied);
        }

        let mut remaining = read;
        while remaining > 0 {
            let written = writer
                .async_io(tokio::io::Interest::WRITABLE, || {
                    splice_fd_to_fd(pipe.read_fd, writer_fd, remaining, splice_flags)
                })
                .await?;
            if written == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "tcp splice wrote zero bytes",
                ));
            }
            remaining -= written;
            copied = copied.saturating_add(written as u64);
        }
    }
}

#[cfg(target_os = "linux")]
fn splice_fd_to_fd(
    in_fd: std::os::fd::RawFd,
    out_fd: std::os::fd::RawFd,
    len: usize,
    flags: libc::c_uint,
) -> std::io::Result<usize> {
    loop {
        let moved = unsafe {
            libc::splice(
                in_fd,
                std::ptr::null_mut(),
                out_fd,
                std::ptr::null_mut(),
                len,
                flags,
            )
        };
        if moved >= 0 {
            return Ok(moved as usize);
        }

        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(error);
    }
}

#[cfg(target_os = "linux")]
fn shutdown_fd_write(fd: std::os::fd::RawFd) {
    unsafe {
        let _ = libc::shutdown(fd, libc::SHUT_WR);
    }
}

fn dedicated_tcp_stream_runtimes() -> &'static [tokio::runtime::Runtime] {
    dedicated_http_connection_runtimes()
}

fn dedicated_tcp_stream_runtime(worker_index: usize) -> &'static tokio::runtime::Runtime {
    dedicated_http_connection_runtime(worker_index)
}

fn tune_tcp_stream_for_gateway(stream: &TcpStream) {
    tune_tcp_stream_for_linux(stream, TcpSocketTuneProfile::Gateway);
}

fn tune_tcp_stream_for_latency(stream: &TcpStream) {
    tune_tcp_stream_for_linux(stream, TcpSocketTuneProfile::Realtime);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpSocketTuneProfile {
    Gateway,
    Realtime,
}

#[cfg(target_os = "linux")]
fn tune_tcp_stream_for_linux(stream: &TcpStream, profile: TcpSocketTuneProfile) {
    let level = *RUNTIME_SOCKET_TUNE_LEVEL
        .get()
        .unwrap_or(&linux_tune::RuntimeSocketTuneLevel::PortableLinux);
    if matches!(level, linux_tune::RuntimeSocketTuneLevel::Disabled) {
        return;
    }

    let fd = stream.as_raw_fd();

    // TCP_QUICKACK: disable delayed ACK for lower latency round-trips
    let quickack: libc::c_int = 1;
    unsafe {
        let _ = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_QUICKACK,
            &quickack as *const _ as *const libc::c_void,
            std::mem::size_of_val(&quickack) as libc::socklen_t,
        );
    }

    // TCP_NODELAY: disable Nagle algorithm for immediate packet dispatch.
    // Critical for game-long-connection and tcp-stream latency.
    let nodelay: libc::c_int = 1;
    unsafe {
        let _ = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_NODELAY,
            &nodelay as *const _ as *const libc::c_void,
            std::mem::size_of_val(&nodelay) as libc::socklen_t,
        );
    }

    // Large request/response gateway sockets benefit from deep queues. Keep
    // realtime game/MQTT/tool streams on Linux autotuning: forcing 1 MiB per
    // direction wastes kernel memory at 100k connections and can add queueing
    // without helping one-frame-at-a-time traffic.
    if matches!(profile, TcpSocketTuneProfile::Gateway) {
        let buf_size: libc::c_int = match level {
            linux_tune::RuntimeSocketTuneLevel::Ubuntu24Extreme
            | linux_tune::RuntimeSocketTuneLevel::FutureLinuxExtreme => 8 * 1024 * 1024,
            _ => 2 * 1024 * 1024,
        };
        unsafe {
            let _ = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
            let _ = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
        }
    }

    if matches!(
        level,
        linux_tune::RuntimeSocketTuneLevel::Ubuntu24Extreme
            | linux_tune::RuntimeSocketTuneLevel::FutureLinuxExtreme
    ) {
        if matches!(profile, TcpSocketTuneProfile::Gateway) {
            let lowat: libc::c_int = match level {
                linux_tune::RuntimeSocketTuneLevel::FutureLinuxExtreme => 8 * 1024 * 1024,
                _ => 4 * 1024 * 1024,
            };
            unsafe {
                let _ = libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_NOTSENT_LOWAT,
                    &lowat as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&lowat) as libc::socklen_t,
                );
            }
        }

        // TCP_USER_TIMEOUT: fail stalled upstream/downstream sockets promptly
        // during load spikes instead of letting them occupy gateway resources.
        let user_timeout_ms: libc::c_int = 30_000;
        unsafe {
            let _ = libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_USER_TIMEOUT,
                &user_timeout_ms as *const _ as *const libc::c_void,
                std::mem::size_of_val(&user_timeout_ms) as libc::socklen_t,
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn tune_tcp_stream_for_linux(_stream: &TcpStream, _profile: TcpSocketTuneProfile) {}

/// Enlarge UDP socket buffers so bursty game/KCP datagram floods are absorbed by
/// the kernel instead of being dropped when a worker is momentarily busy. UDP has
/// no flow control, so an undersized receive buffer is the dominant cause of
/// datagram loss (and the throughput cliff) under high-rate large-datagram load.
#[cfg(target_os = "linux")]
fn tune_udp_socket_for_gateway(socket: &UdpSocket) {
    let level = *RUNTIME_SOCKET_TUNE_LEVEL
        .get()
        .unwrap_or(&linux_tune::RuntimeSocketTuneLevel::PortableLinux);
    if matches!(level, linux_tune::RuntimeSocketTuneLevel::Disabled) {
        return;
    }

    let fd = socket.as_raw_fd();
    let buf_size: libc::c_int = match level {
        linux_tune::RuntimeSocketTuneLevel::Ubuntu24Extreme
        | linux_tune::RuntimeSocketTuneLevel::FutureLinuxExtreme => 16 * 1024 * 1024,
        _ => 4 * 1024 * 1024,
    };

    // SO_RCVBUF/SO_SNDBUF are clamped by net.core.rmem_max/wmem_max (raised by
    // `proxysss tune linux`). SO_RCVBUFFORCE/SO_SNDBUFFORCE bypass that ceiling
    // when the process is privileged (CAP_NET_ADMIN); both are best-effort and
    // silently ignored when unsupported or unprivileged.
    for (opt, force) in [
        (libc::SO_RCVBUF, libc::SO_RCVBUFFORCE),
        (libc::SO_SNDBUF, libc::SO_SNDBUFFORCE),
    ] {
        unsafe {
            let _ = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                opt,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
            let _ = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                force,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn tune_udp_socket_for_gateway(_socket: &UdpSocket) {}

#[cfg(target_os = "linux")]
fn set_tcp_cork(stream: &TcpStream, enabled: bool) {
    let value: libc::c_int = if enabled { 1 } else { 0 };
    unsafe {
        let _ = libc::setsockopt(
            stream.as_raw_fd(),
            libc::IPPROTO_TCP,
            libc::TCP_CORK,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of_val(&value) as libc::socklen_t,
        );
    }
}

struct UdpAssociation {
    socket: Arc<UdpSocket>,
    last_seen_epoch: AtomicU64,
    active: AtomicBool,
}

struct PendingUdpSessionGuard {
    sessions: Arc<DashSet<SocketAddr>>,
    addr: SocketAddr,
}

impl Drop for PendingUdpSessionGuard {
    fn drop(&mut self) {
        self.sessions.remove(&self.addr);
    }
}

struct LocalUdpAssociation {
    association: Arc<UdpAssociation>,
    last_seen_epoch: u64,
}

struct UdpPruneState {
    last_prune_epoch: AtomicU64,
    create_counter: AtomicU64,
    pruning: AtomicBool,
}

impl UdpPruneState {
    fn new() -> Self {
        Self {
            last_prune_epoch: AtomicU64::new(now_unix_secs()),
            create_counter: AtomicU64::new(0),
            pruning: AtomicBool::new(false),
        }
    }
}

struct DirectUdpRouteCache {
    state: Option<Arc<DynamicState>>,
    upstream: Option<SocketAddr>,
}

impl DirectUdpRouteCache {
    fn new() -> Self {
        Self {
            state: None,
            upstream: None,
        }
    }

    fn get_or_refresh(
        &mut self,
        state: Arc<DynamicState>,
        listener_name: &str,
    ) -> Option<SocketAddr> {
        if self
            .state
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, &state))
        {
            return self.upstream;
        }

        self.upstream = if state.script.is_none() {
            direct_udp_listener_upstream(&state.config, listener_name).and_then(|upstream| {
                match upstream.parse::<SocketAddr>() {
                    Ok(addr) => Some(addr),
                    Err(error) => {
                        tracing::warn!(
                            ?error,
                            listener = %listener_name,
                            upstream = %upstream,
                            "direct udp fast path ignored invalid upstream"
                        );
                        None
                    }
                }
            })
        } else {
            None
        };
        self.state = Some(state);
        self.upstream
    }
}

struct UdpAssociationBuildContext<'a> {
    gateway: &'a Arc<Gateway>,
    state: &'a Arc<DynamicState>,
    listener_name: &'a str,
    listener_socket: &'a Arc<UdpSocket>,
    associations: &'a Arc<DashMap<SocketAddr, Arc<UdpAssociation>>>,
    prune_state: &'a Arc<UdpPruneState>,
    protocol_hint: &'a str,
    client_addr: SocketAddr,
    payload: &'a Bytes,
    request_id: &'a str,
    session_ttl_secs: u64,
    max_associations: usize,
}

struct DirectUdpAssociationBuildContext<'a> {
    gateway: &'a Arc<Gateway>,
    listener_name: &'a str,
    listener_socket: &'a Arc<UdpSocket>,
    associations: &'a Arc<DashMap<SocketAddr, Arc<UdpAssociation>>>,
    prune_state: &'a Arc<UdpPruneState>,
    upstream_addr: SocketAddr,
    client_addr: SocketAddr,
    session_ttl_secs: u64,
    max_associations: usize,
}

pub(crate) struct GatewayHttpResponse {
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
    stream_body: Option<GatewayBody>,
    upstream: String,
}

type GatewayResponse = Response<GatewayBody>;

enum HyperFastPathResponse {
    Direct(GatewayResponse),
    Gateway(GatewayHttpResponse),
}

enum GatewayBody {
    Full(Option<Bytes>),
    Stream(UnsyncBoxBody<Bytes, anyhow::Error>),
}

impl HyperBody for GatewayBody {
    type Data = Bytes;
    type Error = anyhow::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<std::result::Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self {
            Self::Full(body) => Poll::Ready(body.take().map(|body| Ok(Frame::data(body)))),
            Self::Stream(body) => Pin::new(body).poll_frame(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Full(body) => body.as_ref().map(|body| body.is_empty()).unwrap_or(true),
            Self::Stream(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Full(Some(body)) => SizeHint::with_exact(body.len() as u64),
            Self::Full(None) => SizeHint::with_exact(0),
            Self::Stream(body) => body.size_hint(),
        }
    }
}
const STATIC_STREAM_THRESHOLD_BYTES: u64 = 32 * 1024 * 1024;
const STATIC_SENDFILE_FAST_PATH_THRESHOLD_BYTES: u64 = 32 * 1024 * 1024;
#[cfg(target_os = "linux")]
const STATIC_SENDFILE_SMALL_CHUNK_BYTES: u64 = 2 * 1024 * 1024;
#[cfg(target_os = "linux")]
const STATIC_SENDFILE_BALANCED_CHUNK_BYTES: u64 = 16 * 1024 * 1024;
#[cfg(target_os = "linux")]
const STATIC_SENDFILE_BULK_CHUNK_BYTES: u64 = 16 * 1024 * 1024;
#[cfg(target_os = "linux")]
const STATIC_SENDFILE_BALANCED_FAIR_CHUNK_BYTES: u64 = 16 * 1024 * 1024;
#[cfg(target_os = "linux")]
const STATIC_SENDFILE_QOS_DELAY: Duration = Duration::from_micros(125);
const STATIC_MMAP_THRESHOLD_BYTES: u64 = 1024 * 1024;
const STATIC_FILE_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;
const STATIC_FILE_CACHE_MAX_ENTRIES: usize = 256;
const STATIC_FILE_CACHE_REVALIDATE_SECS: u64 = 1;
const STATIC_PRELOAD_MAX_FILES_PER_SITE: usize = 64;
const STATIC_PRELOAD_SMALL_MAX_BYTES: u64 = 1024 * 1024;
const RAW_REVERSE_RESPONSE_CACHE_MAX_HEAD_BYTES: usize = 4096;
// Socket reads/writes already yield when the peer is not ready. Amortize the
// explicit cooperative yield over a larger batch so tiny cached objects do not
// pay scheduler overhead on every response. Raw reverse requests cross their
// own upstream/downstream readiness points and need no extra periodic yield.
const PLAIN_FAST_LANE_FAIRNESS_BATCH: usize = 32;
const PLAIN_FAST_LANE_LOW_DENSITY_BATCH: usize = 8;
const PLAIN_FAST_LANE_HIGH_DENSITY_CONNECTIONS: usize = 300;
const UPSTREAM_STREAM_THRESHOLD_BYTES: u64 = 64 * 1024;
#[cfg(target_os = "linux")]
const LINUX_STREAM_REACTOR_ENABLED: bool = false;
const TCP_LISTEN_BACKLOG: u32 = 262_144;
// With the Linux fair scheduler enabled, a hot listen backlog can keep
// `accept()` immediately ready long enough to queue hundreds of TLS tasks
// before any handshake runs. Yield in small batches: enough multi_accept-style
// amortization for throughput, but bounded queue delay for connection p95/p99.
const TLS_ACCEPT_LOW_DENSITY_BATCH: usize = 1;
const TLS_ACCEPT_HIGH_DENSITY_BATCH: usize = 1;
const TLS_ACCEPT_HIGH_DENSITY_PER_SHARD: usize = 4_096;
const TLS_ELASTIC_CONNECTIONS_PER_BASE_SHARD: usize = 64;
// Data-plane shards mostly run connection-local tasks. Checking Tokio's global
// injection queue every two polls and the I/O driver every three polls spends
// a material part of small-packet CPU on scheduler bookkeeping. Keep I/O
// polling substantially more frequent than Tokio's throughput-oriented
// default while amortizing both checks across a useful ready-task batch.
const DATA_RUNTIME_GLOBAL_QUEUE_INTERVAL: u32 = 31;
const DATA_RUNTIME_EVENT_INTERVAL: u32 = 8;
#[cfg(target_os = "linux")]
static RUNTIME_SOCKET_TUNE_LEVEL: OnceLock<linux_tune::RuntimeSocketTuneLevel> = OnceLock::new();
static HTTP_CONNECTION_RUNTIMES: OnceLock<Vec<tokio::runtime::Runtime>> = OnceLock::new();
static TLS_CONNECTION_RUNTIMES: OnceLock<Vec<tokio::runtime::Runtime>> = OnceLock::new();
static UDP_CONNECTION_RUNTIMES: OnceLock<Vec<tokio::runtime::Runtime>> = OnceLock::new();
static SHARED_BALANCED_UDP_RUNTIMES: AtomicBool = AtomicBool::new(false);
static PLAIN_HTTP_CONNECTIONS_ACTIVE: AtomicUsize = AtomicUsize::new(0);
static TLS_HTTP_RUNTIME_CPU_DIVISOR: AtomicUsize = AtomicUsize::new(1);
static TLS_HTTP_RUNTIME_NICE: AtomicI32 = AtomicI32::new(0);
static UDP_RUNTIME_CPU_DIVISOR: AtomicUsize = AtomicUsize::new(1);
static UDP_RUNTIME_NICE: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "linux")]
static STATIC_SENDFILE_QOS_ENABLED: AtomicBool = AtomicBool::new(true);
#[cfg(target_os = "linux")]
static STATIC_SENDFILE_REACTOR_ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "linux")]
static STATIC_SENDFILE_MAX_CHUNK_BYTES: AtomicU64 =
    AtomicU64::new(STATIC_SENDFILE_SMALL_CHUNK_BYTES);
#[cfg(target_os = "linux")]
static STATIC_SENDFILE_REACTOR_NICE: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "linux")]
static REALTIME_STREAM_REACTOR_CPU_DIVISOR: AtomicUsize = AtomicUsize::new(2);
#[cfg(target_os = "linux")]
static REALTIME_STREAM_REACTOR_NICE: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "linux")]
static DATA_PLANE_CPU_IDS: OnceLock<Vec<usize>> = OnceLock::new();

fn dedicated_http_connection_runtimes() -> &'static [tokio::runtime::Runtime] {
    HTTP_CONNECTION_RUNTIMES.get_or_init(|| {
        // Keep each SO_REUSEPORT accept shard and its ordinary HTTP sockets on
        // one reactor thread. This avoids work-stealing and global-queue costs
        // under sustained static/reverse-proxy load. TLS connections remain on
        // the accepting shard so rustls sockets are never migrated mid-flight.
        let shard_count = http_data_plane_workers_for(adaptive_data_plane_workers(1));
        tracing::info!(
            runtime_shards = shard_count,
            "starting sharded plain HTTP data runtimes"
        );
        (0..shard_count)
            .map(|shard_index| {
                let mut builder = tokio::runtime::Builder::new_multi_thread();
                builder
                    .worker_threads(1)
                    .thread_name(format!("proxysss-http-{shard_index}"))
                    .global_queue_interval(DATA_RUNTIME_GLOBAL_QUEUE_INTERVAL)
                    .event_interval(DATA_RUNTIME_EVENT_INTERVAL)
                    .on_thread_start(move || pin_current_data_plane_thread(shard_index))
                    .enable_all();
                builder
                    .build()
                    .expect("failed to build proxysss HTTP runtime shard")
            })
            .collect()
    })
}

fn dedicated_http_connection_runtime(worker_index: usize) -> &'static tokio::runtime::Runtime {
    let runtimes = dedicated_http_connection_runtimes();
    &runtimes[worker_index % runtimes.len()]
}

fn dedicated_tls_connection_runtimes() -> &'static [tokio::runtime::Runtime] {
    TLS_CONNECTION_RUNTIMES.get_or_init(|| {
        let worker_count = tls_http_runtime_workers_for(
            adaptive_data_plane_workers(1),
            TLS_HTTP_RUNTIME_CPU_DIVISOR.load(Ordering::Relaxed),
        );
        let scheduler_nice = TLS_HTTP_RUNTIME_NICE.load(Ordering::Relaxed);
        tracing::info!(
            runtime_workers = worker_count,
            scheduler_nice,
            "starting bounded TLS HTTP data runtime"
        );
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder
            .worker_threads(worker_count)
            .thread_name("proxysss-tls")
            .global_queue_interval(DATA_RUNTIME_GLOBAL_QUEUE_INTERVAL)
            .event_interval(DATA_RUNTIME_EVENT_INTERVAL)
            .on_thread_start(move || set_current_thread_nice(scheduler_nice))
            .enable_all();
        vec![builder
            .build()
            .expect("failed to build proxysss TLS data runtime")]
    })
}

fn dedicated_tls_connection_runtime(worker_index: usize) -> &'static tokio::runtime::Runtime {
    let runtimes = dedicated_tls_connection_runtimes();
    &runtimes[worker_index % runtimes.len()]
}

fn dedicated_udp_connection_runtimes() -> &'static [tokio::runtime::Runtime] {
    UDP_CONNECTION_RUNTIMES.get_or_init(|| {
        let worker_count = udp_runtime_workers_for(
            adaptive_data_plane_workers(1),
            UDP_RUNTIME_CPU_DIVISOR.load(Ordering::Relaxed),
        );
        let scheduler_nice = UDP_RUNTIME_NICE.load(Ordering::Relaxed);
        tracing::info!(
            runtime_workers = worker_count,
            scheduler_nice,
            "starting weighted UDP data runtime"
        );
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder
            .worker_threads(worker_count)
            .thread_name("proxysss-udp")
            .global_queue_interval(DATA_RUNTIME_GLOBAL_QUEUE_INTERVAL)
            .event_interval(DATA_RUNTIME_EVENT_INTERVAL)
            .on_thread_start(move || set_current_thread_nice(scheduler_nice))
            .enable_all();
        vec![builder
            .build()
            .expect("failed to build proxysss UDP data runtime")]
    })
}

fn dedicated_udp_connection_runtime(worker_index: usize) -> &'static tokio::runtime::Runtime {
    if SHARED_BALANCED_UDP_RUNTIMES.load(Ordering::Relaxed) {
        return dedicated_http_connection_runtime(worker_index);
    }
    let runtimes = dedicated_udp_connection_runtimes();
    &runtimes[worker_index % runtimes.len()]
}

fn initialize_udp_connection_runtimes() {
    if SHARED_BALANCED_UDP_RUNTIMES.load(Ordering::Relaxed) {
        let _ = dedicated_http_connection_runtimes();
    } else {
        let _ = dedicated_udp_connection_runtimes();
    }
}

#[cfg(target_os = "linux")]
fn data_plane_cpu_ids() -> &'static [usize] {
    DATA_PLANE_CPU_IDS.get_or_init(|| {
        let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        let result = unsafe {
            libc::sched_getaffinity(
                0,
                std::mem::size_of::<libc::cpu_set_t>(),
                &mut set as *mut libc::cpu_set_t,
            )
        };
        let mut cpus = Vec::new();
        if result == 0 {
            for cpu in 0..libc::CPU_SETSIZE as usize {
                if unsafe { libc::CPU_ISSET(cpu, &set) } {
                    cpus.push(cpu);
                }
            }
        }
        if cpus.is_empty() {
            cpus.push(0);
        }
        cpus
    })
}

#[cfg(target_os = "linux")]
fn pin_current_data_plane_thread(worker_index: usize) {
    let cpus = data_plane_cpu_ids();
    let cpu = cpus[worker_index % cpus.len()];
    let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_SET(cpu, &mut set);
        let _ = libc::sched_setaffinity(
            0,
            std::mem::size_of::<libc::cpu_set_t>(),
            &set as *const libc::cpu_set_t,
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn pin_current_data_plane_thread(_worker_index: usize) {}

#[cfg(target_os = "linux")]
fn set_current_thread_nice(nice: i32) {
    if nice > 0 {
        unsafe {
            let _ = libc::setpriority(libc::PRIO_PROCESS, 0, nice.clamp(0, 19));
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn set_current_thread_nice(_nice: i32) {}

fn spawn_http_connection<Connection>(
    performance_enabled: bool,
    worker_index: usize,
    connection: Connection,
) where
    Connection: Future<Output = ()> + Send + 'static,
{
    if cfg!(target_os = "linux") && performance_enabled {
        std::mem::drop(dedicated_http_connection_runtime(worker_index).spawn(connection));
    } else {
        std::mem::drop(tokio::spawn(connection));
    }
}

pub(crate) fn configure_runtime_performance(config: &GatewayConfig) -> linux_tune::RuntimeTunePlan {
    let profile = match config.runtime.performance.profile {
        crate::config::RuntimePerformanceProfile::Edge => TcpTuneProfile::Edge,
        crate::config::RuntimePerformanceProfile::Bulk => TcpTuneProfile::Bulk,
        crate::config::RuntimePerformanceProfile::Latency => TcpTuneProfile::Latency,
    };
    let plan = linux_tune::build_runtime_tune_plan(
        config.runtime.performance.enabled,
        config.runtime.performance.adaptive_system,
        config.runtime.performance.socket_extreme,
        profile,
    );
    #[cfg(target_os = "linux")]
    {
        let _ = RUNTIME_SOCKET_TUNE_LEVEL.set(plan.socket_level);
        SHARED_BALANCED_UDP_RUNTIMES.store(
            config.runtime.performance.enabled
                && shared_udp_runtime_profile(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        TLS_HTTP_RUNTIME_CPU_DIVISOR.store(
            tls_http_runtime_cpu_divisor(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        TLS_HTTP_RUNTIME_NICE.store(
            tls_http_runtime_nice_for(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        UDP_RUNTIME_CPU_DIVISOR.store(
            udp_runtime_cpu_divisor(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        UDP_RUNTIME_NICE.store(
            udp_runtime_nice_for(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        STATIC_SENDFILE_QOS_ENABLED.store(
            config.runtime.performance.enabled
                && matches!(
                    config.runtime.performance.traffic_profile,
                    RuntimePerformanceTrafficProfile::Small
                ),
            Ordering::Relaxed,
        );
        STATIC_SENDFILE_REACTOR_ENABLED.store(
            config.runtime.performance.enabled
                && sendfile_reactor_profile_enabled(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        let stream_reactor_divisor =
            realtime_stream_reactor_cpu_divisor(config.runtime.performance.traffic_profile);
        REALTIME_STREAM_REACTOR_CPU_DIVISOR.store(stream_reactor_divisor, Ordering::Relaxed);
        REALTIME_STREAM_REACTOR_NICE.store(
            realtime_stream_reactor_nice_for(config.runtime.performance.traffic_profile),
            Ordering::Relaxed,
        );
        let sendfile_chunk_bytes = match config.runtime.performance.traffic_profile {
            RuntimePerformanceTrafficProfile::Small => STATIC_SENDFILE_SMALL_CHUNK_BYTES,
            RuntimePerformanceTrafficProfile::Balanced => STATIC_SENDFILE_BALANCED_CHUNK_BYTES,
            RuntimePerformanceTrafficProfile::Bulk => STATIC_SENDFILE_BULK_CHUNK_BYTES,
        };
        STATIC_SENDFILE_MAX_CHUNK_BYTES.store(sendfile_chunk_bytes, Ordering::Relaxed);
        STATIC_SENDFILE_REACTOR_NICE.store(0, Ordering::Relaxed);
    }
    plan
}

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

#[derive(Clone)]
struct CachedStaticFile {
    len: u64,
    modified: Option<SystemTime>,
    body: Bytes,
    sendfile: Option<Arc<std::fs::File>>,
    content_type: HeaderValue,
    content_length: HeaderValue,
    checked_at: Instant,
    revalidating: bool,
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
struct HttpRouteConfig<'a> {
    runtime_scope: Option<String>,
    decision: RouteDecision,
    compression: Cow<'a, ResponseCompressionConfig>,
    cache: Cow<'a, ResponseCacheConfig>,
    rate_limit: Cow<'a, HttpRateLimitConfig>,
    forward_headers: bool,
}

impl<'a> HttpRouteConfig<'a> {
    fn to_owned_config(&self) -> HttpRouteConfig<'static> {
        HttpRouteConfig {
            runtime_scope: self.runtime_scope.clone(),
            decision: self.decision.clone(),
            compression: Cow::Owned(self.compression.as_ref().clone()),
            cache: Cow::Owned(self.cache.as_ref().clone()),
            rate_limit: Cow::Owned(self.rate_limit.as_ref().clone()),
            forward_headers: self.forward_headers,
        }
    }
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
    udp_payload: String,
    udp_expect_response: bool,
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
    critical_task_failures_total: AtomicU64,
    watchdog_heartbeat_total: AtomicU64,
    /// Set true once startup/reload warm-up (static preload + upstream pre-dial)
    /// has completed. Listeners bind only after the initial warm-up, so a
    /// successful connection already implies a warm data plane.
    warm: AtomicBool,
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

struct ActiveConnectionGuard(Arc<AtomicUsize>);

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

struct ActivePlainHttpConnectionGuard;

impl ActivePlainHttpConnectionGuard {
    fn enter() -> Self {
        PLAIN_HTTP_CONNECTIONS_ACTIVE.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for ActivePlainHttpConnectionGuard {
    fn drop(&mut self) {
        PLAIN_HTTP_CONNECTIONS_ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
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
        rustls::crypto::aws_lc_rs::default_provider()
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

        let dynamic_state = build_dynamic_state(config.clone()).await?;
        let bootstrap_fast_lane = dynamic_state.fast_lane.clone();
        let dynamic = Arc::new(dynamic_state);
        let (on_demand_trigger, on_demand_rx) = tokio::sync::mpsc::unbounded_channel();
        let dynamic_blacklist =
            DynamicBlacklist::load_from_disk(&config.security.dynamic_blacklist);

        let gateway = Arc::new(Self {
            config_path,
            bootstrap_config: config,
            bootstrap_fast_lane,
            dynamic: Arc::new(RwLock::new(dynamic)),
            stats: Arc::new(GatewayStats::default()),
            sticky_affinity: Arc::new(DashMap::new()),
            round_robin_state: Arc::new(DashMap::new()),
            upstream_runtime: Arc::new(DashMap::new()),
            http_rate_limits: Arc::new(DashMap::new()),
            stream_rate_limits: Arc::new(DashMap::new()),
            http_connection_limits: Arc::new(DashMap::new()),
            http_cache: Arc::new(DashMap::new()),
            raw_http_pools: Arc::new(DashMap::new()),
            static_route_cache: Arc::new(DashMap::new()),
            static_file_cache: Arc::new(DashMap::new()),
            static_file_cache_bytes: Arc::new(AtomicU64::new(0)),
            static_file_load_locks: Arc::new(DashMap::new()),
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
        gateway.warm_up(&gateway.bootstrap_config).await;
        Ok(gateway)
    }

    /// Startup/reload self-optimization: preload hot static files into the
    /// bounded fast-lane cache and pre-dial upstream keepalive pools so the
    /// first live request never pays a cold connect. Runs during `new()` before
    /// any listener binds, so a connectable port already implies a warm data
    /// plane and a benchmark naturally starts only after warm-up.
    async fn warm_up(&self, config: &GatewayConfig) {
        let started = Instant::now();
        self.preload_static_fast_lane_cache(config).await;
        let predialed = self.prewarm_upstream_pools(config).await;
        self.stats.warm.store(true, Ordering::Release);
        tracing::info!(
            upstream_predial = predialed,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "gateway warm-up complete"
        );
    }

    /// Best-effort pre-dial of raw HTTP keepalive pools for reverse-proxy and
    /// AI-proxy routes so the first request reuses a warm socket instead of
    /// connecting cold. Bounded work with a short per-attempt timeout.
    async fn prewarm_upstream_pools(&self, config: &GatewayConfig) -> usize {
        if !config.runtime.performance.enabled {
            return 0;
        }
        let mut upstreams: Vec<String> = config
            .services
            .reverse_proxy
            .routes
            .iter()
            .filter(|route| reverse_proxy_route_fast_path_eligible(route))
            .map(|route| route.upstream.clone())
            .collect();
        if config.services.ai_proxy.enabled {
            upstreams.extend(
                config
                    .services
                    .ai_proxy
                    .routes
                    .iter()
                    .filter(|route| ai_proxy_route_fast_path_eligible(route))
                    .map(|route| route.upstream.clone()),
            );
        }

        let mut predialed = 0_usize;
        for upstream in upstreams {
            let Ok(Some((key, host, port))) = raw_http_pool_parts_from_upstream(&upstream) else {
                continue;
            };
            let pool = self.raw_http_pool_for_parts(key, host, port);
            let mut warmed = Vec::with_capacity(2);
            for _ in 0..2 {
                match tokio::time::timeout(Duration::from_millis(250), pool.checkout()).await {
                    Ok(Ok(stream)) => {
                        warmed.push(stream);
                        predialed = predialed.saturating_add(1);
                    }
                    _ => break,
                }
            }
            for stream in warmed {
                pool.checkin(stream);
            }
        }
        predialed
    }

    async fn preload_static_fast_lane_cache(&self, config: &GatewayConfig) {
        if !config.runtime.performance.enabled || !plain_static_fast_path_allowed(config) {
            return;
        }

        let profile = config.runtime.performance.traffic_profile;
        let sendfile_threshold = static_sendfile_fast_path_threshold_bytes(config);
        let mut preloaded = 0_usize;
        for site in &config.services.static_sites {
            match preload_static_site_fast_lane_cache(
                site,
                profile,
                sendfile_threshold,
                &self.static_file_cache,
                &self.static_file_cache_bytes,
                &self.static_file_load_locks,
                &self.static_route_cache,
            )
            .await
            {
                Ok(count) => preloaded = preloaded.saturating_add(count),
                Err(error) => tracing::debug!(
                    ?error,
                    site = %site.name,
                    "static fast lane preload skipped for site"
                ),
            }
        }

        if preloaded > 0 {
            tracing::info!(
                files = preloaded,
                traffic_profile = ?profile,
                "static fast lane cache preloaded"
            );
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut tasks = JoinSet::new();

        if self.bootstrap_config.runtime.hot_reload.enabled {
            Self::spawn_supervised_task(&mut tasks, self.clone(), "hot_reload", |gateway| async {
                gateway.run_hot_reload_loop().await
            });
        }

        match self.bootstrap_config.http.tls.mode {
            TlsMode::AcmeManaged => {
                Self::spawn_supervised_task(
                    &mut tasks,
                    self.clone(),
                    "managed_acme_renew",
                    |gateway| async { gateway.run_managed_acme_renew_loop().await },
                );
            }
            TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal => {
                Self::spawn_supervised_task(
                    &mut tasks,
                    self.clone(),
                    "external_acme_renew",
                    |gateway| async { gateway.run_acme_renew_loop().await },
                );
            }
            TlsMode::SelfSigned | TlsMode::Manual => {}
        }

        Self::spawn_supervised_task(
            &mut tasks,
            self.clone(),
            "listener_supervisor",
            |gateway| async { gateway.run_listener_supervisor().await },
        );

        Self::spawn_supervised_task(&mut tasks, self.clone(), "active_health", |gateway| async {
            gateway.run_active_health_loop().await
        });

        if self.bootstrap_config.http.tls.on_demand.enabled {
            Self::spawn_supervised_task(
                &mut tasks,
                self.clone(),
                "on_demand_tls_cleanup",
                |gateway| async { gateway.run_on_demand_tls_cleanup_loop().await },
            );
        }

        if self.bootstrap_config.runtime.watchdog.enabled {
            let gateway = self.clone();
            tasks.spawn(async move { gateway.run_watchdog_loop().await });
        }

        while let Some(result) = tasks.join_next().await {
            result??;
        }

        Ok(())
    }

    fn spawn_supervised_task<F, Fut>(
        tasks: &mut JoinSet<Result<()>>,
        gateway: Arc<Self>,
        name: &'static str,
        run: F,
    ) where
        F: Fn(Arc<Self>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        tasks.spawn(async move {
            loop {
                let result = run.clone()(gateway.clone()).await;
                gateway
                    .stats
                    .critical_task_failures_total
                    .fetch_add(1, Ordering::Relaxed);

                let watchdog = {
                    let state = gateway.current_state().await;
                    state.config.runtime.watchdog.clone()
                };

                match &result {
                    Ok(()) => tracing::warn!(task = name, "critical gateway task exited"),
                    Err(error) => {
                        tracing::error!(task = name, ?error, "critical gateway task failed")
                    }
                }

                if !watchdog.enabled || !watchdog.restart_critical_tasks {
                    return result.with_context(|| format!("critical task {name} stopped"));
                }

                let backoff = Duration::from_secs(watchdog.restart_backoff_secs.max(1));
                tracing::warn!(
                    task = name,
                    backoff_secs = backoff.as_secs(),
                    "restarting critical gateway task"
                );
                tokio::time::sleep(backoff).await;
            }
        });
    }

    async fn run_watchdog_loop(self: Arc<Self>) -> Result<()> {
        loop {
            let interval = {
                let state = self.current_state().await;
                Duration::from_secs(state.config.runtime.watchdog.heartbeat_interval_secs.max(1))
            };
            tokio::time::sleep(interval).await;
            self.stats
                .watchdog_heartbeat_total
                .fetch_add(1, Ordering::Relaxed);
            tracing::debug!(
                http_requests = self.stats.http_requests.load(Ordering::Relaxed),
                tcp_sessions_active = self.stats.tcp_sessions_active.load(Ordering::Relaxed),
                udp_packets_total = self.stats.udp_packets_total.load(Ordering::Relaxed),
                critical_task_failures_total = self
                    .stats
                    .critical_task_failures_total
                    .load(Ordering::Relaxed),
                "watchdog heartbeat"
            );
        }
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
                let handle = if matches!(spec, ListenerSpec::Tcp(_))
                    && state.config.runtime.performance.enabled
                {
                    dedicated_tcp_stream_runtime(0)
                        .spawn(async move { gateway.run_listener_spec(spec).await })
                } else {
                    tokio::spawn(async move { gateway.run_listener_spec(spec).await })
                };
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
        let listener = bind_tcp_listener(bind_addr, "admin listener").await?;

        tracing::info!(bind = %bind_addr, "admin listener ready");

        loop {
            let (stream, remote_addr) = listener.accept().await.context("admin accept failed")?;
            if let Err(error) = stream.set_nodelay(true) {
                tracing::debug!(?error, %remote_addr, "failed setting TCP_NODELAY on admin connection");
            }
            tune_tcp_stream_for_gateway(&stream);
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

                let result = optimized_http_server_builder()
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
                serde_json::json!({"ok": true, "service": "proxysss", "warm": self.stats.warm.load(Ordering::Relaxed), "remote_addr": remote_addr.to_string()}),
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
            let challenge = payload.challenge;
            match self
                .persist_auto_https_and_reload(&state.config, payload)
                .await
            {
                Ok(domains) => {
                    return Ok(json_response(
                        StatusCode::OK,
                        serde_json::json!({"ok": true, "domains": domains, "mode": "acme_managed", "challenge": challenge}),
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
        self.prune_raw_http_pools(&new_config);
        self.warm_up(&new_config).await;

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
        security::validate_domains(&payload.domains)?;

        let domains = payload.domains.clone();
        let mut candidate = current_config.clone();
        candidate.http.tls.auto_https.enabled = true;
        candidate.http.tls.auto_https.domains = domains.clone();
        candidate.http.tls.auto_https.email = payload.email.clone();
        candidate.http.tls.auto_https.production = payload.production;
        candidate.http.tls.auto_https.challenge = payload.challenge;
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
        let worker_count = plain_http_accept_worker_count(&self.bootstrap_config);
        if cfg!(target_os = "linux") && self.bootstrap_config.runtime.performance.enabled {
            let _ = dedicated_http_connection_runtimes();
        }
        let mut workers = JoinSet::new();
        for worker_index in 0..worker_count {
            let listener = bind_tcp_listener(bind_addr, "plain http listener").await?;
            tracing::info!(
                bind = %bind_addr,
                worker = worker_index,
                workers = worker_count,
                "plain http listener ready"
            );
            let gateway = self.clone();
            if cfg!(target_os = "linux") && self.bootstrap_config.runtime.performance.enabled {
                // Keep accept and connection I/O on the same reactor. Moving
                // every accepted fd through into_std/from_std made high-rate
                // TLS/WebSocket connection ramps pay two registrations and a
                // global cross-runtime queue hop per socket.
                let listener = listener
                    .into_std()
                    .context("failed detaching plain HTTP listener for data runtime")?;
                let accept_task =
                    dedicated_http_connection_runtime(worker_index).spawn(async move {
                        let listener = TcpListener::from_std(listener)
                            .context("failed registering plain HTTP listener on data runtime")?;
                        gateway
                            .run_plain_http_accept_loop(listener, bind_addr, worker_index, true)
                            .await
                    });
                workers.spawn(async move {
                    accept_task
                        .await
                        .context("plain HTTP data-runtime accept task failed")?
                });
            } else {
                workers.spawn(async move {
                    gateway
                        .run_plain_http_accept_loop(listener, bind_addr, worker_index, false)
                        .await
                });
            }
        }

        while let Some(result) = workers.join_next().await {
            result
                .context("plain http accept worker task failed")?
                .context("plain http accept worker stopped")?;
        }

        Ok(())
    }

    async fn run_plain_http_accept_loop(
        self: Arc<Self>,
        listener: TcpListener,
        _bind_addr: SocketAddr,
        worker_index: usize,
        accept_on_data_runtime: bool,
    ) -> Result<()> {
        loop {
            let (stream, remote_addr) = listener
                .accept()
                .await
                .context("plain http accept failed")?;
            tune_tcp_stream_for_latency(&stream);
            if accept_on_data_runtime {
                if let Err(error) = stream.set_nodelay(true) {
                    tracing::debug!(?error, %remote_addr, "failed setting TCP_NODELAY on plain http connection");
                }
                let gateway = self.clone();
                std::mem::drop(tokio::spawn(async move {
                    gateway
                        .serve_plain_http_connection(
                            stream,
                            remote_addr,
                            worker_index,
                            Bytes::new(),
                        )
                        .await;
                }));
                continue;
            }
            let stream = match stream.into_std() {
                Ok(stream) => stream,
                Err(error) => {
                    tracing::warn!(?error, %remote_addr, "failed detaching plain HTTP socket for worker shard");
                    continue;
                }
            };
            if let Err(error) = stream.set_nodelay(true) {
                tracing::debug!(?error, %remote_addr, "failed setting TCP_NODELAY on plain http connection");
            }
            let gateway = self.clone();
            let performance_enabled = self.bootstrap_config.runtime.performance.enabled;

            spawn_http_connection(performance_enabled, worker_index, async move {
                let stream = match TcpStream::from_std(stream) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, worker = worker_index, "failed registering plain HTTP socket on worker shard");
                        return;
                    }
                };
                gateway
                    .serve_plain_http_connection(stream, remote_addr, worker_index, Bytes::new())
                    .await;
            });
        }
    }

    async fn serve_plain_http_connection(
        self: Arc<Self>,
        stream: TcpStream,
        remote_addr: SocketAddr,
        worker_index: usize,
        initial_prefix: Bytes,
    ) {
        let _active_plain_http = ActivePlainHttpConnectionGuard::enter();
        let (stream, prefix) = if self.plain_http_data_fast_lane_enabled().await {
            match self
                .try_plain_http_large_static_fast_path(stream, remote_addr, initial_prefix)
                .await
            {
                Ok(PlainHttpFastLaneAttempt::Served) => return,
                Ok(PlainHttpFastLaneAttempt::Fallback { stream, prefix }) => (stream, prefix),
                Err(error) => {
                    tracing::debug!(?error, %remote_addr, "plain http data fast path failed");
                    return;
                }
            }
        } else {
            (stream, initial_prefix)
        };

        self.serve_plain_hyper_connection(stream, prefix, remote_addr, worker_index)
            .await;
    }

    async fn serve_plain_hyper_connection(
        self: Arc<Self>,
        stream: TcpStream,
        prefix: Bytes,
        remote_addr: SocketAddr,
        worker_index: usize,
    ) {
        let gateway = self.clone();
        let service = service_fn(move |request| {
            let gateway = gateway.clone();
            async move {
                gateway
                    .handle_hyper_request(request, remote_addr, "http")
                    .await
            }
        });

        let result = optimized_http_server_builder()
            .serve_connection_with_upgrades(TokioIo::new(PrefixedIo::new(stream, prefix)), service)
            .await;

        if let Err(error) = result {
            tracing::warn!(?error, %remote_addr, worker = worker_index, "plain http connection failed");
        }
    }

    async fn try_plain_http_large_static_fast_path(
        &self,
        mut stream: TcpStream,
        remote_addr: SocketAddr,
        initial_prefix: Bytes,
    ) -> Result<PlainHttpFastLaneAttempt> {
        let dynamic_state;
        let (config, fast_lane) = if self.bootstrap_config.runtime.hot_reload.enabled
            || self.bootstrap_config.admin.enabled
        {
            dynamic_state = self.current_state().await;
            (&dynamic_state.config, &dynamic_state.fast_lane)
        } else {
            (&self.bootstrap_config, &self.bootstrap_fast_lane)
        };
        if !fast_lane.plain_http_static_sendfile
            && !fast_lane.raw_sse_proxy
            && !fast_lane.raw_reverse_proxy
            && !fast_lane.raw_websocket_proxy
        {
            tracing::debug!(%remote_addr, "plain http static fast path disabled by config");
            return Ok(PlainHttpFastLaneAttempt::Fallback {
                stream,
                prefix: initial_prefix,
            });
        }
        let yield_after_sendfile_response = config.runtime.performance.enabled
            && matches!(
                config.runtime.performance.traffic_profile,
                RuntimePerformanceTrafficProfile::Balanced
            );
        let mut served_any = false;
        let mut served_since_yield = 0_usize;
        let mut prefix = BytesMut::with_capacity(4096.max(initial_prefix.len()));
        prefix.extend_from_slice(&initial_prefix);
        let mut static_header = String::with_capacity(160);
        let mut small_static_response = Vec::with_capacity(4096);
        let mut static_response_cache: Option<ConnectionStaticFastPathCache> = None;
        let mut raw_reverse_upstream: Option<RawReverseLaneUpstream> = None;
        let mut raw_reverse_request_cache: Option<RawReverseParsedRequestCache> = None;
        let mut raw_reverse_response_cache: Option<RawReverseResponseCache> = None;
        let mut raw_reverse_upstream_response_buffer = Vec::with_capacity(4096);
        let mut balanced_sendfile_response_sequence =
            balanced_sendfile_response_sequence_seed(remote_addr);
        let outcome = 'fast_lane: loop {
            let head_end = read_fast_lane_http_prefix(&mut stream, &mut prefix)
                .await
                .context("failed reading plain http fast-lane request")?;
            if prefix.is_empty() {
                break if served_any {
                    PlainHttpFastLaneDecision::Served
                } else {
                    PlainHttpFastLaneDecision::Fallback(prefix.freeze())
                };
            }
            let Some(head_end) = head_end else {
                tracing::debug!(%remote_addr, bytes = prefix.len(), "plain http fast path header incomplete");
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            };
            let request_head = &prefix[..head_end];
            let leftover = &prefix[head_end..];
            if let Some(cached) = static_response_cache
                .as_ref()
                .filter(|cached| cached.raw_request_matches(request_head))
            {
                // reqwest, browsers, and CDN probes commonly repeat the exact
                // same keep-alive GET bytes. Once validated, skip UTF-8/header
                // parsing and route lookup until the revalidation deadline.
                let force_yield = yield_after_sendfile_response && cached.sendfile.is_some();
                let mid_yield = balanced_sendfile_mid_yield_for_next_response(
                    &mut balanced_sendfile_response_sequence,
                    force_yield,
                );
                send_connection_static_fast_path(&mut stream, cached, mid_yield).await?;
                served_any = true;
                discard_fast_lane_http_head(&mut prefix, head_end);
                served_since_yield = served_since_yield.saturating_add(1);
                if plain_fast_lane_should_yield(served_since_yield) {
                    served_since_yield = 0;
                    tokio::task::yield_now().await;
                }
                continue;
            }
            let Some(path_hint) = peek_static_fast_path_path(request_head) else {
                tracing::debug!(%remote_addr, bytes = request_head.len(), "plain http static fast path request-line miss");
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            };
            let raw_reverse_exact_head_hit = raw_reverse_request_cache
                .as_ref()
                .is_some_and(|cached| cached.request_head.as_ref() == request_head);
            let static_prefix_hit = !raw_reverse_exact_head_hit
                && config
                    .services
                    .static_sites
                    .iter()
                    .any(|site| static_site_path_matches(site, path_hint));
            let raw_sse_prefix_hit = !raw_reverse_exact_head_hit
                && fast_lane.raw_sse_proxy
                && ai_proxy_fast_lane_path_matches(config, path_hint);
            let raw_reverse_prefix_hit = raw_reverse_exact_head_hit
                || (fast_lane.raw_reverse_proxy
                    && reverse_proxy_fast_lane_path_matches(config, path_hint));
            let raw_websocket_prefix_hit = !raw_reverse_exact_head_hit
                && fast_lane.raw_websocket_proxy
                && websocket_fast_lane_path_matches(config, path_hint);
            if !static_prefix_hit
                && !raw_sse_prefix_hit
                && !raw_reverse_prefix_hit
                && !raw_websocket_prefix_hit
            {
                tracing::debug!(%remote_addr, path = %path_hint, "plain http static fast path prefix miss");
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            }

            if !static_prefix_hit
                && (raw_sse_prefix_hit || raw_reverse_prefix_hit || raw_websocket_prefix_hit)
            {
                if raw_websocket_prefix_hit {
                    let Some(request) = parse_plain_websocket_fast_lane_request(request_head)
                    else {
                        tracing::debug!(%remote_addr, bytes = request_head.len(), "plain http websocket fast lane parse miss");
                        break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                    };
                    let mut downstream_started = false;
                    let mut downstream_detached = false;
                    #[cfg(target_os = "linux")]
                    let plain_downstream_fd = stream.as_raw_fd();
                    if self
                        .try_serve_raw_websocket_fast_lane(
                            config,
                            &mut stream,
                            &request,
                            RawWebSocketFastLaneOptions {
                                remote_addr,
                                scheme: "http",
                                downstream_leftover: leftover,
                                downstream_started: &mut downstream_started,
                                downstream_detached: &mut downstream_detached,
                                #[cfg(target_os = "linux")]
                                plain_downstream_fd: Some(plain_downstream_fd),
                            },
                        )
                        .await?
                    {
                        break 'fast_lane if downstream_detached {
                            PlainHttpFastLaneDecision::Detached
                        } else {
                            PlainHttpFastLaneDecision::Served
                        };
                    }
                    break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                }
                let parsed_request;
                let reverse_request_cache_hit = raw_reverse_exact_head_hit;
                let cached_upstream_request = reverse_request_cache_hit.then(|| {
                    raw_reverse_request_cache
                        .as_ref()
                        .expect("raw reverse request cache hit checked")
                        .upstream_request
                        .clone()
                });
                let cached_prepared_route = reverse_request_cache_hit.then(|| {
                    raw_reverse_request_cache
                        .as_ref()
                        .expect("raw reverse request cache hit checked")
                        .prepared_route
                        .clone()
                });
                let request = if reverse_request_cache_hit {
                    &raw_reverse_request_cache
                        .as_ref()
                        .expect("raw reverse request cache hit checked")
                        .request
                } else {
                    let Some(request) = parse_plain_fast_lane_request(request_head) else {
                        tracing::debug!(%remote_addr, bytes = request_head.len(), "plain http raw fast lane parse miss");
                        break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                    };
                    parsed_request = request;
                    &parsed_request
                };
                if raw_sse_prefix_hit && plain_raw_sse_fast_lane_matches(config, request) {
                    // SSE owns the HTTP connection for the lifetime of the
                    // stream. Preserve an unusual pipelined request by handing
                    // the untouched bytes to Hyper instead of dropping it.
                    if !leftover.is_empty() {
                        break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                    }
                    if self
                        .try_serve_plain_raw_sse_fast_lane(
                            config,
                            &mut stream,
                            request,
                            remote_addr,
                        )
                        .await?
                    {
                        break 'fast_lane PlainHttpFastLaneDecision::Served;
                    }
                    break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                }
                if raw_reverse_prefix_hit {
                    let mut serialized_upstream_request = None;
                    let mut prepared_route = None;
                    if self
                        .try_serve_plain_raw_reverse_fast_lane(
                            config,
                            &mut stream,
                            request,
                            RawReverseFastLaneOptions {
                                remote_addr,
                                cached_upstream_request: cached_upstream_request.as_ref(),
                                cached_prepared_route,
                                serialized_upstream_request: &mut serialized_upstream_request,
                                prepared_route: &mut prepared_route,
                                upstream_response_buffer: &mut raw_reverse_upstream_response_buffer,
                                response_cache: &mut raw_reverse_response_cache,
                                lane_upstream: &mut raw_reverse_upstream,
                            },
                        )
                        .await?
                    {
                        served_any = true;
                        let keep_alive = request.keep_alive;
                        let request_to_cache =
                            (!reverse_request_cache_hit).then(|| request.clone());
                        if !keep_alive {
                            break 'fast_lane PlainHttpFastLaneDecision::Served;
                        }
                        if let Some(request) = request_to_cache {
                            raw_reverse_request_cache = Some(RawReverseParsedRequestCache {
                                request_head: Bytes::copy_from_slice(request_head),
                                request,
                                upstream_request: serialized_upstream_request.expect(
                                    "raw reverse cache miss serialized a successful upstream request",
                                ),
                                prepared_route: prepared_route.expect(
                                    "raw reverse cache miss prepared a successful route",
                                ),
                            });
                        }
                        discard_fast_lane_http_head(&mut prefix, head_end);
                        // A raw reverse request already crosses upstream and
                        // downstream readiness points. Reset the amortized
                        // static-lane counter instead of adding a redundant
                        // cooperative yield on every 32nd response.
                        served_since_yield = 0;
                        continue;
                    }
                    break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
                }
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            }

            let Some(request) = parse_static_fast_path_request(request_head) else {
                tracing::debug!(%remote_addr, bytes = request_head.len(), "plain http static fast path parse miss");
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            };
            if let Some(cached) = static_response_cache
                .as_ref()
                .filter(|cached| cached.matches(&request))
            {
                let force_yield = yield_after_sendfile_response && cached.sendfile.is_some();
                let mid_yield = balanced_sendfile_mid_yield_for_next_response(
                    &mut balanced_sendfile_response_sequence,
                    force_yield,
                );
                send_connection_static_fast_path(&mut stream, cached, mid_yield).await?;
                served_any = true;
                discard_fast_lane_http_head(&mut prefix, head_end);
                served_since_yield = served_since_yield.saturating_add(1);
                if plain_fast_lane_should_yield(served_since_yield) {
                    served_since_yield = 0;
                    tokio::task::yield_now().await;
                }
                continue;
            }
            let cached_candidate = self
                .static_route_cache
                .get(request.path)
                .map(|target| target.clone())
                .and_then(|target| {
                    stale_cached_static_file_candidate(
                        &target,
                        request.method,
                        &self.static_file_cache,
                        static_sendfile_fast_path_threshold_bytes(config),
                    )
                    .map(|(candidate, revalidate)| {
                        if revalidate {
                            self.spawn_static_cache_revalidation(target);
                        }
                        candidate
                    })
                });
            let candidate = if let Some(candidate) = cached_candidate {
                Some(candidate)
            } else {
                resolve_large_static_fast_path_candidate(
                    config,
                    &request,
                    &self.static_route_cache,
                    &self.static_file_cache,
                    &self.static_file_cache_bytes,
                    &self.static_file_load_locks,
                )
                .await?
            };
            let Some(candidate) = candidate else {
                tracing::debug!(%remote_addr, path = %request.path, "plain http static fast path candidate miss");
                break PlainHttpFastLaneDecision::Fallback(prefix.freeze());
            };
            if let Some(cached) = static_response_cache.as_mut().filter(|cached| {
                cached.identity_matches(&request) && cached.same_payload(&candidate)
            }) {
                // Revalidation confirmed the same immutable Bytes/file Arc.
                // Keep the already-serialized response instead of making all
                // keep-alive connections rebuild it on the same TTL boundary.
                cached.checked_at = Instant::now();
                let force_yield = yield_after_sendfile_response && cached.sendfile.is_some();
                let mid_yield = balanced_sendfile_mid_yield_for_next_response(
                    &mut balanced_sendfile_response_sequence,
                    force_yield,
                );
                send_connection_static_fast_path(&mut stream, cached, mid_yield).await?;
                served_any = true;
                discard_fast_lane_http_head(&mut prefix, head_end);
                served_since_yield = served_since_yield.saturating_add(1);
                if plain_fast_lane_should_yield(served_since_yield) {
                    served_since_yield = 0;
                    tokio::task::yield_now().await;
                }
                continue;
            }

            let connection = if request.keep_alive {
                "keep-alive"
            } else {
                "close"
            };
            static_header.clear();
            std::fmt::Write::write_fmt(
                &mut static_header,
                format_args!(
                    "HTTP/1.1 200 OK\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: {}\r\n\r\n",
                    candidate.content_type, candidate.len, connection
                ),
            )
            .expect("writing static response header to String cannot fail");

            if request.method == "GET"
                && request.keep_alive
                && (candidate.cached_body.is_some() || candidate.sendfile.is_some())
            {
                let combined_response = candidate.cached_body.as_ref().and_then(|body| {
                    (static_header.len() + body.len() <= POOL_BUFFER_BYTES).then(|| {
                        small_static_response.clear();
                        small_static_response.reserve(static_header.len() + body.len());
                        small_static_response.extend_from_slice(static_header.as_bytes());
                        small_static_response.extend_from_slice(body);
                        Bytes::copy_from_slice(&small_static_response)
                    })
                });
                let cached = ConnectionStaticFastPathCache {
                    request_head: Bytes::copy_from_slice(request_head),
                    target: request.target.to_string(),
                    host: request.host.map(str::to_string),
                    checked_at: Instant::now(),
                    header: Bytes::copy_from_slice(static_header.as_bytes()),
                    combined_response,
                    body: candidate.cached_body.clone(),
                    file_path: candidate.path.clone(),
                    len: candidate.len,
                    sendfile: candidate.sendfile.clone(),
                };
                let force_yield = yield_after_sendfile_response && cached.sendfile.is_some();
                let mid_yield = balanced_sendfile_mid_yield_for_next_response(
                    &mut balanced_sendfile_response_sequence,
                    force_yield,
                );
                send_connection_static_fast_path(&mut stream, &cached, mid_yield).await?;
                static_response_cache = Some(cached);
                served_any = true;
                discard_fast_lane_http_head(&mut prefix, head_end);
                served_since_yield = served_since_yield.saturating_add(1);
                if plain_fast_lane_should_yield(served_since_yield) {
                    served_since_yield = 0;
                    tokio::task::yield_now().await;
                }
                continue;
            }

            // Fast path for small cached static bodies: copy head + body into a
            // single pooled buffer and issue ONE write syscall. This matches
            // nginx's single-segment small-file delivery and avoids the extra
            // syscalls of TCP_CORK on/off plus a separate header/body write.
            let small_single_write = request.method == "GET"
                && candidate.cached_body.is_some()
                && static_header.len() + candidate.len as usize <= POOL_BUFFER_BYTES;

            let send_result = if small_single_write {
                let body = candidate
                    .cached_body
                    .as_ref()
                    .expect("cached body present for single-write static");
                small_static_response.clear();
                small_static_response.reserve(static_header.len() + body.len());
                small_static_response.extend_from_slice(static_header.as_bytes());
                small_static_response.extend_from_slice(body);
                stream
                    .write_all(&small_static_response)
                    .await
                    .context("failed writing combined static fast path response")
            } else {
                // Coalesce head + body into a single TCP segment (nginx
                // `tcp_nopush`/TCP_CORK) for sendfile and larger cached bodies.
                #[cfg(target_os = "linux")]
                let cork_static = request.method == "GET" && candidate.len > 0;
                #[cfg(target_os = "linux")]
                if cork_static {
                    set_tcp_cork(&stream, true);
                }

                let header_result = stream
                    .write_all(static_header.as_bytes())
                    .await
                    .context("failed writing plain http fast path response head");

                let body_result =
                    if header_result.is_ok() && request.method == "GET" && candidate.len > 0 {
                        if let Some(body) = candidate.cached_body.as_ref() {
                            stream
                                .write_all(body)
                                .await
                                .context("failed writing cached static fast path body")
                        } else {
                            send_static_file_fast(
                                &mut stream,
                                &candidate.path,
                                candidate.len,
                                candidate.sendfile.clone(),
                                false,
                            )
                            .await
                            .map(|_| ())
                        }
                    } else {
                        Ok(())
                    };
                #[cfg(target_os = "linux")]
                if cork_static {
                    set_tcp_cork(&stream, false);
                }
                header_result.and(body_result)
            };
            send_result?;

            if config.logging.access_log {
                tracing::info!(
                    target: "access",
                    method = %request.method,
                    path = %request.path,
                    status = 200,
                    upstream = "proxysss://static-sendfile",
                    remote_addr = %remote_addr,
                    "access"
                );
            }

            served_any = true;
            if !request.keep_alive {
                break PlainHttpFastLaneDecision::Served;
            }
            discard_fast_lane_http_head(&mut prefix, head_end);
            served_since_yield = served_since_yield.saturating_add(1);
            if plain_fast_lane_should_yield(served_since_yield) {
                served_since_yield = 0;
                tokio::task::yield_now().await;
            }
        };

        if let Some(upstream) = raw_reverse_upstream.take() {
            upstream.pool.checkin(upstream.stream);
        }
        Ok(match outcome {
            PlainHttpFastLaneDecision::Served => {
                let _ = stream.shutdown().await;
                PlainHttpFastLaneAttempt::Served
            }
            PlainHttpFastLaneDecision::Detached => PlainHttpFastLaneAttempt::Served,
            PlainHttpFastLaneDecision::Fallback(prefix) => {
                PlainHttpFastLaneAttempt::Fallback { stream, prefix }
            }
        })
    }

    async fn try_tls_raw_websocket_fast_lane<Stream>(
        &self,
        downstream: &mut Stream,
        remote_addr: SocketAddr,
        prefix: Bytes,
    ) -> Result<TlsRawWebSocketAttempt>
    where
        Stream: AsyncRead + AsyncWrite + Unpin + ?Sized,
    {
        let dynamic_state;
        let (config, fast_lane) = if self.bootstrap_config.runtime.hot_reload.enabled
            || self.bootstrap_config.admin.enabled
        {
            dynamic_state = self.current_state().await;
            (&dynamic_state.config, &dynamic_state.fast_lane)
        } else {
            (&self.bootstrap_config, &self.bootstrap_fast_lane)
        };
        if !fast_lane.raw_websocket_proxy {
            return Ok(TlsRawWebSocketAttempt::Fallback(prefix));
        }

        let Some(head_end) = memmem::find(&prefix, b"\r\n\r\n").map(|index| index + 4) else {
            return Ok(TlsRawWebSocketAttempt::Fallback(prefix));
        };
        let Some(request) = parse_plain_websocket_fast_lane_request(&prefix[..head_end]) else {
            return Ok(TlsRawWebSocketAttempt::Fallback(prefix));
        };

        let mut downstream_started = false;
        let mut downstream_detached = false;
        match self
            .try_serve_raw_websocket_fast_lane(
                config,
                downstream,
                &request,
                RawWebSocketFastLaneOptions {
                    remote_addr,
                    scheme: "https",
                    downstream_leftover: &prefix[head_end..],
                    downstream_started: &mut downstream_started,
                    downstream_detached: &mut downstream_detached,
                    #[cfg(target_os = "linux")]
                    plain_downstream_fd: None,
                },
            )
            .await
        {
            Ok(true) => Ok(TlsRawWebSocketAttempt::Served),
            Ok(false) => Ok(TlsRawWebSocketAttempt::Fallback(prefix)),
            Err(error) if !downstream_started => {
                tracing::debug!(?error, %remote_addr, "TLS raw WebSocket fast lane fell back before response");
                Ok(TlsRawWebSocketAttempt::Fallback(prefix))
            }
            Err(error) => Err(error.context("TLS raw WebSocket fast lane failed after response")),
        }
    }

    async fn try_tls_small_static_fast_lane(
        &self,
        downstream: &mut tokio_rustls::server::TlsStream<TcpStream>,
        remote_addr: SocketAddr,
        initial_prefix: Bytes,
    ) -> Result<TlsStaticFastLaneAttempt> {
        let dynamic_state;
        let (config, fast_lane) = if self.bootstrap_config.runtime.hot_reload.enabled
            || self.bootstrap_config.admin.enabled
        {
            dynamic_state = self.current_state().await;
            (&dynamic_state.config, &dynamic_state.fast_lane)
        } else {
            (&self.bootstrap_config, &self.bootstrap_fast_lane)
        };
        if !fast_lane.hyper_static_success {
            return Ok(TlsStaticFastLaneAttempt::Fallback(initial_prefix));
        }

        let mut prefix = BytesMut::with_capacity(4096.max(initial_prefix.len()));
        prefix.extend_from_slice(&initial_prefix);
        let mut header = String::with_capacity(160);
        let mut response = Vec::with_capacity(4096);
        let mut served = 0_usize;
        loop {
            let head_end = read_fast_lane_http_prefix(downstream, &mut prefix)
                .await
                .context("failed reading TLS static fast-lane request")?;
            if prefix.is_empty() {
                return Ok(if served > 0 {
                    TlsStaticFastLaneAttempt::Served
                } else {
                    TlsStaticFastLaneAttempt::Fallback(Bytes::new())
                });
            }
            let Some(head_end) = head_end else {
                return Ok(TlsStaticFastLaneAttempt::Fallback(prefix.freeze()));
            };
            let Some(request) = parse_static_fast_path_request(&prefix[..head_end]) else {
                return Ok(TlsStaticFastLaneAttempt::Fallback(prefix.freeze()));
            };
            let Some(target) = self
                .static_route_cache
                .get(request.path)
                .map(|target| target.clone())
            else {
                return Ok(TlsStaticFastLaneAttempt::Fallback(prefix.freeze()));
            };
            let Some((candidate, revalidate)) = stale_cached_static_file_candidate(
                &target,
                request.method,
                &self.static_file_cache,
                u64::MAX,
            ) else {
                return Ok(TlsStaticFastLaneAttempt::Fallback(prefix.freeze()));
            };
            let Some(body) = candidate.cached_body.as_ref() else {
                return Ok(TlsStaticFastLaneAttempt::Fallback(prefix.freeze()));
            };
            if revalidate {
                self.spawn_static_cache_revalidation(target);
            }

            let connection = if request.keep_alive {
                "keep-alive"
            } else {
                "close"
            };
            let keep_alive = request.keep_alive;
            header.clear();
            std::fmt::Write::write_fmt(
                &mut header,
                format_args!(
                    "HTTP/1.1 200 OK\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: {}\r\n\r\n",
                    candidate.content_type, candidate.len, connection
                ),
            )
            .expect("writing TLS static response header cannot fail");
            response.clear();
            response.extend_from_slice(header.as_bytes());
            if request.method == "GET" {
                response.extend_from_slice(body);
            }
            downstream
                .write_all(&response)
                .await
                .context("failed writing TLS static fast-lane response")?;
            self.stats.http_requests.fetch_add(1, Ordering::Relaxed);
            if config.logging.access_log {
                tracing::info!(
                    target: "access",
                    method = %request.method,
                    path = %request.path,
                    status = 200,
                    upstream = "proxysss://tls-static-cache",
                    remote_addr = %remote_addr,
                    "access"
                );
            }

            discard_fast_lane_http_head(&mut prefix, head_end);
            served = served.saturating_add(1);
            if !keep_alive {
                let _ = downstream.shutdown().await;
                return Ok(TlsStaticFastLaneAttempt::Served);
            }
            if served.is_multiple_of(PLAIN_FAST_LANE_FAIRNESS_BATCH) {
                tokio::task::yield_now().await;
            }
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
        let base_worker_count = plain_http_accept_worker_count(&self.bootstrap_config);
        let max_worker_count =
            if cfg!(target_os = "linux") && self.bootstrap_config.runtime.performance.enabled {
                adaptive_data_plane_workers(1)
            } else {
                1
            };
        if cfg!(target_os = "linux") && self.bootstrap_config.runtime.performance.enabled {
            let _ = dedicated_tls_connection_runtimes();
        }
        let active_connections = Arc::new(AtomicUsize::new(0));
        let mut workers = JoinSet::new();
        for worker_index in 0..base_worker_count {
            let listener = bind_tcp_listener(bind_addr, "tls http listener").await?;
            tracing::info!(
                bind = %bind_addr,
                worker = worker_index,
                workers = base_worker_count,
                max_workers = max_worker_count,
                "tls http listener ready"
            );
            let gateway = self.clone();
            let acceptor = tls_acceptor.clone();
            let active_connections = active_connections.clone();
            if cfg!(target_os = "linux") && self.bootstrap_config.runtime.performance.enabled {
                let listener = listener
                    .into_std()
                    .context("failed detaching TLS HTTP listener for data runtime")?;
                let accept_task =
                    dedicated_tls_connection_runtime(worker_index).spawn(async move {
                        let listener = TcpListener::from_std(listener)
                            .context("failed registering TLS HTTP listener on data runtime")?;
                        gateway
                            .run_tls_http_accept_loop(
                                listener,
                                acceptor,
                                worker_index,
                                true,
                                active_connections,
                            )
                            .await
                    });
                workers.spawn(async move {
                    accept_task
                        .await
                        .context("TLS HTTP data-runtime accept task failed")?
                });
            } else {
                workers.spawn(async move {
                    gateway
                        .run_tls_http_accept_loop(
                            listener,
                            acceptor,
                            worker_index,
                            false,
                            active_connections,
                        )
                        .await
                });
            }
        }

        let elastic_threshold = base_worker_count
            .saturating_mul(TLS_ELASTIC_CONNECTIONS_PER_BASE_SHARD)
            .max(TLS_ELASTIC_CONNECTIONS_PER_BASE_SHARD);
        for worker_index in base_worker_count..max_worker_count {
            let gateway = self.clone();
            let acceptor = tls_acceptor.clone();
            let active_connections = active_connections.clone();
            workers.spawn(async move {
                while active_connections.load(Ordering::Relaxed) < elastic_threshold {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                let listener = bind_tcp_listener(bind_addr, "elastic tls http listener").await?;
                tracing::info!(
                    bind = %bind_addr,
                    worker = worker_index,
                    threshold = elastic_threshold,
                    "elastic TLS HTTP listener activated"
                );
                let listener = listener
                    .into_std()
                    .context("failed detaching elastic TLS listener for data runtime")?;
                let accept_task = dedicated_tls_connection_runtime(worker_index).spawn({
                    let active_connections = active_connections.clone();
                    async move {
                        let listener = TcpListener::from_std(listener)
                            .context("failed registering elastic TLS listener")?;
                        gateway
                            .run_tls_http_accept_loop(
                                listener,
                                acceptor,
                                worker_index,
                                true,
                                active_connections,
                            )
                            .await
                    }
                });
                accept_task
                    .await
                    .context("elastic TLS data-runtime accept task failed")?
            });
        }

        while let Some(result) = workers.join_next().await {
            result
                .context("tls http accept worker task failed")?
                .context("tls http accept worker stopped")?;
        }

        Ok(())
    }

    async fn run_tls_http_accept_loop(
        self: Arc<Self>,
        listener: TcpListener,
        tls_acceptor: TlsAcceptor,
        worker_index: usize,
        accept_on_data_runtime: bool,
        active_connections: Arc<AtomicUsize>,
    ) -> Result<()> {
        let mut accepted_since_yield = 0_usize;
        loop {
            let (stream, remote_addr) =
                listener.accept().await.context("tls http accept failed")?;
            if accept_on_data_runtime {
                if let Err(error) = stream.set_nodelay(true) {
                    tracing::debug!(?error, %remote_addr, "failed setting TCP_NODELAY on tls http connection");
                }
                tune_tcp_stream_for_latency(&stream);
                let gateway = self.clone();
                let acceptor = tls_acceptor.clone();
                let active_connection_count = active_connections
                    .fetch_add(1, Ordering::Relaxed)
                    .saturating_add(1);
                let connection_guard = ActiveConnectionGuard(active_connections.clone());
                std::mem::drop(tokio::spawn(async move {
                    let _connection_guard = connection_guard;
                    gateway
                        .serve_tls_http_connection(stream, acceptor, remote_addr, worker_index)
                        .await;
                }));
                accepted_since_yield += 1;
                let handshake_batch = if active_connection_count > TLS_ACCEPT_HIGH_DENSITY_PER_SHARD
                {
                    TLS_ACCEPT_HIGH_DENSITY_BATCH
                } else {
                    TLS_ACCEPT_LOW_DENSITY_BATCH
                };
                if accepted_since_yield >= handshake_batch {
                    accepted_since_yield = 0;
                    tokio::task::yield_now().await;
                }
                continue;
            }
            let stream = match stream.into_std() {
                Ok(stream) => stream,
                Err(error) => {
                    tracing::warn!(?error, %remote_addr, "failed detaching TLS HTTP socket for worker shard");
                    continue;
                }
            };
            if let Err(error) = stream.set_nodelay(true) {
                tracing::debug!(?error, %remote_addr, "failed setting TCP_NODELAY on tls http connection");
            }
            let gateway = self.clone();
            let acceptor = tls_acceptor.clone();
            let performance_enabled = self.bootstrap_config.runtime.performance.enabled;

            spawn_http_connection(performance_enabled, worker_index, async move {
                let stream = match TcpStream::from_std(stream) {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, worker = worker_index, "failed registering TLS HTTP socket on worker shard");
                        return;
                    }
                };
                tune_tcp_stream_for_latency(&stream);
                gateway
                    .serve_tls_http_connection(stream, acceptor, remote_addr, worker_index)
                    .await;
            });
        }
    }

    async fn serve_tls_http_connection(
        self: Arc<Self>,
        stream: TcpStream,
        acceptor: TlsAcceptor,
        remote_addr: SocketAddr,
        worker_index: usize,
    ) {
        let mut tls_stream = match acceptor.accept(stream).await {
            Ok(stream) => stream,
            Err(error) => {
                tracing::warn!(?error, %remote_addr, "tls handshake failed");
                return;
            }
        };

        let is_http2 = tls_stream.get_ref().1.alpn_protocol() == Some(b"h2");
        let prefix = if is_http2 {
            Bytes::new()
        } else {
            match read_tls_fast_lane_http_prefix(&mut tls_stream).await {
                Ok(prefix) => prefix,
                Err(error) => {
                    tracing::debug!(?error, %remote_addr, "failed reading TLS HTTP request head");
                    return;
                }
            }
        };

        self.serve_tls_http_after_handshake(
            tls_stream,
            prefix,
            remote_addr,
            worker_index,
            !is_http2,
            is_http2,
        )
        .await;
    }

    async fn serve_tls_http_after_handshake(
        self: Arc<Self>,
        mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        prefix: Bytes,
        remote_addr: SocketAddr,
        worker_index: usize,
        try_websocket: bool,
        is_http2: bool,
    ) {
        let prefix = if try_websocket {
            match self
                .try_tls_raw_websocket_fast_lane(&mut tls_stream, remote_addr, prefix)
                .await
            {
                Ok(TlsRawWebSocketAttempt::Served) => return,
                Ok(TlsRawWebSocketAttempt::Fallback(prefix)) => prefix,
                Err(error) => {
                    if is_expected_websocket_disconnect(&error) {
                        tracing::debug!(?error, %remote_addr, "TLS raw WebSocket tunnel closed by peer");
                    } else {
                        tracing::warn!(?error, %remote_addr, "TLS raw WebSocket tunnel failed");
                    }
                    return;
                }
            }
        } else {
            prefix
        };

        let prefix = if !is_http2 {
            match self
                .try_tls_small_static_fast_lane(&mut tls_stream, remote_addr, prefix)
                .await
            {
                Ok(TlsStaticFastLaneAttempt::Served) => return,
                Ok(TlsStaticFastLaneAttempt::Fallback(prefix)) => prefix,
                Err(error) => {
                    tracing::debug!(?error, %remote_addr, "TLS static fast lane failed");
                    return;
                }
            }
        } else {
            prefix
        };

        let gateway = self.clone();
        let service = service_fn(move |request| {
            let gateway = gateway.clone();
            async move {
                gateway
                    .handle_hyper_request(request, remote_addr, "https")
                    .await
            }
        });

        let io = TokioIo::new(PrefixedIo::new(tls_stream, prefix));
        let result = if is_http2 {
            optimized_http2_server_builder()
                .serve_connection(io, service)
                .await
                .map_err(|error| anyhow!("HTTP/2 connection failed: {error}"))
        } else {
            optimized_http_server_builder()
                .serve_connection_with_upgrades(io, service)
                .await
                .map_err(|error| anyhow!("TLS HTTP connection failed: {error}"))
        };

        if let Err(error) = result {
            tracing::warn!(?error, %remote_addr, worker = worker_index, "tls http connection failed");
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

                            let mut response = match gateway
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

                            if let Some(mut body) = response.stream_body.take() {
                                while let Some(frame) = body.frame().await {
                                    match frame {
                                        Ok(frame) => {
                                            if let Some(data) = frame.data_ref() {
                                                if !data.is_empty() {
                                                    if let Err(error) =
                                                        stream.send_data(data.clone()).await
                                                    {
                                                        tracing::warn!(?error, %remote_addr, "failed sending h3 response stream");
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        Err(error) => {
                                            tracing::warn!(?error, %remote_addr, "failed reading h3 response stream");
                                            break;
                                        }
                                    }
                                }
                            } else if !response.body.is_empty() {
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
        let state = self.current_state().await;
        let worker_count = tcp_stream_accept_worker_count(&state.config, &listener_config);
        let sharded_runtime_enabled =
            cfg!(target_os = "linux") && state.config.runtime.performance.enabled;
        if sharded_runtime_enabled {
            let _ = dedicated_tcp_stream_runtimes();
        }
        let mut workers = JoinSet::new();
        for worker_index in 0..worker_count {
            let listener = match bind_tcp_listener(bind_addr, "tcp listener").await {
                Ok(listener) => listener,
                Err(error) if worker_index > 0 => {
                    tracing::warn!(
                        ?error,
                        listener = %listener_config.name,
                        bind = %bind_addr,
                        worker = worker_index,
                        started_workers = worker_index,
                        "tcp reuseport accept worker bind failed; continuing with started workers"
                    );
                    break;
                }
                Err(error) => return Err(error),
            };
            tracing::info!(
                listener = %listener_config.name,
                bind = %bind_addr,
                worker = worker_index,
                workers = worker_count,
                "tcp listener ready"
            );
            let gateway = self.clone();
            let listener_config = listener_config.clone();
            if sharded_runtime_enabled {
                let listener = listener
                    .into_std()
                    .context("failed detaching TCP listener for stream runtime shard")?;
                let accept_task = dedicated_tcp_stream_runtime(worker_index).spawn(async move {
                    let listener = TcpListener::from_std(listener)
                        .context("failed registering TCP listener on stream runtime shard")?;
                    gateway
                        .run_tcp_listener_accept_loop(listener, listener_config, worker_index)
                        .await
                });
                workers.spawn(async move {
                    accept_task
                        .await
                        .context("TCP stream runtime accept task failed")?
                });
            } else {
                workers.spawn(async move {
                    gateway
                        .run_tcp_listener_accept_loop(listener, listener_config, worker_index)
                        .await
                });
            }
        }

        while let Some(result) = workers.join_next().await {
            result
                .context("tcp accept worker task failed")?
                .context("tcp accept worker stopped")?;
        }

        Ok(())
    }

    async fn run_tcp_listener_accept_loop(
        self: Arc<Self>,
        listener: TcpListener,
        listener_config: TcpListenerConfig,
        worker_index: usize,
    ) -> Result<()> {
        loop {
            let (inbound, remote_addr) = listener.accept().await.context("tcp accept failed")?;
            let connection_worker = worker_index;
            if listener_config.nodelay {
                inbound
                    .set_nodelay(true)
                    .with_context(|| format!("failed setting TCP_NODELAY for {remote_addr}"))?;
            }
            tune_tcp_stream_for_latency(&inbound);
            let gateway = self.clone();
            let listener_name = listener_config.name.clone();
            let listener_bind = listener_config.bind.clone();
            let listener_default_upstream = listener_config.upstream.clone();
            let listener_protocol = listener_config.protocol.clone();
            let listener_nodelay = listener_config.nodelay;
            let listener_connect_timeout_ms = listener_config.connect_timeout_ms;
            let accept_state = self.current_state().await;
            if !apply_stream_rate_limit(
                &self.stream_rate_limits,
                &accept_state.config.services.rate_limit.stream,
                remote_addr,
            ) {
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
            if self.is_stream_connection_blocked(&accept_state.config, remote_addr) {
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

            let session = async move {
                let mut inbound = inbound;
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

                    if state.script.is_none() {
                        if let Some(upstream) =
                            direct_tcp_listener_upstream(&state.config, &listener_name)
                        {
                            relay_direct_tcp_fast_path(
                                inbound,
                                DirectTcpFastPath {
                                    upstream,
                                    listener_name: &listener_name,
                                    protocol: &listener_protocol,
                                    nodelay: listener_nodelay,
                                    connect_timeout_ms: listener_connect_timeout_ms,
                                    first_payload: BytesMut::new(),
                                    remote_addr,
                                    worker_index: connection_worker,
                                },
                            )
                            .await?;
                            return Ok(());
                        }
                    }

                    let needs_first_payload = listener_name.starts_with("stream|")
                        || state.config.affinity.enabled
                        || state.script.is_some();
                    let stream_cfg = state.config.affinity.stream.clone();
                    let mut first_payload = BytesMut::new();

                    if needs_first_payload && stream_cfg.peek_bytes > 0 {
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

                    let player_id = if state.config.affinity.enabled {
                        extract_stream_player_id(&first_payload, &stream_cfg)
                    } else {
                        None
                    };
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

                    if state.script.is_none() && listener_name.starts_with("stream|") {
                        if let Some(upstream) = direct_tcp_route_upstream(&state.config, &route) {
                            relay_direct_tcp_fast_path(
                                inbound,
                                DirectTcpFastPath {
                                    upstream,
                                    listener_name: &listener_name,
                                    protocol: &listener_protocol,
                                    nodelay: listener_nodelay,
                                    connect_timeout_ms: listener_connect_timeout_ms,
                                    first_payload,
                                    remote_addr,
                                    worker_index: connection_worker,
                                },
                            )
                            .await?;
                            return Ok(());
                        }
                    }

                    let remote_addr_for_affinity = if player_id.is_none()
                        && state.config.affinity.enabled
                        && state.config.affinity.fallback_to_remote_addr
                    {
                        Some(remote_addr.to_string())
                    } else {
                        None
                    };
                    let upstream_plan = gateway.select_upstream_plan(
                        &state.config,
                        &route,
                        "tcp",
                        Some(&listener_name),
                        player_id.as_deref(),
                        remote_addr_for_affinity.as_deref(),
                    );
                    let max_attempts = if state.config.load_balance.retries.enabled {
                        (state.config.load_balance.retries.max_retries as usize)
                            .saturating_add(1)
                            .min(upstream_plan.len().max(1))
                    } else {
                        1
                    };

                    let mut selected: Option<(TcpStream, Option<UpstreamLease>, String)> = None;
                    let mut last_error: Option<anyhow::Error> = None;

                    for upstream in upstream_plan.iter().take(max_attempts) {
                        let track_runtime = gateway.should_track_upstream_runtime(
                            &state.config,
                            "tcp",
                            Some(&listener_name),
                            upstream,
                        );
                        let lease = if track_runtime {
                            Some(gateway.acquire_upstream_lease(
                                "tcp",
                                Some(&listener_name),
                                upstream,
                            ))
                        } else {
                            None
                        };
                        match tokio::time::timeout(
                            Duration::from_millis(listener_connect_timeout_ms.max(1)),
                            TcpStream::connect(upstream),
                        )
                        .await
                        {
                            Ok(Ok(stream)) => {
                                if listener_nodelay {
                                    stream.set_nodelay(true).with_context(|| {
                                        format!("failed setting TCP_NODELAY for upstream {upstream}")
                                    })?;
                                }
                                tune_tcp_stream_for_latency(&stream);
                                if !listener_protocol.trim().is_empty() {
                                    tracing::debug!(
                                        protocol = %listener_protocol,
                                        listener = %listener_name,
                                        upstream = %upstream,
                                        %remote_addr,
                                        "tcp protocol hint selected"
                                    );
                                }
                                if track_runtime {
                                    gateway.on_upstream_success(
                                        "tcp",
                                        Some(&listener_name),
                                        upstream,
                                    );
                                }
                                selected = Some((stream, lease, upstream.clone()));
                                break;
                            }
                            Ok(Err(error)) => {
                                if track_runtime {
                                    gateway.on_upstream_failure(
                                        &state.config,
                                        "tcp",
                                        Some(&listener_name),
                                        upstream,
                                    );
                                }
                                last_error = Some(anyhow!(
                                    "failed to connect tcp upstream {upstream}: {error}"
                                ));
                            }
                            Err(_) => {
                                if track_runtime {
                                    gateway.on_upstream_failure(
                                        &state.config,
                                        "tcp",
                                        Some(&listener_name),
                                        upstream,
                                    );
                                }
                                last_error = Some(anyhow!(
                                    "timed out connecting tcp upstream {upstream} after {listener_connect_timeout_ms}ms"
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

                    copy_tcp_bidirectional_adaptive(
                        inbound,
                        outbound,
                        state.config.runtime.performance.enabled,
                        tcp_relay_profile(&listener_protocol, first_payload.len()),
                    )
                    .await
                    .context("tcp proxy copy failed")?;

                    if gateway.should_track_upstream_runtime(
                        &state.config,
                        "tcp",
                        Some(&listener_name),
                        &upstream,
                    ) {
                        gateway.on_upstream_success("tcp", Some(&listener_name), &upstream);
                    }

                    Ok::<_, anyhow::Error>(())
                }
                .await
                {
                    tracing::warn!(?error, request_id, listener = %listener_name, worker = connection_worker, %remote_addr, "tcp session failed");
                }

                gateway
                    .stats
                    .tcp_sessions_active
                    .fetch_sub(1, Ordering::Relaxed);
            };
            std::mem::drop(tokio::spawn(session));
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
        let associations = Arc::new(DashMap::<SocketAddr, Arc<UdpAssociation>>::new());
        let pending_sessions = Arc::new(DashSet::<SocketAddr>::new());
        let prune_state = Arc::new(UdpPruneState::new());
        let state = self.current_state().await;
        let worker_count = udp_listener_worker_count(&state.config);
        let weighted_runtime =
            cfg!(target_os = "linux") && state.config.runtime.performance.enabled;
        if weighted_runtime {
            initialize_udp_connection_runtimes();
        }
        let mut workers = JoinSet::new();
        for worker_index in 0..worker_count {
            let listener_socket = match bind_udp_listener_socket(bind_addr, "udp listener").await {
                Ok(socket) => socket,
                Err(error) if worker_index > 0 => {
                    tracing::warn!(
                        ?error,
                        listener = %listener_config.name,
                        bind = %bind_addr,
                        worker = worker_index,
                        started_workers = worker_index,
                        "udp reuseport worker bind failed; continuing with started workers"
                    );
                    break;
                }
                Err(error) => return Err(error),
            };
            tracing::info!(
                listener = %listener_config.name,
                bind = %bind_addr,
                worker = worker_index,
                workers = worker_count,
                "udp listener ready"
            );

            let gateway = self.clone();
            let listener_config = listener_config.clone();
            let associations = associations.clone();
            let pending_sessions = pending_sessions.clone();
            let prune_state = prune_state.clone();
            if weighted_runtime {
                let listener_socket = listener_socket
                    .into_std()
                    .context("failed detaching UDP listener for weighted runtime")?;
                let recv_task = dedicated_udp_connection_runtime(worker_index).spawn(async move {
                    let listener_socket = Arc::new(
                        UdpSocket::from_std(listener_socket)
                            .context("failed registering UDP listener on weighted runtime")?,
                    );
                    gateway
                        .run_udp_listener_recv_loop(
                            listener_socket,
                            listener_config,
                            associations,
                            prune_state,
                            worker_index,
                            pending_sessions,
                        )
                        .await
                });
                workers.spawn(async move {
                    recv_task
                        .await
                        .context("UDP weighted-runtime receive task failed")?
                });
            } else {
                let listener_socket = Arc::new(listener_socket);
                workers.spawn(async move {
                    gateway
                        .run_udp_listener_recv_loop(
                            listener_socket,
                            listener_config,
                            associations,
                            prune_state,
                            worker_index,
                            pending_sessions,
                        )
                        .await
                });
            }
        }

        while let Some(result) = workers.join_next().await {
            result
                .context("udp listener worker task failed")?
                .context("udp listener worker stopped")?;
        }

        Ok(())
    }

    async fn run_udp_listener_recv_loop(
        self: Arc<Self>,
        listener_socket: Arc<UdpSocket>,
        listener_config: UdpListenerConfig,
        associations: Arc<DashMap<SocketAddr, Arc<UdpAssociation>>>,
        prune_state: Arc<UdpPruneState>,
        worker_index: usize,
        pending_sessions: Arc<DashSet<SocketAddr>>,
    ) -> Result<()> {
        let session_ttl_secs = listener_config.session_ttl_secs.max(1);
        let max_associations = listener_config.max_associations;
        let protocol_hint = listener_config.protocol.clone();
        let mut local_associations = FxHashMap::<SocketAddr, LocalUdpAssociation>::default();
        let mut pending_udp_packets = 0_u64;
        let mut pending_udp_bytes = 0_u64;
        let mut cached_now_secs = now_unix_secs();
        let mut cached_now_refreshed = Instant::now();
        let local_prune_interval_secs = session_ttl_secs.clamp(1, 30);
        let mut next_local_prune_epoch = cached_now_secs.saturating_add(local_prune_interval_secs);
        let mut direct_udp_cache = DirectUdpRouteCache::new();
        // A policy-free listener's upstream identity only changes after a
        // control-plane reload. Do not acquire the dynamic config RwLock for
        // every game datagram; refresh the worker-local snapshot once a second
        // together with the association clock.
        let mut direct_udp_state = self.current_state().await;
        let mut direct_udp_state_refreshed = Instant::now();

        let mut buffer = udp_buffer_pool().acquire();
        loop {
            let (received, client_addr) = listener_socket
                .recv_from(&mut buffer)
                .await
                .context("udp recv failed")?;
            pending_udp_packets = pending_udp_packets.saturating_add(1);
            pending_udp_bytes = pending_udp_bytes.saturating_add(received as u64);
            if pending_udp_packets >= UDP_STATS_FLUSH_PACKETS {
                flush_udp_stats(
                    &self.stats,
                    &mut pending_udp_packets,
                    &mut pending_udp_bytes,
                );
            }

            if cached_now_refreshed.elapsed() >= Duration::from_secs(1) {
                cached_now_secs = now_unix_secs();
                cached_now_refreshed = Instant::now();
            }
            if direct_udp_state_refreshed.elapsed() >= Duration::from_secs(1) {
                direct_udp_state = self.current_state().await;
                direct_udp_state_refreshed = Instant::now();
            }
            let now = cached_now_secs;
            if now >= next_local_prune_epoch {
                local_associations.retain(|_, local| {
                    let keep = local.association.active.load(Ordering::Relaxed)
                        && now.saturating_sub(local.last_seen_epoch) <= session_ttl_secs;
                    if !keep {
                        local.association.active.store(false, Ordering::Relaxed);
                    }
                    keep
                });
                next_local_prune_epoch = now.saturating_add(local_prune_interval_secs);
            }
            let mut existing_association = None;
            let mut remove_local_association = false;
            if let Some(local) = local_associations.get_mut(&client_addr) {
                if local.association.active.load(Ordering::Relaxed)
                    && now.saturating_sub(local.last_seen_epoch) <= session_ttl_secs
                {
                    if local.last_seen_epoch != now {
                        local.last_seen_epoch = now;
                        local
                            .association
                            .last_seen_epoch
                            .store(now, Ordering::Relaxed);
                    }
                    existing_association = Some(local.association.clone());
                } else {
                    local.association.active.store(false, Ordering::Relaxed);
                    remove_local_association = true;
                }
            }
            if remove_local_association {
                local_associations.remove(&client_addr);
                associations.remove(&client_addr);
            }
            if existing_association.is_none() {
                if let Some(existing) = associations.get(&client_addr) {
                    let association = existing.clone();
                    drop(existing);
                    if udp_association_is_live(&association, session_ttl_secs, now) {
                        association.last_seen_epoch.store(now, Ordering::Relaxed);
                        local_associations.insert(
                            client_addr,
                            LocalUdpAssociation {
                                association: association.clone(),
                                last_seen_epoch: now,
                            },
                        );
                        existing_association = Some(association);
                    } else {
                        association.active.store(false, Ordering::Relaxed);
                        associations.remove(&client_addr);
                    }
                }
            }

            if let Some(existing) = existing_association {
                let upstream_socket = existing.socket.clone();
                if let Err(error) = send_udp_connected(&upstream_socket, &buffer[..received]).await
                {
                    tracing::warn!(
                        ?error,
                        %client_addr,
                        listener = %listener_config.name,
                        worker = worker_index,
                        "failed forwarding udp payload to existing upstream association"
                    );
                    existing.active.store(false, Ordering::Relaxed);
                    local_associations.remove(&client_addr);
                    associations.remove(&client_addr);
                }
                continue;
            }

            let gateway = self.clone();
            let listener_name = listener_config.name.clone();
            let listener_socket = listener_socket.clone();
            let associations = associations.clone();
            let prune_state = prune_state.clone();
            let protocol_hint = protocol_hint.clone();
            if let Some(upstream_addr) =
                direct_udp_cache.get_or_refresh(direct_udp_state.clone(), &listener_config.name)
            {
                if !pending_sessions.insert(client_addr) {
                    continue;
                }
                let _pending_guard = PendingUdpSessionGuard {
                    sessions: pending_sessions.clone(),
                    addr: client_addr,
                };

                match Self::create_direct_udp_upstream_association(
                    DirectUdpAssociationBuildContext {
                        gateway: &self,
                        listener_name: &listener_name,
                        listener_socket: &listener_socket,
                        associations: &associations,
                        prune_state: &prune_state,
                        upstream_addr,
                        client_addr,
                        session_ttl_secs,
                        max_associations,
                    },
                )
                .await
                {
                    Ok(association) => {
                        if let Err(error) =
                            send_udp_connected(&association.socket, &buffer[..received]).await
                        {
                            tracing::warn!(
                                ?error,
                                %client_addr,
                                listener = %listener_config.name,
                                worker = worker_index,
                                "failed forwarding udp payload to direct upstream association"
                            );
                            association.active.store(false, Ordering::Relaxed);
                            associations.remove(&client_addr);
                        } else {
                            local_associations.insert(
                                client_addr,
                                LocalUdpAssociation {
                                    association,
                                    last_seen_epoch: now,
                                },
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            ?error,
                            listener = %listener_name,
                            worker = worker_index,
                            %client_addr,
                            "direct udp session failed"
                        );
                    }
                }
                continue;
            }
            let payload = Bytes::copy_from_slice(&buffer[..received]);
            if !pending_sessions.insert(client_addr) {
                continue;
            }

            let pending_sessions = pending_sessions.clone();
            tokio::spawn(async move {
                let _pending_guard = PendingUdpSessionGuard {
                    sessions: pending_sessions,
                    addr: client_addr,
                };
                let request_id = Uuid::new_v4().to_string();

                if let Err(error) = async {
                    let state = gateway.current_state().await;

                    let upstream_socket = if let Some(existing) = associations.get(&client_addr) {
                        let association = existing.clone();
                        drop(existing);
                        let now = now_unix_secs();
                        if udp_association_is_live(&association, session_ttl_secs, now) {
                            association.last_seen_epoch.store(now, Ordering::Relaxed);
                            association.socket.clone()
                        } else {
                            association.active.store(false, Ordering::Relaxed);
                            associations.remove(&client_addr);
                            Self::create_udp_upstream_association(UdpAssociationBuildContext {
                                gateway: &gateway,
                                state: &state,
                                listener_name: &listener_name,
                                listener_socket: &listener_socket,
                                associations: &associations,
                                prune_state: &prune_state,
                                protocol_hint: &protocol_hint,
                                client_addr,
                                payload: &payload,
                                request_id: &request_id,
                                session_ttl_secs,
                                max_associations,
                            })
                            .await?
                        }
                    } else {
                        Self::create_udp_upstream_association(UdpAssociationBuildContext {
                            gateway: &gateway,
                            state: &state,
                            listener_name: &listener_name,
                            listener_socket: &listener_socket,
                            associations: &associations,
                            prune_state: &prune_state,
                            protocol_hint: &protocol_hint,
                            client_addr,
                            payload: &payload,
                            request_id: &request_id,
                            session_ttl_secs,
                            max_associations,
                        })
                        .await?
                    };

                    send_udp_connected(&upstream_socket, &payload)
                        .await
                        .context("failed forwarding udp payload to upstream")?;

                    Ok::<_, anyhow::Error>(())
                }
                .await
                {
                    tracing::warn!(?error, request_id, listener = %listener_name, worker = worker_index, %client_addr, "udp session failed");
                }
            });
        }
    }

    async fn create_direct_udp_upstream_association(
        ctx: DirectUdpAssociationBuildContext<'_>,
    ) -> Result<Arc<UdpAssociation>> {
        let DirectUdpAssociationBuildContext {
            gateway,
            listener_name,
            listener_socket,
            associations,
            prune_state,
            upstream_addr,
            client_addr,
            session_ttl_secs,
            max_associations,
        } = ctx;

        maybe_prune_udp_associations(
            associations,
            prune_state,
            session_ttl_secs,
            max_associations,
        );
        if max_associations > 0 && associations.len() >= max_associations {
            gateway
                .stats
                .blocked_requests_total
                .fetch_add(1, Ordering::Relaxed);
            return Err(anyhow!(
                "udp listener {} reached max_associations {}",
                listener_name,
                max_associations
            ));
        }

        let bind_any = if upstream_addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = Arc::new(
            UdpSocket::bind(bind_any)
                .await
                .context("failed to bind direct udp upstream socket")?,
        );
        socket
            .connect(upstream_addr)
            .await
            .with_context(|| format!("failed to connect direct udp upstream {upstream_addr}"))?;
        tune_udp_socket_for_gateway(&socket);

        let upstream = upstream_addr.to_string();
        let lease = gateway.acquire_upstream_lease("udp", Some(listener_name), &upstream);
        let association = Arc::new(UdpAssociation {
            socket: socket.clone(),
            last_seen_epoch: AtomicU64::new(now_unix_secs()),
            active: AtomicBool::new(true),
        });
        associations.insert(client_addr, association.clone());
        spawn_udp_association_reader(
            listener_socket.clone(),
            socket,
            associations.clone(),
            association.clone(),
            client_addr,
            session_ttl_secs,
            lease,
        );

        Ok(association)
    }

    async fn create_udp_upstream_association(
        ctx: UdpAssociationBuildContext<'_>,
    ) -> Result<Arc<UdpSocket>> {
        let UdpAssociationBuildContext {
            gateway,
            state,
            listener_name,
            listener_socket,
            associations,
            prune_state,
            protocol_hint,
            client_addr,
            payload,
            request_id,
            session_ttl_secs,
            max_associations,
        } = ctx;

        maybe_prune_udp_associations(
            associations,
            prune_state,
            session_ttl_secs,
            max_associations,
        );
        if max_associations > 0 && associations.len() >= max_associations {
            gateway
                .stats
                .blocked_requests_total
                .fetch_add(1, Ordering::Relaxed);
            return Err(anyhow!(
                "udp listener {} reached max_associations {}",
                listener_name,
                max_associations
            ));
        }
        let player_id = extract_stream_player_id(payload, &state.config.affinity.stream);
        let route = if let Some(route) =
            configured_udp_listener_route(&state.config, listener_name, player_id.clone())
        {
            route
        } else if let Some(script) = &state.script {
            script
                .route_udp(StreamContext {
                    request_id: request_id.to_string(),
                    listener: listener_name.to_string(),
                    protocol: "udp".to_string(),
                    remote_addr: client_addr.to_string(),
                    player_id: player_id.clone(),
                    first_packet_preview: Some(first_packet_preview(payload)),
                    payload_len: payload.len(),
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
                "udp listener {} has no configured upstream and script runtime is disabled",
                listener_name
            ));
        };

        let remote_addr_for_affinity = if player_id.is_none()
            && state.config.affinity.enabled
            && state.config.affinity.fallback_to_remote_addr
        {
            Some(client_addr.to_string())
        } else {
            None
        };
        let upstream_plan = gateway.select_upstream_plan(
            &state.config,
            &route,
            "udp",
            Some(listener_name),
            player_id.as_deref(),
            remote_addr_for_affinity.as_deref(),
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
                    gateway.on_upstream_failure(
                        &state.config,
                        "udp",
                        Some(listener_name),
                        upstream,
                    );
                    last_error = Some(anyhow!("invalid udp upstream {upstream}: {error}"));
                    continue;
                }
            };

            let bind_any = if upstream_addr.is_ipv4() {
                "0.0.0.0:0"
            } else {
                "[::]:0"
            };
            let socket = match UdpSocket::bind(bind_any).await {
                Ok(value) => Arc::new(value),
                Err(error) => {
                    gateway.on_upstream_failure(
                        &state.config,
                        "udp",
                        Some(listener_name),
                        upstream,
                    );
                    last_error = Some(anyhow!("failed to bind udp upstream socket: {error}"));
                    continue;
                }
            };

            match socket.connect(upstream_addr).await {
                Ok(()) => {
                    gateway.on_upstream_success("udp", Some(listener_name), upstream);
                    tune_udp_socket_for_gateway(&socket);
                    let lease =
                        gateway.acquire_upstream_lease("udp", Some(listener_name), upstream);
                    selected = Some((socket, lease));
                    break;
                }
                Err(error) => {
                    gateway.on_upstream_failure(
                        &state.config,
                        "udp",
                        Some(listener_name),
                        upstream,
                    );
                    last_error = Some(anyhow!(
                        "failed to connect udp upstream {upstream}: {error}"
                    ));
                }
            }
        }

        let (socket, lease) = selected.ok_or_else(|| {
            last_error.unwrap_or_else(|| anyhow!("failed to connect any udp upstream"))
        })?;

        if !protocol_hint.trim().is_empty() {
            tracing::debug!(
                protocol = %protocol_hint,
                listener = %listener_name,
                %client_addr,
                "udp protocol hint selected"
            );
        }
        let association = Arc::new(UdpAssociation {
            socket: socket.clone(),
            last_seen_epoch: AtomicU64::new(now_unix_secs()),
            active: AtomicBool::new(true),
        });
        associations.insert(client_addr, association.clone());
        spawn_udp_association_reader(
            listener_socket.clone(),
            socket.clone(),
            associations.clone(),
            association,
            client_addr,
            session_ttl_secs,
            lease,
        );

        Ok(socket)
    }

    async fn handle_hyper_request(
        self: Arc<Self>,
        mut request: Request<Incoming>,
        remote_addr: SocketAddr,
        scheme: &'static str,
    ) -> Result<GatewayResponse, Infallible> {
        self.stats.http_requests.fetch_add(1, Ordering::Relaxed);

        match self
            .try_http_static_success_fast_path(&request, scheme)
            .await
        {
            Ok(Some(response)) => return Ok(response),
            Ok(None) => {}
            Err(error) => {
                self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?error, %remote_addr, scheme, "http static fast path failed");
                return Ok(
                    GatewayHttpResponse::error(StatusCode::BAD_GATEWAY, error.to_string())
                        .into_hyper(),
                );
            }
        }

        let started = Instant::now();
        let method = request.method().clone();
        let uri = request.uri().clone();
        let version = version_label(request.version());
        match self
            .try_hyper_simple_http_proxy_fast_path(
                &method,
                &uri,
                request.headers(),
                remote_addr,
                scheme,
                version,
            )
            .await
        {
            Ok(Some(HyperFastPathResponse::Direct(response))) => {
                return Ok(response);
            }
            Ok(Some(HyperFastPathResponse::Gateway(response))) => {
                let response = self
                    .finish_hyper_http_response(&request, response, remote_addr, started)
                    .await;
                return Ok(response.into_hyper());
            }
            Ok(None) => {}
            Err(error) => {
                self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?error, %remote_addr, scheme, "http proxy fast path failed");
                return Ok(
                    GatewayHttpResponse::error(StatusCode::BAD_GATEWAY, error.to_string())
                        .into_hyper(),
                );
            }
        }

        let on_upgrade = websocket_upgrade_requested(request.headers())
            .then(|| hyper::upgrade::on(&mut request));
        let headers = request.headers().clone();
        let body = match collect_request_body_if_needed(&method, &headers, request.body_mut()).await
        {
            Ok(body) => body,
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

        let response = self
            .finish_hyper_http_response(&request, response, remote_addr, started)
            .await;
        Ok(response.into_hyper())
    }

    async fn finish_hyper_http_response(
        &self,
        request: &Request<Incoming>,
        response: GatewayHttpResponse,
        remote_addr: SocketAddr,
        started: Instant,
    ) -> GatewayHttpResponse {
        let elapsed = started.elapsed();
        if response.status.is_server_error() {
            self.stats.http_errors.fetch_add(1, Ordering::Relaxed);
        }

        let mut response = response;
        if response.status.is_client_error() || response.status.is_server_error() {
            let state = self.current_state().await;
            response = decorate_error_response(&state.config, request.headers(), response);
            write_access_log_if_enabled(&state.config, request, &response, remote_addr, elapsed);
        } else if self.bootstrap_config.logging.access_log
            || self.bootstrap_config.runtime.hot_reload.enabled
        {
            let state = self.current_state().await;
            write_access_log_if_enabled(&state.config, request, &response, remote_addr, elapsed);
        }

        response
    }

    async fn try_hyper_simple_http_proxy_fast_path(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
    ) -> Result<Option<HyperFastPathResponse>> {
        if !request_body_declared_empty(method, headers)
            || method.as_str() == "PURGE"
            || websocket_upgrade_requested(headers)
        {
            return Ok(None);
        }

        let state = self.current_state().await;
        if !state.fast_lane.simple_http_proxy {
            return Ok(None);
        }

        if !security::request_uri_is_safe(uri) {
            return Ok(Some(HyperFastPathResponse::Gateway(
                GatewayHttpResponse::bytes(
                    StatusCode::BAD_REQUEST,
                    "text/plain; charset=utf-8",
                    Bytes::from_static(b"invalid request path"),
                    "proxysss://security",
                ),
            )));
        }

        if let Some(status) =
            security::reject_ambiguous_http1_request(headers, &state.config.security)
        {
            return Ok(Some(HyperFastPathResponse::Gateway(
                GatewayHttpResponse::bytes(
                    status,
                    "text/plain; charset=utf-8",
                    Bytes::from_static(b"ambiguous http/1 request"),
                    "proxysss://security",
                ),
            )));
        }

        let host = request_host(headers, uri);
        let empty_body = Bytes::new();

        if let Some(route) = state
            .config
            .services
            .ai_proxy
            .enabled
            .then_some(&state.config.services.ai_proxy.routes)
            .into_iter()
            .flatten()
            .filter(|route| crate::ai_proxy::route_matches(route, &host, uri.path()))
            .max_by_key(|route| route.path_prefix.len())
        {
            if !ai_proxy_route_fast_path_eligible(route) {
                return Ok(None);
            }

            let rewrite_path = ai_proxy_rewrite_path(route, uri);
            if state.fast_lane.raw_sse_proxy && request_accepts_sse(headers) {
                return self
                    .dispatch_raw_sse_upstream_http(
                        &state.config,
                        method,
                        uri,
                        headers,
                        &empty_body,
                        remote_addr,
                        scheme,
                        &host,
                        &route.upstream,
                        rewrite_path.as_deref(),
                    )
                    .await
                    .map(|response| Some(HyperFastPathResponse::Gateway(response)));
            }

            return self
                .dispatch_simple_upstream_http(
                    &state,
                    method,
                    uri,
                    headers,
                    &empty_body,
                    remote_addr,
                    scheme,
                    version,
                    &host,
                    &route.upstream,
                    rewrite_path.as_deref(),
                )
                .await
                .map(|response| Some(HyperFastPathResponse::Gateway(response)));
        }

        if let Some(route) = state
            .config
            .services
            .reverse_proxy
            .routes
            .iter()
            .filter(|route| reverse_proxy_route_matches(route, &host, uri.path()))
            .max_by_key(|route| route.path_prefix.len())
        {
            if !reverse_proxy_route_fast_path_eligible(route) {
                return Ok(None);
            }

            let rewrite_path = reverse_proxy_rewrite_path(route, uri);
            return self
                .dispatch_simple_upstream_http_fast_response(
                    &state,
                    method,
                    uri,
                    headers,
                    &empty_body,
                    remote_addr,
                    scheme,
                    version,
                    &host,
                    &route.upstream,
                    rewrite_path.as_deref(),
                )
                .await
                .map(Some);
        }

        Ok(None)
    }

    async fn try_http_static_success_fast_path(
        &self,
        request: &Request<Incoming>,
        scheme: &'static str,
    ) -> Result<Option<GatewayResponse>> {
        let method = request.method();
        if method != Method::GET && method != Method::HEAD {
            return Ok(None);
        }

        // HTTP/2 cannot carry HTTP/1 Upgrade/Transfer-Encoding ambiguity. For
        // an already-ended GET stream, take the immutable precompiled static
        // lane before any HTTP/1-oriented header scans.
        if request.version() == Version::HTTP_2
            && method == Method::GET
            && request.body().is_end_stream()
            && !request.headers().contains_key(RANGE)
            && !self.bootstrap_config.runtime.hot_reload.enabled
            && !self.bootstrap_config.admin.enabled
            && self.bootstrap_fast_lane.hyper_static_success
            && !monitoring_path_matches(&self.bootstrap_config.monitoring, request.uri().path())
        {
            if let Some(target) = self.static_route_cache.get(request.uri().path()) {
                if let Some(cached) = cached_static_file_response_stale_while_revalidate(
                    target.as_path(),
                    method,
                    &self.static_file_cache,
                ) {
                    if cached.revalidate {
                        self.spawn_static_cache_revalidation(target.clone());
                    }
                    return Ok(Some(cached.response));
                }
            }
        }

        if !request_body_declared_empty(method, request.headers()) {
            return Ok(None);
        }
        if websocket_upgrade_requested(request.headers()) {
            return Ok(None);
        }

        // Immutable startup config can take the precompiled H2/static lane
        // before repeated policy scans, path normalization, and site lookup.
        // An exact warmed route-cache hit is already a validated static path;
        // hot-reload/admin configurations use the complete path below.
        if !self.bootstrap_config.runtime.hot_reload.enabled
            && !self.bootstrap_config.admin.enabled
            && self.bootstrap_fast_lane.hyper_static_success
            && !request.headers().contains_key(RANGE)
            && !monitoring_path_matches(&self.bootstrap_config.monitoring, request.uri().path())
        {
            if let Some(target) = self.static_route_cache.get(request.uri().path()) {
                if let Some(cached) = cached_static_file_response_stale_while_revalidate(
                    target.as_path(),
                    method,
                    &self.static_file_cache,
                ) {
                    if cached.revalidate {
                        self.spawn_static_cache_revalidation(target.clone());
                    }
                    return Ok(Some(cached.response));
                }
            }
        }

        let dynamic_state;
        let config = if self.bootstrap_config.runtime.hot_reload.enabled
            || self.bootstrap_config.admin.enabled
        {
            dynamic_state = self.current_state().await;
            &dynamic_state.config
        } else {
            &self.bootstrap_config
        };
        if !http_static_success_fast_path_allowed(config, scheme, request.uri()) {
            return Ok(None);
        }
        if !security::request_uri_is_safe(request.uri()) {
            return Ok(None);
        }
        if security::reject_ambiguous_http1_request(request.headers(), &config.security).is_some() {
            return Ok(None);
        }

        let Some(site) = config
            .services
            .static_sites
            .iter()
            .find(|site| static_site_path_matches(site, request.uri().path()))
        else {
            return Ok(None);
        };

        // HTTP/2 and TLS cannot use the plain-text sendfile lane, but hot
        // static objects are already immutable `Bytes` in the bounded cache.
        // Resolve the path without touching the filesystem and serve a fresh
        // cache entry directly. Falling through once per revalidation window
        // preserves hot-update correctness while removing a metadata syscall
        // from every ordinary H2 request.
        if !request.headers().contains_key(RANGE) {
            if let Some(target) = self.static_route_cache.get(request.uri().path()) {
                if let Some(cached) = cached_static_file_response_stale_while_revalidate(
                    target.as_path(),
                    method,
                    &self.static_file_cache,
                ) {
                    if cached.revalidate {
                        self.spawn_static_cache_revalidation(target.clone());
                    }
                    return Ok(Some(cached.response));
                }
            }
            if let Some(target) = static_site_filesystem_path(site, request.uri().path())? {
                if let Some(cached) = cached_static_file_response_stale_while_revalidate(
                    &target,
                    method,
                    &self.static_file_cache,
                ) {
                    if cached.revalidate {
                        self.spawn_static_cache_revalidation(target);
                    }
                    return Ok(Some(cached.response));
                }
            }
        }

        let response = dispatch_static_site(
            site,
            method,
            request.uri(),
            request.headers(),
            &self.static_file_cache,
            &self.static_file_cache_bytes,
            &self.static_file_load_locks,
        )
        .await?;

        if response.status.is_success() {
            Ok(Some(response.into_hyper()))
        } else {
            Ok(None)
        }
    }

    fn spawn_static_cache_revalidation(&self, target: PathBuf) {
        let static_file_cache = self.static_file_cache.clone();
        let static_file_cache_bytes = self.static_file_cache_bytes.clone();
        let static_file_load_locks = self.static_file_load_locks.clone();
        std::mem::drop(tokio::spawn(async move {
            let key = target.to_string_lossy().to_string();
            match tokio::fs::metadata(&target).await {
                Ok(metadata) if metadata.is_file() => {
                    let sendfile_entry = static_file_cache
                        .get(&key)
                        .is_some_and(|entry| entry.sendfile.is_some() && entry.body.is_empty());
                    let result = if sendfile_entry {
                        cached_static_sendfile(&target, &metadata, &static_file_cache).map(|_| ())
                    } else {
                        cached_static_file_body(
                            &target,
                            &metadata,
                            &static_file_cache,
                            &static_file_cache_bytes,
                            &static_file_load_locks,
                        )
                        .await
                        .map(|_| ())
                    };
                    if let Err(error) = result {
                        tracing::debug!(?error, path = %target.display(), "background static cache revalidation failed");
                        finish_failed_static_revalidation(&key, &static_file_cache);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if let Some((_, stale)) = static_file_cache.remove(&key) {
                        static_file_cache_bytes
                            .fetch_sub(stale.body.len() as u64, Ordering::Relaxed);
                    }
                }
                Ok(_) => {
                    if let Some((_, stale)) = static_file_cache.remove(&key) {
                        static_file_cache_bytes
                            .fetch_sub(stale.body.len() as u64, Ordering::Relaxed);
                    }
                }
                Err(error) => {
                    tracing::debug!(?error, path = %target.display(), "background static metadata revalidation failed");
                    finish_failed_static_revalidation(&key, &static_file_cache);
                }
            }
        }));
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

        let host = request_host(&headers, &uri).into_owned();

        let script_route_available = state.script.is_some();
        let player_id = if state.config.affinity.enabled || script_route_available {
            extract_http_player_id(&uri, &headers, &state.config.affinity.http)
        } else {
            None
        };

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
                Err(response) => return Ok(*response),
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
                Err(response) => return Ok(*response),
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
                Err(response) => return Ok(*response),
            };
            let response = dispatch_static_site(
                site,
                &method,
                &uri,
                &headers,
                &self.static_file_cache,
                &self.static_file_cache_bytes,
                &self.static_file_load_locks,
            )
            .await?;
            return finalize_http_response(
                &headers,
                &state.config.services.response_policy.compression,
                response,
            );
        }

        if let Some(response) = self
            .try_simple_http_proxy_fast_path(
                &state,
                &method,
                &uri,
                &headers,
                &body,
                remote_addr,
                scheme,
                version,
                &host,
            )
            .await?
        {
            return Ok(response);
        }

        let route = if let Some(route) = configured_http_route(&state.config, &host, &uri) {
            route
        } else if let Some(script) = &state.script {
            let request_id = Uuid::new_v4().to_string();
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
                compression: Cow::Borrowed(&state.config.services.response_policy.compression),
                cache: Cow::Borrowed(&state.config.services.response_policy.cache),
                rate_limit: Cow::Borrowed(&state.config.services.rate_limit.http),
                forward_headers: true,
            }
        } else if let Some(route) = builtin_http_route(uri.path()) {
            HttpRouteConfig {
                runtime_scope: Some("builtin".to_string()),
                decision: route,
                compression: Cow::Borrowed(&state.config.services.response_policy.compression),
                cache: Cow::Borrowed(&state.config.services.response_policy.cache),
                rate_limit: Cow::Borrowed(&state.config.services.rate_limit.http),
                forward_headers: true,
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
                Err(response) => return Ok(*response),
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
                    route.forward_headers,
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
                    let route_for_refresh = route.to_owned_config();
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

        let affinity_key = route
            .decision
            .affinity_key
            .as_deref()
            .or(player_id.as_deref());
        let remote_addr_for_affinity = if affinity_key.is_none()
            && state.config.affinity.enabled
            && state.config.affinity.fallback_to_remote_addr
        {
            Some(remote_addr.to_string())
        } else {
            None
        };
        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route.decision,
            "http",
            route.runtime_scope.as_deref(),
            affinity_key,
            remote_addr_for_affinity.as_deref(),
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
            let upstream_target = build_upstream_http_target(upstream, &route.decision, &uri)?;
            let upstream_headers = build_upstream_headers(
                &headers,
                &route.decision,
                &host,
                remote_addr,
                scheme,
                route.forward_headers,
            )?;
            let track_runtime = self.should_track_upstream_runtime(
                &state.config,
                "http",
                route.runtime_scope.as_deref(),
                upstream,
            );
            let _lease = if track_runtime {
                Some(self.acquire_upstream_lease("http", route.runtime_scope.as_deref(), upstream))
            } else {
                None
            };

            let send_result = state
                .dispatch_upstream_http(
                    method.clone(),
                    upstream_target,
                    upstream_headers,
                    body.clone(),
                )
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
            let stream_upstream_body = should_stream_upstream_body(
                status,
                &upstream_response.headers,
                version,
                cache_key.as_deref(),
                &route.compression,
            );
            let mut response_headers = Vec::with_capacity(upstream_response.headers.len());
            response_headers.extend(
                upstream_response
                    .headers
                    .iter()
                    .filter(|(name, _)| !is_hop_header(name.as_str()))
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
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

            if track_runtime {
                self.on_upstream_success("http", route.runtime_scope.as_deref(), upstream);
            }
            let (body, stream_body) = if stream_upstream_body {
                (Bytes::new(), Some(upstream_response.into_stream_body()))
            } else {
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
    async fn try_simple_http_proxy_fast_path(
        &self,
        state: &Arc<DynamicState>,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
        host: &str,
    ) -> Result<Option<GatewayHttpResponse>> {
        let config = &state.config;
        if !state.fast_lane.simple_http_proxy
            || method.as_str() == "PURGE"
            || websocket_upgrade_requested(headers)
        {
            return Ok(None);
        }

        if let Some(route) = config
            .services
            .ai_proxy
            .enabled
            .then_some(&config.services.ai_proxy.routes)
            .into_iter()
            .flatten()
            .filter(|route| crate::ai_proxy::route_matches(route, host, uri.path()))
            .max_by_key(|route| route.path_prefix.len())
        {
            if ai_proxy_route_fast_path_eligible(route) {
                let rewrite_path = ai_proxy_rewrite_path(route, uri);
                if state.fast_lane.raw_sse_proxy && request_accepts_sse(headers) {
                    return self
                        .dispatch_raw_sse_upstream_http(
                            &state.config,
                            method,
                            uri,
                            headers,
                            body,
                            remote_addr,
                            scheme,
                            host,
                            &route.upstream,
                            rewrite_path.as_deref(),
                        )
                        .await
                        .map(Some);
                }
                return self
                    .dispatch_simple_upstream_http(
                        state,
                        method,
                        uri,
                        headers,
                        body,
                        remote_addr,
                        scheme,
                        version,
                        host,
                        &route.upstream,
                        rewrite_path.as_deref(),
                    )
                    .await
                    .map(Some);
            }
            return Ok(None);
        }

        if let Some(route) = config
            .services
            .reverse_proxy
            .routes
            .iter()
            .filter(|route| reverse_proxy_route_matches(route, host, uri.path()))
            .max_by_key(|route| route.path_prefix.len())
        {
            if reverse_proxy_route_fast_path_eligible(route) {
                let rewrite_path = reverse_proxy_rewrite_path(route, uri);
                return self
                    .dispatch_simple_upstream_http(
                        state,
                        method,
                        uri,
                        headers,
                        body,
                        remote_addr,
                        scheme,
                        version,
                        host,
                        &route.upstream,
                        rewrite_path.as_deref(),
                    )
                    .await
                    .map(Some);
            }
        }

        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_simple_upstream_http(
        &self,
        state: &DynamicState,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
        host: &str,
        upstream: &str,
        rewrite_path: Option<&str>,
    ) -> Result<GatewayHttpResponse> {
        let upstream_target = build_upstream_http_target_with_rewrite(upstream, rewrite_path, uri)?;
        let upstream_headers =
            build_simple_upstream_headers(headers, host, remote_addr, scheme, false)?;
        let upstream_response = state
            .dispatch_upstream_http(
                method.clone(),
                upstream_target,
                upstream_headers,
                body.clone(),
            )
            .await?;

        let status = upstream_response.status();
        let stream_upstream_body = should_stream_upstream_body(
            status,
            &upstream_response.headers,
            version,
            None,
            &state.config.services.response_policy.compression,
        );
        let response_headers = upstream_response
            .headers
            .iter()
            .filter(|(name, _)| !is_hop_header(name.as_str()))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<Vec<_>>();
        let (body, stream_body) = if stream_upstream_body {
            (Bytes::new(), Some(upstream_response.into_stream_body()))
        } else {
            (upstream_response.bytes().await?, None)
        };

        let response = GatewayHttpResponse {
            status,
            headers: response_headers,
            body,
            stream_body,
            upstream: upstream.to_string(),
        };
        finalize_http_response(
            headers,
            &state.config.services.response_policy.compression,
            response,
        )
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_simple_upstream_http_fast_response(
        &self,
        state: &DynamicState,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        version: &str,
        host: &str,
        upstream: &str,
        rewrite_path: Option<&str>,
    ) -> Result<HyperFastPathResponse> {
        let upstream_target = build_upstream_http_target_with_rewrite(upstream, rewrite_path, uri)?;
        let UpstreamHttpTarget::Hyper(upstream_uri) = upstream_target else {
            return self
                .dispatch_simple_upstream_http(
                    state,
                    method,
                    uri,
                    headers,
                    body,
                    remote_addr,
                    scheme,
                    version,
                    host,
                    upstream,
                    rewrite_path,
                )
                .await
                .map(HyperFastPathResponse::Gateway);
        };

        let upstream_headers =
            build_empty_simple_upstream_headers(headers, host, remote_addr, scheme, false)?;
        let retry_transport = body.is_empty() && matches!(*method, Method::GET | Method::HEAD);
        let mut attempt = 0_u8;
        let upstream_response = loop {
            let mut upstream_request = Request::builder()
                .method(method.clone())
                .uri(upstream_uri.clone())
                .body(Full::new(body.clone()))
                .context("failed building upstream request")?;
            *upstream_request.headers_mut() = upstream_headers.clone();

            match state.http_fast_client.request(upstream_request).await {
                Ok(response) => break response,
                Err(error) if retry_transport && attempt == 0 => {
                    attempt = 1;
                    tracing::debug!(?error, %upstream, "retrying idempotent upstream transport failure");
                    tokio::task::yield_now().await;
                }
                Err(error) => return Err(error).context("upstream request failed"),
            }
        };
        let (mut parts, upstream_body) = upstream_response.into_parts();
        remove_hop_headers_from_map(&mut parts.headers);

        let status = parts.status;
        let stream_upstream_body = should_stream_upstream_body(
            status,
            &parts.headers,
            version,
            None,
            &state.config.services.response_policy.compression,
        );

        if self.simple_hyper_response_can_return_direct(status, &parts.headers) {
            if upstream_response_is_sse(&parts.headers) {
                apply_streaming_response_headers_map(&mut parts.headers)?;
            }
            let response_body = if stream_upstream_body {
                GatewayBody::Stream(
                    upstream_body
                        .map_err(|error| anyhow!("upstream response stream failed: {error}"))
                        .boxed_unsync(),
                )
            } else {
                let body_bytes = upstream_body
                    .collect()
                    .await
                    .map(|collected| collected.to_bytes())
                    .context("failed reading upstream response body")?;
                full_body(body_bytes)
            };
            return Ok(HyperFastPathResponse::Direct(Response::from_parts(
                parts,
                response_body,
            )));
        }

        let response_headers = parts
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<Vec<_>>();
        let (body, stream_body) = if stream_upstream_body {
            (
                Bytes::new(),
                Some(GatewayBody::Stream(
                    upstream_body
                        .map_err(|error| anyhow!("upstream response stream failed: {error}"))
                        .boxed_unsync(),
                )),
            )
        } else {
            let body_bytes = upstream_body
                .collect()
                .await
                .map(|collected| collected.to_bytes())
                .context("failed reading upstream response body")?;
            (body_bytes, None)
        };

        let response = GatewayHttpResponse {
            status,
            headers: response_headers,
            body,
            stream_body,
            upstream: upstream.to_string(),
        };
        finalize_http_response(
            headers,
            &state.config.services.response_policy.compression,
            response,
        )
        .map(HyperFastPathResponse::Gateway)
    }

    fn simple_hyper_response_can_return_direct(
        &self,
        status: StatusCode,
        _headers: &HeaderMap,
    ) -> bool {
        status.is_success()
            && !self.bootstrap_config.logging.access_log
            && !self.bootstrap_config.runtime.hot_reload.enabled
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_raw_sse_upstream_http(
        &self,
        config: &GatewayConfig,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &Bytes,
        remote_addr: SocketAddr,
        scheme: &str,
        host: &str,
        upstream: &str,
        rewrite_path: Option<&str>,
    ) -> Result<GatewayHttpResponse> {
        let upstream_url = build_upstream_url_with_rewrite(upstream, rewrite_path, uri)?;
        let mut upstream_headers =
            build_simple_upstream_headers(headers, host, remote_addr, scheme, false)?;
        if !body.is_empty() && !upstream_headers.contains_key(CONTENT_LENGTH) {
            upstream_headers.insert(
                CONTENT_LENGTH,
                HeaderValue::from_str(&body.len().to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );
        }

        let mut upstream_io =
            connect_upgrade_upstream(&upstream_url, config.http.allow_insecure_upstreams)
                .await
                .with_context(|| format!("failed connecting raw SSE upstream {upstream_url}"))?;
        let request_bytes = serialize_http_request(method, &upstream_url, &upstream_headers, body)?;
        upstream_io
            .write_all(&request_bytes)
            .await
            .with_context(|| format!("failed sending raw SSE request to {upstream_url}"))?;

        let (status, response_headers, leftover) = read_http_response_head(&mut upstream_io)
            .await
            .with_context(|| format!("failed reading raw SSE response from {upstream_url}"))?;
        let response_headers = response_headers
            .into_iter()
            .filter(|(name, _)| !is_hop_header(name.as_str()))
            .collect::<Vec<_>>();
        let response = GatewayHttpResponse {
            status,
            headers: response_headers,
            body: Bytes::new(),
            stream_body: Some(raw_sse_streaming_body(upstream_io, leftover)),
            upstream: upstream_url.to_string(),
        };
        finalize_http_response(
            headers,
            &config.services.response_policy.compression,
            response,
        )
    }

    async fn try_serve_plain_raw_sse_fast_lane(
        &self,
        config: &GatewayConfig,
        downstream: &mut TcpStream,
        request: &PlainFastLaneRequest,
        remote_addr: SocketAddr,
    ) -> Result<bool> {
        if !request.accepts_sse {
            return Ok(false);
        }
        let host = request.host.as_deref().unwrap_or("localhost");
        let Some(route) = config
            .services
            .ai_proxy
            .enabled
            .then_some(&config.services.ai_proxy.routes)
            .into_iter()
            .flatten()
            .filter(|route| crate::ai_proxy::route_matches(route, host, &request.path))
            .max_by_key(|route| route.path_prefix.len())
        else {
            return Ok(false);
        };
        if !ai_proxy_route_fast_path_eligible(route) {
            return Ok(false);
        }

        let rewrite_path = ai_proxy_raw_rewrite_path(route, &request.target, &request.path);
        let Some((_, pool)) = self.raw_http_pool_for_upstream(&route.upstream)? else {
            return Ok(false);
        };
        let path_and_query = rewrite_path.as_deref().unwrap_or(&request.target);
        let mut upstream_io = pool.checkout().await.with_context(|| {
            format!(
                "failed connecting plain raw SSE upstream {}",
                route.upstream
            )
        })?;
        let _ = upstream_io.set_nodelay(true);
        tune_tcp_stream_for_gateway(&upstream_io);
        let request_bytes = if route.emit_metadata_headers {
            let prefix = config
                .services
                .ai_proxy
                .header_prefix
                .trim()
                .trim_start_matches('/');
            let ai_route_header = format!("{prefix}ai-route");
            let ai_provider_header = format!("{prefix}ai-provider");
            let ai_original_path_header = format!("{prefix}ai-original-path");
            let provider = if route.provider.trim().is_empty() {
                route.name.as_str()
            } else {
                route.provider.as_str()
            };
            let extra_headers = [
                (ai_route_header.as_str(), route.name.as_str()),
                (ai_provider_header.as_str(), provider),
                (ai_original_path_header.as_str(), request.path.as_str()),
            ];
            serialize_raw_fast_lane_request(
                request,
                path_and_query,
                host,
                RawFastLaneSerializeOptions {
                    connection: None,
                    remote_addr,
                    scheme: "http",
                    forward_headers: route.forward_headers,
                    extra_headers: &extra_headers,
                },
            )
        } else {
            serialize_raw_fast_lane_request(
                request,
                path_and_query,
                host,
                RawFastLaneSerializeOptions {
                    connection: None,
                    remote_addr,
                    scheme: "http",
                    forward_headers: route.forward_headers,
                    extra_headers: &[],
                },
            )
        };
        upstream_io
            .write_all(&request_bytes)
            .await
            .with_context(|| {
                format!("failed sending plain raw SSE request to {}", route.upstream)
            })?;

        let response_head = read_raw_fast_http_response_head(&mut upstream_io, true)
            .await
            .with_context(|| {
                format!(
                    "failed reading plain raw SSE response from {}",
                    route.upstream
                )
            })?;
        let upstream_keep_alive = !response_head.connection_close;
        let content_length = response_head.content_length;
        let transfer_chunked = response_head.transfer_chunked;
        let response_head_bytes = build_raw_http_response_head_bytes(
            response_head.status,
            &response_head.headers,
            upstream_keep_alive,
            transfer_chunked,
        );
        if request.method == Method::HEAD || status_has_no_body(response_head.status) {
            downstream
                .write_all(&response_head_bytes)
                .await
                .context("failed writing raw SSE response head")?;
            if upstream_keep_alive {
                pool.checkin(upstream_io);
            }
        } else if let Some(len) = content_length {
            let reusable = if let Some(leftover) = response_head.leftover {
                let leftover_len = leftover.len() as u64;
                if leftover_len >= len {
                    let mut response_bytes =
                        Vec::with_capacity(response_head_bytes.len() + len as usize);
                    response_bytes.extend_from_slice(&response_head_bytes);
                    response_bytes.extend_from_slice(&leftover[..len as usize]);
                    downstream
                        .write_all(&response_bytes)
                        .await
                        .context("failed writing raw SSE response head/body")?;
                    leftover_len == len
                } else {
                    downstream
                        .write_all(&response_head_bytes)
                        .await
                        .context("failed writing raw SSE response head")?;
                    relay_fixed_http_body(&mut upstream_io, downstream, Some(leftover), len).await?
                }
            } else {
                downstream
                    .write_all(&response_head_bytes)
                    .await
                    .context("failed writing raw SSE response head")?;
                relay_fixed_http_body(&mut upstream_io, downstream, None, len).await?
            };
            if reusable && upstream_keep_alive {
                pool.checkin(upstream_io);
            }
        } else if transfer_chunked {
            downstream
                .write_all(&response_head_bytes)
                .await
                .context("failed writing raw SSE response head")?;
            let reusable = relay_passthrough_chunked_http_body(
                &mut upstream_io,
                downstream,
                response_head.leftover,
            )
            .await
            .context("plain raw SSE stream relay failed")?;
            if reusable && upstream_keep_alive {
                pool.checkin(upstream_io);
            }
        } else {
            downstream
                .write_all(&response_head_bytes)
                .await
                .context("failed writing raw SSE response head")?;
            relay_raw_http_body(&mut upstream_io, downstream, response_head.leftover)
                .await
                .context("plain raw SSE stream relay failed")?;
        }
        Ok(true)
    }

    async fn try_serve_plain_raw_reverse_fast_lane(
        &self,
        config: &GatewayConfig,
        downstream: &mut TcpStream,
        request: &PlainFastLaneRequest,
        options: RawReverseFastLaneOptions<'_>,
    ) -> Result<bool> {
        let RawReverseFastLaneOptions {
            remote_addr,
            cached_upstream_request,
            cached_prepared_route,
            serialized_upstream_request,
            prepared_route,
            upstream_response_buffer,
            response_cache,
            lane_upstream,
        } = options;
        let host = request.host.as_deref().unwrap_or("localhost");
        let mut route_for_serialization = None;
        let prepared = if let Some(cached) = cached_prepared_route {
            cached
        } else {
            let Some(route) = config
                .services
                .reverse_proxy
                .routes
                .iter()
                .filter(|route| reverse_proxy_route_matches(route, host, &request.path))
                .max_by_key(|route| route.path_prefix.len())
            else {
                return Ok(false);
            };
            if !reverse_proxy_route_fast_path_eligible(route) {
                return Ok(false);
            }
            let Some((pool_key, pool)) = self.raw_http_pool_for_upstream(&route.upstream)? else {
                return Ok(false);
            };
            route_for_serialization = Some(route);
            let prepared = Arc::new(RawReversePreparedRoute {
                pool_key,
                pool,
                upstream: route.upstream.clone(),
            });
            *prepared_route = Some(prepared.clone());
            prepared
        };
        let pool_key = &prepared.pool_key;
        let pool = &prepared.pool;
        let upstream = &prepared.upstream;
        let mut upstream_io = if lane_upstream
            .as_ref()
            .map(|upstream| upstream.key.as_str() == pool_key.as_str())
            .unwrap_or(false)
        {
            lane_upstream
                .take()
                .map(|upstream| upstream.stream)
                .expect("checked lane upstream exists")
        } else {
            if let Some(previous) = lane_upstream.take() {
                previous.pool.checkin(previous.stream);
            }
            pool.checkout().await.with_context(|| {
                format!("failed checking out plain raw reverse upstream {upstream}")
            })?
        };
        let request_bytes = if let Some(cached) = cached_upstream_request {
            cached
        } else {
            let route = route_for_serialization
                .expect("raw reverse cache miss retained route for serialization");
            let path_and_query =
                reverse_proxy_raw_path_and_query(route, &request.target, &request.path);
            *serialized_upstream_request = Some(Bytes::from(serialize_raw_fast_lane_request(
                request,
                path_and_query.as_ref(),
                host,
                RawFastLaneSerializeOptions {
                    connection: None,
                    remote_addr,
                    scheme: "http",
                    forward_headers: route.forward_headers,
                    extra_headers: &[],
                },
            )));
            serialized_upstream_request
                .as_ref()
                .expect("raw reverse request serialized")
        };
        upstream_io
            .write_all(request_bytes)
            .await
            .with_context(|| format!("failed sending plain raw reverse request to {upstream}"))?;

        let response = read_raw_reverse_http_response_into(
            &mut upstream_io,
            upstream_response_buffer,
            response_cache,
        )
        .await
        .with_context(|| format!("failed reading plain raw reverse response from {upstream}"))?;
        let upstream_keep_alive = !response.connection_close;
        let buffered_body_len = upstream_response_buffer.len() - response.head_end;
        if request.method == Method::HEAD || status_has_no_body(response.status) {
            downstream
                .write_all(&upstream_response_buffer[..response.head_end])
                .await
                .context("failed writing raw HTTP response head")?;
            if upstream_keep_alive && buffered_body_len == 0 {
                *lane_upstream = Some(RawReverseLaneUpstream {
                    key: pool_key.clone(),
                    pool: pool.clone(),
                    stream: upstream_io,
                });
            }
        } else if let Some(len) = response.content_length {
            let buffered_body_len_u64 = buffered_body_len as u64;
            let buffered_to_write = buffered_body_len_u64.min(len) as usize;
            downstream
                .write_all(
                    &upstream_response_buffer
                        [..response.head_end.saturating_add(buffered_to_write)],
                )
                .await
                .context("failed writing raw HTTP response head/body")?;
            let reusable = if buffered_body_len_u64 > len {
                false
            } else {
                relay_fixed_http_body(
                    &mut upstream_io,
                    downstream,
                    None,
                    len - buffered_body_len_u64,
                )
                .await?
            };
            if reusable && upstream_keep_alive {
                *lane_upstream = Some(RawReverseLaneUpstream {
                    key: pool_key.clone(),
                    pool: pool.clone(),
                    stream: upstream_io,
                });
            }
        } else if response.transfer_chunked {
            let leftover = (buffered_body_len > 0)
                .then(|| Bytes::copy_from_slice(&upstream_response_buffer[response.head_end..]));
            downstream
                .write_all(&upstream_response_buffer[..response.head_end])
                .await
                .context("failed writing raw HTTP response head")?;
            let reusable =
                relay_passthrough_chunked_http_body(&mut upstream_io, downstream, leftover).await?;
            if reusable && upstream_keep_alive {
                *lane_upstream = Some(RawReverseLaneUpstream {
                    key: pool_key.clone(),
                    pool: pool.clone(),
                    stream: upstream_io,
                });
            }
        } else {
            let leftover = (buffered_body_len > 0)
                .then(|| Bytes::copy_from_slice(&upstream_response_buffer[response.head_end..]));
            downstream
                .write_all(&upstream_response_buffer[..response.head_end])
                .await
                .context("failed writing raw HTTP response head")?;
            relay_raw_http_body(&mut upstream_io, downstream, leftover).await?;
        }
        Ok(true)
    }

    async fn try_serve_raw_websocket_fast_lane<Downstream>(
        &self,
        config: &GatewayConfig,
        downstream: &mut Downstream,
        request: &PlainWebSocketFastLaneRequest,
        options: RawWebSocketFastLaneOptions<'_>,
    ) -> Result<bool>
    where
        Downstream: AsyncRead + AsyncWrite + Unpin + ?Sized,
    {
        let RawWebSocketFastLaneOptions {
            remote_addr,
            scheme,
            downstream_leftover,
            downstream_started,
            downstream_detached,
            #[cfg(target_os = "linux")]
            plain_downstream_fd,
        } = options;
        #[cfg(not(target_os = "linux"))]
        let _ = downstream_detached;
        let host = request.host.as_deref().unwrap_or("localhost");
        let Some(route) = config
            .services
            .reverse_proxy
            .routes
            .iter()
            .filter(|route| websocket_route_fast_path_eligible(route))
            .filter(|route| reverse_proxy_route_matches(route, host, &request.path))
            .max_by_key(|route| route.path_prefix.len())
        else {
            return Ok(false);
        };
        let route_decision = RouteDecision {
            upstream: route.upstream.clone(),
            upstreams: route.upstreams.clone(),
            upstream_weights: route.upstream_weights.clone(),
            affinity_key: None,
            rewrite_path: None,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            status: None,
            content_type: None,
        };
        let remote_addr_text = remote_addr.to_string();
        let selected_upstream = self
            .select_upstream_plan(
                config,
                &route_decision,
                "websocket",
                Some(&route.name),
                None,
                Some(&remote_addr_text),
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| route.upstream.clone());
        let Some((_, upstream_host, upstream_port)) =
            raw_websocket_pool_parts_from_upstream(&selected_upstream)?
        else {
            return Ok(false);
        };
        let path_and_query =
            reverse_proxy_raw_path_and_query(route, &request.target, &request.path);
        let mut upstream_io = TcpStream::connect((upstream_host.as_str(), upstream_port))
            .await
            .with_context(|| {
                format!(
                    "failed connecting plain raw websocket upstream {}",
                    selected_upstream
                )
            })?;
        let _ = upstream_io.set_nodelay(true);
        tune_tcp_stream_for_latency(&upstream_io);

        let request_bytes = serialize_raw_websocket_fast_lane_request(
            request,
            path_and_query.as_ref(),
            host,
            remote_addr,
            scheme,
            route.forward_headers,
        );
        upstream_io
            .write_all(&request_bytes)
            .await
            .with_context(|| {
                format!(
                    "failed sending plain raw websocket handshake to {}",
                    selected_upstream
                )
            })?;

        let response_head = read_raw_fast_http_response_head(&mut upstream_io, false)
            .await
            .with_context(|| {
                format!(
                    "failed reading plain raw websocket response from {}",
                    selected_upstream
                )
            })?;
        *downstream_started = true;
        downstream
            .write_all(&response_head.raw_head)
            .await
            .context("failed writing raw websocket response head")?;
        if let Some(leftover) = response_head.leftover {
            if !leftover.is_empty() {
                downstream
                    .write_all(&leftover)
                    .await
                    .context("failed writing raw websocket upstream prelude")?;
            }
        }
        if response_head.status != StatusCode::SWITCHING_PROTOCOLS {
            return Ok(true);
        }

        if !downstream_leftover.is_empty() {
            upstream_io
                .write_all(downstream_leftover)
                .await
                .context("failed forwarding early raw websocket client bytes")?;
        }

        #[cfg(target_os = "linux")]
        if LINUX_STREAM_REACTOR_ENABLED && plain_downstream_fd.is_some() {
            let downstream_fd = plain_downstream_fd.expect("plain downstream fd checked");
            match crate::stream_reactor::dispatch(
                downstream_fd,
                upstream_io.as_raw_fd(),
                realtime_stream_reactor_workers(),
                realtime_stream_reactor_nice(),
            ) {
                Ok(()) => {
                    *downstream_detached = true;
                    return Ok(true);
                }
                Err(error) => {
                    tracing::debug!(?error, %remote_addr, "plain WebSocket epoll handoff unavailable; using async relay");
                }
            }
        }

        if scheme.eq_ignore_ascii_case("https") {
            copy_bidirectional_with_pooled_buffers_limit::<
                _,
                _,
                WEBSOCKET_RELAY_POLL_STEPS,
                true,
                true,
            >(downstream, &mut upstream_io, websocket_relay_buffer_pool())
            .await
            .context("buffered WSS tunnel relay failed")?;
        } else {
            copy_bidirectional_with_pooled_buffers_limit::<
                _,
                _,
                WEBSOCKET_RELAY_POLL_STEPS,
                false,
                false,
            >(downstream, &mut upstream_io, websocket_relay_buffer_pool())
            .await
            .context("plain WebSocket tunnel relay failed")?;
        }
        Ok(true)
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
        forward_headers: bool,
        on_upgrade: OnUpgrade,
    ) -> Result<GatewayHttpResponse> {
        let state = self.current_state().await;
        let remote_addr_text = remote_addr.to_string();
        let selected_upstream = self
            .select_upstream_plan(
                &state.config,
                route,
                "websocket",
                Some("http"),
                route.affinity_key.as_deref(),
                Some(&remote_addr_text),
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| route.upstream.clone());
        let upstream_url = build_upstream_url(&selected_upstream, route, &uri)?;
        let upstream_host = upstream_host_header(&upstream_url)?;
        let upstream_headers = build_websocket_upstream_headers(
            &headers,
            route,
            &upstream_host,
            remote_addr,
            scheme,
            host,
            forward_headers,
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
                copy_bidirectional_with_pooled_buffers_limit::<
                    _,
                    _,
                    WEBSOCKET_RELAY_POLL_STEPS,
                    true,
                    true,
                >(
                    &mut client,
                    &mut *upstream_io,
                    websocket_relay_buffer_pool(),
                )
                .await
                .context("websocket tunnel relay failed")?;
                Ok::<_, anyhow::Error>(())
            }
            .await;

            if let Err(error) = result {
                if is_expected_websocket_disconnect(&error) {
                    tracing::debug!(upstream = %tunnel_upstream, "websocket tunnel closed by peer");
                } else {
                    tracing::warn!(?error, upstream = %tunnel_upstream, "websocket tunnel failed");
                }
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

    async fn plain_http_data_fast_lane_enabled(&self) -> bool {
        if self.bootstrap_config.runtime.hot_reload.enabled || self.bootstrap_config.admin.enabled {
            let fast_lane = self.current_state().await.fast_lane.clone();
            fast_lane.plain_http_static_sendfile
                || fast_lane.raw_sse_proxy
                || fast_lane.raw_reverse_proxy
                || fast_lane.raw_websocket_proxy
        } else {
            self.bootstrap_fast_lane.plain_http_static_sendfile
                || self.bootstrap_fast_lane.raw_sse_proxy
                || self.bootstrap_fast_lane.raw_reverse_proxy
                || self.bootstrap_fast_lane.raw_websocket_proxy
        }
    }

    fn raw_http_pool_for_upstream(
        &self,
        upstream: &str,
    ) -> Result<Option<(String, Arc<RawHttpUpstreamPool>)>> {
        let Some((key, host, port)) = raw_http_pool_parts_from_upstream(upstream)? else {
            return Ok(None);
        };
        let pool = self.raw_http_pool_for_parts(key.clone(), host, port);
        Ok(Some((key, pool)))
    }

    fn raw_http_pool_for_parts(
        &self,
        key: String,
        host: String,
        port: u16,
    ) -> Arc<RawHttpUpstreamPool> {
        if let Some(pool) = self.raw_http_pools.get(&key) {
            return pool.clone();
        }

        let pool = Arc::new(RawHttpUpstreamPool::new(host, port));
        self.raw_http_pools.insert(key.clone(), pool.clone());
        self.raw_http_pools
            .get(&key)
            .map(|entry| entry.clone())
            .unwrap_or(pool)
    }

    fn prune_raw_http_pools(&self, config: &GatewayConfig) {
        let active_keys = raw_http_pool_keys_for_config(config);
        self.raw_http_pools
            .retain(|key, _| active_keys.contains(key.as_str()));
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
        route: &HttpRouteConfig<'_>,
        cache_key: &str,
        request: &HttpCacheRevalidateRequest<'_>,
    ) -> Result<()> {
        let state = self.current_state().await;
        let remote_addr_for_affinity =
            if state.config.affinity.enabled && state.config.affinity.fallback_to_remote_addr {
                Some(request.remote_addr.to_string())
            } else {
                None
            };
        let upstream_plan = self.select_upstream_plan(
            &state.config,
            &route.decision,
            "http",
            route.runtime_scope.as_deref(),
            route.decision.affinity_key.as_deref(),
            remote_addr_for_affinity.as_deref(),
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
            route.forward_headers,
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
    ) -> std::result::Result<Option<HttpRateLimitLease>, Box<GatewayHttpResponse>> {
        if !config.enabled {
            return Ok(None);
        }

        let Some(key) = http_rate_limit_key(config, host, headers, remote_addr) else {
            return Ok(None);
        };
        if let Some(retry_after) =
            apply_http_rate_limit_to_store(&self.http_rate_limits, config, key.clone())
        {
            return Err(Box::new(rate_limit_rejection_response(
                config,
                &retry_after,
            )));
        }

        if config.max_connections == 0 {
            return Ok(None);
        }

        let mut entry = self.http_connection_limits.entry(key.clone()).or_insert(0);
        if *entry >= config.max_connections {
            return Err(Box::new(rate_limit_rejection_response(config, "1")));
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

        if raw_candidates.len() == 1
            && !config.load_balance.active_health.enabled
            && !config.load_balance.passive_health.enabled
        {
            return raw_candidates;
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
            // Rendezvous is sticky only when there is an affinity key. Without
            // one, ranking against the static listener scope would select the
            // same first upstream for every WebSocket/HTTP/TCP connection and
            // silently defeat an upstream pool. Fall back to lock-free
            // round-robin distribution in that case.
            LoadBalanceAlgorithm::Rendezvous if selected_key.is_none() => {
                self.select_round_robin_plan(&scope_base, candidates)
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

    fn should_track_upstream_runtime(
        &self,
        config: &GatewayConfig,
        _protocol: &str,
        _listener: Option<&str>,
        _upstream: &str,
    ) -> bool {
        config.load_balance.active_health.enabled || config.load_balance.passive_health.enabled
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
                ActiveHealthKind::Udp => {
                    probe_udp_upstream(&target.upstream, &target.settings).await
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
    Udp,
}

impl ActiveHealthKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Tcp => "tcp",
            Self::Udp => "udp",
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

    if config.load_balance.active_health.enabled && config.load_balance.active_health.udp_enabled {
        for listener in &config.udp.listeners {
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
                let key = runtime_scope_key("udp", Some(&listener.name), &upstream);
                upstreams.entry(key).or_insert(ActiveHealthTarget {
                    protocol: "udp".to_string(),
                    listener: Some(listener.name.clone()),
                    upstream,
                    kind: ActiveHealthKind::Udp,
                    settings: resolve_global_active_health_config(
                        &config.load_balance.active_health,
                    ),
                });
            }
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

async fn probe_udp_upstream(
    upstream: &str,
    settings: &ResolvedActiveHealthConfig,
) -> (bool, Option<u16>, Option<String>, Option<u64>) {
    let started_at = Instant::now();
    let timeout = Duration::from_millis(settings.timeout_ms.max(100));
    let target = match tcp_probe_target(upstream) {
        Ok(target) => target,
        Err(error) => return (false, None, Some(error.to_string()), None),
    };
    let bind_any = if target.contains(':') && !target.contains('.') {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = match UdpSocket::bind(bind_any).await {
        Ok(socket) => socket,
        Err(error) => {
            return (
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
            )
        }
    };
    if let Err(error) = socket.connect(&target).await {
        return (
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
        );
    }
    if let Err(error) = socket.send(settings.udp_payload.as_bytes()).await {
        return (
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
        );
    }
    if !settings.udp_expect_response {
        return (
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
        );
    }

    let mut buffer = [0_u8; 2048];
    match tokio::time::timeout(timeout, socket.recv(&mut buffer)).await {
        Ok(Ok(_)) => (
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
        ),
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
                "udp response timeout after {}ms",
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
    GatewayBody::Full(Some(body))
}

fn optimized_http_server_builder() -> AutoBuilder<TokioExecutor> {
    let timer = TokioTimer::new();
    let mut builder = AutoBuilder::new(TokioExecutor::new());
    builder
        .http1()
        .writev(true)
        .max_buf_size(1024 * 1024)
        .timer(timer.clone());
    builder
        .http2()
        .timer(timer)
        .adaptive_window(true)
        .initial_stream_window_size(Some(HTTP2_STREAM_WINDOW_SIZE_BYTES))
        .initial_connection_window_size(Some(HTTP2_CONNECTION_WINDOW_SIZE_BYTES))
        .max_frame_size(Some(HTTP2_MAX_FRAME_SIZE_BYTES))
        .max_concurrent_streams(Some(HTTP2_MAX_CONCURRENT_STREAMS))
        .keep_alive_interval(Some(Duration::from_secs(HTTP2_KEEP_ALIVE_INTERVAL_SECS)))
        .keep_alive_timeout(Duration::from_secs(HTTP2_KEEP_ALIVE_TIMEOUT_SECS))
        .max_send_buf_size(HTTP2_MAX_SEND_BUF_BYTES);
    builder
}

fn optimized_http2_server_builder() -> Http2ServerBuilder<TokioExecutor> {
    let mut builder = Http2ServerBuilder::new(TokioExecutor::new());
    builder
        .timer(TokioTimer::new())
        .adaptive_window(true)
        .initial_stream_window_size(Some(HTTP2_STREAM_WINDOW_SIZE_BYTES))
        .initial_connection_window_size(Some(HTTP2_CONNECTION_WINDOW_SIZE_BYTES))
        .max_frame_size(Some(HTTP2_MAX_FRAME_SIZE_BYTES))
        .max_concurrent_streams(Some(HTTP2_MAX_CONCURRENT_STREAMS))
        .keep_alive_interval(Some(Duration::from_secs(HTTP2_KEEP_ALIVE_INTERVAL_SECS)))
        .keep_alive_timeout(Duration::from_secs(HTTP2_KEEP_ALIVE_TIMEOUT_SECS))
        .max_send_buf_size(HTTP2_MAX_SEND_BUF_BYTES);
    builder
}

struct StaticFastPathRequest<'a> {
    method: &'static str,
    target: &'a str,
    path: &'a str,
    host: Option<&'a str>,
    keep_alive: bool,
}

struct StaticFastPathCandidate {
    path: PathBuf,
    len: u64,
    content_type: &'static str,
    cached_body: Option<Bytes>,
    sendfile: Option<Arc<std::fs::File>>,
}

struct ConnectionStaticFastPathCache {
    request_head: Bytes,
    target: String,
    host: Option<String>,
    checked_at: Instant,
    header: Bytes,
    combined_response: Option<Bytes>,
    body: Option<Bytes>,
    file_path: PathBuf,
    len: u64,
    sendfile: Option<Arc<std::fs::File>>,
}

impl ConnectionStaticFastPathCache {
    fn raw_request_matches(&self, request_head: &[u8]) -> bool {
        self.checked_at.elapsed() <= Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS)
            && self.request_head.as_ref() == request_head
    }

    fn identity_matches(&self, request: &StaticFastPathRequest<'_>) -> bool {
        request.method == "GET"
            && request.keep_alive
            && self.target == request.target
            && self.host.as_deref() == request.host
    }

    fn matches(&self, request: &StaticFastPathRequest<'_>) -> bool {
        self.identity_matches(request)
            && self.checked_at.elapsed() <= Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS)
    }

    fn same_payload(&self, candidate: &StaticFastPathCandidate) -> bool {
        self.len == candidate.len
            && match (&self.body, &candidate.cached_body) {
                (Some(current), Some(candidate)) => {
                    current.len() == candidate.len() && current.as_ptr() == candidate.as_ptr()
                }
                (None, None) => match (&self.sendfile, &candidate.sendfile) {
                    (Some(current), Some(candidate)) => Arc::ptr_eq(current, candidate),
                    (None, None) => true,
                    _ => false,
                },
                _ => false,
            }
    }
}

#[derive(Clone, Debug, Default)]
struct RawForwardingHeaderSnapshot {
    x_real_ip: Option<String>,
    x_forwarded_for: Option<String>,
    x_forwarded_host: Option<String>,
    x_forwarded_proto: Option<String>,
    forwarded: Option<String>,
}

#[derive(Clone)]
struct PlainFastLaneRequest {
    method: Method,
    target: String,
    path: String,
    forward_header_bytes: Vec<u8>,
    forwarding_headers: RawForwardingHeaderSnapshot,
    accepts_sse: bool,
    host: Option<String>,
    keep_alive: bool,
}

struct PlainWebSocketFastLaneRequest {
    target: String,
    path: String,
    header_bytes: Vec<u8>,
    forwarding_headers: RawForwardingHeaderSnapshot,
    host: Option<String>,
}

struct PrefixedIo<S> {
    inner: S,
    prefix: Bytes,
}

impl<S> PrefixedIo<S> {
    fn new(inner: S, prefix: Bytes) -> Self {
        Self { inner, prefix }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for PrefixedIo<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if !self.prefix.is_empty() && buffer.remaining() > 0 {
            let copied = self.prefix.len().min(buffer.remaining());
            buffer.put_slice(&self.prefix[..copied]);
            self.prefix.advance(copied);
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buffer)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PrefixedIo<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

enum TlsRawWebSocketAttempt {
    Served,
    Fallback(Bytes),
}

enum TlsStaticFastLaneAttempt {
    Served,
    Fallback(Bytes),
}

enum PlainHttpFastLaneAttempt {
    Served,
    Fallback { stream: TcpStream, prefix: Bytes },
}

enum PlainHttpFastLaneDecision {
    Served,
    Detached,
    Fallback(Bytes),
}

struct RawWebSocketFastLaneOptions<'a> {
    remote_addr: SocketAddr,
    scheme: &'a str,
    downstream_leftover: &'a [u8],
    downstream_started: &'a mut bool,
    downstream_detached: &'a mut bool,
    #[cfg(target_os = "linux")]
    plain_downstream_fd: Option<i32>,
}

struct RawReverseLaneUpstream {
    key: String,
    pool: Arc<RawHttpUpstreamPool>,
    stream: TcpStream,
}

struct RawReverseParsedRequestCache {
    request_head: Bytes,
    request: PlainFastLaneRequest,
    upstream_request: Bytes,
    prepared_route: Arc<RawReversePreparedRoute>,
}

struct RawReversePreparedRoute {
    pool_key: String,
    pool: Arc<RawHttpUpstreamPool>,
    upstream: String,
}

struct RawReverseFastLaneOptions<'a> {
    remote_addr: SocketAddr,
    cached_upstream_request: Option<&'a Bytes>,
    cached_prepared_route: Option<Arc<RawReversePreparedRoute>>,
    serialized_upstream_request: &'a mut Option<Bytes>,
    prepared_route: &'a mut Option<Arc<RawReversePreparedRoute>>,
    upstream_response_buffer: &'a mut Vec<u8>,
    response_cache: &'a mut Option<RawReverseResponseCache>,
    lane_upstream: &'a mut Option<RawReverseLaneUpstream>,
}

fn plain_static_fast_path_allowed(config: &GatewayConfig) -> bool {
    !config.logging.access_log
        && !config.security.ddos.enabled
        && !config.security.dynamic_blacklist.enabled
        && !config.services.access_control.http.enabled
        && !config.services.rate_limit.http.enabled
        && !config.services.response_policy.compression.enabled
        && !config.services.static_sites.is_empty()
}

fn static_sendfile_fast_path_threshold_bytes(config: &GatewayConfig) -> u64 {
    match config.runtime.performance.traffic_profile {
        RuntimePerformanceTrafficProfile::Bulk => 0,
        RuntimePerformanceTrafficProfile::Small | RuntimePerformanceTrafficProfile::Balanced => {
            STATIC_SENDFILE_FAST_PATH_THRESHOLD_BYTES
        }
    }
}

fn peek_static_fast_path_path(buffer: &[u8]) -> Option<&str> {
    let head = std::str::from_utf8(buffer).ok()?;
    let line_end = head.find("\r\n")?;
    let mut parts = head[..line_end].split_whitespace();
    match parts.next()? {
        "GET" | "HEAD" => {}
        _ => return None,
    }
    let target = parts.next()?;
    if !target.starts_with('/') || target.starts_with("//") {
        return None;
    }
    Some(
        target
            .split_once('?')
            .map(|(path, _)| path)
            .unwrap_or(target),
    )
}

fn parse_static_fast_path_request(buffer: &[u8]) -> Option<StaticFastPathRequest<'_>> {
    let head = std::str::from_utf8(buffer).ok()?;
    let head_end = head.find("\r\n\r\n")?;
    let mut lines = head[..head_end].split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_whitespace();
    let method = match request_parts.next()? {
        "GET" => "GET",
        "HEAD" => "HEAD",
        _ => return None,
    };
    let target = request_parts.next()?;
    let version = request_parts.next()?;
    if request_parts.next().is_some() || (version != "HTTP/1.1" && version != "HTTP/1.0") {
        return None;
    }
    if !target.starts_with('/') || target.starts_with("//") {
        return None;
    }

    let path = target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target);
    let mut host = None;
    let mut connection_close = false;
    let mut connection_keep_alive = false;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let name = name.trim();
        if name.eq_ignore_ascii_case("range")
            || name.eq_ignore_ascii_case("transfer-encoding")
            || name.eq_ignore_ascii_case("upgrade")
        {
            return None;
        }
        if name.eq_ignore_ascii_case("content-length")
            && value.trim().parse::<u64>().ok().unwrap_or(1) != 0
        {
            return None;
        }
        if name.eq_ignore_ascii_case("host") {
            host = Some(value.trim());
        }
        if name.eq_ignore_ascii_case("connection") {
            for token in value.split(',') {
                let token = token.trim();
                if token.eq_ignore_ascii_case("close") {
                    connection_close = true;
                } else if token.eq_ignore_ascii_case("keep-alive") {
                    connection_keep_alive = true;
                }
            }
        }
    }
    let keep_alive = if version == "HTTP/1.1" {
        !connection_close
    } else {
        connection_keep_alive && !connection_close
    };

    Some(StaticFastPathRequest {
        method,
        target,
        path,
        host,
        keep_alive,
    })
}

fn parse_plain_fast_lane_request(buffer: &[u8]) -> Option<PlainFastLaneRequest> {
    let head = std::str::from_utf8(buffer).ok()?;
    let head_end = head.find("\r\n\r\n")?;
    let mut lines = head[..head_end].split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_whitespace();
    let method = Method::from_bytes(request_parts.next()?.as_bytes()).ok()?;
    if method != Method::GET && method != Method::HEAD {
        return None;
    }
    let target = request_parts.next()?;
    let version = request_parts.next()?;
    if request_parts.next().is_some() || (version != "HTTP/1.1" && version != "HTTP/1.0") {
        return None;
    }
    if !target.starts_with('/') || target.starts_with("//") {
        return None;
    }

    let path = target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target);
    let mut forward_header_bytes = Vec::with_capacity(head_end.min(1024));
    let mut forwarding_headers = RawForwardingHeaderSnapshot::default();
    let mut accepts_sse = false;
    let mut host = None;
    let mut connection_close = false;
    let mut connection_keep_alive = false;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding")
            || name.eq_ignore_ascii_case("upgrade")
            || (name.eq_ignore_ascii_case("content-length")
                && value.parse::<u64>().ok().unwrap_or(1) != 0)
        {
            return None;
        }
        if name.eq_ignore_ascii_case("host") {
            host = Some(value.to_string());
        }
        if name.eq_ignore_ascii_case("accept") && header_value_accepts_sse(value) {
            accepts_sse = true;
        }
        if name.eq_ignore_ascii_case("connection") {
            for token in value.split(',') {
                let token = token.trim();
                if token.eq_ignore_ascii_case("close") {
                    connection_close = true;
                } else if token.eq_ignore_ascii_case("keep-alive") {
                    connection_keep_alive = true;
                }
            }
        }
        if capture_forwarding_header_snapshot(&mut forwarding_headers, name, value) {
            continue;
        }
        if !is_hop_header(name)
            && !name.eq_ignore_ascii_case("host")
            && !name.eq_ignore_ascii_case("content-length")
        {
            forward_header_bytes.extend_from_slice(name.as_bytes());
            forward_header_bytes.extend_from_slice(b": ");
            forward_header_bytes.extend_from_slice(value.as_bytes());
            forward_header_bytes.extend_from_slice(b"\r\n");
        }
    }
    let keep_alive = if version == "HTTP/1.1" {
        !connection_close
    } else {
        connection_keep_alive && !connection_close
    };

    Some(PlainFastLaneRequest {
        method,
        target: target.to_string(),
        path: path.to_string(),
        forward_header_bytes,
        forwarding_headers,
        accepts_sse,
        host,
        keep_alive,
    })
}

fn capture_forwarding_header_snapshot(
    snapshot: &mut RawForwardingHeaderSnapshot,
    name: &str,
    value: &str,
) -> bool {
    let trimmed = value.trim();
    if name.eq_ignore_ascii_case("x-real-ip") {
        snapshot.x_real_ip = Some(trimmed.to_string());
        return true;
    }
    if name.eq_ignore_ascii_case("x-forwarded-for") {
        merge_csv_header_value(&mut snapshot.x_forwarded_for, trimmed);
        return true;
    }
    if name.eq_ignore_ascii_case("x-forwarded-host") {
        snapshot.x_forwarded_host = Some(trimmed.to_string());
        return true;
    }
    if name.eq_ignore_ascii_case("x-forwarded-proto") {
        snapshot.x_forwarded_proto = Some(trimmed.to_string());
        return true;
    }
    if name.eq_ignore_ascii_case("forwarded") {
        merge_csv_header_value(&mut snapshot.forwarded, trimmed);
        return true;
    }
    false
}

fn merge_csv_header_value(slot: &mut Option<String>, next: &str) {
    if next.is_empty() {
        return;
    }
    match slot {
        Some(existing) if !existing.trim().is_empty() => {
            existing.push_str(", ");
            existing.push_str(next);
        }
        Some(existing) => *existing = next.to_string(),
        None => *slot = Some(next.to_string()),
    }
}

fn parse_plain_websocket_fast_lane_request(buffer: &[u8]) -> Option<PlainWebSocketFastLaneRequest> {
    let head = std::str::from_utf8(buffer).ok()?;
    let head_end = head.find("\r\n\r\n")?;
    let mut lines = head[..head_end].split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.split_whitespace();
    if request_parts.next()? != "GET" {
        return None;
    }
    let target = request_parts.next()?;
    let version = request_parts.next()?;
    if request_parts.next().is_some() || version != "HTTP/1.1" {
        return None;
    }
    if !target.starts_with('/') || target.starts_with("//") {
        return None;
    }

    let path = target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target);
    let mut header_bytes = Vec::with_capacity(head_end.min(1536));
    let mut forwarding_headers = RawForwardingHeaderSnapshot::default();
    let mut host = None;
    let mut upgrade_websocket = false;
    let mut connection_upgrade = false;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding")
            || (name.eq_ignore_ascii_case("content-length")
                && value.parse::<u64>().ok().unwrap_or(1) != 0)
        {
            return None;
        }
        if name.eq_ignore_ascii_case("host") {
            host = Some(value.to_string());
            continue;
        }
        if capture_forwarding_header_snapshot(&mut forwarding_headers, name, value) {
            continue;
        }
        if name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket") {
            upgrade_websocket = true;
        }
        if name.eq_ignore_ascii_case("connection")
            && value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
        {
            connection_upgrade = true;
        }
        if !name.eq_ignore_ascii_case("proxy-connection")
            && !name.eq_ignore_ascii_case("content-length")
        {
            header_bytes.extend_from_slice(name.as_bytes());
            header_bytes.extend_from_slice(b": ");
            header_bytes.extend_from_slice(value.as_bytes());
            header_bytes.extend_from_slice(b"\r\n");
        }
    }
    if !upgrade_websocket || !connection_upgrade {
        return None;
    }

    Some(PlainWebSocketFastLaneRequest {
        target: target.to_string(),
        path: path.to_string(),
        header_bytes,
        forwarding_headers,
        host,
    })
}

const TLS_FAST_LANE_HTTP_HEAD_MAX_BYTES: usize = 64 * 1024;

async fn read_tls_fast_lane_http_prefix<Stream>(stream: &mut Stream) -> std::io::Result<Bytes>
where
    Stream: AsyncRead + Unpin + ?Sized,
{
    let mut prefix = BytesMut::with_capacity(4096);
    let _ = read_fast_lane_http_prefix(stream, &mut prefix).await?;
    Ok(prefix.freeze())
}

async fn read_fast_lane_http_prefix<Stream>(
    stream: &mut Stream,
    prefix: &mut BytesMut,
) -> std::io::Result<Option<usize>>
where
    Stream: AsyncRead + Unpin + ?Sized,
{
    loop {
        if let Some(index) = memmem::find(prefix, b"\r\n\r\n") {
            return Ok(Some(index + 4));
        }
        if prefix.len() >= TLS_FAST_LANE_HTTP_HEAD_MAX_BYTES {
            return Ok(None);
        }
        let remaining = TLS_FAST_LANE_HTTP_HEAD_MAX_BYTES - prefix.len();
        prefix.reserve(remaining.min(4096));
        let read = stream.read_buf(prefix).await?;
        if read == 0 {
            return Ok(None);
        }
    }
}

fn discard_fast_lane_http_head(prefix: &mut BytesMut, head_end: usize) {
    debug_assert!(head_end <= prefix.len());
    let remaining = prefix.len().saturating_sub(head_end);
    if remaining > 0 {
        prefix.copy_within(head_end.., 0);
    }
    prefix.truncate(remaining);
}

async fn resolve_large_static_fast_path_candidate(
    config: &GatewayConfig,
    request: &StaticFastPathRequest<'_>,
    static_route_cache: &DashMap<String, PathBuf>,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    static_file_cache_bytes: &AtomicU64,
    static_file_load_locks: &DashMap<String, Arc<TokioMutex<()>>>,
) -> Result<Option<StaticFastPathCandidate>> {
    let mut matched_site = None;
    let route_cached_target = static_route_cache
        .get(request.path)
        .map(|target| target.clone());
    if route_cached_target.is_none() || http_to_https_redirect_can_apply(config) {
        let uri = request
            .target
            .parse::<Uri>()
            .context("invalid fast path request target")?;
        if !security::request_uri_is_safe(&uri) {
            return Ok(None);
        }
        if let Some(host) = request.host {
            if should_redirect_http_to_https(config, host, &uri) {
                return Ok(None);
            }
        }
    }

    let mut target = if let Some(target) = route_cached_target {
        target.clone()
    } else {
        let Some(site) = config
            .services
            .static_sites
            .iter()
            .find(|site| static_site_path_matches(site, request.path))
        else {
            return Ok(None);
        };
        let Some(target) = static_site_filesystem_path(site, request.path)? else {
            return Ok(None);
        };
        matched_site = Some(site);
        target
    };
    let sendfile_threshold = static_sendfile_fast_path_threshold_bytes(config);
    if let Some(candidate) = fresh_cached_static_file_candidate(
        &target,
        request.method,
        static_file_cache,
        sendfile_threshold,
    ) {
        return Ok(Some(candidate));
    }

    let metadata = match tokio::fs::metadata(&target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("failed reading fast path static metadata"),
    };
    let metadata = if metadata.is_dir() {
        let Some(site) = matched_site else {
            return Ok(None);
        };
        let mut found = None;
        for index in &site.index_files {
            let candidate = target.join(index);
            if tokio::fs::metadata(&candidate)
                .await
                .map(|item| item.is_file())
                .unwrap_or(false)
            {
                found = Some(candidate);
                break;
            }
        }
        let Some(index) = found else {
            return Ok(None);
        };
        target = index;
        tokio::fs::metadata(&target)
            .await
            .context("failed reading fast path static index metadata")?
    } else {
        metadata
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let use_sendfile = cfg!(target_os = "linux")
        && request.method == "GET"
        && metadata.len() >= sendfile_threshold;
    let cached_body = if request.method == "GET"
        && !use_sendfile
        && metadata.len() < STATIC_STREAM_THRESHOLD_BYTES
    {
        Some(
            cached_static_file_body(
                &target,
                &metadata,
                static_file_cache,
                static_file_cache_bytes,
                static_file_load_locks,
            )
            .await?,
        )
    } else {
        None
    };
    let sendfile = if use_sendfile {
        Some(cached_static_sendfile(
            &target,
            &metadata,
            static_file_cache,
        )?)
    } else {
        None
    };

    Ok(Some(StaticFastPathCandidate {
        content_type: static_content_type(&target),
        len: metadata.len(),
        path: target,
        cached_body,
        sendfile,
    }))
}

async fn send_connection_static_fast_path(
    stream: &mut TcpStream,
    cached: &ConnectionStaticFastPathCache,
    cooperative_mid_yield: bool,
) -> Result<()> {
    if let Some(response) = cached.combined_response.as_ref() {
        return stream
            .write_all(response)
            .await
            .context("failed writing cached combined static response");
    }

    #[cfg(target_os = "linux")]
    let cork_static = cached.len > 0;
    #[cfg(target_os = "linux")]
    if cork_static {
        set_tcp_cork(stream, true);
    }

    let header_result = stream
        .write_all(&cached.header)
        .await
        .context("failed writing cached static response head");
    let body_result = if header_result.is_ok() && cached.len > 0 {
        if let Some(body) = cached.body.as_ref() {
            stream
                .write_all(body)
                .await
                .context("failed writing cached static response body")
        } else {
            send_static_file_fast(
                stream,
                &cached.file_path,
                cached.len,
                cached.sendfile.clone(),
                cooperative_mid_yield,
            )
            .await
            .map(|_| ())
        }
    } else {
        Ok(())
    };

    #[cfg(target_os = "linux")]
    if cork_static {
        set_tcp_cork(stream, false);
    }
    header_result.and(body_result)
}

async fn send_static_file_fast(
    stream: &mut TcpStream,
    path: &Path,
    _len: u64,
    sendfile: Option<Arc<std::fs::File>>,
    cooperative_mid_yield: bool,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let file = match sendfile {
            Some(file) => file,
            None => Arc::new(
                std::fs::File::open(path).context("failed opening static file for sendfile")?,
            ),
        };
        sendfile_all_async(stream, file.as_raw_fd(), _len, cooperative_mid_yield)
            .await
            .context("sendfile static response failed")?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = sendfile;
        let _ = cooperative_mid_yield;
        let mut file = tokio::fs::File::open(path)
            .await
            .context("failed opening static file for fast copy")?;
        tokio::io::copy(&mut file, stream)
            .await
            .context("failed copying static fast path file")?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
async fn sendfile_all_async(
    stream: &TcpStream,
    in_fd: std::os::fd::RawFd,
    len: u64,
    cooperative_mid_yield: bool,
) -> std::io::Result<u64> {
    if len == 0 {
        return Ok(0);
    }
    let out_fd = stream.as_raw_fd();
    let mut offset: libc::off_t = 0;
    let mut sent = 0_u64;
    let mut bytes_since_cooperative_yield = 0_u64;
    let configured_chunk_bytes = STATIC_SENDFILE_MAX_CHUNK_BYTES.load(Ordering::Relaxed);
    let max_chunk_bytes = if cooperative_mid_yield {
        configured_chunk_bytes.min(STATIC_SENDFILE_BALANCED_FAIR_CHUNK_BYTES)
    } else {
        configured_chunk_bytes
    };
    let data_plane_cores = adaptive_data_plane_workers(1);

    if STATIC_SENDFILE_REACTOR_ENABLED.load(Ordering::Relaxed) {
        match crate::sendfile_reactor::dispatch(
            out_fd,
            in_fd,
            0,
            len,
            max_chunk_bytes,
            data_plane_cores,
            STATIC_SENDFILE_REACTOR_NICE.load(Ordering::Relaxed),
        ) {
            Ok(completion) => {
                return completion.await.map_err(|_| {
                    std::io::Error::other("sendfile reactor stopped before job completion")
                })?;
            }
            Err(error) => {
                tracing::debug!(
                    ?error,
                    "sendfile reactor unavailable; using Tokio readiness"
                );
            }
        }
    }

    while sent < len {
        let remaining = len - sent;
        let count = remaining.min(max_chunk_bytes) as usize;
        let written = stream
            .async_io(tokio::io::Interest::WRITABLE, || {
                let mut batch_written = 0_usize;
                loop {
                    let written = unsafe {
                        libc::sendfile(out_fd, in_fd, &mut offset, count - batch_written)
                    };
                    if written > 0 {
                        batch_written = batch_written.saturating_add(written as usize);
                        if batch_written >= count {
                            return Ok(batch_written);
                        }
                        continue;
                    }
                    if written == 0 {
                        return Ok(batch_written);
                    }

                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    if error.kind() == std::io::ErrorKind::WouldBlock && batch_written > 0 {
                        return Ok(batch_written);
                    }
                    return Err(error);
                }
            })
            .await?;
        if written == 0 {
            break;
        }
        sent = sent.saturating_add(written as u64);
        bytes_since_cooperative_yield =
            bytes_since_cooperative_yield.saturating_add(written as u64);
        if sent < len {
            if STATIC_SENDFILE_QOS_ENABLED.load(Ordering::Relaxed) {
                tokio::time::sleep(STATIC_SENDFILE_QOS_DELAY).await;
            } else if cooperative_mid_yield && bytes_since_cooperative_yield >= max_chunk_bytes {
                bytes_since_cooperative_yield = 0;
                tokio::task::yield_now().await;
            }
        }
    }

    Ok(sent)
}

async fn bind_tcp_listener(bind_addr: SocketAddr, label: &str) -> Result<TcpListener> {
    let socket = if bind_addr.is_ipv4() {
        TcpSocket::new_v4()
    } else {
        TcpSocket::new_v6()
    }
    .with_context(|| format!("failed creating socket for {label} {bind_addr}"))?;

    socket
        .set_reuseaddr(true)
        .with_context(|| format!("failed setting SO_REUSEADDR for {label} {bind_addr}"))?;

    // SO_REUSEPORT: kernel-level load balancing across multiple accept loops
    // on multi-core systems.  Critical for TCP stream / game-long-connection perf.
    #[cfg(target_os = "linux")]
    {
        let fd = socket.as_raw_fd();
        let enable: libc::c_int = 1;
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_REUSEPORT,
                &enable as *const _ as *const libc::c_void,
                std::mem::size_of_val(&enable) as libc::socklen_t,
            )
        };
        if rc != 0 {
            tracing::warn!(label = %label, bind = %bind_addr, "SO_REUSEPORT failed (non-fatal)");
        }
    }

    socket
        .bind(bind_addr)
        .with_context(|| format!("failed to bind {label} {bind_addr}"))?;
    let listener = socket
        .listen(TCP_LISTEN_BACKLOG)
        .with_context(|| format!("failed to listen on {label} {bind_addr}"))?;

    // TCP_FASTOPEN on the listener: reduces handshake RTT for repeat clients
    #[cfg(target_os = "linux")]
    {
        let fd = listener.as_raw_fd();
        let tfo_queue: libc::c_int = 5;
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_FASTOPEN,
                &tfo_queue as *const _ as *const libc::c_void,
                std::mem::size_of_val(&tfo_queue) as libc::socklen_t,
            )
        };
        if rc != 0 {
            tracing::debug!(label = %label, "TCP_FASTOPEN on listener failed (non-fatal)");
        }
    }

    Ok(listener)
}

async fn bind_udp_listener_socket(bind_addr: SocketAddr, label: &str) -> Result<UdpSocket> {
    let socket = Socket::new(
        Domain::for_address(bind_addr),
        Type::DGRAM,
        Some(SocketProtocol::UDP),
    )
    .with_context(|| format!("failed creating udp socket for {label} {bind_addr}"))?;

    socket
        .set_reuse_address(true)
        .with_context(|| format!("failed setting SO_REUSEADDR for {label} {bind_addr}"))?;

    #[cfg(target_os = "linux")]
    {
        if let Err(error) = socket.set_reuse_port(true) {
            tracing::warn!(?error, label = %label, bind = %bind_addr, "SO_REUSEPORT failed for udp listener (non-fatal)");
        }
    }

    socket
        .bind(&bind_addr.into())
        .with_context(|| format!("failed to bind {label} {bind_addr}"))?;
    socket.set_nonblocking(true).with_context(|| {
        format!("failed setting udp listener nonblocking for {label} {bind_addr}")
    })?;
    let std_socket: std::net::UdpSocket = socket.into();
    UdpSocket::from_std(std_socket)
        .with_context(|| format!("failed creating tokio udp socket for {label} {bind_addr}"))
        .inspect(tune_udp_socket_for_gateway)
}

fn plain_http_accept_worker_count(config: &GatewayConfig) -> usize {
    if !cfg!(target_os = "linux") || !config.runtime.performance.enabled {
        return 1;
    }
    http_data_plane_workers_for(adaptive_data_plane_workers(1))
}

fn udp_listener_worker_count(config: &GatewayConfig) -> usize {
    udp_listener_worker_count_for(config, adaptive_data_plane_workers(1))
}

fn udp_listener_worker_count_for(config: &GatewayConfig, available_parallelism: usize) -> usize {
    if !cfg!(target_os = "linux") || !config.runtime.performance.enabled {
        return 1;
    }
    available_parallelism.max(1)
}

fn tcp_stream_accept_worker_count(config: &GatewayConfig, listener: &TcpListenerConfig) -> usize {
    tcp_stream_accept_worker_count_for(config, listener, adaptive_data_plane_workers(1))
}

fn tcp_stream_accept_worker_count_for(
    config: &GatewayConfig,
    listener: &TcpListenerConfig,
    available_parallelism: usize,
) -> usize {
    if !cfg!(target_os = "linux") || !config.runtime.performance.enabled {
        return 1;
    }
    if listener.name == "ftp" && config.services.ftp.native_control {
        return 1;
    }
    adaptive_stream_runtime_workers_for(available_parallelism)
}

fn adaptive_data_plane_workers(min_workers: usize) -> usize {
    #[cfg(target_os = "linux")]
    {
        return data_plane_cpu_ids().len().max(min_workers.max(1));
    }

    #[cfg(not(target_os = "linux"))]
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(min_workers.max(1))
        .max(min_workers.max(1))
}

#[cfg(any(test, target_os = "linux"))]
fn realtime_stream_reactor_workers_for(cores: usize, cpu_divisor: usize) -> usize {
    // Keep native relay ownership proportional to the allowed cpuset without
    // running a fixed worker cap. The profile-selected divisor scales with the
    // full cpuset, and balanced mode uses a low-CFS-weight per-core lane to
    // avoid queueing unrelated long connections behind one shared owner.
    cores.max(1).div_ceil(cpu_divisor.max(1))
}

#[cfg(any(test, target_os = "linux"))]
fn realtime_stream_reactor_cpu_divisor(profile: RuntimePerformanceTrafficProfile) -> usize {
    match profile {
        RuntimePerformanceTrafficProfile::Small => 2,
        RuntimePerformanceTrafficProfile::Balanced => 4,
        RuntimePerformanceTrafficProfile::Bulk => 4,
    }
}

#[cfg(any(test, target_os = "linux"))]
fn realtime_stream_reactor_nice_for(profile: RuntimePerformanceTrafficProfile) -> i32 {
    match profile {
        RuntimePerformanceTrafficProfile::Small => 0,
        // One movable owner per four CPUs avoids a permanently-runnable CFS
        // sibling on every HTTP shard. The count still scales with the full
        // cpuset, and fd-indexed slots keep each owner's queue inexpensive.
        RuntimePerformanceTrafficProfile::Balanced => 0,
        RuntimePerformanceTrafficProfile::Bulk => 5,
    }
}

fn http_data_plane_workers_for(cores: usize) -> usize {
    cores.max(1)
}

#[cfg(any(test, target_os = "linux"))]
fn shared_udp_runtime_profile(profile: RuntimePerformanceTrafficProfile) -> bool {
    matches!(profile, RuntimePerformanceTrafficProfile::Balanced)
}

#[cfg(any(test, target_os = "linux"))]
fn tls_http_runtime_cpu_divisor(profile: RuntimePerformanceTrafficProfile) -> usize {
    match profile {
        RuntimePerformanceTrafficProfile::Small => 1,
        RuntimePerformanceTrafficProfile::Balanced => 2,
        RuntimePerformanceTrafficProfile::Bulk => 4,
    }
}

fn tls_http_runtime_workers_for(cores: usize, cpu_divisor: usize) -> usize {
    cores.max(1).div_ceil(cpu_divisor.max(1))
}

#[cfg(any(test, target_os = "linux"))]
fn tls_http_runtime_nice_for(profile: RuntimePerformanceTrafficProfile) -> i32 {
    match profile {
        RuntimePerformanceTrafficProfile::Small | RuntimePerformanceTrafficProfile::Balanced => 0,
        RuntimePerformanceTrafficProfile::Bulk => 5,
    }
}

#[cfg(any(test, target_os = "linux"))]
fn udp_runtime_cpu_divisor(profile: RuntimePerformanceTrafficProfile) -> usize {
    match profile {
        RuntimePerformanceTrafficProfile::Small => 1,
        RuntimePerformanceTrafficProfile::Balanced => 2,
        RuntimePerformanceTrafficProfile::Bulk => 4,
    }
}

fn udp_runtime_workers_for(cores: usize, cpu_divisor: usize) -> usize {
    cores.max(1).div_ceil(cpu_divisor.max(1))
}

fn plain_fast_lane_fairness_batch_for(active_connections: usize) -> usize {
    if active_connections < PLAIN_FAST_LANE_HIGH_DENSITY_CONNECTIONS {
        PLAIN_FAST_LANE_LOW_DENSITY_BATCH
    } else {
        PLAIN_FAST_LANE_FAIRNESS_BATCH
    }
}

fn plain_fast_lane_should_yield(served_since_yield: usize) -> bool {
    served_since_yield >= PLAIN_FAST_LANE_LOW_DENSITY_BATCH
        && served_since_yield
            >= plain_fast_lane_fairness_batch_for(
                PLAIN_HTTP_CONNECTIONS_ACTIVE.load(Ordering::Relaxed),
            )
}

#[cfg(any(test, target_os = "linux"))]
fn udp_runtime_nice_for(profile: RuntimePerformanceTrafficProfile) -> i32 {
    match profile {
        RuntimePerformanceTrafficProfile::Small => 0,
        RuntimePerformanceTrafficProfile::Balanced => 12,
        RuntimePerformanceTrafficProfile::Bulk => 12,
    }
}

fn balanced_sendfile_mid_yield_for_next_response(sequence: &mut usize, enabled: bool) -> bool {
    if !enabled {
        return false;
    }
    *sequence = sequence.wrapping_add(1);
    !sequence.is_multiple_of(3)
}

fn balanced_sendfile_response_sequence_seed(remote_addr: SocketAddr) -> usize {
    // Linux ephemeral source ports advance across the accept fanout. Spread
    // the three response phases across connections so large-file owners do
    // not all hit the 8 MiB cooperative yield in the same scheduler wave.
    usize::from(remote_addr.port()) % 3
}

#[cfg(any(test, target_os = "linux"))]
fn sendfile_reactor_profile_enabled(profile: RuntimePerformanceTrafficProfile) -> bool {
    matches!(profile, RuntimePerformanceTrafficProfile::Bulk)
}

#[cfg(target_os = "linux")]
fn realtime_stream_reactor_workers() -> usize {
    realtime_stream_reactor_workers_for(
        adaptive_data_plane_workers(1),
        REALTIME_STREAM_REACTOR_CPU_DIVISOR.load(Ordering::Relaxed),
    )
}

#[cfg(target_os = "linux")]
fn realtime_stream_reactor_nice() -> i32 {
    REALTIME_STREAM_REACTOR_NICE.load(Ordering::Relaxed)
}

fn adaptive_stream_runtime_workers_for(cores: usize) -> usize {
    // TCP/game streams are first-class and nginx uses every configured worker
    // for SO_REUSEPORT stream listeners. Keep accept fanout and the dedicated
    // relay runtime proportional to every detected core as connection and tick
    // rates rise.
    cores.max(1)
}

fn direct_tcp_listener_upstream(config: &GatewayConfig, listener_name: &str) -> Option<String> {
    if !direct_tcp_fast_path_allowed(config) || listener_name.starts_with("stream|") {
        return None;
    }
    if listener_name == "ftp" && config.services.ftp.native_control {
        return None;
    }
    let listener = config
        .tcp
        .listeners
        .iter()
        .find(|listener| listener.name == listener_name)?;
    if !listener.upstream_weights.is_empty() {
        return None;
    }
    single_direct_tcp_upstream(&listener.upstream, &listener.upstreams)
}

fn direct_udp_listener_upstream(config: &GatewayConfig, listener_name: &str) -> Option<String> {
    if !direct_tcp_fast_path_allowed(config) {
        return None;
    }
    let listener = config
        .udp
        .listeners
        .iter()
        .find(|listener| listener.name == listener_name)?;
    if !listener.upstream_weights.is_empty() {
        return None;
    }
    single_direct_tcp_upstream(&listener.upstream, &listener.upstreams)
}

fn direct_tcp_route_upstream(config: &GatewayConfig, route: &RouteDecision) -> Option<String> {
    if !direct_tcp_fast_path_allowed(config) || !route.upstream_weights.is_empty() {
        return None;
    }
    if !route.set_headers.is_empty()
        || !route.strip_headers.is_empty()
        || route.rewrite_path.is_some()
        || route.status.is_some()
        || route.content_type.is_some()
    {
        return None;
    }
    single_direct_tcp_upstream(&route.upstream, &route.upstreams)
}

fn direct_tcp_fast_path_allowed(config: &GatewayConfig) -> bool {
    config.runtime.performance.enabled
        && !config.affinity.enabled
        && !config.load_balance.active_health.enabled
        && !config.load_balance.passive_health.enabled
}

fn single_direct_tcp_upstream(upstream: &str, upstreams: &[String]) -> Option<String> {
    let mut selected: Option<&str> = None;
    for candidate in std::iter::once(upstream)
        .chain(upstreams.iter().map(String::as_str))
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        match selected {
            Some(existing) if existing != candidate => return None,
            Some(_) => {}
            None => selected = Some(candidate),
        }
    }
    selected.map(str::to_string)
}

fn streaming_body<S>(stream: S) -> GatewayBody
where
    S: futures::Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let stream = stream
        .map_ok(Frame::data)
        .map_err(|error| anyhow!("upstream response stream failed: {error}"));
    GatewayBody::Stream(StreamBody::new(stream).boxed_unsync())
}

fn file_streaming_body<R>(file: R) -> GatewayBody
where
    R: AsyncRead + Send + 'static,
{
    // 1MB buffer: large static files (game hot-update, CDN assets) benefit from
    // fewer read syscalls and better kernel readahead alignment on Linux.
    let stream = ReaderStream::with_capacity(file, 1024 * 1024)
        .map_ok(Frame::data)
        .map_err(|error| anyhow!("static file stream failed: {error}"));
    GatewayBody::Stream(StreamBody::new(stream).boxed_unsync())
}

struct RawSseStreamState {
    upstream: BoxedProxyIo,
    leftover: Option<Bytes>,
    buffer: PooledBuffer,
}

fn raw_sse_streaming_body(upstream: BoxedProxyIo, leftover: Option<Bytes>) -> GatewayBody {
    let state = RawSseStreamState {
        upstream,
        leftover,
        buffer: relay_buffer_pool().acquire(),
    };
    let stream = futures::stream::try_unfold(state, |mut state| async move {
        // Forward the head-read leftover first, then stream upstream reads as
        // they arrive with NO event-boundary buffering (nginx `proxy_buffering
        // off`): lowest first-token and inter-token latency for AI SSE. Upstream
        // EOF ends the stream, which naturally covers `data: [DONE]` + close.
        if let Some(leftover) = state.leftover.take() {
            if !leftover.is_empty() {
                return Ok(Some((leftover, state)));
            }
        }

        let read = state
            .upstream
            .read(&mut state.buffer)
            .await
            .context("raw SSE upstream stream read failed")?;
        if read == 0 {
            return Ok::<_, anyhow::Error>(None);
        }
        let chunk = Bytes::copy_from_slice(&state.buffer[..read]);
        Ok(Some((chunk, state)))
    })
    .map_ok(Frame::data);
    GatewayBody::Stream(StreamBody::new(stream).boxed_unsync())
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
            "critical_task_failures_total": self.critical_task_failures_total.load(Ordering::Relaxed),
            "watchdog_heartbeat_total": self.watchdog_heartbeat_total.load(Ordering::Relaxed),
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
            (
                "proxysss_critical_task_failures_total",
                "Critical gateway background tasks that exited or failed",
                self.critical_task_failures_total.load(Ordering::Relaxed),
            ),
            (
                "proxysss_watchdog_heartbeat_total",
                "Runtime watchdog heartbeat ticks",
                self.watchdog_heartbeat_total.load(Ordering::Relaxed),
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

#[cfg(target_os = "linux")]
fn current_process_memory_bytes() -> Option<u64> {
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return None;
    }
    Some(resident_pages.saturating_mul(page_size as u64))
}

#[cfg(target_os = "macos")]
fn current_process_memory_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let resident_bytes = unsafe { usage.assume_init() }.ru_maxrss;
    (resident_bytes > 0).then_some(resident_bytes as u64)
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

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "macos")]
fn total_system_memory_bytes() -> Option<u64> {
    let name = b"hw.memsize\0";
    let mut memory_bytes = 0_u64;
    let mut size = std::mem::size_of::<u64>();
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr().cast(),
            (&mut memory_bytes as *mut u64).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || size != std::mem::size_of::<u64>() || memory_bytes == 0 {
        return None;
    }
    Some(memory_bytes)
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

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
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
    let mut http_connector = HttpConnector::new();
    http_connector.set_nodelay(true);
    http_connector.set_keepalive(Some(Duration::from_secs(90)));
    http_connector.enforce_http(true);
    let http_fast_client = HyperClient::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(4096)
        .build(http_connector);

    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(config.http.allow_insecure_upstreams)
        .tcp_nodelay(true)
        .pool_max_idle_per_host(4096)
        .pool_idle_timeout(Some(Duration::from_secs(90)))
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .http2_adaptive_window(true)
        .http2_initial_stream_window_size(HTTP2_STREAM_WINDOW_SIZE_BYTES)
        .http2_initial_connection_window_size(HTTP2_CONNECTION_WINDOW_SIZE_BYTES)
        .http2_keep_alive_timeout(Duration::from_secs(HTTP2_KEEP_ALIVE_TIMEOUT_SECS))
        .http2_keep_alive_interval(Some(Duration::from_secs(HTTP2_KEEP_ALIVE_INTERVAL_SECS)))
        .http2_keep_alive_while_idle(true)
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
        fast_lane: FastLaneState::compile(&config),
        config,
        http_client: client,
        http_fast_client,
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
            protocol: "ftp".to_string(),
            nodelay: true,
            connect_timeout_ms: 3_000,
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
            protocol: routes
                .first()
                .map(|route| route.protocol.clone())
                .unwrap_or_default(),
            nodelay: true,
            connect_timeout_ms: 3_000,
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
    // Rustls otherwise keeps only a small process-local stateful resumption
    // cache. Stateless, automatically rotated tickets let high-churn WSS/API
    // clients resume across every SO_REUSEPORT accept worker without a shared
    // hot-path lock or a cache sized in proportion to connection cardinality.
    // This also matches production nginx's default TLS ticket behavior.
    server_config.ticketer = rustls::crypto::aws_lc_rs::Ticketer::new()
        .context("failed initializing rotating TLS session tickets")?;
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
    let provider = rustls::crypto::aws_lc_rs::default_provider();
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
    let provider = rustls::crypto::aws_lc_rs::default_provider();
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
        let contact = (!tls.acme.email.trim().is_empty())
            .then(|| format!("mailto:{}", tls.acme.email.trim()));
        let contacts = contact
            .as_deref()
            .map(|value| vec![value])
            .unwrap_or_default();
        if contacts.is_empty() {
            tracing::warn!(
                "creating managed ACME account without a contact email; certificate issuance remains automatic but expiry/security notices cannot be delivered"
            );
        }
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
    build_upstream_url_with_rewrite(base_upstream, route.rewrite_path.as_deref(), uri)
}

fn build_upstream_url_with_rewrite(
    base_upstream: &str,
    rewrite_path: Option<&str>,
    uri: &Uri,
) -> Result<Url> {
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

    let rewritten = rewrite_path.map(str::to_string).unwrap_or_else(|| {
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

fn build_upstream_http_target(
    base_upstream: &str,
    route: &RouteDecision,
    uri: &Uri,
) -> Result<UpstreamHttpTarget> {
    build_upstream_http_target_with_rewrite(base_upstream, route.rewrite_path.as_deref(), uri)
}

fn build_upstream_http_target_with_rewrite(
    base_upstream: &str,
    rewrite_path: Option<&str>,
    uri: &Uri,
) -> Result<UpstreamHttpTarget> {
    if let Some(authority) = http_upstream_authority(base_upstream) {
        let path_and_query = upstream_path_and_query(rewrite_path, uri);
        let mut target =
            String::with_capacity("http://".len() + authority.len() + path_and_query.len());
        target.push_str("http://");
        target.push_str(authority);
        target.push_str(&path_and_query);
        let upstream_uri: Uri = target
            .parse()
            .with_context(|| format!("invalid upstream uri {}", base_upstream))?;
        return Ok(UpstreamHttpTarget::Hyper(upstream_uri));
    }

    build_upstream_url_with_rewrite(base_upstream, rewrite_path, uri)
        .map(UpstreamHttpTarget::Reqwest)
}

fn http_upstream_authority(base_upstream: &str) -> Option<&str> {
    let value = base_upstream.trim();
    if value.starts_with("https://")
        || value.starts_with("ws://")
        || value.starts_with("wss://")
        || (!value.starts_with("http://") && value.contains("://"))
    {
        return None;
    }

    let authority = value
        .strip_prefix("http://")
        .unwrap_or(value)
        .split('/')
        .next()
        .unwrap_or_default();
    if authority.is_empty() {
        return None;
    }

    Some(authority)
}

fn upstream_path_and_query(rewrite_path: Option<&str>, uri: &Uri) -> String {
    let rewritten = rewrite_path.unwrap_or_else(|| {
        uri.path_and_query()
            .map(|value| value.as_str())
            .unwrap_or_else(|| uri.path())
    });
    match rewritten.split_once('?') {
        Some((path, query)) => format!("{}?{}", upstream_path(path), query),
        None => match uri.query() {
            Some(query) => format!("{}?{}", upstream_path(rewritten), query),
            None => upstream_path(rewritten),
        },
    }
}

fn upstream_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn build_upstream_headers(
    original: &HeaderMap,
    route: &RouteDecision,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
    forward_headers: bool,
) -> Result<HeaderMap> {
    let forwarding_capacity = if forward_headers { 6 } else { 1 };
    let mut headers =
        HeaderMap::with_capacity(original.len() + route.set_headers.len() + forwarding_capacity);

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
    if forward_headers {
        apply_forwarding_headers(&mut headers, host, remote_addr, scheme)?;
    }

    Ok(headers)
}

fn build_simple_upstream_headers(
    original: &HeaderMap,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
    forward_headers: bool,
) -> Result<HeaderMap> {
    let forwarding_capacity = if forward_headers { 6 } else { 1 };
    let mut headers = HeaderMap::with_capacity(original.len() + forwarding_capacity);

    for (name, value) in original {
        if is_hop_header(name.as_str()) || name == HOST {
            continue;
        }
        headers.append(name.clone(), value.clone());
    }

    headers.insert(
        HOST,
        HeaderValue::from_str(host).context("invalid host header")?,
    );
    if forward_headers {
        apply_forwarding_headers(&mut headers, host, remote_addr, scheme)?;
    }

    Ok(headers)
}

fn build_empty_simple_upstream_headers(
    original: &HeaderMap,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
    forward_headers: bool,
) -> Result<HeaderMap> {
    let forwarding_capacity = if forward_headers { 6 } else { 1 };
    let mut headers = HeaderMap::with_capacity(original.len() + forwarding_capacity);

    for (name, value) in original {
        if is_hop_header(name.as_str()) || name == HOST || name == CONTENT_LENGTH {
            continue;
        }
        headers.append(name.clone(), value.clone());
    }

    headers.insert(
        HOST,
        HeaderValue::from_str(host).context("invalid host header")?,
    );
    if forward_headers {
        apply_forwarding_headers(&mut headers, host, remote_addr, scheme)?;
    }

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
    if !response_content_type_is(response, "text/event-stream") {
        return Ok(());
    }
    append_header_token_once(&mut response.headers, CACHE_CONTROL, "no-cache")?;
    append_header_token_once(&mut response.headers, CACHE_CONTROL, "no-transform")?;
    response
        .headers
        .retain(|(name, _)| *name != CONTENT_LENGTH && *name != CONTENT_ENCODING);
    set_header(
        &mut response.headers,
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    Ok(())
}

fn apply_streaming_response_headers_map(headers: &mut HeaderMap) -> Result<()> {
    if !upstream_response_is_sse(headers) {
        return Ok(());
    }

    append_header_token_once_map(headers, CACHE_CONTROL, "no-cache")?;
    append_header_token_once_map(headers, CACHE_CONTROL, "no-transform")?;
    headers.remove(CONTENT_LENGTH);
    headers.remove(CONTENT_ENCODING);
    headers.insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    Ok(())
}

fn append_header_token_once_map(
    headers: &mut HeaderMap,
    name: HeaderName,
    value: &str,
) -> Result<()> {
    let existing_name = name.clone();
    if let Some(existing) = headers.get_mut(&existing_name) {
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
        *existing = HeaderValue::from_str(merged.trim())
            .with_context(|| format!("invalid header value for {}", name.as_str()))?;
        return Ok(());
    }
    let header_value = HeaderValue::from_str(value)
        .with_context(|| format!("invalid header value for {}", name.as_str()))?;
    headers.insert(name, header_value);
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
        && response.status != StatusCode::PARTIAL_CONTENT
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
        let candidate_addr = SocketAddr::new(candidate_ip, port);
        match bind_tcp_listener(candidate_addr, "ftp data listener").await {
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
            copy_bidirectional_with_pooled_buffers(
                &mut downstream_data,
                &mut upstream_data,
                relay_buffer_pool(),
            )
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
            copy_bidirectional_with_pooled_buffers(
                &mut server_data,
                &mut client_data,
                relay_buffer_pool(),
            )
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
    append_csv_header_value(existing.and_then(|value| value.to_str().ok()), next)
}

fn append_forwarded_header(
    existing: Option<&HeaderValue>,
    remote_ip: &str,
    host: &str,
    scheme: &str,
) -> String {
    append_forwarded_header_value(
        existing.and_then(|value| value.to_str().ok()),
        remote_ip,
        host,
        scheme,
    )
}

fn append_csv_header_value(existing: Option<&str>, next: &str) -> String {
    match existing.map(str::trim) {
        Some(value) if !value.is_empty() => format!("{value}, {next}"),
        _ => next.to_string(),
    }
}

fn append_forwarded_header_value(
    existing: Option<&str>,
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
    match existing.map(str::trim) {
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
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if cfg
                .query_keys
                .iter()
                .any(|target| target.eq_ignore_ascii_case(key.as_ref()))
            {
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
        for chunk in cookie.split(';') {
            let trimmed = chunk.trim();
            if let Some((name, value)) = trimmed.split_once('=') {
                if cfg
                    .cookie_keys
                    .iter()
                    .any(|target| target.eq_ignore_ascii_case(name.trim()))
                {
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

fn udp_association_is_live(association: &UdpAssociation, session_ttl_secs: u64, now: u64) -> bool {
    association.active.load(Ordering::Relaxed)
        && now.saturating_sub(association.last_seen_epoch.load(Ordering::Relaxed))
            <= session_ttl_secs
}

const UDP_STATS_FLUSH_PACKETS: u64 = 1024;

fn spawn_udp_association_reader(
    send_socket: Arc<UdpSocket>,
    read_socket: Arc<UdpSocket>,
    associations_for_reader: Arc<DashMap<SocketAddr, Arc<UdpAssociation>>>,
    association_for_reader: Arc<UdpAssociation>,
    client_addr: SocketAddr,
    session_ttl_secs: u64,
    lease: UpstreamLease,
) {
    tokio::spawn(async move {
        let _lease = lease;
        let mut response = udp_buffer_pool().acquire();
        let check_interval = Duration::from_secs(session_ttl_secs.clamp(1, 30));
        'reader: loop {
            match tokio::time::timeout(check_interval, read_socket.recv(&mut response)).await {
                Ok(Ok(size)) => {
                    if let Err(error) =
                        send_udp_to(&send_socket, &response[..size], client_addr).await
                    {
                        tracing::warn!(?error, %client_addr, "failed relaying udp response to client");
                        association_for_reader
                            .active
                            .store(false, Ordering::Relaxed);
                        associations_for_reader.remove(&client_addr);
                        break;
                    }
                    loop {
                        match read_socket.try_recv(&mut response) {
                            Ok(size) => {
                                if let Err(error) =
                                    send_udp_to(&send_socket, &response[..size], client_addr).await
                                {
                                    tracing::warn!(?error, %client_addr, "failed relaying udp response to client");
                                    association_for_reader
                                        .active
                                        .store(false, Ordering::Relaxed);
                                    associations_for_reader.remove(&client_addr);
                                    break 'reader;
                                }
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(error) => {
                                tracing::warn!(?error, %client_addr, "udp upstream association closed");
                                association_for_reader
                                    .active
                                    .store(false, Ordering::Relaxed);
                                associations_for_reader.remove(&client_addr);
                                break 'reader;
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    tracing::warn!(?error, %client_addr, "udp upstream association closed");
                    association_for_reader
                        .active
                        .store(false, Ordering::Relaxed);
                    associations_for_reader.remove(&client_addr);
                    break;
                }
                Err(_) => {
                    let expired = associations_for_reader
                        .get(&client_addr)
                        .map(|entry| {
                            now_unix_secs()
                                .saturating_sub(entry.last_seen_epoch.load(Ordering::Relaxed))
                                > session_ttl_secs
                        })
                        .unwrap_or(true);
                    if expired {
                        association_for_reader
                            .active
                            .store(false, Ordering::Relaxed);
                        associations_for_reader.remove(&client_addr);
                        tracing::debug!(
                            %client_addr,
                            ttl_secs = session_ttl_secs,
                            "udp upstream association expired"
                        );
                        break;
                    }
                }
            }
        }
    });
}

fn flush_udp_stats(stats: &GatewayStats, packets: &mut u64, bytes: &mut u64) {
    if *packets > 0 {
        stats
            .udp_packets_total
            .fetch_add(*packets, Ordering::Relaxed);
        *packets = 0;
    }
    if *bytes > 0 {
        stats.udp_bytes_total.fetch_add(*bytes, Ordering::Relaxed);
        *bytes = 0;
    }
}

async fn send_udp_connected(socket: &UdpSocket, payload: &[u8]) -> std::io::Result<()> {
    match socket.try_send(payload) {
        Ok(written) if written == payload.len() => Ok(()),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "udp connected send wrote a partial datagram",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            let written = socket.send(payload).await?;
            if written == payload.len() {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "udp connected send wrote a partial datagram",
                ))
            }
        }
        Err(error) => Err(error),
    }
}

async fn send_udp_to(socket: &UdpSocket, payload: &[u8], addr: SocketAddr) -> std::io::Result<()> {
    match socket.try_send_to(payload, addr) {
        Ok(written) if written == payload.len() => Ok(()),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "udp send_to wrote a partial datagram",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            let written = socket.send_to(payload, addr).await?;
            if written == payload.len() {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "udp send_to wrote a partial datagram",
                ))
            }
        }
        Err(error) => Err(error),
    }
}

const UDP_ASSOCIATION_PRUNE_INTERVAL_SECS: u64 = 5;
const UDP_ASSOCIATION_PRUNE_CREATE_STRIDE: u64 = 256;

fn maybe_prune_udp_associations(
    associations: &DashMap<SocketAddr, Arc<UdpAssociation>>,
    prune_state: &UdpPruneState,
    session_ttl_secs: u64,
    max_associations: usize,
) {
    let create_count = prune_state
        .create_counter
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let association_count = associations.len();
    let cap_pressure =
        max_associations > 0 && association_count >= udp_prune_pressure_threshold(max_associations);
    let now = now_unix_secs();
    let last_prune = prune_state.last_prune_epoch.load(Ordering::Relaxed);
    let periodic = now.saturating_sub(last_prune) >= UDP_ASSOCIATION_PRUNE_INTERVAL_SECS
        || create_count.is_multiple_of(UDP_ASSOCIATION_PRUNE_CREATE_STRIDE);

    if !cap_pressure && !periodic {
        return;
    }
    if prune_state
        .pruning
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    prune_udp_associations(associations, session_ttl_secs, max_associations);
    prune_state.last_prune_epoch.store(now, Ordering::Relaxed);
    prune_state.pruning.store(false, Ordering::Release);
}

fn udp_prune_pressure_threshold(max_associations: usize) -> usize {
    if max_associations <= 10 {
        max_associations
    } else {
        max_associations.saturating_sub((max_associations / 20).max(1))
    }
}

fn prune_udp_associations(
    associations: &DashMap<SocketAddr, Arc<UdpAssociation>>,
    session_ttl_secs: u64,
    max_associations: usize,
) {
    let now = now_unix_secs();
    associations.retain(|_, association| {
        let keep = udp_association_is_live(association, session_ttl_secs, now);
        if !keep {
            association.active.store(false, Ordering::Relaxed);
        }
        keep
    });

    if max_associations == 0 || associations.len() < max_associations {
        return;
    }

    let mut oldest = associations
        .iter()
        .map(|entry| (*entry.key(), entry.last_seen_epoch.load(Ordering::Relaxed)))
        .collect::<Vec<_>>();
    oldest.sort_by_key(|(_, last_seen)| *last_seen);

    let remove_count = associations.len().saturating_sub(
        max_associations
            .saturating_sub(max_associations / 10)
            .max(1),
    );
    for (addr, _) in oldest.into_iter().take(remove_count) {
        if let Some((_, association)) = associations.remove(&addr) {
            association.active.store(false, Ordering::Relaxed);
        }
    }
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
    #[serde(default)]
    email: String,
    #[serde(default = "default_true")]
    production: bool,
    #[serde(default = "default_admin_auto_https_challenge")]
    challenge: AcmeChallengeType,
}

fn default_admin_auto_https_challenge() -> AcmeChallengeType {
    AcmeChallengeType::TlsAlpn01
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
        (
            serde_yaml::Value::String("challenge".to_string()),
            serde_yaml::to_value(payload.challenge)
                .expect("ACME challenge serialization cannot fail"),
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

fn remove_hop_headers_from_map(headers: &mut HeaderMap) {
    headers.remove(CONNECTION);
    headers.remove(HeaderName::from_static("keep-alive"));
    headers.remove(HeaderName::from_static("proxy-authenticate"));
    headers.remove(HeaderName::from_static("proxy-authorization"));
    headers.remove(HeaderName::from_static("te"));
    headers.remove(HeaderName::from_static("trailers"));
    headers.remove(TRANSFER_ENCODING);
    headers.remove(HeaderName::from_static("upgrade"));
    headers.remove(HeaderName::from_static("proxy-connection"));
}

fn should_stream_upstream_body(
    status: StatusCode,
    headers: &HeaderMap,
    version: &str,
    cache_key: Option<&str>,
    compression: &ResponseCompressionConfig,
) -> bool {
    if !status.is_success() || version == "HTTP/3" || cache_key.is_some() || compression.enabled {
        return false;
    }

    if upstream_response_is_sse(headers) {
        return true;
    }

    match upstream_content_length(headers) {
        Some(len) => len >= UPSTREAM_STREAM_THRESHOLD_BYTES,
        None => true,
    }
}

fn request_accepts_sse(headers: &HeaderMap) -> bool {
    headers
        .get_all(ACCEPT)
        .iter()
        .any(|value| value.to_str().ok().is_some_and(header_value_accepts_sse))
}

fn header_value_accepts_sse(raw: &str) -> bool {
    raw.split(',').any(|item| {
        item.split(';')
            .next()
            .map(|media_type| media_type.trim().eq_ignore_ascii_case("text/event-stream"))
            .unwrap_or(false)
    })
}

fn ai_proxy_fast_lane_path_matches(config: &GatewayConfig, path: &str) -> bool {
    config.services.ai_proxy.enabled
        && config.services.ai_proxy.routes.iter().any(|route| {
            ai_proxy_route_fast_path_eligible(route)
                && raw_http_pool_key_from_upstream(&route.upstream).is_some()
                && route_prefix_matches(&route.path_prefix, path)
        })
}

#[allow(dead_code)]
fn reverse_proxy_fast_lane_path_matches(config: &GatewayConfig, path: &str) -> bool {
    config.services.reverse_proxy.routes.iter().any(|route| {
        reverse_proxy_route_fast_path_eligible(route)
            && raw_http_pool_key_from_upstream(&route.upstream).is_some()
            && route_prefix_matches(&route.path_prefix, path)
    })
}

fn websocket_fast_lane_path_matches(config: &GatewayConfig, path: &str) -> bool {
    config.services.reverse_proxy.routes.iter().any(|route| {
        websocket_route_fast_path_eligible(route)
            && raw_websocket_pool_key_from_upstream(&route.upstream).is_some()
            && route_prefix_matches(&route.path_prefix, path)
    })
}

fn plain_raw_sse_fast_lane_matches(config: &GatewayConfig, request: &PlainFastLaneRequest) -> bool {
    if !request.accepts_sse {
        return false;
    }
    let host = request.host.as_deref().unwrap_or("localhost");
    config.services.ai_proxy.enabled
        && config.services.ai_proxy.routes.iter().any(|route| {
            ai_proxy_route_fast_path_eligible(route)
                && raw_http_pool_key_from_upstream(&route.upstream).is_some()
                && crate::ai_proxy::route_matches(route, host, &request.path)
        })
}

fn http_static_success_fast_path_allowed(config: &GatewayConfig, scheme: &str, uri: &Uri) -> bool {
    if !hyper_static_success_fast_path_globally_allowed(config) {
        return false;
    }

    if monitoring_path_matches(&config.monitoring, uri.path()) {
        return false;
    }

    if scheme == "http" && http_to_https_redirect_can_apply(config) {
        return false;
    }

    true
}

fn hyper_static_success_fast_path_globally_allowed(config: &GatewayConfig) -> bool {
    if config.logging.access_log
        || config.security.ddos.enabled
        || config.security.dynamic_blacklist.enabled
        || config.services.access_control.http.enabled
        || config.services.rate_limit.http.enabled
        || config.services.response_policy.compression.enabled
        || config.services.filecloud.enabled
        || config.services.webdav.enabled
        || config.services.static_sites.is_empty()
    {
        return false;
    }

    true
}

fn http_to_https_redirect_can_apply(config: &GatewayConfig) -> bool {
    if config.http.tls_bind.trim().is_empty() {
        return false;
    }

    matches!(
        config.http.tls.mode,
        TlsMode::AcmeManaged | TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal
    ) || !config.http.tls.certificates.is_empty()
        || config
            .services
            .domain_routes
            .iter()
            .any(|route| route.ssl.effective_mode() != DomainTlsMode::Disabled)
}

fn simple_http_proxy_fast_path_allowed(config: &GatewayConfig) -> bool {
    !config.logging.access_log
        && !config.script.enabled
        && !config.plugins.enabled
        && !config.security.ddos.enabled
        && !config.security.dynamic_blacklist.enabled
        && !config.services.access_control.http.enabled
        && !config.services.response_policy.compression.enabled
        && !config.services.response_policy.cache.enabled
        && !config.services.rate_limit.http.enabled
        && !config.load_balance.retries.enabled
        && !config.load_balance.active_health.enabled
        && !config.load_balance.passive_health.enabled
        && !config.affinity.enabled
}

fn request_host<'a>(headers: &'a HeaderMap, uri: &'a Uri) -> Cow<'a, str> {
    if let Some(host) = headers.get(HOST).and_then(|value| value.to_str().ok()) {
        return Cow::Borrowed(host);
    }

    if let Some(authority) = uri.authority() {
        return Cow::Borrowed(authority.as_str());
    }

    Cow::Borrowed("localhost")
}

fn reverse_proxy_route_fast_path_eligible(route: &ReverseProxyRouteConfig) -> bool {
    route.upstreams.is_empty()
        && route.upstream_weights.is_empty()
        && route.set_headers.is_empty()
        && route.strip_headers.is_empty()
        && !route.compression.enabled
        && !route.cache.enabled
        && !route.rate_limit.enabled
}

fn websocket_route_fast_path_eligible(route: &ReverseProxyRouteConfig) -> bool {
    route.set_headers.is_empty()
        && route.strip_headers.is_empty()
        && !route.compression.enabled
        && !route.cache.enabled
        && !route.rate_limit.enabled
        && {
            let candidates = if route.upstreams.is_empty() {
                std::slice::from_ref(&route.upstream)
            } else {
                route.upstreams.as_slice()
            };
            !candidates.is_empty()
                && candidates
                    .iter()
                    .all(|upstream| raw_websocket_pool_key_from_upstream(upstream).is_some())
        }
}

fn ai_proxy_route_fast_path_eligible(route: &crate::ai_proxy::AiProxyRouteConfig) -> bool {
    route.add_headers.is_empty() && route.strip_headers.is_empty()
}

fn reverse_proxy_rewrite_path(route: &ReverseProxyRouteConfig, uri: &Uri) -> Option<String> {
    route
        .strip_prefix
        .then(|| rewrite_path_with_prefix(&route.path_prefix, None, uri))
}

fn reverse_proxy_raw_path_and_query<'a>(
    route: &ReverseProxyRouteConfig,
    target: &'a str,
    path: &'a str,
) -> Cow<'a, str> {
    if !route.strip_prefix {
        return Cow::Borrowed(target);
    }

    let suffix = route_prefix_suffix(&route.path_prefix, path).unwrap_or(path);
    let rewritten_path = if suffix.is_empty() {
        Cow::Borrowed("/")
    } else if suffix.starts_with('/') {
        Cow::Borrowed(suffix)
    } else {
        Cow::Owned(format!("/{suffix}"))
    };

    match target.split_once('?').map(|(_, query)| query) {
        Some(query) => Cow::Owned(format!("{}?{query}", rewritten_path.as_ref())),
        None => rewritten_path,
    }
}

fn ai_proxy_rewrite_path(route: &crate::ai_proxy::AiProxyRouteConfig, uri: &Uri) -> Option<String> {
    let rewrite_base = route.rewrite_base_path.trim();
    if rewrite_base.is_empty() {
        None
    } else {
        Some(rewrite_path_with_prefix(
            &route.path_prefix,
            Some(rewrite_base),
            uri,
        ))
    }
}

fn ai_proxy_raw_rewrite_path(
    route: &crate::ai_proxy::AiProxyRouteConfig,
    target: &str,
    path: &str,
) -> Option<String> {
    let rewrite_base = route.rewrite_base_path.trim();
    if rewrite_base.is_empty()
        || rewrite_base.trim_end_matches('/') == route.path_prefix.trim_end_matches('/')
    {
        None
    } else {
        Some(rewrite_path_with_prefix_parts(
            &route.path_prefix,
            Some(rewrite_base),
            target,
            path,
        ))
    }
}

fn rewrite_path_with_prefix(prefix: &str, base: Option<&str>, uri: &Uri) -> String {
    rewrite_path_with_prefix_parts(
        prefix,
        base,
        uri.path_and_query()
            .map(|path_and_query| path_and_query.as_str())
            .unwrap_or(uri.path()),
        uri.path(),
    )
}

fn rewrite_path_with_prefix_parts(
    prefix: &str,
    base: Option<&str>,
    target: &str,
    path: &str,
) -> String {
    let suffix = route_prefix_suffix(prefix, path).unwrap_or(path);
    let path = if let Some(base) = base {
        let suffix = if suffix.is_empty() {
            "/"
        } else if suffix.starts_with('/') {
            suffix
        } else {
            path
        };
        format!("{}{}", base.trim_end_matches('/'), suffix)
    } else if suffix.is_empty() {
        "/".to_string()
    } else if suffix.starts_with('/') {
        suffix.to_string()
    } else {
        format!("/{suffix}")
    };

    match target.split_once('?').map(|(_, query)| query) {
        Some(query) => format!("{path}?{query}"),
        None => path,
    }
}

fn upstream_response_is_sse(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("text/event-stream"))
        .unwrap_or(false)
}

fn upstream_content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn status_has_no_body(status: StatusCode) -> bool {
    status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
}

fn raw_http_pool_keys_for_config(config: &GatewayConfig) -> HashSet<String> {
    let mut keys: HashSet<String> = config
        .services
        .reverse_proxy
        .routes
        .iter()
        .filter(|route| reverse_proxy_route_fast_path_eligible(route))
        .filter_map(|route| raw_http_pool_key_from_upstream(&route.upstream))
        .collect();
    if config.services.ai_proxy.enabled {
        keys.extend(
            config
                .services
                .ai_proxy
                .routes
                .iter()
                .filter(|route| ai_proxy_route_fast_path_eligible(route))
                .filter_map(|route| raw_http_pool_key_from_upstream(&route.upstream)),
        );
    }
    keys
}

fn raw_http_pool_key_from_upstream(upstream: &str) -> Option<String> {
    raw_http_pool_parts_from_upstream(upstream)
        .ok()
        .flatten()
        .map(|(key, _, _)| key)
}

fn raw_websocket_pool_key_from_upstream(upstream: &str) -> Option<String> {
    raw_websocket_pool_parts_from_upstream(upstream)
        .ok()
        .flatten()
        .map(|(key, _, _)| key)
}

fn raw_websocket_pool_parts_from_upstream(upstream: &str) -> Result<Option<(String, String, u16)>> {
    let Some(authority) = upstream.strip_prefix("ws://") else {
        return Ok(None);
    };
    if let Some(parts) = raw_http_pool_parts_from_authority(authority)? {
        return Ok(Some(parts));
    }
    let url = Url::parse(upstream).context("invalid raw websocket upstream url")?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("raw websocket upstream URL missing host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(80);
    Ok(Some((format!("{host}:{port}"), host, port)))
}

fn raw_http_pool_parts_from_upstream(upstream: &str) -> Result<Option<(String, String, u16)>> {
    let Some(authority) = upstream.strip_prefix("http://").or(Some(upstream)) else {
        return Ok(None);
    };
    if upstream.contains("://") && !upstream.starts_with("http://") {
        return Ok(None);
    }
    if let Some(parts) = raw_http_pool_parts_from_authority(authority)? {
        return Ok(Some(parts));
    }

    let url = if upstream.starts_with("http://") {
        Url::parse(upstream).context("invalid raw HTTP upstream url")?
    } else {
        Url::parse(&format!("http://{upstream}")).context("invalid raw HTTP upstream url")?
    };
    raw_http_pool_key(&url)
}

fn raw_http_pool_parts_from_authority(authority: &str) -> Result<Option<(String, String, u16)>> {
    if authority.is_empty()
        || authority
            .as_bytes()
            .iter()
            .any(|byte| matches!(byte, b'/' | b'?' | b'#' | b'@' | b'[' | b']'))
    {
        return Ok(None);
    }
    let colon_count = authority
        .as_bytes()
        .iter()
        .filter(|&&byte| byte == b':')
        .count();
    if colon_count > 1 {
        return Ok(None);
    }

    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .with_context(|| format!("invalid raw HTTP upstream port in {authority}"))?;
            (host, port)
        }
        None => (authority, 80),
    };
    if host.is_empty() {
        return Ok(None);
    }
    let host = host.to_string();
    Ok(Some((format!("{host}:{port}"), host, port)))
}

fn raw_http_pool_key(url: &Url) -> Result<Option<(String, String, u16)>> {
    if url.scheme() != "http" {
        return Ok(None);
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("raw HTTP upstream URL missing host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(80);
    Ok(Some((format!("{host}:{port}"), host, port)))
}

fn write_access_log_if_enabled(
    config: &GatewayConfig,
    request: &Request<Incoming>,
    response: &GatewayHttpResponse,
    remote_addr: SocketAddr,
    elapsed: Duration,
) {
    if !config.logging.access_log {
        return;
    }

    let request_id = Uuid::new_v4().to_string();
    if !should_sample(config.logging.access_sample_rate, &request_id) {
        return;
    }

    let slow = elapsed.as_millis() as u64 >= config.logging.slow_request_ms;
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

fn configured_http_route<'a>(
    config: &'a GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig<'a>> {
    configured_ai_proxy_route(config, host, uri)
        .or_else(|| configured_domain_route(config, host, uri))
        .or_else(|| configured_reverse_proxy_route(config, host, uri))
}

fn configured_ai_proxy_route<'a>(
    config: &'a GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig<'a>> {
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
        compression: Cow::Borrowed(&config.services.response_policy.compression),
        cache: disabled_ai_cache_policy(&config.services.response_policy.cache),
        rate_limit: Cow::Borrowed(&config.services.rate_limit.http),
        forward_headers: route.forward_headers,
    })
}

fn disabled_ai_cache_policy(base: &ResponseCacheConfig) -> Cow<'_, ResponseCacheConfig> {
    if base.enabled {
        Cow::Owned(ResponseCacheConfig {
            enabled: false,
            ..Default::default()
        })
    } else {
        Cow::Borrowed(base)
    }
}

fn configured_domain_route<'a>(
    config: &'a GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig<'a>> {
    config
        .services
        .domain_routes
        .iter()
        .filter(|route| domain_route_matches(route, host, uri.path()))
        .max_by_key(|route| route.path_prefix.len())
        .map(|route| domain_route_config(config, route, uri))
}

fn configured_reverse_proxy_route<'a>(
    config: &'a GatewayConfig,
    host: &str,
    uri: &Uri,
) -> Option<HttpRouteConfig<'a>> {
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
            forward_headers: route.forward_headers,
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
    route_prefix_matches(prefix, path)
}

fn route_prefix_matches(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim().trim_end_matches('/');
    if prefix.is_empty() {
        return path.starts_with('/');
    }

    let suffix = if prefix.starts_with('/') {
        path.strip_prefix(prefix)
    } else {
        path.strip_prefix('/')
            .and_then(|path_without_slash| path_without_slash.strip_prefix(prefix))
    };

    match suffix {
        Some("") => true,
        Some(rest) => rest.starts_with('/'),
        None => false,
    }
}

fn route_prefix_suffix<'a>(prefix: &str, path: &'a str) -> Option<&'a str> {
    let prefix = prefix.trim().trim_end_matches('/');
    if prefix.is_empty() {
        return Some(path);
    }

    if prefix.starts_with('/') {
        path.strip_prefix(prefix)
    } else {
        path.strip_prefix('/')
            .and_then(|path_without_slash| path_without_slash.strip_prefix(prefix))
    }
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
        let suffix = route_prefix_suffix(&route.path_prefix, uri.path()).unwrap_or(uri.path());
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

fn domain_route_config<'a>(
    config: &'a GatewayConfig,
    route: &'a DomainRouteConfig,
    uri: &Uri,
) -> HttpRouteConfig<'a> {
    HttpRouteConfig {
        runtime_scope: Some(route.name.clone()),
        decision: RouteDecision {
            upstream: route.upstream.clone(),
            upstreams: route.upstreams.clone(),
            upstream_weights: route.upstream_weights.clone(),
            affinity_key: None,
            rewrite_path: route.strip_prefix.then(|| {
                let suffix =
                    route_prefix_suffix(&route.path_prefix, uri.path()).unwrap_or(uri.path());
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
        forward_headers: route.forward_headers,
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
        udp_payload: base.udp_payload.clone(),
        udp_expect_response: base.udp_expect_response,
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
        udp_payload: base.udp_payload.clone(),
        udp_expect_response: base.udp_expect_response,
    }
}

fn merge_compression_policy<'a>(
    base: &'a ResponseCompressionConfig,
    override_policy: &'a ResponseCompressionConfig,
) -> Cow<'a, ResponseCompressionConfig> {
    if override_policy.enabled {
        Cow::Borrowed(override_policy)
    } else {
        Cow::Borrowed(base)
    }
}

fn merge_cache_policy<'a>(
    base: &'a ResponseCacheConfig,
    override_policy: &'a ResponseCacheConfig,
) -> Cow<'a, ResponseCacheConfig> {
    if override_policy.enabled {
        Cow::Borrowed(override_policy)
    } else {
        Cow::Borrowed(base)
    }
}

fn merge_rate_limit_policy<'a>(
    base: &'a HttpRateLimitConfig,
    override_policy: &'a HttpRateLimitConfig,
) -> Cow<'a, HttpRateLimitConfig> {
    if override_policy.enabled {
        Cow::Borrowed(override_policy)
    } else {
        Cow::Borrowed(base)
    }
}

async fn dispatch_static_site(
    site: &StaticSiteConfig,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    static_file_cache_bytes: &AtomicU64,
    static_file_load_locks: &DashMap<String, Arc<TokioMutex<()>>>,
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

    match parse_static_range_header(headers, Some(metadata.len())) {
        StaticRangeDecision::NoRange => {}
        StaticRangeDecision::Invalid | StaticRangeDecision::Unsatisfiable => {
            let mut response = GatewayHttpResponse::bytes(
                StatusCode::RANGE_NOT_SATISFIABLE,
                "text/plain; charset=utf-8",
                Bytes::from_static(b"range not satisfiable"),
                "proxysss://static",
            );
            response
                .headers
                .push((ACCEPT_RANGES, HeaderValue::from_static("bytes")));
            response.headers.push((
                CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{}", metadata.len()))
                    .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
            ));
            return Ok(response);
        }
        StaticRangeDecision::Range(range) => {
            return static_range_response(&target, &metadata, method, range).await;
        }
    }

    if let Some(response) = fresh_cached_static_file_response(&target, method, static_file_cache) {
        return Ok(response);
    }

    let (body, stream_body) = if method == Method::HEAD {
        (Bytes::new(), None)
    } else if metadata.len() >= STATIC_STREAM_THRESHOLD_BYTES {
        let file = tokio::fs::File::open(&target)
            .await
            .context("failed opening static file")?;
        (Bytes::new(), Some(file_streaming_body(file)))
    } else {
        (
            cached_static_file_body(
                &target,
                &metadata,
                static_file_cache,
                static_file_cache_bytes,
                static_file_load_locks,
            )
            .await?,
            None,
        )
    };
    let mut response = GatewayHttpResponse::bytes(
        StatusCode::OK,
        static_content_type(&target),
        body,
        "proxysss://static",
    );
    response.stream_body = stream_body;
    response.headers.push((
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    response
        .headers
        .push((ACCEPT_RANGES, HeaderValue::from_static("bytes")));
    Ok(response)
}

/// Inclusive byte range selected from an HTTP `Range: bytes=...` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticByteRange {
    start: u64,
    end: u64,
}

/// Parsed Range outcome for static file serving.
///
/// Unsupported units are treated as `NoRange` so normal GET semantics still
/// work. Malformed byte ranges and valid-but-outside-file ranges both produce
/// `416`, matching the behavior clients expect for resumable downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticRangeDecision {
    NoRange,
    Range(StaticByteRange),
    Invalid,
    Unsatisfiable,
}

/// Parse a single HTTP byte range for static assets.
///
/// proxysss intentionally supports one range per response. Multipart range
/// bodies add substantial response-building complexity and are rarely needed
/// for CDN origin, media seek, or interrupted-download resume paths.
fn parse_static_range_header(headers: &HeaderMap, file_len: Option<u64>) -> StaticRangeDecision {
    let Some(value) = headers.get(RANGE).and_then(|value| value.to_str().ok()) else {
        return StaticRangeDecision::NoRange;
    };
    let value = value.trim();
    let Some(spec) = value.strip_prefix("bytes=") else {
        return StaticRangeDecision::NoRange;
    };
    if spec.contains(',') {
        return StaticRangeDecision::Invalid;
    }
    let Some(file_len) = file_len else {
        return StaticRangeDecision::NoRange;
    };
    if file_len == 0 {
        return StaticRangeDecision::Unsatisfiable;
    }

    let Some((start_raw, end_raw)) = spec.split_once('-') else {
        return StaticRangeDecision::Invalid;
    };
    let start_raw = start_raw.trim();
    let end_raw = end_raw.trim();
    if start_raw.is_empty() && end_raw.is_empty() {
        return StaticRangeDecision::Invalid;
    }

    let (start, end) = if start_raw.is_empty() {
        let Ok(suffix_len) = end_raw.parse::<u64>() else {
            return StaticRangeDecision::Invalid;
        };
        if suffix_len == 0 {
            return StaticRangeDecision::Unsatisfiable;
        }
        let length = suffix_len.min(file_len);
        (file_len - length, file_len - 1)
    } else {
        let Ok(start) = start_raw.parse::<u64>() else {
            return StaticRangeDecision::Invalid;
        };
        if start >= file_len {
            return StaticRangeDecision::Unsatisfiable;
        }
        let end = if end_raw.is_empty() {
            file_len - 1
        } else {
            let Ok(end) = end_raw.parse::<u64>() else {
                return StaticRangeDecision::Invalid;
            };
            end.min(file_len - 1)
        };
        (start, end)
    };

    if start > end {
        return StaticRangeDecision::Unsatisfiable;
    }
    StaticRangeDecision::Range(StaticByteRange { start, end })
}

/// Build the `206 Partial Content` response for a static file range.
///
/// Small ranges are read into memory like other small static objects. Large
/// ranges seek once and stream only the requested bytes, preserving
/// backpressure and avoiding whole-file buffering for media/download traffic.
async fn static_range_response(
    target: &Path,
    metadata: &std::fs::Metadata,
    method: &Method,
    range: StaticByteRange,
) -> Result<GatewayHttpResponse> {
    let len = range.end - range.start + 1;
    let (body, stream_body) = if method == Method::HEAD {
        (Bytes::new(), None)
    } else if len >= STATIC_STREAM_THRESHOLD_BYTES {
        let mut file = tokio::fs::File::open(target)
            .await
            .context("failed opening static range file")?;
        file.seek(SeekFrom::Start(range.start))
            .await
            .context("failed seeking static range file")?;
        (Bytes::new(), Some(file_streaming_body(file.take(len))))
    } else {
        let bytes = tokio::fs::read(target)
            .await
            .context("failed reading static range file")?;
        let slice = bytes[range.start as usize..=range.end as usize].to_vec();
        (Bytes::from(slice), None)
    };

    let mut response = GatewayHttpResponse::bytes(
        StatusCode::PARTIAL_CONTENT,
        static_content_type(target),
        body,
        "proxysss://static-range",
    );
    response.stream_body = stream_body;
    response
        .headers
        .push((ACCEPT_RANGES, HeaderValue::from_static("bytes")));
    response.headers.push((
        CONTENT_RANGE,
        HeaderValue::from_str(&format!(
            "bytes {}-{}/{}",
            range.start,
            range.end,
            metadata.len()
        ))
        .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
    ));
    response.headers.push((
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    Ok(response)
}

async fn cached_static_file_body(
    target: &Path,
    metadata: &std::fs::Metadata,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    static_file_cache_bytes: &AtomicU64,
    static_file_load_locks: &DashMap<String, Arc<TokioMutex<()>>>,
) -> Result<Bytes> {
    let key = target.to_string_lossy().to_string();
    let modified = metadata.modified().ok();
    if let Some(body) = fresh_static_cache_body(&key, metadata, modified, static_file_cache) {
        return Ok(body);
    }
    evict_stale_static_cache_entry(
        &key,
        metadata,
        modified,
        static_file_cache,
        static_file_cache_bytes,
    );

    let load_lock = if metadata.len() <= STATIC_STREAM_THRESHOLD_BYTES {
        Some(
            static_file_load_locks
                .entry(key.clone())
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone(),
        )
    } else {
        None
    };
    let guard = match &load_lock {
        Some(lock) => Some(lock.lock().await),
        None => None,
    };

    if load_lock.is_some() {
        if let Some(body) = fresh_static_cache_body(&key, metadata, modified, static_file_cache) {
            drop(guard);
            static_file_load_locks.remove(&key);
            return Ok(body);
        }
        evict_stale_static_cache_entry(
            &key,
            metadata,
            modified,
            static_file_cache,
            static_file_cache_bytes,
        );
    }

    let body = if metadata.len() >= STATIC_MMAP_THRESHOLD_BYTES {
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(move || mmap_static_file_bytes(&target))
            .await
            .context("static mmap task failed")??
    } else {
        Bytes::from(
            tokio::fs::read(target)
                .await
                .context("failed reading static file")?,
        )
    };
    let body_len = body.len() as u64;
    if body_len <= STATIC_STREAM_THRESHOLD_BYTES
        && static_file_cache.len() < STATIC_FILE_CACHE_MAX_ENTRIES
    {
        let current = static_file_cache_bytes.load(Ordering::Relaxed);
        if current.saturating_add(body_len) <= STATIC_FILE_CACHE_MAX_BYTES {
            let sendfile = static_file_cache
                .get(&key)
                .and_then(|entry| entry.sendfile.clone());
            static_file_cache.insert(
                key.clone(),
                CachedStaticFile {
                    len: metadata.len(),
                    modified,
                    body: body.clone(),
                    sendfile,
                    content_type: HeaderValue::from_static(static_content_type(target)),
                    content_length: HeaderValue::from_str(&metadata.len().to_string())
                        .unwrap_or_else(|_| HeaderValue::from_static("0")),
                    checked_at: Instant::now(),
                    revalidating: false,
                },
            );
            static_file_cache_bytes.fetch_add(body_len, Ordering::Relaxed);
        }
    }
    drop(guard);
    if load_lock.is_some() {
        static_file_load_locks.remove(&key);
    }
    Ok(body)
}

fn mmap_static_file_bytes(target: &Path) -> Result<Bytes> {
    let file = std::fs::File::open(target)
        .with_context(|| format!("failed opening static file for mmap {}", target.display()))?;
    let mmap = unsafe {
        memmap2::MmapOptions::new()
            .map(&file)
            .with_context(|| format!("failed mmap static file {}", target.display()))?
    };
    Ok(Bytes::from_owner(mmap))
}

fn fresh_static_cache_body(
    key: &str,
    metadata: &std::fs::Metadata,
    modified: Option<SystemTime>,
    static_file_cache: &DashMap<String, CachedStaticFile>,
) -> Option<Bytes> {
    let mut entry = static_file_cache.get_mut(key)?;
    if entry.len == metadata.len()
        && entry.modified == modified
        && entry.body.len() as u64 == entry.len
    {
        entry.checked_at = Instant::now();
        entry.revalidating = false;
        return Some(entry.body.clone());
    }
    None
}

fn evict_stale_static_cache_entry(
    key: &str,
    metadata: &std::fs::Metadata,
    modified: Option<SystemTime>,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    static_file_cache_bytes: &AtomicU64,
) {
    let old_len = match static_file_cache.get(key) {
        Some(entry) if entry.len != metadata.len() || entry.modified != modified => {
            entry.body.len() as u64
        }
        _ => return,
    };
    static_file_cache.remove(key);
    static_file_cache_bytes.fetch_sub(old_len, Ordering::Relaxed);
}

struct CachedStaticResponse {
    response: GatewayResponse,
    revalidate: bool,
}

fn fresh_cached_static_file_response(
    target: &Path,
    method: &Method,
    static_file_cache: &DashMap<String, CachedStaticFile>,
) -> Option<GatewayHttpResponse> {
    if method != Method::GET {
        return None;
    }
    let key = target.to_string_lossy();
    let entry = static_file_cache.get(key.as_ref())?;
    if entry.body.len() as u64 != entry.len
        || entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS)
    {
        return None;
    }
    let mut response = GatewayHttpResponse::bytes(
        StatusCode::OK,
        static_content_type(target),
        entry.body.clone(),
        "proxysss://static-cache",
    );
    response.headers.push((
        CONTENT_LENGTH,
        HeaderValue::from_str(&entry.len.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    ));
    response
        .headers
        .push((ACCEPT_RANGES, HeaderValue::from_static("bytes")));
    Some(response)
}

fn cached_static_file_response_stale_while_revalidate(
    target: &Path,
    method: &Method,
    static_file_cache: &DashMap<String, CachedStaticFile>,
) -> Option<CachedStaticResponse> {
    if method != Method::GET {
        return None;
    }
    let key = target.to_string_lossy();
    let entry = static_file_cache.get(key.as_ref())?;
    if entry.body.len() as u64 != entry.len {
        return None;
    }
    let stale = entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS);

    let mut response = Response::new(full_body(entry.body.clone()));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, entry.content_type.clone());
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, entry.content_length.clone());
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    drop(entry);

    let revalidate = if stale {
        let mut entry = static_file_cache.get_mut(key.as_ref())?;
        if entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS)
            && !entry.revalidating
        {
            entry.revalidating = true;
            true
        } else {
            false
        }
    } else {
        false
    };
    Some(CachedStaticResponse {
        response,
        revalidate,
    })
}

fn finish_failed_static_revalidation(
    key: &str,
    static_file_cache: &DashMap<String, CachedStaticFile>,
) {
    if let Some(mut entry) = static_file_cache.get_mut(key) {
        entry.checked_at = Instant::now();
        entry.revalidating = false;
    }
}

fn stale_cached_static_file_candidate(
    target: &Path,
    method: &str,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    sendfile_threshold: u64,
) -> Option<(StaticFastPathCandidate, bool)> {
    if method != "GET" {
        return None;
    }
    let key = target.to_string_lossy().to_string();
    let entry = static_file_cache.get(&key)?;
    let stale = entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS);
    let sendfile = if cfg!(target_os = "linux") && entry.len >= sendfile_threshold {
        Some(entry.sendfile.as_ref()?.clone())
    } else {
        None
    };
    let cached_body = if sendfile.is_none() {
        if entry.body.len() as u64 != entry.len {
            return None;
        }
        Some(entry.body.clone())
    } else {
        None
    };
    let candidate = StaticFastPathCandidate {
        path: target.to_path_buf(),
        len: entry.len,
        content_type: static_content_type(target),
        cached_body,
        sendfile,
    };
    drop(entry);

    let revalidate = if stale {
        let mut entry = static_file_cache.get_mut(&key)?;
        if entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS)
            && !entry.revalidating
        {
            entry.revalidating = true;
            true
        } else {
            false
        }
    } else {
        false
    };
    Some((candidate, revalidate))
}

fn fresh_cached_static_file_candidate(
    target: &Path,
    method: &str,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    sendfile_threshold: u64,
) -> Option<StaticFastPathCandidate> {
    if method != "GET" {
        return None;
    }
    let key = target.to_string_lossy().to_string();
    let entry = static_file_cache.get(&key)?;
    if entry.checked_at.elapsed() > Duration::from_secs(STATIC_FILE_CACHE_REVALIDATE_SECS) {
        return None;
    }

    let sendfile = if cfg!(target_os = "linux") && entry.len >= sendfile_threshold {
        Some(entry.sendfile.as_ref()?.clone())
    } else {
        None
    };
    let cached_body = if sendfile.is_none() {
        if entry.body.len() as u64 != entry.len {
            return None;
        }
        Some(entry.body.clone())
    } else {
        None
    };

    Some(StaticFastPathCandidate {
        path: target.to_path_buf(),
        len: entry.len,
        content_type: static_content_type(target),
        cached_body,
        sendfile,
    })
}

fn cached_static_sendfile(
    target: &Path,
    metadata: &std::fs::Metadata,
    static_file_cache: &DashMap<String, CachedStaticFile>,
) -> Result<Arc<std::fs::File>> {
    let key = target.to_string_lossy().to_string();
    let modified = metadata.modified().ok();
    if let Some(mut entry) = static_file_cache.get_mut(&key) {
        if entry.len == metadata.len() && entry.modified == modified {
            entry.checked_at = Instant::now();
            entry.revalidating = false;
            if let Some(file) = &entry.sendfile {
                return Ok(file.clone());
            }
            let file = Arc::new(std::fs::File::open(target).with_context(|| {
                format!(
                    "failed opening static file for sendfile {}",
                    target.display()
                )
            })?);
            entry.sendfile = Some(file.clone());
            return Ok(file);
        }
    }

    let file = Arc::new(std::fs::File::open(target).with_context(|| {
        format!(
            "failed opening static file for sendfile {}",
            target.display()
        )
    })?);
    if static_file_cache.len() >= STATIC_FILE_CACHE_MAX_ENTRIES {
        return Ok(file);
    }
    static_file_cache.insert(
        key,
        CachedStaticFile {
            len: metadata.len(),
            modified,
            body: Bytes::new(),
            sendfile: Some(file.clone()),
            content_type: HeaderValue::from_static(static_content_type(target)),
            content_length: HeaderValue::from_str(&metadata.len().to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("0")),
            checked_at: Instant::now(),
            revalidating: false,
        },
    );
    Ok(file)
}

async fn preload_static_site_fast_lane_cache(
    site: &StaticSiteConfig,
    traffic_profile: RuntimePerformanceTrafficProfile,
    sendfile_threshold: u64,
    static_file_cache: &DashMap<String, CachedStaticFile>,
    static_file_cache_bytes: &AtomicU64,
    static_file_load_locks: &DashMap<String, Arc<TokioMutex<()>>>,
    static_route_cache: &DashMap<String, PathBuf>,
) -> Result<usize> {
    let mut candidates = HashMap::<String, PathBuf>::new();
    let prefix = normalize_webdav_prefix(&site.path_prefix);
    for index in &site.index_files {
        let path = site.root.join(index);
        candidates.insert(format!("{}/{}", prefix.trim_end_matches('/'), index), path);
    }

    if let Ok(entries) = fs::read_dir(&site.root) {
        for entry in entries.flatten().take(STATIC_PRELOAD_MAX_FILES_PER_SITE) {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    candidates.insert(format!("{}/{}", prefix.trim_end_matches('/'), name), path);
                }
            }
        }
    }

    let mut preloaded = 0_usize;
    for (request_path, target) in candidates {
        let metadata = match tokio::fs::metadata(&target).await {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).context("failed reading static preload metadata"),
        };
        static_route_cache.insert(request_path, target.clone());

        let should_cache_body = matches!(
            traffic_profile,
            RuntimePerformanceTrafficProfile::Small | RuntimePerformanceTrafficProfile::Balanced
        ) && metadata.len() <= STATIC_PRELOAD_SMALL_MAX_BYTES;
        let should_cache_sendfile = cfg!(target_os = "linux")
            && matches!(
                traffic_profile,
                RuntimePerformanceTrafficProfile::Balanced | RuntimePerformanceTrafficProfile::Bulk
            )
            && metadata.len() >= sendfile_threshold;

        if should_cache_body {
            let body = cached_static_file_body(
                &target,
                &metadata,
                static_file_cache,
                static_file_cache_bytes,
                static_file_load_locks,
            )
            .await?;
            if !body.is_empty() {
                preloaded = preloaded.saturating_add(1);
            }
        } else if should_cache_sendfile {
            cached_static_sendfile(&target, &metadata, static_file_cache)?;
            preloaded = preloaded.saturating_add(1);
        }
    }

    Ok(preloaded)
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
    let mut html = r###"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>proxysss docs</title>
  <style>
    :root {
      --bg: #f5efe6;
      --panel: rgba(255, 252, 247, 0.94);
      --panel-strong: #fffdf8;
      --ink: #0f172a;
      --muted: #526071;
      --line: rgba(15, 23, 42, 0.12);
      --accent: #ff6b35;
      --accent-soft: rgba(255, 107, 53, 0.14);
      --teal: #0f766e;
      --teal-soft: rgba(15, 118, 110, 0.14);
      --shadow: 0 24px 60px rgba(15, 23, 42, 0.12);
      --radius-xl: 30px;
      --radius-lg: 22px;
      --radius-md: 16px;
      --mono: "IBM Plex Mono", "SFMono-Regular", Consolas, monospace;
      --sans: "IBM Plex Sans", "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      font-family: var(--sans);
      color: var(--ink);
      line-height: 1.65;
      background:
        radial-gradient(circle at top left, rgba(255, 107, 53, 0.12), transparent 28%),
        radial-gradient(circle at top right, rgba(15, 118, 110, 0.12), transparent 32%),
        linear-gradient(180deg, #f7f2ea 0%, #f3eadf 46%, #efe6d9 100%);
    }

    .shell {
      width: min(calc(100% - 24px), 1200px);
      margin: 0 auto;
      padding: 18px 0 44px;
    }

    .hero,
    .panel,
    .card,
    .path {
      background: var(--panel);
      border: 1px solid rgba(255, 255, 255, 0.8);
      box-shadow: var(--shadow);
    }

    .hero {
      position: relative;
      overflow: hidden;
      border-radius: 34px;
      padding: 28px;
      margin-bottom: 18px;
    }

    .hero::after {
      content: "";
      position: absolute;
      right: -70px;
      bottom: -90px;
      width: 240px;
      height: 240px;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(255, 107, 53, 0.28), transparent 68%);
      pointer-events: none;
    }

    .eyebrow {
      display: inline-flex;
      align-items: center;
      gap: 10px;
      padding: 8px 12px;
      border-radius: 999px;
      background: rgba(15, 23, 42, 0.05);
      color: var(--muted);
      font-size: 12px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }

    h1 {
      margin: 18px 0 10px;
      font-size: clamp(34px, 6vw, 58px);
      line-height: 1.02;
      letter-spacing: -0.045em;
    }

    h2 {
      margin: 0;
      font-size: clamp(24px, 4vw, 34px);
      line-height: 1.08;
      letter-spacing: -0.03em;
    }

    h3 {
      margin: 14px 0 10px;
      font-size: 21px;
      line-height: 1.2;
      letter-spacing: -0.02em;
    }

    p {
      margin: 0;
    }

    .lead,
    .subtle,
    li,
    td {
      color: var(--muted);
    }

    .lead {
      max-width: 760px;
      font-size: 17px;
    }

    .meta {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
      gap: 12px;
      margin-top: 22px;
    }

    .meta-item,
    .panel,
    .card,
    .path {
      border-radius: var(--radius-lg);
    }

    .meta-item {
      padding: 14px 16px;
      background: rgba(255, 255, 255, 0.42);
      border: 1px solid rgba(255, 255, 255, 0.72);
    }

    .meta-label {
      font-size: 12px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--muted);
      margin-bottom: 6px;
    }

    .meta-value {
      font-size: 17px;
      font-weight: 700;
    }

    .layout {
      display: grid;
      gap: 18px;
    }

    .panel {
      padding: 24px;
    }

    .section-head {
      display: flex;
      flex-wrap: wrap;
      align-items: end;
      justify-content: space-between;
      gap: 10px;
      margin-bottom: 18px;
    }

    .kicker {
      color: var(--accent);
      font-size: 12px;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      font-weight: 700;
      margin-bottom: 6px;
    }

    .paths,
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 16px;
    }

    .path,
    .card {
      padding: 20px;
    }

    .tag {
      display: inline-flex;
      align-items: center;
      padding: 6px 10px;
      border-radius: 999px;
      font-size: 12px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }

    .tag.beginner {
      background: var(--accent-soft);
      color: #a53d18;
    }

    .tag.expert {
      background: var(--teal-soft);
      color: var(--teal);
    }

    ul {
      margin: 0;
      padding-left: 18px;
    }

    li + li {
      margin-top: 8px;
    }

    pre {
      margin: 16px 0 0;
      padding: 18px;
      overflow: auto;
      border-radius: var(--radius-md);
      background: #111827;
      color: #e5eef9;
      border: 1px solid rgba(255, 255, 255, 0.08);
      font: 13px/1.55 var(--mono);
    }

    code {
      font-family: var(--mono);
    }

    table {
      width: 100%;
      border-collapse: collapse;
      border: 1px solid var(--line);
      border-radius: var(--radius-md);
      overflow: hidden;
      margin-top: 18px;
      background: var(--panel-strong);
    }

    th,
    td {
      text-align: left;
      vertical-align: top;
      padding: 14px 16px;
      border-bottom: 1px solid var(--line);
    }

    th {
      background: rgba(15, 23, 42, 0.04);
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--muted);
    }

    td strong {
      display: block;
      margin-bottom: 4px;
      color: var(--ink);
    }

    .command-block {
      margin-top: 16px;
    }

    .note {
      margin-top: 18px;
      padding: 16px 18px;
      border-radius: var(--radius-md);
      background: rgba(15, 23, 42, 0.92);
      color: rgba(255, 255, 255, 0.84);
    }

    .note strong {
      color: #fff;
    }

    @media (max-width: 720px) {
      .hero,
      .panel,
      .card,
      .path {
        padding: 18px;
      }

      h1 {
        font-size: 38px;
      }
    }
  </style>
</head>
<body>
  <div class="shell">
    <header class="hero">
      <div class="eyebrow">proxysss docs / human first</div>
      <h1>先复制成功，再看完整能力面。</h1>
      <p class="lead">这页内建文档同时服务两类人：新手要能在几分钟内跑起一个站点；高手要能立刻找到路由面、TLS、AI SSE、TCP/UDP、reload 和运维边界，不需要先看一屏宣传。</p>
      <div class="meta">
        <div class="meta-item">
          <div class="meta-label">Default HTTP</div>
          <div class="meta-value">80</div>
        </div>
        <div class="meta-item">
          <div class="meta-label">Default HTTPS / H2 / H3</div>
          <div class="meta-value">443</div>
        </div>
        <div class="meta-item">
          <div class="meta-label">Admin</div>
          <div class="meta-value">127.0.0.1:7777</div>
        </div>
        <div class="meta-item">
          <div class="meta-label">Main config</div>
          <div class="meta-value">proxysss.yaml</div>
        </div>
      </div>
    </header>

    <main class="layout">
      <section class="panel">
        <div class="section-head">
          <div>
            <div class="kicker">Beginner path</div>
            <h2>如果你是新手，先跑通一条 HTTP 链路。</h2>
          </div>
          <p class="subtle">你最需要的不是术语，而是一份能工作的最小配置和一段清楚的解释。</p>
        </div>

        <div class="paths">
          <article class="path">
            <span class="tag beginner">5 分钟可用</span>
            <h3>第一个反向代理</h3>
            <p class="subtle">把 `app.example.com` 代理到本地 9000 端口，先确认域名、端口和源站都通。</p>
            <pre><code>{{REVERSE_PROXY}}</code></pre>
          </article>

          <article class="path">
            <span class="tag beginner">正式上线</span>
            <h3>只填域名就给 WebSocket 加 WSS</h3>
            <p class="subtle">单域名只填域名即可走正式 TLS-ALPN-01；无需证书脚本、DNS API 或邮箱，只需公网 443。显式 HTTP-01（需 80）继续兼容；泛域名使用 DNS-01。</p>
            <pre><code>{{ACME_DNS}}</code></pre>
          </article>
        </div>
      </section>

      <section class="panel">
        <div class="section-head">
          <div>
            <div class="kicker">Copy-paste recipes</div>
            <h2>核心场景都给可复制案例，而且告诉你为什么这样配。</h2>
          </div>
          <p class="subtle">官方文档不该让你一边猜字段名，一边猜设计意图。下面这些场景是最常见的第一批生产配置。</p>
        </div>

        <div class="grid">
          <article class="card">
            <span class="tag beginner">Static site</span>
            <h3>静态站点</h3>
            <p class="subtle">适合官网、文档站、CDN 回源、小型下载站。HTML/CSS/JS/图片/字体/音视频会参与 warm-up，大文件支持 Range 断点续传。</p>
            <pre><code>{{STATIC_SITE}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">All scenarios</span>
            <h3>全场景 Docker 验证</h3>
            <p class="subtle">用 Ubuntu 24 容器检查静态 Range、注册中心配置、能力矩阵、nginx-parity 和全场景样例。</p>
            <pre><code>proxysss -config examples/all-scenarios.example.yaml check-config
scripts/verify-docker-scenarios.sh
.\scripts\verify-docker-scenarios.ps1</code></pre>
          </article>

          <article class="card">
            <span class="tag beginner">AI / SSE</span>
            <h3>流式 AI 代理</h3>
            <p class="subtle">面向 OpenAI-compatible / New API / SSE，优先关注小包 flush、nodelay 和上游健康。</p>
            <pre><code>{{AI_PROXY}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">TCP / UDP</span>
            <h3>游戏 / 原生协议网关</h3>
            <p class="subtle">HTTP 面解决不了的实时流量，应该直接落到 `tcp.listeners` / `udp.listeners`。</p>
            <pre><code>{{STREAMS}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">MQTT / IoT</span>
            <h3>设备边缘接入</h3>
            <p class="subtle">MQTT TCP、MQTT over WebSocket、CoAP-style UDP 都能组合，但 proxysss 不是 broker。</p>
            <pre><code>{{IOT}}</code></pre>
          </article>

          <article class="card">
            <span class="tag beginner">WebDAV</span>
            <h3>文件协作入口</h3>
            <p class="subtle">当你需要轻量文件写入、设计资产共享或受控协作时，用内建 WebDAV，而不是把所有文件需求都塞进静态站点。</p>
            <pre><code>{{WEBDAV}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">FTP</span>
            <h3>FTP 控制与传输治理</h3>
            <p class="subtle">适合兼容旧客户端和传统分发链路，同时还能表达 allow/deny、命令策略、传输策略和按用户规则。</p>
            <pre><code>{{FTP}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">Error pages</span>
            <h3>错误页与维护窗口</h3>
            <p class="subtle">生产入口不仅要能代理，还要能在失败和维护时给出可控反馈，而不是把异常页面交给默认实现碰运气。</p>
            <pre><code>{{ERROR_PAGES}}</code></pre>
          </article>

          <article class="card">
            <span class="tag expert">Maintenance</span>
            <h3>维护状态切换</h3>
            <p class="subtle">通过独立维护状态文件控制网关行为，避免靠临时改配置或人工停服务来完成维护窗口切换。</p>
            <pre><code>{{MAINTENANCE}}</code></pre>
          </article>
        </div>
      </section>

      <section class="panel">
        <div class="section-head">
          <div>
            <div class="kicker">Expert path</div>
            <h2>高手速查：先确认应该把配置落在哪个能力面。</h2>
          </div>
          <p class="subtle">这张表的目标是减少“功能明明支持，但找不到入口”的摩擦。</p>
        </div>

        <table>
          <thead>
            <tr>
              <th>配置面</th>
              <th>适合什么</th>
              <th>常见误用</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>services.reverse_proxy.routes</strong>普通 HTTP / API / WebSocket / SSE</td>
              <td>网站、后台、HTTP/2 gRPC、长响应 API、缓存、限流、健康检查都从这里开始。</td>
              <td>不要拿它去承载 TCP / UDP 原生协议。</td>
            </tr>
            <tr>
              <td><strong>services.ai_proxy</strong>OpenAI-compatible / New API</td>
              <td>更明确的 AI 网关语义、上游池和流式传输细节。</td>
              <td>不要把所有 HTTP 都无脑归进 AI proxy。</td>
            </tr>
            <tr>
              <td><strong>services.domain_routes</strong>站点级编排</td>
              <td>同一域名下把静态、API、WebDAV 等组合成统一入口。</td>
              <td>不要把它理解成唯一执行面。</td>
            </tr>
            <tr>
              <td><strong>services.static_sites</strong>静态资源 / CDN 回源</td>
              <td>HTML、CSS、JS、图片、字体、音视频、大文件下载、Range 断点续传。</td>
              <td>不要把复杂业务鉴权写进静态文件层。</td>
            </tr>
            <tr>
              <td><strong>services.service_discovery</strong>Consul / etcd / Nacos</td>
              <td>把注册中心服务映射到 HTTP、TCP、UDP 上游池。</td>
              <td>不要让每个数据面请求临时查注册中心。</td>
            </tr>
            <tr>
              <td><strong>tcp.listeners</strong>TCP 原生流量</td>
              <td>数据库、游戏长连接、MQTT TCP、TLS passthrough/SNI 场景。</td>
              <td>不要期待 HTTP 头级别策略。</td>
            </tr>
            <tr>
              <td><strong>udp.listeners</strong>UDP、KCP、QCP、CoAP-style</td>
              <td>实时设备、游戏、会话型 UDP 网关。</td>
              <td>不要把它包装成应用终端本身。</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section class="panel">
        <div class="section-head">
          <div>
            <div class="kicker">Ops and validation</div>
            <h2>文档最后一定要回到运维命令和压测纪律。</h2>
          </div>
          <p class="subtle">这是官方文档最容易写丢的部分，但它决定了你的配置是不是能真正上线。</p>
        </div>

        <div class="command-block">
          <pre><code>proxysss config explain
proxysss config capabilities
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
proxysss token show
{{HEALTH}}</code></pre>
        </div>

        <div class="note">
          <strong>性能与压测纪律：</strong> 官方 Linux benchmark 不该只讲口径，也要直接展示历史 nginx 对标基线。后续所有性能优化都要压测，而且必须是无副作用优化。SSE、静态、HTTP reverse proxy、TCP、UDP、KCP、QCP 都要一起看，不能出现“这一项快了，另一项明显变差”还把它当成成功。
        </div>
      </section>
    </main>
  </div>
</body>
</html>"###
    .to_string();

    for (token, value) in [
        ("{{REVERSE_PROXY}}", docs_template_reverse_proxy()),
        ("{{AI_PROXY}}", docs_template_ai_proxy()),
        ("{{STATIC_SITE}}", docs_template_static_site()),
        ("{{STREAMS}}", docs_template_streams()),
        ("{{IOT}}", docs_template_iot()),
        ("{{WEBDAV}}", docs_template_webdav()),
        ("{{FTP}}", docs_template_ftp()),
        ("{{ACME_DNS}}", docs_template_acme_dns()),
        ("{{HEALTH}}", docs_template_health()),
        ("{{ERROR_PAGES}}", docs_template_error_pages()),
        ("{{MAINTENANCE}}", docs_template_maintenance()),
    ] {
        html = html.replace(token, value);
    }

    html
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
    "tcp:\n  listeners:\n    - name: game-tcp\n      bind: 0.0.0.0:7000\n      protocol: game_tcp\n      nodelay: true\n      connect_timeout_ms: 3000\n      upstreams: [127.0.0.1:9000, 127.0.0.1:9001]\nudp:\n  listeners:\n    - name: game-kcp\n      bind: 0.0.0.0:7001\n      protocol: kcp\n      session_ttl_secs: 180\n      max_associations: 262144\n      upstreams: [127.0.0.1:9100, 127.0.0.1:9101]\n    - name: game-qcp\n      bind: 0.0.0.0:7002\n      protocol: qcp\n      session_ttl_secs: 180\n      max_associations: 262144\n      upstreams: [127.0.0.1:9200, 127.0.0.1:9201]\n"
}

fn docs_template_iot() -> &'static str {
    "tcp:\n  listeners:\n    - name: mqtt\n      bind: 0.0.0.0:1883\n      protocol: mqtt\n      nodelay: true\n      connect_timeout_ms: 3000\n      upstreams: [127.0.0.1:18831, 127.0.0.1:18832]\n  stream_routes:\n    - name: mqtt-tls\n      domains: [mqtt.example.com]\n      listen: 0.0.0.0:8883\n      upstream: 127.0.0.1:88831\n      protocol: mqtt\n      tls_mode: passthrough\nudp:\n  listeners:\n    - name: coap\n      bind: 0.0.0.0:5683\n      protocol: coap\n      session_ttl_secs: 120\n      max_associations: 262144\n      upstreams: [127.0.0.1:56831]\nservices:\n  reverse_proxy:\n    routes:\n      - name: mqtt-websocket\n        hosts: [mqtt-ws.example.com]\n        path_prefix: /mqtt\n        upstream: ws://127.0.0.1:8083\n"
}

fn docs_template_ftp() -> &'static str {
    "services:\n  ftp:\n    enabled: true\n    bind: 0.0.0.0:21\n    upstream: 127.0.0.1:2121\n    native_control: true\n    public_ip: 203.0.113.10\n    passive_port_start: 50000\n    passive_port_end: 50100\n    log_commands: true\n    log_transfers: true\n    allow: [198.51.100.0/24]\n    deny: [203.0.113.9]\n    command_deny: [SITE, STAT]\n    transfer_allow: [RETR, STOR]\n    user_policies:\n      - user: readonly\n        transfer_allow: [RETR]\n        transfer_deny: [STOR, DELE]\n"
}

fn docs_template_acme_dns() -> &'static str {
    "http:\n  plain_bind: 0.0.0.0:80\n  tls_bind: 0.0.0.0:443\n  tls:\n    # domains 非空即自动启用内建 ACME：正式 Let's Encrypt + TLS-ALPN-01\n    auto_https:\n      domains: [wss.example.com]\n      # email: ops@example.com # 可选；仅用于到期/安全通知\nservices:\n  domain_routes:\n    - name: game-wss\n      domains: [wss.example.com]\n      path_prefix: /ws\n      upstream: ws://127.0.0.1:9000\n"
}

fn docs_template_health() -> &'static str {
    "load_balance:\n  active_health:\n    enabled: true\n    http_enabled: true\n    tcp_enabled: true\n    udp_enabled: false\n    interval_secs: 10\n    timeout_ms: 2000\n    path: /healthz\n    expected_statuses: [200, 204]\n    failure_threshold: 2\n    success_threshold: 2\n    jitter_percent: 20\n    udp_payload: proxysss-health\n    udp_expect_response: true\n    alert_webhooks:\n      - https://ops.example.com/webhooks/proxysss\nruntime:\n  performance:\n    enabled: true\n    profile: edge\n    traffic_profile: small\n    adaptive_system: true\n    socket_extreme: true\n    log_on_start: true\n  watchdog:\n    enabled: true\n    restart_critical_tasks: true\n    restart_backoff_secs: 2\n    heartbeat_interval_secs: 30\n"
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
                    <div class="filter"><label for="auto-email">ACME Email（可选）</label><input id="auto-email" placeholder="admin@example.com（留空也可自动签发；不会收到到期提醒）" /></div>
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
    let _ = tcp.set_nodelay(true);

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

async fn read_http_response_head<T>(
    upstream: &mut T,
) -> Result<(StatusCode, Vec<(HeaderName, HeaderValue)>, Option<Bytes>)>
where
    T: AsyncRead + Unpin + ?Sized,
{
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

struct RawFastHttpResponseHead {
    status: StatusCode,
    raw_head: Bytes,
    headers: Vec<u8>,
    content_length: Option<u64>,
    transfer_chunked: bool,
    connection_close: bool,
    leftover: Option<Bytes>,
}

struct RawReverseHttpResponse {
    status: StatusCode,
    head_end: usize,
    content_length: Option<u64>,
    transfer_chunked: bool,
    connection_close: bool,
}

struct RawReverseResponseCache {
    raw_head: Bytes,
    status: StatusCode,
    content_length: Option<u64>,
    transfer_chunked: bool,
    connection_close: bool,
}

impl RawReverseResponseCache {
    fn response(&self) -> RawReverseHttpResponse {
        RawReverseHttpResponse {
            status: self.status,
            head_end: self.raw_head.len(),
            content_length: self.content_length,
            transfer_chunked: self.transfer_chunked,
            connection_close: self.connection_close,
        }
    }
}

async fn read_raw_reverse_http_response_into(
    upstream: &mut TcpStream,
    buffer: &mut Vec<u8>,
    response_cache: &mut Option<RawReverseResponseCache>,
) -> Result<RawReverseHttpResponse> {
    buffer.clear();

    loop {
        let cached_prefix = response_cache.as_ref().is_some_and(|cached| {
            let compared = buffer.len().min(cached.raw_head.len());
            buffer[..compared] == cached.raw_head[..compared]
        });
        if cached_prefix {
            let cached = response_cache
                .as_ref()
                .expect("cached prefix requires response cache");
            if buffer.len() >= cached.raw_head.len() {
                return Ok(cached.response());
            }
        }

        let header_end = if cached_prefix {
            None
        } else {
            memmem::find(buffer, b"\r\n\r\n")
        };
        if let Some(position) = header_end {
            let head_end = position + 4;
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut response = httparse::Response::new(&mut headers);
            let status = response
                .parse(&buffer[..head_end])
                .context("failed parsing upstream raw reverse response")?;
            if !matches!(status, httparse::Status::Complete(_)) {
                return Err(anyhow!("incomplete upstream raw reverse response"));
            }

            let status = StatusCode::from_u16(
                response
                    .code
                    .ok_or_else(|| anyhow!("missing upstream status code"))?,
            )?;
            let mut content_length = None;
            let mut transfer_chunked = false;
            let mut connection_close = false;
            for header in response.headers.iter() {
                if header.name.eq_ignore_ascii_case("content-length") {
                    content_length = std::str::from_utf8(header.value)
                        .ok()
                        .and_then(|value| value.trim().parse::<u64>().ok());
                } else if header.name.eq_ignore_ascii_case("transfer-encoding") {
                    transfer_chunked = std::str::from_utf8(header.value).ok().is_some_and(|raw| {
                        raw.split(',')
                            .any(|item| item.trim().eq_ignore_ascii_case("chunked"))
                    });
                } else if header.name.eq_ignore_ascii_case("connection") {
                    connection_close = std::str::from_utf8(header.value).ok().is_some_and(|raw| {
                        raw.split(',')
                            .any(|item| item.trim().eq_ignore_ascii_case("close"))
                    });
                }
            }

            let parsed = RawReverseHttpResponse {
                status,
                head_end,
                content_length,
                transfer_chunked,
                connection_close,
            };
            if head_end <= RAW_REVERSE_RESPONSE_CACHE_MAX_HEAD_BYTES {
                *response_cache = Some(RawReverseResponseCache {
                    raw_head: Bytes::copy_from_slice(&buffer[..head_end]),
                    status,
                    content_length,
                    transfer_chunked,
                    connection_close,
                });
            } else {
                *response_cache = None;
            }
            return Ok(parsed);
        }

        if buffer.len() > 64 * 1024 {
            return Err(anyhow!("upstream response headers exceeded 64KiB"));
        }

        buffer.reserve(4096);
        let read = upstream.read_buf(buffer).await?;
        if read == 0 {
            return Err(anyhow!("upstream closed during handshake"));
        }
    }
}

async fn read_raw_fast_http_response_head(
    upstream: &mut TcpStream,
    filter_hop_headers: bool,
) -> Result<RawFastHttpResponseHead> {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        if let Some(position) = find_header_end(&buffer) {
            let head = buffer.split_to(position + 4).freeze();
            let leftover = if buffer.is_empty() {
                None
            } else {
                Some(buffer.freeze())
            };
            return parse_raw_fast_http_response_head(head, leftover, filter_hop_headers);
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

fn parse_raw_fast_http_response_head(
    head: Bytes,
    leftover: Option<Bytes>,
    filter_hop_headers: bool,
) -> Result<RawFastHttpResponseHead> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    let status = response
        .parse(&head)
        .context("failed parsing upstream raw fast response")?;
    if !matches!(status, httparse::Status::Complete(_)) {
        return Err(anyhow!("incomplete upstream raw fast response"));
    }

    let status = StatusCode::from_u16(
        response
            .code
            .ok_or_else(|| anyhow!("missing upstream status code"))?,
    )?;
    let mut content_length = None;
    let mut transfer_chunked = false;
    let mut connection_close = false;
    let mut filtered = if filter_hop_headers {
        Vec::with_capacity(head.len().min(4096))
    } else {
        Vec::new()
    };

    for header in response.headers.iter() {
        let name = header.name;
        let value = header.value;
        if name.eq_ignore_ascii_case("content-length") {
            content_length = std::str::from_utf8(value)
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok());
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            transfer_chunked = std::str::from_utf8(value).ok().is_some_and(|raw| {
                raw.split(',')
                    .any(|item| item.trim().eq_ignore_ascii_case("chunked"))
            });
        } else if name.eq_ignore_ascii_case("connection") {
            connection_close = std::str::from_utf8(value).ok().is_some_and(|raw| {
                raw.split(',')
                    .any(|item| item.trim().eq_ignore_ascii_case("close"))
            });
        }

        if filter_hop_headers {
            if is_hop_header(name) {
                continue;
            }
            filtered.extend_from_slice(name.as_bytes());
            filtered.extend_from_slice(b": ");
            filtered.extend_from_slice(value);
            filtered.extend_from_slice(b"\r\n");
        }
    }

    Ok(RawFastHttpResponseHead {
        status,
        raw_head: head,
        headers: filtered,
        content_length,
        transfer_chunked,
        connection_close,
        leftover,
    })
}

fn build_raw_http_response_head_bytes(
    status: StatusCode,
    headers: &[u8],
    keep_alive: bool,
    chunked: bool,
) -> Vec<u8> {
    let reason = status.canonical_reason().unwrap_or("");
    let mut head = Vec::with_capacity(128 + headers.len());
    head.extend_from_slice(b"HTTP/1.1 ");
    append_u16_decimal(&mut head, status.as_u16());
    if !reason.is_empty() {
        head.push(b' ');
        head.extend_from_slice(reason.as_bytes());
    }
    head.extend_from_slice(b"\r\n");
    head.extend_from_slice(headers);
    if chunked {
        head.extend_from_slice(b"transfer-encoding: chunked\r\n");
    }
    if !keep_alive {
        head.extend_from_slice(b"connection: close\r\n");
    }
    head.extend_from_slice(b"\r\n");
    head
}

fn append_u16_decimal(out: &mut Vec<u8>, value: u16) {
    let hundreds = value / 100;
    let tens = (value / 10) % 10;
    let ones = value % 10;
    out.push(b'0' + hundreds as u8);
    out.push(b'0' + tens as u8);
    out.push(b'0' + ones as u8);
}

async fn relay_raw_http_body(
    upstream: &mut (impl AsyncRead + Unpin + ?Sized),
    downstream: &mut TcpStream,
    leftover: Option<Bytes>,
) -> Result<()> {
    if let Some(leftover) = leftover {
        if !leftover.is_empty() {
            downstream
                .write_all(&leftover)
                .await
                .context("failed writing raw upstream prelude to downstream")?;
        }
    }

    let mut buffer = relay_buffer_pool().acquire();
    loop {
        let read = upstream
            .read(&mut buffer)
            .await
            .context("failed reading raw upstream body")?;
        if read == 0 {
            break;
        }
        downstream
            .write_all(&buffer[..read])
            .await
            .context("failed writing raw upstream body")?;
    }
    Ok(())
}

async fn relay_passthrough_chunked_http_body(
    upstream: &mut (impl AsyncRead + Unpin + ?Sized),
    downstream: &mut TcpStream,
    leftover: Option<Bytes>,
) -> Result<bool> {
    let mut scanner = ChunkedBodyScanner::default();
    if let Some(leftover) = leftover {
        if !leftover.is_empty() {
            downstream
                .write_all(&leftover)
                .await
                .context("failed writing chunked upstream prelude")?;
            if scanner.observe(&leftover)? {
                return Ok(true);
            }
        }
    }

    let mut buffer = relay_buffer_pool().acquire();
    loop {
        let read = upstream
            .read(&mut buffer)
            .await
            .context("failed reading chunked raw upstream body")?;
        if read == 0 {
            return Ok(false);
        }
        downstream
            .write_all(&buffer[..read])
            .await
            .context("failed writing chunked raw upstream body")?;
        if scanner.observe(&buffer[..read])? {
            return Ok(true);
        }
    }
}

#[derive(Default)]
struct ChunkedBodyScanner {
    state: ChunkedScanState,
}

enum ChunkedScanState {
    SizeLine(Vec<u8>),
    Body(u64),
    BodyCrLf(usize),
    Trailer(Vec<u8>),
    Done,
}

impl Default for ChunkedScanState {
    fn default() -> Self {
        Self::SizeLine(Vec::with_capacity(32))
    }
}

impl ChunkedBodyScanner {
    fn observe(&mut self, mut bytes: &[u8]) -> Result<bool> {
        while !bytes.is_empty() {
            match &mut self.state {
                ChunkedScanState::SizeLine(line) => {
                    let Some(pos) = bytes.iter().position(|byte| *byte == b'\n') else {
                        line.extend_from_slice(bytes);
                        if line.len() > 16 * 1024 {
                            return Err(anyhow!("chunk size line exceeded 16KiB"));
                        }
                        return Ok(false);
                    };
                    line.extend_from_slice(&bytes[..=pos]);
                    bytes = &bytes[pos + 1..];
                    if !line.ends_with(b"\r\n") {
                        return Err(anyhow!("invalid chunk size line"));
                    }
                    let size = parse_chunk_size_line(line)?;
                    line.clear();
                    self.state = if size == 0 {
                        ChunkedScanState::Trailer(Vec::with_capacity(64))
                    } else {
                        ChunkedScanState::Body(size)
                    };
                }
                ChunkedScanState::Body(remaining) => {
                    let consumed = (*remaining).min(bytes.len() as u64) as usize;
                    *remaining -= consumed as u64;
                    bytes = &bytes[consumed..];
                    if *remaining == 0 {
                        self.state = ChunkedScanState::BodyCrLf(0);
                    }
                }
                ChunkedScanState::BodyCrLf(seen) => {
                    let expected = [b'\r', b'\n'];
                    while *seen < 2 && !bytes.is_empty() {
                        if bytes[0] != expected[*seen] {
                            return Err(anyhow!("invalid chunk body terminator"));
                        }
                        *seen += 1;
                        bytes = &bytes[1..];
                    }
                    if *seen == 2 {
                        self.state = ChunkedScanState::SizeLine(Vec::with_capacity(32));
                    }
                }
                ChunkedScanState::Trailer(trailer) => {
                    trailer.extend_from_slice(bytes);
                    if trailer.len() > 64 * 1024 {
                        return Err(anyhow!("chunk trailer exceeded 64KiB"));
                    }
                    if trailer.starts_with(b"\r\n") || find_header_end(trailer).is_some() {
                        self.state = ChunkedScanState::Done;
                        return Ok(true);
                    }
                    return Ok(false);
                }
                ChunkedScanState::Done => return Ok(true),
            }
        }

        Ok(matches!(self.state, ChunkedScanState::Done))
    }
}

fn parse_chunk_size_line(line: &[u8]) -> Result<u64> {
    let line = std::str::from_utf8(line)
        .context("chunk size line is not utf-8")?
        .trim_end_matches("\r\n");
    let size_hex = line
        .split_once(';')
        .map(|(size, _)| size)
        .unwrap_or(line)
        .trim();
    u64::from_str_radix(size_hex, 16).context("invalid chunk size")
}

async fn relay_fixed_http_body(
    upstream: &mut (impl AsyncRead + Unpin + ?Sized),
    downstream: &mut TcpStream,
    leftover: Option<Bytes>,
    len: u64,
) -> Result<bool> {
    let mut remaining = len;
    if let Some(leftover) = leftover {
        if !leftover.is_empty() {
            let to_write = remaining.min(leftover.len() as u64) as usize;
            if to_write > 0 {
                downstream
                    .write_all(&leftover[..to_write])
                    .await
                    .context("failed writing fixed raw upstream prelude")?;
                remaining -= to_write as u64;
            }
            if to_write < leftover.len() {
                return Ok(false);
            }
        }
    }

    let mut buffer = relay_buffer_pool().acquire();
    while remaining > 0 {
        let read_target = remaining.min(buffer.len() as u64) as usize;
        let read = upstream
            .read(&mut buffer[..read_target])
            .await
            .context("failed reading fixed raw upstream body")?;
        if read == 0 {
            return Ok(false);
        }
        downstream
            .write_all(&buffer[..read])
            .await
            .context("failed writing fixed raw upstream body")?;
        remaining -= read as u64;
    }

    Ok(true)
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
    forward_headers: bool,
) -> Result<HeaderMap> {
    let forwarding_capacity = if forward_headers { 6 } else { 1 };
    let mut headers =
        HeaderMap::with_capacity(original.len() + route.set_headers.len() + forwarding_capacity);

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
    if forward_headers {
        apply_forwarding_headers(&mut headers, original_host, remote_addr, scheme)?;
    }

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

fn serialize_raw_fast_lane_request(
    request: &PlainFastLaneRequest,
    path_and_query: &str,
    host: &str,
    options: RawFastLaneSerializeOptions<'_>,
) -> Vec<u8> {
    let extra_headers_bytes = options
        .extra_headers
        .iter()
        .map(|(name, value)| name.len() + value.len() + 4)
        .sum::<usize>();
    let mut bytes = Vec::with_capacity(
        192 + request.forward_header_bytes.len()
            + path_and_query.len()
            + host.len()
            + extra_headers_bytes,
    );
    bytes.extend_from_slice(request.method.as_str().as_bytes());
    bytes.extend_from_slice(b" ");
    bytes.extend_from_slice(path_and_query.as_bytes());
    bytes.extend_from_slice(b" HTTP/1.1\r\n");
    bytes.extend_from_slice(&request.forward_header_bytes);
    if options.forward_headers {
        append_raw_forwarding_headers(
            &mut bytes,
            &request.forwarding_headers,
            host,
            options.remote_addr,
            options.scheme,
        );
    } else {
        append_preserved_raw_forwarding_headers(&mut bytes, &request.forwarding_headers);
    }
    for (name, value) in options.extra_headers {
        append_raw_header_line(&mut bytes, name, value);
    }

    bytes.extend_from_slice(b"host: ");
    bytes.extend_from_slice(host.as_bytes());
    bytes.extend_from_slice(b"\r\n");
    if let Some(connection) = options.connection {
        bytes.extend_from_slice(b"connection: ");
        bytes.extend_from_slice(connection.as_bytes());
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n");
    bytes
}

struct RawFastLaneSerializeOptions<'a> {
    connection: Option<&'a str>,
    remote_addr: SocketAddr,
    scheme: &'a str,
    forward_headers: bool,
    extra_headers: &'a [(&'a str, &'a str)],
}

fn append_raw_forwarding_headers(
    bytes: &mut Vec<u8>,
    forwarding_headers: &RawForwardingHeaderSnapshot,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
) {
    let remote_ip = remote_addr.ip().to_string();
    let xff = append_csv_header_value(forwarding_headers.x_forwarded_for.as_deref(), &remote_ip);
    let forwarded = append_forwarded_header_value(
        forwarding_headers.forwarded.as_deref(),
        &remote_ip,
        host,
        scheme,
    );
    append_raw_header_line(bytes, "x-real-ip", &remote_ip);
    append_raw_header_line(bytes, "x-forwarded-for", &xff);
    append_raw_header_line(bytes, "x-forwarded-host", host);
    append_raw_header_line(bytes, "x-forwarded-proto", scheme);
    append_raw_header_line(bytes, "forwarded", &forwarded);
}

fn append_preserved_raw_forwarding_headers(
    bytes: &mut Vec<u8>,
    forwarding_headers: &RawForwardingHeaderSnapshot,
) {
    if let Some(value) = forwarding_headers.x_real_ip.as_deref() {
        append_raw_header_line(bytes, "x-real-ip", value);
    }
    if let Some(value) = forwarding_headers.x_forwarded_for.as_deref() {
        append_raw_header_line(bytes, "x-forwarded-for", value);
    }
    if let Some(value) = forwarding_headers.x_forwarded_host.as_deref() {
        append_raw_header_line(bytes, "x-forwarded-host", value);
    }
    if let Some(value) = forwarding_headers.x_forwarded_proto.as_deref() {
        append_raw_header_line(bytes, "x-forwarded-proto", value);
    }
    if let Some(value) = forwarding_headers.forwarded.as_deref() {
        append_raw_header_line(bytes, "forwarded", value);
    }
}

fn append_raw_header_line(bytes: &mut Vec<u8>, name: &str, value: &str) {
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(b": ");
    bytes.extend_from_slice(value.as_bytes());
    bytes.extend_from_slice(b"\r\n");
}

fn serialize_raw_websocket_fast_lane_request(
    request: &PlainWebSocketFastLaneRequest,
    path_and_query: &str,
    host: &str,
    remote_addr: SocketAddr,
    scheme: &str,
    forward_headers: bool,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(320 + request.header_bytes.len());
    bytes.extend_from_slice(b"GET ");
    bytes.extend_from_slice(path_and_query.as_bytes());
    bytes.extend_from_slice(b" HTTP/1.1\r\n");
    bytes.extend_from_slice(&request.header_bytes);
    if forward_headers {
        append_raw_forwarding_headers(
            &mut bytes,
            &request.forwarding_headers,
            host,
            remote_addr,
            scheme,
        );
    } else {
        append_preserved_raw_forwarding_headers(&mut bytes, &request.forwarding_headers);
    }
    bytes.extend_from_slice(b"host: ");
    bytes.extend_from_slice(host.as_bytes());
    bytes.extend_from_slice(b"\r\n\r\n");
    bytes
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

    #[tokio::test]
    async fn raw_http_pool_discards_upstream_socket_closed_while_idle() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind raw pool fixture");
        let address = listener.local_addr().expect("raw pool fixture address");
        let (replacement_accepted_tx, replacement_accepted_rx) = tokio::sync::oneshot::channel();
        let (release_replacement_tx, release_replacement_rx) = tokio::sync::oneshot::channel();
        let fixture = tokio::spawn(async move {
            let (first, _) = listener.accept().await.expect("accept first raw socket");
            drop(first);
            let (_second, _) = listener.accept().await.expect("accept replacement socket");
            replacement_accepted_tx
                .send(())
                .expect("report replacement socket");
            let _ = release_replacement_rx.await;
        });
        let pool = RawHttpUpstreamPool::new(address.ip().to_string(), address.port());
        let first = pool.checkout().await.expect("connect first raw socket");
        first.readable().await.expect("observe closed raw socket");
        pool.checkin(first);

        let replacement = pool
            .checkout()
            .await
            .expect("connect replacement raw socket");
        replacement_accepted_rx
            .await
            .expect("replacement socket reached fixture");
        assert!(raw_http_idle_stream_reusable(&replacement));
        let _ = release_replacement_tx.send(());
        drop(replacement);
        fixture.await.expect("raw pool fixture task");
    }

    #[tokio::test]
    async fn pooled_bidirectional_copy_preserves_game_session_half_close() {
        let (mut client, mut proxy_client) = tokio::io::duplex(1024);
        let (mut proxy_upstream, mut upstream) = tokio::io::duplex(1024);
        let relay = tokio::spawn(async move {
            copy_bidirectional_with_pooled_buffers(
                &mut proxy_client,
                &mut proxy_upstream,
                latency_relay_buffer_pool(),
            )
            .await
        });

        client
            .write_all(b"player-input")
            .await
            .expect("write client");
        client.shutdown().await.expect("half-close client");

        let mut upstream_input = Vec::new();
        upstream
            .read_to_end(&mut upstream_input)
            .await
            .expect("read client input upstream");
        assert_eq!(upstream_input, b"player-input");

        upstream
            .write_all(b"server-reply")
            .await
            .expect("write server reply");
        upstream.shutdown().await.expect("half-close upstream");

        let mut client_reply = Vec::new();
        client
            .read_to_end(&mut client_reply)
            .await
            .expect("read server reply client");
        assert_eq!(client_reply, b"server-reply");
        assert_eq!(
            relay.await.expect("relay task").expect("relay result"),
            (12, 12)
        );
    }

    #[tokio::test]
    async fn prefixed_io_replays_consumed_tls_http_bytes_before_socket_data() {
        let (mut client, server) = tokio::io::duplex(128);
        client.write_all(b"body").await.expect("write socket data");
        let mut prefixed = PrefixedIo::new(server, Bytes::from_static(b"head-"));
        let mut combined = [0_u8; 9];
        prefixed
            .read_exact(&mut combined)
            .await
            .expect("read replayed bytes");
        assert_eq!(&combined, b"head-body");
    }

    #[test]
    fn fast_lane_head_discard_preserves_pipelined_request_and_buffer() {
        let first = b"GET /bench/a HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let second = b"GET /bench/b HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut prefix = BytesMut::with_capacity(4096);
        prefix.extend_from_slice(first);
        prefix.extend_from_slice(second);
        let original_capacity = prefix.capacity();

        discard_fast_lane_http_head(&mut prefix, first.len());

        assert_eq!(&prefix[..], second);
        assert_eq!(prefix.capacity(), original_capacity);
    }

    #[tokio::test]
    async fn fast_lane_reader_returns_head_end_and_reuses_pipelined_buffer() {
        let first = b"GET /bench/a HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let second = b"GET /bench/b HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (mut writer, mut reader) = tokio::io::duplex(256);
        writer.write_all(first).await.expect("write first request");
        writer
            .write_all(second)
            .await
            .expect("write second request");

        let mut prefix = BytesMut::with_capacity(4096);
        let allocation = prefix.as_ptr();
        let first_end = read_fast_lane_http_prefix(&mut reader, &mut prefix)
            .await
            .expect("read pipelined requests")
            .expect("first request head");
        assert_eq!(first_end, first.len());
        assert_eq!(&prefix[..first_end], first);
        discard_fast_lane_http_head(&mut prefix, first_end);

        let second_end = read_fast_lane_http_prefix(&mut reader, &mut prefix)
            .await
            .expect("reuse pipelined request")
            .expect("second request head");
        assert_eq!(second_end, second.len());
        assert_eq!(&prefix[..second_end], second);
        assert_eq!(prefix.as_ptr(), allocation);
    }

    #[tokio::test]
    async fn raw_reverse_response_reader_reuses_connection_buffer() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind response fixture");
        let address = listener.local_addr().expect("fixture address");
        let (continue_second_tx, continue_second_rx) = tokio::sync::oneshot::channel();
        let (continue_third_tx, continue_third_rx) = tokio::sync::oneshot::channel();
        let fixture = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept response fixture");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: keep-alive\r\n\r\npong",
                )
                .await
                .expect("write first response");
            continue_second_rx.await.expect("continue second response");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: keep-alive\r\n\r\npong",
                )
                .await
                .expect("write cached response");
            continue_third_rx.await.expect("continue third response");
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .await
                .expect("write changed response");
        });

        let mut stream = TcpStream::connect(address).await.expect("connect fixture");
        let mut buffer = Vec::with_capacity(4096);
        let mut response_cache = None;
        let allocation = buffer.as_ptr();
        let first =
            read_raw_reverse_http_response_into(&mut stream, &mut buffer, &mut response_cache)
                .await
                .expect("read first response");
        assert_eq!(first.status, StatusCode::OK);
        assert_eq!(first.content_length, Some(4));
        assert!(!first.transfer_chunked);
        assert!(!first.connection_close);
        assert_eq!(&buffer[first.head_end..], b"pong");
        assert_eq!(buffer.as_ptr(), allocation);
        let cached_head = response_cache.as_ref().expect("cache first response");
        assert_eq!(cached_head.raw_head.len(), first.head_end);
        let cached_head_allocation = cached_head.raw_head.as_ptr();

        continue_second_tx
            .send(())
            .expect("continue cached response fixture");
        let second =
            read_raw_reverse_http_response_into(&mut stream, &mut buffer, &mut response_cache)
                .await
                .expect("read second response");
        assert_eq!(second.status, StatusCode::OK);
        assert_eq!(second.content_length, Some(4));
        assert_eq!(&buffer[second.head_end..], b"pong");
        assert_eq!(buffer.as_ptr(), allocation);
        assert_eq!(
            response_cache
                .as_ref()
                .expect("retain exact response cache")
                .raw_head
                .as_ptr(),
            cached_head_allocation
        );

        continue_third_tx
            .send(())
            .expect("continue changed response fixture");
        let third =
            read_raw_reverse_http_response_into(&mut stream, &mut buffer, &mut response_cache)
                .await
                .expect("read third response");
        assert_eq!(third.status, StatusCode::NO_CONTENT);
        assert!(third.content_length.is_none());
        assert!(third.connection_close);
        assert_eq!(buffer.len(), third.head_end);
        assert_eq!(buffer.as_ptr(), allocation);
        assert_eq!(
            response_cache
                .as_ref()
                .expect("replace changed response cache")
                .raw_head
                .len(),
            third.head_end
        );
        fixture.await.expect("response fixture task");
    }

    #[test]
    fn cached_h2_static_response_keeps_range_and_length_headers() {
        let cache = DashMap::new();
        cache.insert(
            "asset.js".to_string(),
            CachedStaticFile {
                len: 4,
                modified: None,
                body: Bytes::from_static(b"test"),
                sendfile: None,
                content_type: HeaderValue::from_static("application/javascript; charset=utf-8"),
                content_length: HeaderValue::from_static("4"),
                checked_at: Instant::now(),
                revalidating: false,
            },
        );

        let response = cached_static_file_response_stale_while_revalidate(
            Path::new("asset.js"),
            &Method::GET,
            &cache,
        )
        .expect("fresh cached response");

        assert!(!response.revalidate);
        assert_eq!(response.response.status(), StatusCode::OK);
        match response.response.body() {
            GatewayBody::Full(Some(body)) => assert_eq!(body, &Bytes::from_static(b"test")),
            _ => panic!("cached H2 response must use a full body"),
        }
        assert_eq!(
            response.response.headers().get(CONTENT_LENGTH),
            Some(&HeaderValue::from_static("4"))
        );
        assert_eq!(
            response.response.headers().get(ACCEPT_RANGES),
            Some(&HeaderValue::from_static("bytes"))
        );
    }

    #[test]
    fn plain_static_stale_candidate_serves_body_and_coalesces_revalidation() {
        let cache = DashMap::new();
        cache.insert(
            "asset.js".to_string(),
            CachedStaticFile {
                len: 4,
                modified: None,
                body: Bytes::from_static(b"test"),
                sendfile: None,
                content_type: HeaderValue::from_static("application/javascript; charset=utf-8"),
                content_length: HeaderValue::from_static("4"),
                checked_at: Instant::now() - Duration::from_secs(2),
                revalidating: false,
            },
        );

        let (candidate, revalidate) = stale_cached_static_file_candidate(
            Path::new("asset.js"),
            "GET",
            &cache,
            STATIC_SENDFILE_FAST_PATH_THRESHOLD_BYTES,
        )
        .expect("stale cached candidate");
        assert!(revalidate);
        assert_eq!(candidate.cached_body.as_deref(), Some(b"test".as_slice()));

        let (_, duplicate_revalidate) = stale_cached_static_file_candidate(
            Path::new("asset.js"),
            "GET",
            &cache,
            STATIC_SENDFILE_FAST_PATH_THRESHOLD_BYTES,
        )
        .expect("stale candidate remains immediately serviceable");
        assert!(!duplicate_revalidate);
    }

    #[test]
    fn sendfile_revalidation_keeps_body_empty_and_clears_inflight_flag() {
        let path = std::env::temp_dir().join(format!(
            "proxysss-sendfile-revalidate-{}.bin",
            Uuid::new_v4()
        ));
        std::fs::write(&path, b"sendfile-data").expect("write sendfile fixture");
        let metadata = std::fs::metadata(&path).expect("sendfile metadata");
        let file = Arc::new(std::fs::File::open(&path).expect("open sendfile fixture"));
        let key = path.to_string_lossy().to_string();
        let cache = DashMap::new();
        cache.insert(
            key.clone(),
            CachedStaticFile {
                len: metadata.len(),
                modified: metadata.modified().ok(),
                body: Bytes::new(),
                sendfile: Some(file.clone()),
                content_type: HeaderValue::from_static("application/octet-stream"),
                content_length: HeaderValue::from_str(&metadata.len().to_string())
                    .expect("content length"),
                checked_at: Instant::now() - Duration::from_secs(2),
                revalidating: true,
            },
        );

        let refreshed =
            cached_static_sendfile(&path, &metadata, &cache).expect("revalidate sendfile entry");
        assert!(Arc::ptr_eq(&file, &refreshed));
        let entry = cache.get(&key).expect("cached sendfile entry");
        assert!(entry.body.is_empty());
        assert!(!entry.revalidating);
        drop(entry);
        std::fs::remove_file(path).expect("remove sendfile fixture");
    }

    #[test]
    fn raw_websocket_fast_lane_appends_forwarding_chain() {
        let request = parse_plain_websocket_fast_lane_request(
            b"GET /ws HTTP/1.1\r\nHost: game.example.com\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: abc\r\nSec-WebSocket-Version: 13\r\nX-Forwarded-For: 10.0.0.1\r\n\r\n",
        )
        .expect("parse websocket request");
        let serialized = serialize_raw_websocket_fast_lane_request(
            &request,
            "/ws",
            "game.example.com",
            "192.0.2.10:4567".parse().expect("remote address"),
            "https",
            true,
        );
        let serialized = String::from_utf8(serialized).expect("utf8 request");
        assert!(serialized.contains("x-real-ip: 192.0.2.10\r\n"));
        assert!(serialized.contains("x-forwarded-for: 10.0.0.1, 192.0.2.10\r\n"));
        assert!(serialized.contains("x-forwarded-proto: https\r\n"));
        assert!(serialized
            .contains("forwarded: for=192.0.2.10;host=\"game.example.com\";proto=https\r\n"));
    }

    #[test]
    fn expected_websocket_disconnects_do_not_escalate_to_warning_logs() {
        let unexpected_eof =
            anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
                .context("websocket tunnel relay failed");
        assert!(is_expected_websocket_disconnect(&unexpected_eof));

        let upstream_timeout = anyhow!("websocket upstream handshake timed out");
        assert!(!is_expected_websocket_disconnect(&upstream_timeout));
    }

    #[test]
    fn websocket_fast_lane_supports_multi_upstream_pool() {
        let route = ReverseProxyRouteConfig {
            upstream: "ws://10.0.0.1:60102".to_string(),
            upstreams: vec![
                "ws://10.0.0.1:60102".to_string(),
                "ws://10.0.0.2:60102".to_string(),
                "ws://10.0.0.3:60102".to_string(),
                "ws://10.0.0.4:60102".to_string(),
            ],
            ..Default::default()
        };

        assert!(websocket_route_fast_path_eligible(&route));
    }

    #[test]
    fn raw_proxy_fast_lane_never_bypasses_observability_or_policy_hooks() {
        let mut config = GatewayConfig::default();
        config.logging.access_log = false;
        config.script.enabled = false;
        config.plugins.enabled = false;
        config.load_balance.retries.enabled = false;
        config.load_balance.active_health.enabled = false;
        config.load_balance.passive_health.enabled = false;
        config.affinity.enabled = false;
        assert!(simple_http_proxy_fast_path_allowed(&config));

        config.logging.access_log = true;
        assert!(!simple_http_proxy_fast_path_allowed(&config));
        config.logging.access_log = false;
        config.script.enabled = true;
        assert!(!simple_http_proxy_fast_path_allowed(&config));
        config.script.enabled = false;
        config.plugins.enabled = true;
        assert!(!simple_http_proxy_fast_path_allowed(&config));
        config.plugins.enabled = false;
        config.security.ddos.enabled = true;
        assert!(!simple_http_proxy_fast_path_allowed(&config));
    }

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
    fn domain_only_auto_https_admin_payload_defaults_to_production() {
        let payload: AutoHttpsUpsertRequest =
            serde_json::from_str(r#"{"domains":["wss.example.com"]}"#)
                .expect("decode domain-only auto https payload");
        assert!(payload.email.is_empty());
        assert!(payload.production);
        assert_eq!(payload.challenge, AcmeChallengeType::TlsAlpn01);

        let rendered = render_config_with_auto_https("plugins:\n  enabled: false\n", &payload)
            .expect("render domain-only auto https config");
        let value: serde_yaml::Value =
            serde_yaml::from_str(&rendered).expect("decode rendered domain-only config");
        let auto_https = value
            .get("http")
            .and_then(|http| http.get("tls"))
            .and_then(|tls| tls.get("auto_https"))
            .expect("rendered auto https config");
        assert_eq!(
            auto_https
                .get("domains")
                .and_then(serde_yaml::Value::as_sequence),
            Some(&vec![serde_yaml::Value::String(
                "wss.example.com".to_string()
            )])
        );
        assert_eq!(
            auto_https
                .get("production")
                .and_then(serde_yaml::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            auto_https
                .get("challenge")
                .and_then(serde_yaml::Value::as_str),
            Some("tls_alpn01")
        );
    }

    #[test]
    fn auto_https_admin_payload_preserves_explicit_http01_compatibility() {
        let payload: AutoHttpsUpsertRequest =
            serde_json::from_str(r#"{"domains":["legacy.example.com"],"challenge":"http01"}"#)
                .expect("decode explicit HTTP-01 payload");
        assert_eq!(payload.challenge, AcmeChallengeType::Http01);

        let rendered = render_config_with_auto_https("plugins:\n  enabled: false\n", &payload)
            .expect("render explicit HTTP-01 config");
        let value: serde_yaml::Value = serde_yaml::from_str(&rendered).expect("decode config");
        assert_eq!(
            value
                .get("http")
                .and_then(|http| http.get("tls"))
                .and_then(|tls| tls.get("auto_https"))
                .and_then(|auto| auto.get("challenge"))
                .and_then(serde_yaml::Value::as_str),
            Some("http01")
        );
    }

    #[test]
    fn admin_https_writes_require_tls_material() {
        let mut config = GatewayConfig::default();
        config.admin.enable_write_ops = true;
        config.admin.https.enabled = true;
        config.http.tls.cert_path = std::env::temp_dir().join("proxysss-missing-admin-cert.pem");
        config.http.tls.key_path = std::env::temp_dir().join("proxysss-missing-admin-key.pem");
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
                forward_headers: true,
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
                forward_headers: true,
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
                forward_headers: true,
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
                protocol: "game_tcp".to_string(),
                nodelay: true,
                connect_timeout_ms: 3_000,
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
                protocol: "kcp".to_string(),
                session_ttl_secs: 180,
                max_associations: 262_144,
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
            forward_headers: true,
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
                    forward_headers: true,
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
                    forward_headers: true,
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
            forward_headers: true,
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
            forward_headers: true,
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
            forward_headers: true,
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
    fn finalize_http_response_preserves_length_for_non_sse_streams() {
        let request_headers = HeaderMap::new();
        let mut response = GatewayHttpResponse::bytes(
            StatusCode::OK,
            "application/octet-stream",
            Bytes::new(),
            "proxysss://static",
        );
        response.stream_body = Some(full_body(Bytes::from_static(b"chunk")));
        response.push_header(CONTENT_LENGTH, HeaderValue::from_static("5"));
        let compression = ResponseCompressionConfig {
            enabled: true,
            algorithms: vec![CompressionAlgorithm::Gzip],
            min_length: 1,
            content_types: vec!["application/".to_string()],
        };

        let response = finalize_http_response(&request_headers, &compression, response)
            .expect("finalize response");
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == CONTENT_LENGTH && value == "5"));
        assert!(!response
            .headers
            .iter()
            .any(|(name, _)| name == CACHE_CONTROL));
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
    fn tcp_stream_accept_worker_count_uses_reuseport_on_linux_performance() {
        let mut config = GatewayConfig::default();
        config.runtime.performance.enabled = true;
        let listener = TcpListenerConfig {
            name: "tcp-echo".to_string(),
            bind: "127.0.0.1:7000".to_string(),
            upstream: "127.0.0.1:7001".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "game_tcp".to_string(),
            nodelay: true,
            connect_timeout_ms: 1_000,
        };

        let workers = tcp_stream_accept_worker_count_for(&config, &listener, 4);
        if cfg!(target_os = "linux") {
            assert_eq!(workers, 4);
            assert_eq!(
                tcp_stream_accept_worker_count_for(&config, &listener, 96),
                96
            );
        } else {
            assert_eq!(workers, 1);
        }

        config.runtime.performance.enabled = false;
        assert_eq!(tcp_stream_accept_worker_count_for(&config, &listener, 4), 1);
    }

    #[test]
    fn linux_http_and_realtime_shards_adapt_to_profile_and_detected_cores() {
        assert_eq!(realtime_stream_reactor_workers_for(1, 2), 1);
        assert_eq!(realtime_stream_reactor_workers_for(4, 2), 2);
        assert_eq!(realtime_stream_reactor_workers_for(96, 2), 48);
        assert_eq!(realtime_stream_reactor_workers_for(4, 1), 4);
        assert_eq!(realtime_stream_reactor_workers_for(96, 1), 96);
        assert_eq!(realtime_stream_reactor_workers_for(4, 4), 1);
        assert_eq!(realtime_stream_reactor_workers_for(96, 4), 24);
        assert_eq!(
            realtime_stream_reactor_cpu_divisor(RuntimePerformanceTrafficProfile::Small),
            2
        );
        assert_eq!(
            realtime_stream_reactor_cpu_divisor(RuntimePerformanceTrafficProfile::Balanced),
            4
        );
        assert_eq!(
            realtime_stream_reactor_cpu_divisor(RuntimePerformanceTrafficProfile::Bulk),
            4
        );
        assert_eq!(
            realtime_stream_reactor_nice_for(RuntimePerformanceTrafficProfile::Small),
            0
        );
        assert_eq!(
            realtime_stream_reactor_nice_for(RuntimePerformanceTrafficProfile::Balanced),
            0
        );
        assert_eq!(
            realtime_stream_reactor_nice_for(RuntimePerformanceTrafficProfile::Bulk),
            5
        );
        assert_eq!(http_data_plane_workers_for(4), 4);
        assert_eq!(http_data_plane_workers_for(96), 96);
        assert!(!shared_udp_runtime_profile(
            RuntimePerformanceTrafficProfile::Small
        ));
        assert!(shared_udp_runtime_profile(
            RuntimePerformanceTrafficProfile::Balanced
        ));
        assert!(!shared_udp_runtime_profile(
            RuntimePerformanceTrafficProfile::Bulk
        ));
        assert_eq!(
            tls_http_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Small),
            1
        );
        assert_eq!(
            tls_http_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Balanced),
            2
        );
        assert_eq!(
            tls_http_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Bulk),
            4
        );
        assert_eq!(tls_http_runtime_workers_for(1, 2), 1);
        assert_eq!(tls_http_runtime_workers_for(4, 2), 2);
        assert_eq!(tls_http_runtime_workers_for(96, 2), 48);
        assert_eq!(tls_http_runtime_workers_for(4, 4), 1);
        assert_eq!(tls_http_runtime_workers_for(96, 4), 24);
        assert_eq!(
            tls_http_runtime_nice_for(RuntimePerformanceTrafficProfile::Small),
            0
        );
        assert_eq!(
            tls_http_runtime_nice_for(RuntimePerformanceTrafficProfile::Balanced),
            0
        );
        assert_eq!(
            tls_http_runtime_nice_for(RuntimePerformanceTrafficProfile::Bulk),
            5
        );
        assert_eq!(
            udp_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Small),
            1
        );
        assert_eq!(
            udp_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Balanced),
            2
        );
        assert_eq!(
            udp_runtime_cpu_divisor(RuntimePerformanceTrafficProfile::Bulk),
            4
        );
        assert_eq!(udp_runtime_workers_for(1, 2), 1);
        assert_eq!(udp_runtime_workers_for(4, 2), 2);
        assert_eq!(udp_runtime_workers_for(96, 2), 48);
        assert_eq!(plain_fast_lane_fairness_batch_for(1), 8);
        assert_eq!(plain_fast_lane_fairness_batch_for(299), 8);
        assert_eq!(plain_fast_lane_fairness_batch_for(300), 32);
        assert_eq!(plain_fast_lane_fairness_batch_for(30_000), 32);
        assert_eq!(
            udp_runtime_nice_for(RuntimePerformanceTrafficProfile::Small),
            0
        );
        assert_eq!(
            udp_runtime_nice_for(RuntimePerformanceTrafficProfile::Balanced),
            12
        );
        assert_eq!(
            udp_runtime_nice_for(RuntimePerformanceTrafficProfile::Bulk),
            12
        );
        assert!(!sendfile_reactor_profile_enabled(
            RuntimePerformanceTrafficProfile::Small
        ));
        assert!(!sendfile_reactor_profile_enabled(
            RuntimePerformanceTrafficProfile::Balanced
        ));
        assert!(sendfile_reactor_profile_enabled(
            RuntimePerformanceTrafficProfile::Bulk
        ));
        let mut sendfile_sequence = 0;
        assert!(balanced_sendfile_mid_yield_for_next_response(
            &mut sendfile_sequence,
            true
        ));
        assert!(balanced_sendfile_mid_yield_for_next_response(
            &mut sendfile_sequence,
            true
        ));
        assert!(!balanced_sendfile_mid_yield_for_next_response(
            &mut sendfile_sequence,
            true
        ));
        assert!(!balanced_sendfile_mid_yield_for_next_response(
            &mut sendfile_sequence,
            false
        ));
        assert_eq!(sendfile_sequence, 3);
        assert_eq!(
            balanced_sendfile_response_sequence_seed(
                "127.0.0.1:30000".parse().expect("phase zero address")
            ),
            0
        );
        assert_eq!(
            balanced_sendfile_response_sequence_seed(
                "127.0.0.1:30001".parse().expect("phase one address")
            ),
            1
        );
        assert_eq!(
            balanced_sendfile_response_sequence_seed(
                "127.0.0.1:30002".parse().expect("phase two address")
            ),
            2
        );
    }

    #[test]
    fn udp_listener_worker_count_uses_reuseport_on_linux_performance() {
        let mut config = GatewayConfig::default();
        config.runtime.performance.enabled = true;

        let workers = udp_listener_worker_count_for(&config, 4);
        if cfg!(target_os = "linux") {
            assert_eq!(workers, 4);
            assert_eq!(udp_listener_worker_count_for(&config, 96), 96);
        } else {
            assert_eq!(workers, 1);
        }

        config.runtime.performance.enabled = false;
        assert_eq!(udp_listener_worker_count_for(&config, 4), 1);
    }

    #[test]
    fn direct_tcp_listener_upstream_requires_policy_free_single_upstream() {
        let mut config = GatewayConfig::default();
        config.runtime.performance.enabled = true;
        config.affinity.enabled = false;
        config.load_balance.active_health.enabled = false;
        config.load_balance.passive_health.enabled = false;
        config.tcp.listeners.push(TcpListenerConfig {
            name: "direct".to_string(),
            bind: "127.0.0.1:7000".to_string(),
            upstream: "127.0.0.1:7001".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "game_tcp".to_string(),
            nodelay: true,
            connect_timeout_ms: 1_000,
        });

        assert_eq!(
            direct_tcp_listener_upstream(&config, "direct").as_deref(),
            Some("127.0.0.1:7001")
        );

        config.tcp.listeners[0]
            .upstreams
            .push("127.0.0.1:7002".to_string());
        assert!(direct_tcp_listener_upstream(&config, "direct").is_none());

        config.tcp.listeners[0].upstreams.clear();
        config.load_balance.active_health.enabled = true;
        assert!(direct_tcp_listener_upstream(&config, "direct").is_none());
    }

    #[test]
    fn direct_udp_listener_upstream_requires_policy_free_single_upstream() {
        let mut config = GatewayConfig::default();
        config.runtime.performance.enabled = true;
        config.affinity.enabled = false;
        config.load_balance.active_health.enabled = false;
        config.load_balance.passive_health.enabled = false;
        config.udp.listeners.push(UdpListenerConfig {
            name: "direct-udp".to_string(),
            bind: "127.0.0.1:7000".to_string(),
            upstream: "127.0.0.1:7001".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "qcp".to_string(),
            session_ttl_secs: 30,
            max_associations: 1024,
        });

        assert_eq!(
            direct_udp_listener_upstream(&config, "direct-udp").as_deref(),
            Some("127.0.0.1:7001")
        );

        config.udp.listeners[0]
            .upstreams
            .push("127.0.0.1:7002".to_string());
        assert!(direct_udp_listener_upstream(&config, "direct-udp").is_none());

        config.udp.listeners[0].upstreams.clear();
        config.load_balance.passive_health.enabled = true;
        assert!(direct_udp_listener_upstream(&config, "direct-udp").is_none());
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
            true,
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
    fn build_upstream_headers_can_skip_forwarding_chain() {
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
        let headers = build_upstream_headers(
            &HeaderMap::new(),
            &route,
            "api.example.com",
            "203.0.113.20:443".parse().expect("remote addr"),
            "https",
            false,
        )
        .expect("headers");

        assert_eq!(
            headers.get(HOST).and_then(|value| value.to_str().ok()),
            Some("api.example.com")
        );
        assert!(!headers.contains_key("x-forwarded-for"));
        assert!(!headers.contains_key("x-real-ip"));
        assert!(!headers.contains_key("forwarded"));
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
        assert!(body.contains("proxysss_critical_task_failures_total"));
        assert!(body.contains("proxysss_watchdog_heartbeat_total"));
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
            bootstrap_fast_lane: FastLaneState::compile(&GatewayConfig::default()),
            dynamic: Arc::new(RwLock::new(Arc::new(DynamicState {
                config: GatewayConfig::default(),
                fast_lane: FastLaneState::compile(&GatewayConfig::default()),
                http_client: reqwest::Client::new(),
                http_fast_client: HyperClient::builder(TokioExecutor::new())
                    .build(HttpConnector::new()),
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
            raw_http_pools: Arc::new(DashMap::new()),
            static_route_cache: Arc::new(DashMap::new()),
            static_file_cache: Arc::new(DashMap::new()),
            static_file_cache_bytes: Arc::new(AtomicU64::new(0)),
            static_file_load_locks: Arc::new(DashMap::new()),
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

        let mut config = GatewayConfig::default();
        config.load_balance.algorithm = LoadBalanceAlgorithm::Rendezvous;
        config.load_balance.active_health.enabled = false;
        config.load_balance.passive_health.enabled = false;
        config.affinity.enabled = false;
        let route = RouteDecision {
            upstream: "ws://127.0.0.1:9000".to_string(),
            upstreams: vec![
                "ws://127.0.0.1:9000".to_string(),
                "ws://127.0.0.1:9001".to_string(),
            ],
            upstream_weights: BTreeMap::new(),
            affinity_key: None,
            rewrite_path: None,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            status: None,
            content_type: None,
        };
        let first = gateway.select_upstream_plan(
            &config,
            &route,
            "websocket",
            Some("ws-capacity"),
            None,
            Some("203.0.113.10:50000"),
        );
        let second = gateway.select_upstream_plan(
            &config,
            &route,
            "websocket",
            Some("ws-capacity"),
            None,
            Some("203.0.113.11:50000"),
        );
        assert_ne!(first[0], second[0]);
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

    async fn dispatch_static_site_for_test(
        site: &StaticSiteConfig,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Result<GatewayHttpResponse> {
        let cache = DashMap::new();
        let cache_bytes = AtomicU64::new(0);
        let load_locks = DashMap::new();
        dispatch_static_site(
            site,
            method,
            uri,
            headers,
            &cache,
            &cache_bytes,
            &load_locks,
        )
        .await
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
        let response = dispatch_static_site_for_test(&site, &Method::GET, &uri, &HeaderMap::new())
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
        let response = dispatch_static_site_for_test(&site, &Method::GET, &uri, &HeaderMap::new())
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, Bytes::from_static(b"hello"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn static_site_serves_byte_ranges() {
        let root =
            std::env::temp_dir().join(format!("proxysss-static-range-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create static root");
        tokio::fs::write(root.join("video.bin"), b"0123456789")
            .await
            .expect("write file");

        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/assets".to_string(),
            root: root.clone(),
            index_files: vec!["index.html".to_string()],
            autoindex: false,
        };
        let uri: Uri = "/assets/video.bin".parse().expect("valid uri");
        let mut headers = HeaderMap::new();
        headers.insert(RANGE, HeaderValue::from_static("bytes=2-5"));
        let response = dispatch_static_site_for_test(&site, &Method::GET, &uri, &headers)
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.body, Bytes::from_static(b"2345"));
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| { name == CONTENT_RANGE && value == "bytes 2-5/10" }));
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| { name == ACCEPT_RANGES && value == "bytes" }));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn static_site_rejects_unsatisfiable_byte_range() {
        let root = std::env::temp_dir().join(format!(
            "proxysss-static-range-unsat-test-{}",
            Uuid::new_v4()
        ));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create static root");
        tokio::fs::write(root.join("asset.bin"), b"abc")
            .await
            .expect("write file");

        let site = StaticSiteConfig {
            name: "public".to_string(),
            path_prefix: "/assets".to_string(),
            root: root.clone(),
            index_files: vec!["index.html".to_string()],
            autoindex: false,
        };
        let uri: Uri = "/assets/asset.bin".parse().expect("valid uri");
        let mut headers = HeaderMap::new();
        headers.insert(RANGE, HeaderValue::from_static("bytes=99-100"));
        let response = dispatch_static_site_for_test(&site, &Method::GET, &uri, &headers)
            .await
            .expect("static response");

        assert_eq!(response.status, StatusCode::RANGE_NOT_SATISFIABLE);
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| { name == CONTENT_RANGE && value == "bytes */3" }));

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
        let response = dispatch_static_site_for_test(&site, &Method::GET, &uri, &HeaderMap::new())
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
            protocol: "game_tcp".to_string(),
            nodelay: true,
            connect_timeout_ms: 3_000,
        });
        config.udp.listeners.push(UdpListenerConfig {
            name: "game-udp".to_string(),
            bind: "0.0.0.0:7001".to_string(),
            upstream: String::new(),
            upstreams: vec!["127.0.0.1:9100".to_string(), "127.0.0.1:9101".to_string()],
            upstream_weights: BTreeMap::new(),
            protocol: "kcp".to_string(),
            session_ttl_secs: 180,
            max_associations: 262_144,
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
    fn active_health_collects_udp_targets_when_enabled() {
        let mut config = GatewayConfig::default();
        config.load_balance.active_health.http_enabled = false;
        config.load_balance.active_health.tcp_enabled = false;
        config.load_balance.active_health.udp_enabled = true;
        config.udp.listeners.push(UdpListenerConfig {
            name: "game-kcp".to_string(),
            bind: "0.0.0.0:7001".to_string(),
            upstream: String::new(),
            upstreams: vec!["127.0.0.1:9100".to_string(), "127.0.0.1:9101".to_string()],
            upstream_weights: BTreeMap::new(),
            protocol: "kcp".to_string(),
            session_ttl_secs: 180,
            max_associations: 262_144,
        });

        let targets = collect_active_health_targets(&config);
        let udp_targets = targets
            .iter()
            .filter(|target| target.kind.as_str() == "udp")
            .collect::<Vec<_>>();

        assert_eq!(udp_targets.len(), 2);
        assert!(udp_targets
            .iter()
            .any(|target| target.upstream == "127.0.0.1:9100"));
        assert!(udp_targets
            .iter()
            .all(|target| target.settings.udp_payload == "proxysss-health"));
    }

    #[tokio::test]
    async fn udp_association_prune_expires_and_caps_entries() {
        let associations = DashMap::<SocketAddr, Arc<UdpAssociation>>::new();
        let std_socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("udp socket");
        std_socket.set_nonblocking(true).expect("nonblocking");
        let socket = Arc::new(UdpSocket::from_std(std_socket).expect("tokio udp socket"));
        let now = now_unix_secs();

        for port in 20_000..20_005 {
            associations.insert(
                format!("127.0.0.1:{port}").parse().expect("addr"),
                Arc::new(UdpAssociation {
                    socket: socket.clone(),
                    last_seen_epoch: AtomicU64::new(now.saturating_sub(1_000 + port as u64)),
                    active: AtomicBool::new(true),
                }),
            );
        }
        associations.insert(
            "127.0.0.1:30000".parse().expect("addr"),
            Arc::new(UdpAssociation {
                socket,
                last_seen_epoch: AtomicU64::new(now),
                active: AtomicBool::new(true),
            }),
        );

        prune_udp_associations(&associations, 180, 4);

        assert_eq!(associations.len(), 1);
        assert!(associations.contains_key(&"127.0.0.1:30000".parse::<SocketAddr>().expect("addr")));
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
