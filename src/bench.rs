use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use futures::{SinkExt, StreamExt};
use reqwest::Method;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Barrier;
use tokio::task::JoinSet;
use tokio_tungstenite::{
    connect_async_tls_with_config, connect_async_with_config, tungstenite::Message, Connector,
    MaybeTlsStream, WebSocketStream,
};

#[derive(Subcommand, Debug, Clone)]
pub enum BenchCommand {
    Http(HttpBenchArgs),
    Sse(SseBenchArgs),
    Websocket(WebSocketBenchArgs),
    Tcp(TcpBenchArgs),
    Udp(UdpBenchArgs),
}

#[derive(Args, Debug, Clone)]
pub struct HttpBenchArgs {
    #[arg(long)]
    pub url: String,
    #[arg(long, default_value_t = 512)]
    pub concurrency: usize,
    #[arg(long, default_value_t = 30)]
    pub duration_secs: u64,
    #[arg(long, default_value_t = 0)]
    pub body_bytes: usize,
    #[arg(long, default_value = "GET")]
    pub method: String,
    #[arg(long, default_value_t = false)]
    pub insecure: bool,
    #[arg(long, default_value_t = false)]
    pub http1_only: bool,
}

#[derive(Args, Debug, Clone)]
pub struct SseBenchArgs {
    #[arg(long)]
    pub url: String,
    #[arg(long, default_value_t = 128)]
    pub concurrency: usize,
    #[arg(long, default_value_t = 30)]
    pub duration_secs: u64,
    #[arg(long, default_value_t = 8)]
    pub max_chunks: usize,
    #[arg(long, default_value_t = false)]
    pub insecure: bool,
    #[arg(long, default_value_t = false)]
    pub http1_only: bool,
}

#[derive(Args, Debug, Clone)]
pub struct WebSocketBenchArgs {
    #[arg(long)]
    pub url: String,
    #[arg(long, default_value_t = 512)]
    pub connections: usize,
    #[arg(long, default_value_t = 30)]
    pub duration_secs: u64,
    #[arg(long, default_value_t = 256)]
    pub payload_bytes: usize,
    /// Open the requested connections, keep them idle, then report connection
    /// capacity and WebSocket handshake latency instead of echo message rate.
    #[arg(long, default_value_t = false)]
    pub hold_connections: bool,
    /// Parallel connection-open workers used by --hold-connections.
    #[arg(long, default_value_t = 128)]
    pub connect_workers: usize,
    /// Per-attempt WebSocket handshake deadline used by --hold-connections.
    #[arg(long, default_value_t = 10_000)]
    pub connect_timeout_ms: u64,
    /// Retry count after the initial failed handshake used by --hold-connections.
    #[arg(long, default_value_t = 2)]
    pub connect_retries: usize,
    /// Accept an untrusted WSS certificate. Intended only for isolated benchmark
    /// fixtures that use a generated self-signed certificate.
    #[arg(long, default_value_t = false)]
    pub insecure: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TcpBenchArgs {
    #[arg(long)]
    pub addr: String,
    #[arg(long, default_value_t = 512)]
    pub connections: usize,
    #[arg(long, default_value_t = 30)]
    pub duration_secs: u64,
    #[arg(long, default_value_t = 1024)]
    pub payload_bytes: usize,
}

#[derive(Args, Debug, Clone)]
pub struct UdpBenchArgs {
    #[arg(long)]
    pub addr: String,
    #[arg(long, default_value_t = 512)]
    pub connections: usize,
    #[arg(long, default_value_t = 30)]
    pub duration_secs: u64,
    #[arg(long, default_value_t = 512)]
    pub payload_bytes: usize,
    #[arg(long, default_value_t = 1000)]
    pub timeout_ms: u64,
}

#[derive(Default)]
struct BenchStats {
    success: AtomicU64,
    errors: AtomicU64,
    bytes: AtomicU64,
}

type BenchmarkWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug)]
struct InsecureBenchmarkCertVerifier;

impl ServerCertVerifier for InsecureBenchmarkCertVerifier {
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

fn insecure_wss_client_config(insecure: bool) -> Option<Arc<ClientConfig>> {
    if !insecure {
        return None;
    }

    let _ = rustls::crypto::ring::default_provider().install_default();
    Some(Arc::new(
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureBenchmarkCertVerifier))
            .with_no_client_auth(),
    ))
}

