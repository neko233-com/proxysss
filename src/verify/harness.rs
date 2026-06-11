//! Shared test harness for spinning up gateways and upstream mocks.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_tungstenite::accept_async;

use crate::config::GatewayConfig;
use crate::gateway::Gateway;

static GATEWAY_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static RUSTLS_PROVIDER_INIT: std::sync::Once = std::sync::Once::new();

pub fn ensure_rustls_crypto_provider() {
    RUSTLS_PROVIDER_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

pub async fn gateway_test_guard() -> tokio::sync::MutexGuard<'static, ()> {
    GATEWAY_TEST_LOCK.lock().await
}

pub async fn reserve_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to reserve ephemeral port")?;
    Ok(listener.local_addr()?.port())
}

pub async fn wait_http_ok(url: &str) -> Result<()> {
    wait_http_status(url, |status| status.is_success()).await
}

pub async fn wait_http_status<F>(url: &str, predicate: F) -> Result<()>
where
    F: Fn(reqwest::StatusCode) -> bool,
{
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    for _ in 0..100 {
        if let Ok(response) = client.get(url).send().await {
            if predicate(response.status()) {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("endpoint not ready: {url}")
}

pub async fn wait_tcp_ready(addr: &str) -> Result<()> {
    let probe_addr = loopback_probe_addr(addr);
    for _ in 0..100 {
        if let Ok(stream) = TcpStream::connect(&probe_addr).await {
            drop(stream);
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    anyhow::bail!("tcp endpoint not ready: {addr}")
}

fn loopback_probe_addr(addr: &str) -> String {
    if let Some(port) = addr.strip_prefix("0.0.0.0:") {
        return format!("127.0.0.1:{port}");
    }
    if let Some(port) = addr.strip_prefix("[::]:") {
        return format!("[::1]:{port}");
    }
    addr.to_string()
}

pub fn base_gateway_yaml(gateway_port: u16) -> String {
    format!(
        r#"config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:{gateway_port}
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
"#
    )
}

pub async fn spawn_json_echo_upstream(port: u16) -> tokio::task::JoinHandle<()> {
    let bind = format!("127.0.0.1:{port}");
    tokio::spawn(async move {
        let listener = TcpListener::bind(&bind)
            .await
            .expect("bind json echo upstream");
        loop {
            let Ok((stream, remote_addr)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let service = service_fn(move |request: Request<Incoming>| async move {
                    let path = request.uri().path().to_string();
                    let host = request
                        .headers()
                        .get("host")
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string);
                    let content_type = request
                        .headers()
                        .get("content-type")
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string);
                    let body = request
                        .into_body()
                        .collect()
                        .await
                        .map(|collected| collected.to_bytes())
                        .unwrap_or_default();
                    let payload = serde_json::json!({
                        "ok": true,
                        "remote_addr": remote_addr.to_string(),
                        "path": path,
                        "host": host,
                        "content_type": content_type,
                        "body_len": body.len(),
                    });
                    Ok::<_, hyper::Error>(
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "application/json")
                            .header("cache-control", "public, max-age=60")
                            .body(Full::new(Bytes::from(payload.to_string())))
                            .expect("json echo response"),
                    )
                });

                let _ = AutoBuilder::new(TokioExecutor::new())
                    .serve_connection_with_upgrades(TokioIo::new(stream), service)
                    .await;
            });
        }
    })
}

pub async fn spawn_sse_hold_upstream(port: u16) -> tokio::task::JoinHandle<()> {
    let bind = format!("127.0.0.1:{port}");
    tokio::spawn(async move {
        let listener = TcpListener::bind(&bind).await.expect("bind sse upstream");
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer).await;
                let head = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "content-type: text/event-stream\r\n",
                    "cache-control: no-cache\r\n",
                    "connection: close\r\n",
                    "\r\n",
                    "data: first\r\n\r\n"
                );
                if stream.write_all(head.as_bytes()).await.is_ok() {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });
        }
    })
}

pub async fn spawn_ws_echo_upstream(port: u16) -> tokio::task::JoinHandle<()> {
    let bind = format!("127.0.0.1:{port}");
    tokio::spawn(async move {
        let listener = TcpListener::bind(&bind)
            .await
            .expect("bind websocket echo upstream");
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                if let Ok(ws) = accept_async(stream).await {
                    let (mut write, mut read) = ws.split();
                    while let Some(Ok(message)) = read.next().await {
                        if write.send(message).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    })
}

pub async fn spawn_tcp_echo_upstream(port: u16) -> tokio::task::JoinHandle<()> {
    let bind = format!("127.0.0.1:{port}");
    tokio::spawn(async move {
        let listener = TcpListener::bind(&bind)
            .await
            .expect("bind tcp echo upstream");
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buffer = vec![0_u8; 16 * 1024];
                loop {
                    match stream.read(&mut buffer).await {
                        Ok(0) => break,
                        Ok(read) => {
                            if stream.write_all(&buffer[..read]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }
    })
}

pub async fn spawn_udp_echo_upstream(port: u16) -> Result<tokio::task::JoinHandle<()>> {
    let bind = format!("127.0.0.1:{port}");
    let probe = UdpSocket::bind(&bind).await?;
    drop(probe);
    Ok(tokio::spawn(async move {
        let socket = UdpSocket::bind(&bind)
            .await
            .expect("bind udp echo upstream");
        let mut buffer = vec![0_u8; 65_536];
        loop {
            let Ok((size, peer)) = socket.recv_from(&mut buffer).await else {
                break;
            };
            let _ = socket.send_to(&buffer[..size], peer).await;
        }
    }))
}

pub fn write_config(root: &Path, yaml: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(root)?;
    let config_path = root.join("proxysss.yaml");
    std::fs::write(&config_path, yaml)?;
    Ok(config_path)
}

pub async fn spawn_gateway(
    config_path: PathBuf,
) -> Result<(Arc<Gateway>, tokio::task::JoinHandle<()>)> {
    ensure_rustls_crypto_provider();
    let _guard = gateway_test_guard().await;
    let config = GatewayConfig::load(&config_path)?;
    let plain_bind = config.http.plain_bind.clone();
    let tls_bind = config.http.tls_bind.clone();
    let gateway = Gateway::from_config(config_path, config).await?;
    let runner = {
        let gateway = gateway.clone();
        tokio::spawn(async move {
            let _ = gateway.run().await;
        })
    };
    if !plain_bind.trim().is_empty() {
        wait_tcp_ready(&plain_bind).await?;
    } else if !tls_bind.trim().is_empty() {
        wait_tcp_ready(&tls_bind).await?;
    }
    if runner.is_finished() {
        anyhow::bail!("gateway task exited before it became usable");
    }
    Ok((gateway, runner))
}

pub fn cleanup(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}

pub fn temp_root(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
}

pub async fn tcp_roundtrip(addr: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .with_context(|| format!("tcp connect failed: {addr}"))?;
    stream.write_all(payload).await?;
    let mut buffer = vec![0_u8; payload.len()];
    stream.read_exact(&mut buffer).await?;
    Ok(buffer)
}

pub async fn udp_roundtrip(addr: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    socket.send_to(payload, addr).await?;
    let mut buffer = vec![0_u8; payload.len().max(512)];
    let (size, _) = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buffer))
        .await
        .context("udp recv timeout")??;
    Ok(buffer[..size].to_vec())
}
