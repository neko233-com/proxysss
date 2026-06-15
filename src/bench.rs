use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use futures::{SinkExt, StreamExt};
use reqwest::Method;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::task::JoinSet;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
    let stats = Arc::new(BenchStats::default());
    let payload = Arc::new(vec![b'x'; args.payload_bytes.max(1)]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let url = Arc::new(args.url);

    let mut tasks = JoinSet::new();
    for _ in 0..args.connections.max(1) {
        let stats = stats.clone();
        let payload = payload.clone();
        let url = url.clone();

        tasks.spawn(async move {
            let mut local = TaskStats::default();
            let mut websocket = loop {
                match connect_async(url.as_str()).await {
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
                        match connect_async(url.as_str()).await {
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

fn percentile(values: &[u64], quantile: f64) -> u64 {
    let last = values.len().saturating_sub(1);
    let index = ((last as f64) * quantile).round() as usize;
    values[index.min(last)]
}