async fn connect_websocket(
    url: &str,
    insecure_tls_config: Option<Arc<ClientConfig>>,
) -> std::result::Result<
    (
        BenchmarkWebSocket,
        tokio_tungstenite::tungstenite::handshake::client::Response,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    match insecure_tls_config {
        Some(config) => {
            connect_async_tls_with_config(url, None, true, Some(Connector::Rustls(config))).await
        }
        None => connect_async_with_config(url, None, true).await,
    }
}

#[derive(Default)]
struct TaskStats {
    success: u64,
    errors: u64,
    bytes: u64,
    latencies_us: Vec<u64>,
}

impl BenchStats {
    fn add_task(&self, task: &TaskStats) {
        self.success.fetch_add(task.success, Ordering::Relaxed);
        self.errors.fetch_add(task.errors, Ordering::Relaxed);
        self.bytes.fetch_add(task.bytes, Ordering::Relaxed);
    }
}

impl TaskStats {
    fn record_success(&mut self, latency: Duration, bytes: usize) {
        self.success = self.success.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes as u64);
        self.latencies_us.push(latency.as_micros() as u64);
    }

    fn record_error(&mut self) {
        self.errors = self.errors.saturating_add(1);
    }
}

pub async fn run(command: BenchCommand) -> Result<()> {
    match command {
        BenchCommand::Http(args) => run_http(args).await,
        BenchCommand::Sse(args) => run_sse(args).await,
        BenchCommand::Websocket(args) => run_websocket(args).await,
        BenchCommand::Tcp(args) => run_tcp(args).await,
        BenchCommand::Udp(args) => run_udp(args).await,
    }
}

async fn run_http(args: HttpBenchArgs) -> Result<()> {
    let stats = Arc::new(BenchStats::default());
    let method = Method::from_bytes(args.method.as_bytes()).context("invalid http method")?;
    let payload = Arc::new(vec![b'x'; args.body_bytes]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(args.insecure)
        .http2_adaptive_window(true)
        .timeout(Duration::from_secs(10));
    if args.http1_only {
        builder = builder.http1_only();
    }
    let client = builder.build().context("failed to build reqwest client")?;
    let url = Arc::new(args.url);

    let mut tasks = JoinSet::new();
    for _ in 0..args.concurrency.max(1) {
        let stats = stats.clone();
        let client = client.clone();
        let url = url.clone();
        let payload = payload.clone();
        let method = method.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            while Instant::now() < deadline {
                let started = Instant::now();
                match client
                    .request(method.clone(), url.as_str())
                    .body(payload.as_ref().clone())
                    .send()
                    .await
                {
                    Ok(response) => match response.bytes().await {
                        Ok(bytes) => local.record_success(started.elapsed(), bytes.len()),
                        Err(_) => local.record_error(),
                    },
                    Err(_) => local.record_error(),
                }
            }
            stats.add_task(&local);
            local.latencies_us
        });
    }

    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        latencies.extend(result.context("http benchmark task failed")?);
    }

    print_summary("http", args.duration_secs, &stats, latencies);
    Ok(())
}

async fn run_sse(args: SseBenchArgs) -> Result<()> {
    let stats = Arc::new(BenchStats::default());
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let request_timeout = Duration::from_secs(10);
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(args.insecure)
        .tcp_nodelay(true)
        .http2_adaptive_window(true)
        .http2_keep_alive_timeout(Duration::from_secs(90))
        .http2_keep_alive_interval(Some(Duration::from_secs(30)))
        .http2_keep_alive_while_idle(true)
        .timeout(request_timeout);
    if args.http1_only {
        builder = builder.http1_only();
    }
    let client = builder.build().context("failed to build reqwest client")?;
    let url = Arc::new(args.url);
    let max_chunks = args.max_chunks.max(1);

    let mut tasks = JoinSet::new();
    for _ in 0..args.concurrency.max(1) {
        let stats = stats.clone();
        let client = client.clone();
        let url = url.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            while Instant::now() < deadline {
                let started = Instant::now();
                let exchange = async {
                    let response = client
                        .get(url.as_str())
                        .header(reqwest::header::ACCEPT, "text/event-stream")
                        .send()
                        .await
                        .map_err(|_| ())?;

                    if !response.status().is_success() {
                        return Err(());
                    }

                    let mut stream = response.bytes_stream();
                    let mut chunks = 0_usize;
                    let mut bytes = 0_usize;
                    let mut first_chunk_latency = None;
                    while chunks < max_chunks {
                        match stream.next().await {
                            Some(Ok(chunk)) => {
                                if first_chunk_latency.is_none() {
                                    first_chunk_latency = Some(started.elapsed());
                                }
                                chunks += 1;
                                bytes = bytes.saturating_add(chunk.len());
                            }
                            Some(Err(_)) => return Err(()),
                            None => break,
                        }
                    }

                    if chunks > 0 {
                        Ok((
                            first_chunk_latency.unwrap_or_else(|| started.elapsed()),
                            bytes,
                        ))
                    } else {
                        Err(())
                    }
                };

                match tokio::time::timeout(request_timeout, exchange).await {
                    Ok(Ok((latency, bytes))) => local.record_success(latency, bytes),
                    Ok(Err(())) | Err(_) => local.record_error(),
                }
            }
            stats.add_task(&local);
            local.latencies_us
        });
    }

    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        latencies.extend(result.context("sse benchmark task failed")?);
    }

    print_summary("sse", args.duration_secs, &stats, latencies);
    Ok(())
}

