use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use reqwest::Method;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::task::JoinSet;

#[derive(Subcommand, Debug, Clone)]
pub enum BenchCommand {
    Http(HttpBenchArgs),
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
}

#[derive(Default)]
struct BenchStats {
    success: AtomicU64,
    errors: AtomicU64,
    bytes: AtomicU64,
    latencies_us: Mutex<Vec<u64>>,
}

impl BenchStats {
    fn record_success(&self, latency: Duration, bytes: usize) {
        self.success.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.latencies_us.lock().expect("latency lock poisoned").push(latency.as_micros() as u64);
    }

    fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
}

pub async fn run(command: BenchCommand) -> Result<()> {
    match command {
        BenchCommand::Http(args) => run_http(args).await,
        BenchCommand::Tcp(args) => run_tcp(args).await,
        BenchCommand::Udp(args) => run_udp(args).await,
    }
}

async fn run_http(args: HttpBenchArgs) -> Result<()> {
    let stats = Arc::new(BenchStats::default());
    let method = Method::from_bytes(args.method.as_bytes()).context("invalid http method")?;
    let payload = Arc::new(vec![b'x'; args.body_bytes]);
    let deadline = Instant::now() + Duration::from_secs(args.duration_secs.max(1));
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(args.insecure)
        .http2_adaptive_window(true)
        .build()
        .context("failed to build reqwest client")?;
    let url = Arc::new(args.url);

    let mut tasks = JoinSet::new();
    for _ in 0..args.concurrency.max(1) {
        let stats = stats.clone();
        let client = client.clone();
        let url = url.clone();
        let payload = payload.clone();
        let method = method.clone();

        tasks.spawn(async move {
            while Instant::now() < deadline {
                let started = Instant::now();
                match client.request(method.clone(), url.as_str()).body(payload.as_ref().clone()).send().await {
                    Ok(response) => match response.bytes().await {
                        Ok(bytes) => stats.record_success(started.elapsed(), bytes.len()),
                        Err(_) => stats.record_error(),
                    },
                    Err(_) => stats.record_error(),
                }
            }
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.context("http benchmark task failed")?;
    }

    print_summary("http", args.duration_secs, &stats);
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
            let mut stream = loop {
                match TcpStream::connect(addr.as_str()).await {
                    Ok(stream) => break stream,
                    Err(_) => {
                        stats.record_error();
                        if Instant::now() >= deadline {
                            return;
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
                    Ok(()) => stats.record_success(started.elapsed(), payload.len() * 2),
                    Err(_) => {
                        stats.record_error();
                        match TcpStream::connect(addr.as_str()).await {
                            Ok(new_stream) => stream = new_stream,
                            Err(_) => break,
                        }
                    }
                }
            }
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.context("tcp benchmark task failed")?;
    }

    print_summary("tcp", args.duration_secs, &stats);
    Ok(())
}

async fn run_udp(args: UdpBenchArgs) -> Result<()> {
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
            let bind_any = if addr.contains(':') && !addr.contains('.') { "[::]:0" } else { "0.0.0.0:0" };
            let socket = match UdpSocket::bind(bind_any).await {
                Ok(socket) => socket,
                Err(_) => {
                    stats.record_error();
                    return;
                }
            };

            if socket.connect(addr.as_str()).await.is_err() {
                stats.record_error();
                return;
            }

            let mut buffer = vec![0_u8; payload.len().max(65_536)];
            while Instant::now() < deadline {
                let started = Instant::now();
                let result = async {
                    socket.send(&payload).await?;
                    let size = match tokio::time::timeout(Duration::from_millis(1000), socket.recv(&mut buffer)).await {
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
                    Ok(size) => stats.record_success(started.elapsed(), payload.len() + size),
                    Err(_) => stats.record_error(),
                }
            }
        });
    }

    while let Some(result) = tasks.join_next().await {
        result.context("udp benchmark task failed")?;
    }

    print_summary("udp", args.duration_secs, &stats);
    Ok(())
}

fn print_summary(protocol: &str, duration_secs: u64, stats: &BenchStats) {
    let success = stats.success.load(Ordering::Relaxed);
    let errors = stats.errors.load(Ordering::Relaxed);
    let bytes = stats.bytes.load(Ordering::Relaxed);
    let mut latencies = stats.latencies_us.lock().expect("latency lock poisoned").clone();
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
        println!("latency p50   : {:.3} ms", percentile(&latencies, 0.50) as f64 / 1000.0);
        println!("latency p95   : {:.3} ms", percentile(&latencies, 0.95) as f64 / 1000.0);
        println!("latency p99   : {:.3} ms", percentile(&latencies, 0.99) as f64 / 1000.0);
    }
}

fn percentile(values: &[u64], quantile: f64) -> u64 {
    let last = values.len().saturating_sub(1);
    let index = ((last as f64) * quantile).round() as usize;
    values[index.min(last)]
}