async fn run_websocket(args: WebSocketBenchArgs) -> Result<()> {
    if args.hold_connections {
        return run_websocket_connection_capacity(args).await;
    }

    let stats = Arc::new(BenchStats::default());
    let payload = Arc::new(vec![b'x'; args.payload_bytes.max(1)]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let url = Arc::new(args.url);
    let insecure_tls_config = insecure_wss_client_config(args.insecure);

    let mut tasks = JoinSet::new();
    for _ in 0..args.connections.max(1) {
        let stats = stats.clone();
        let payload = payload.clone();
        let url = url.clone();
        let insecure_tls_config = insecure_tls_config.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            let mut websocket = loop {
                match connect_websocket(url.as_str(), insecure_tls_config.clone()).await {
                    Ok((stream, _)) => break stream,
                    Err(_) => {
                        local.record_error();
                        if Instant::now() >= deadline {
                            stats.add_task(&local);
                            return local.latencies_us;
                        }
                    }
                }
            };

            while Instant::now() < deadline {
                let started = Instant::now();
                let result = async {
                    websocket
                        .send(Message::Binary(payload.as_ref().clone().into()))
                        .await?;
                    loop {
                        match websocket.next().await {
                            Some(Ok(Message::Binary(bytes))) => {
                                return Ok::<usize, tokio_tungstenite::tungstenite::Error>(
                                    bytes.len(),
                                );
                            }
                            Some(Ok(Message::Text(text))) => return Ok(text.len()),
                            Some(Ok(Message::Ping(payload))) => {
                                websocket.send(Message::Pong(payload)).await?;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                return Err(
                                    tokio_tungstenite::tungstenite::Error::ConnectionClosed,
                                );
                            }
                            Some(Ok(_)) => {}
                            Some(Err(error)) => return Err(error),
                        }
                    }
                }
                .await;

                match result {
                    Ok(size) => local.record_success(started.elapsed(), payload.len() + size),
                    Err(_) if Instant::now() < deadline => {
                        local.record_error();
                        match connect_websocket(url.as_str(), insecure_tls_config.clone()).await {
                            Ok((stream, _)) => websocket = stream,
                            Err(_) => break,
                        }
                    }
                    Err(_) => {}
                }
            }
            stats.add_task(&local);
            local.latencies_us
        });
    }

    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        latencies.extend(result.context("websocket benchmark task failed")?);
    }

    print_summary("websocket", args.duration_secs, &stats, latencies);
    Ok(())
}

const MAX_CONNECTION_LATENCY_SAMPLES_PER_WORKER: usize = 2_048;

struct WebSocketConnectionCapacityTask {
    opened: u64,
    failed: u64,
    attempts: u64,
    latencies_us: Vec<u64>,
}

/// Measure idle WebSocket connection capacity without turning every socket into
/// a message-rate benchmark. A bounded number of workers opens the requested
/// sockets, synchronizes once all workers have finished their attempts, then
/// holds successful connections open for `duration_secs`.
///
/// For 100k+ connections, start this command from multiple client network
/// namespaces or hosts. A single IPv4 source address has only about 64k source
/// ports per destination tuple, and most Linux hosts expose a smaller ephemeral
/// port range by default.
async fn run_websocket_connection_capacity(args: WebSocketBenchArgs) -> Result<()> {
    let requested = args.connections.max(1);
    let worker_count = args.connect_workers.clamp(1, requested);
    let url = Arc::new(args.url);
    let insecure_tls_config = insecure_wss_client_config(args.insecure);
    let connect_timeout = Duration::from_millis(args.connect_timeout_ms.max(1));
    let hold_duration = Duration::from_secs(args.duration_secs.max(1));
    let barrier = Arc::new(Barrier::new(worker_count));
    let started = Instant::now();
    let mut tasks = JoinSet::new();

    for worker_index in 0..worker_count {
        let start = requested * worker_index / worker_count;
        let end = requested * (worker_index + 1) / worker_count;
        let url = url.clone();
        let barrier = barrier.clone();
        let connect_retries = args.connect_retries;
        let insecure_tls_config = insecure_tls_config.clone();

        tasks.spawn(async move {
            let mut sockets = Vec::with_capacity(end - start);
            let mut result = WebSocketConnectionCapacityTask {
                opened: 0,
                failed: 0,
                attempts: 0,
                latencies_us: Vec::with_capacity(
                    (end - start).min(MAX_CONNECTION_LATENCY_SAMPLES_PER_WORKER),
                ),
            };

            for _ in start..end {
                let mut opened = false;
                for attempt in 0..=connect_retries {
                    result.attempts = result.attempts.saturating_add(1);
                    let handshake_started = Instant::now();
                    match tokio::time::timeout(
                        connect_timeout,
                        connect_websocket(url.as_str(), insecure_tls_config.clone()),
                    )
                    .await
                    {
                        Ok(Ok((stream, _))) => {
                            result.opened = result.opened.saturating_add(1);
                            if result.latencies_us.len() < MAX_CONNECTION_LATENCY_SAMPLES_PER_WORKER
                            {
                                result
                                    .latencies_us
                                    .push(handshake_started.elapsed().as_micros() as u64);
                            }
                            sockets.push(stream);
                            opened = true;
                            break;
                        }
                        Ok(Err(_)) | Err(_) if attempt < connect_retries => {
                            let retry_delay_ms = 10_u64.saturating_mul(1_u64 << attempt.min(6));
                            tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
                        }
                        Ok(Err(_)) | Err(_) => break,
                    }
                }
                if !opened {
                    result.failed = result.failed.saturating_add(1);
                }
            }

            barrier.wait().await;
            tokio::time::sleep(hold_duration).await;
            // Keep the stream vector live through the hold interval. `len` is
            // intentionally observed after the sleep so the optimizer cannot
            // drop the WebSocket handles before this capacity sample ends.
            let _held_open = sockets.len();
            result
        });
    }

    let mut opened = 0_u64;
    let mut failed = 0_u64;
    let mut attempts = 0_u64;
    let mut latencies_us = Vec::new();
    while let Some(task) = tasks.join_next().await {
        let task = task.context("websocket connection-capacity worker failed")?;
        opened = opened.saturating_add(task.opened);
        failed = failed.saturating_add(task.failed);
        attempts = attempts.saturating_add(task.attempts);
        latencies_us.extend(task.latencies_us);
    }

    print_websocket_connection_capacity_summary(
        requested,
        opened,
        failed,
        attempts,
        started.elapsed(),
        args.duration_secs.max(1),
        latencies_us,
    );
    Ok(())
}

async fn run_tcp(args: TcpBenchArgs) -> Result<()> {
    let stats = Arc::new(BenchStats::default());
    let payload = Arc::new(vec![b'x'; args.payload_bytes.max(1)]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let addr = Arc::new(args.addr);

    let mut tasks = JoinSet::new();
    for _ in 0..args.connections.max(1) {
        let stats = stats.clone();
        let payload = payload.clone();
        let addr = addr.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            let mut stream = loop {
                match TcpStream::connect(addr.as_str()).await {
                    Ok(stream) => break stream,
                    Err(_) => {
                        local.record_error();
                        if Instant::now() >= deadline {
                            stats.add_task(&local);
                            return local.latencies_us;
                        }
                    }
                }
            };

            let mut buffer = vec![0_u8; payload.len()];
            while Instant::now() < deadline {
                let started = Instant::now();
                let result = async {
                    stream.write_all(&payload).await?;
                    stream.read_exact(&mut buffer).await?;
                    Ok::<(), std::io::Error>(())
                }
                .await;

                match result {
                    Ok(()) => local.record_success(started.elapsed(), payload.len() * 2),
                    Err(_) => {
                        local.record_error();
                        match TcpStream::connect(addr.as_str()).await {
                            Ok(new_stream) => stream = new_stream,
                            Err(_) => break,
                        }
                    }
                }
            }
            stats.add_task(&local);
            local.latencies_us
        });
    }

    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        latencies.extend(result.context("tcp benchmark task failed")?);
    }

    print_summary("tcp", args.duration_secs, &stats, latencies);
    Ok(())
}

async fn run_udp(args: UdpBenchArgs) -> Result<()> {
    let stats = Arc::new(BenchStats::default());
    let payload = Arc::new(vec![b'x'; args.payload_bytes.max(1)]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let addr = Arc::new(args.addr);
    let timeout_ms = args.timeout_ms.max(1);

    let mut tasks = JoinSet::new();
    for _ in 0..args.connections.max(1) {
        let stats = stats.clone();
        let payload = payload.clone();
        let addr = addr.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            let bind_any = if addr.contains(':') && !addr.contains('.') {
                "[::]:0"
            } else {
                "0.0.0.0:0"
            };
            let socket = match UdpSocket::bind(bind_any).await {
                Ok(socket) => socket,
                Err(_) => {
                    local.record_error();
                    stats.add_task(&local);
                    return local.latencies_us;
                }
            };

            if socket.connect(addr.as_str()).await.is_err() {
                local.record_error();
                stats.add_task(&local);
                return local.latencies_us;
            }

            let mut buffer = vec![0_u8; payload.len().max(65_536)];
            while Instant::now() < deadline {
                let started = Instant::now();
                let result = async {
                    socket.send(&payload).await?;
                    let size = match tokio::time::timeout(
                        Duration::from_millis(timeout_ms),
                        socket.recv(&mut buffer),
                    )
                    .await
                    {
                        Ok(result) => result?,
                        Err(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "udp recv timeout",
                            ));
                        }
                    };
                    Ok::<usize, std::io::Error>(size)
                }
                .await;

                match result {
                    Ok(size) => local.record_success(started.elapsed(), payload.len() + size),
                    Err(_) if Instant::now() < deadline => local.record_error(),
                    Err(_) => {}
                }
            }
            stats.add_task(&local);
            local.latencies_us
        });
    }

    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        latencies.extend(result.context("udp benchmark task failed")?);
    }

    print_summary("udp", args.duration_secs, &stats, latencies);
    Ok(())
}

fn print_summary(protocol: &str, duration_secs: u64, stats: &BenchStats, mut latencies: Vec<u64>) {
    let success = stats.success.load(Ordering::Relaxed);
    let errors = stats.errors.load(Ordering::Relaxed);
    let bytes = stats.bytes.load(Ordering::Relaxed);
    latencies.sort_unstable();

    let seconds = duration_secs.max(1) as f64;
    let rps = success as f64 / seconds;
    let mbps = bytes as f64 / 1024.0 / 1024.0 / seconds;

    println!("protocol      : {protocol}");
    println!("success       : {success}");
    println!("errors        : {errors}");
    println!("throughput    : {:.2} MiB/s", mbps);
    println!("ops/sec       : {:.2}", rps);

    if !latencies.is_empty() {
        println!(
            "latency p50   : {:.3} ms",
            percentile(&latencies, 0.50) as f64 / 1000.0
        );
        println!(
            "latency p95   : {:.3} ms",
            percentile(&latencies, 0.95) as f64 / 1000.0
        );
        println!(
            "latency p99   : {:.3} ms",
            percentile(&latencies, 0.99) as f64 / 1000.0
        );
    }
}

fn print_websocket_connection_capacity_summary(
    requested: usize,
    opened: u64,
    failed: u64,
    attempts: u64,
    elapsed: Duration,
    hold_secs: u64,
    mut latencies_us: Vec<u64>,
) {
    latencies_us.sort_unstable();
    let open_seconds = elapsed
        .saturating_sub(Duration::from_secs(hold_secs))
        .as_secs_f64()
        .max(f64::EPSILON);

    println!("protocol              : websocket-connections");
    println!("connections requested : {requested}");
    println!("connections opened    : {opened}");
    println!("connections failed    : {failed}");
    println!("handshake attempts    : {attempts}");
    println!(
        "open rate             : {:.2} connections/s",
        opened as f64 / open_seconds
    );
    println!("hold duration         : {hold_secs} s");

    if !latencies_us.is_empty() {
        println!(
            "handshake p50         : {:.3} ms",
            percentile(&latencies_us, 0.50) as f64 / 1000.0
        );
        println!(
            "handshake p95         : {:.3} ms",
            percentile(&latencies_us, 0.95) as f64 / 1000.0
        );
        println!(
            "handshake p99         : {:.3} ms",
            percentile(&latencies_us, 0.99) as f64 / 1000.0
        );
    }
}

fn percentile(values: &[u64], quantile: f64) -> u64 {
    let last = values.len().saturating_sub(1);
    let index = ((last as f64) * quantile).round() as usize;
    values[index.min(last)]
}
