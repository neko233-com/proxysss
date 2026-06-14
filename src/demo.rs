use std::collections::BTreeMap;
use std::convert::Infallible;

use anyhow::{Context, Result};
use bytes::Bytes;
use clap::{Args, Subcommand};
use futures::{SinkExt, StreamExt};
use http::{Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[allow(clippy::enum_variant_names)]
#[derive(Subcommand, Debug, Clone)]
pub enum DemoCommand {
    HttpEcho(HttpEchoArgs),
    WsEcho(WsEchoArgs),
    TcpEcho(TcpEchoArgs),
    UdpEcho(UdpEchoArgs),
}

#[derive(Args, Debug, Clone)]
pub struct HttpEchoArgs {
    #[arg(long, default_value = "127.0.0.1:8081")]
    pub listen: String,
}

#[derive(Args, Debug, Clone)]
pub struct WsEchoArgs {
    #[arg(long, default_value = "127.0.0.1:8082")]
    pub listen: String,
}

#[derive(Args, Debug, Clone)]
pub struct TcpEchoArgs {
    #[arg(long, default_value = "127.0.0.1:6379")]
    pub listen: String,
}

#[derive(Args, Debug, Clone)]
pub struct UdpEchoArgs {
    #[arg(long, default_value = "127.0.0.1:5353")]
    pub listen: String,
}

pub async fn run(command: DemoCommand) -> Result<()> {
    match command {
        DemoCommand::HttpEcho(args) => run_http_echo(args).await,
        DemoCommand::WsEcho(args) => run_ws_echo(args).await,
        DemoCommand::TcpEcho(args) => run_tcp_echo(args).await,
        DemoCommand::UdpEcho(args) => run_udp_echo(args).await,
    }
}

async fn run_http_echo(args: HttpEchoArgs) -> Result<()> {
    let listener = TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind http echo listener {}", args.listen))?;

    tracing::info!(bind = %args.listen, "http echo demo ready");

    loop {
        let (stream, remote_addr) = listener.accept().await.context("http echo accept failed")?;

        tokio::spawn(async move {
            let service = service_fn(move |mut request: http::Request<Incoming>| async move {
                let body = match request.body_mut().collect().await {
                    Ok(collected) => collected.to_bytes(),
                    Err(_) => Bytes::new(),
                };
                let headers = request
                    .headers()
                    .iter()
                    .filter_map(|(name, value)| {
                        value
                            .to_str()
                            .ok()
                            .map(|value| (name.as_str().to_string(), value.to_string()))
                    })
                    .collect::<BTreeMap<_, _>>();
                let payload = serde_json::json!({
                    "ok": true,
                    "remote_addr": remote_addr.to_string(),
                    "method": request.method().as_str(),
                    "path": request.uri().path(),
                    "query": request.uri().query(),
                    "headers": headers,
                    "body_len": body.len(),
                });

                let response = Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(payload.to_string())))
                    .unwrap_or_else(|_| {
                        Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from_static(b"build error")))
                            .expect("static response build should never fail")
                    });
                Ok::<_, Infallible>(response)
            });

            if let Err(error) = AutoBuilder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(stream), service)
                .await
            {
                tracing::warn!(?error, %remote_addr, "http echo connection failed");
            }
        });
    }
}

async fn run_ws_echo(args: WsEchoArgs) -> Result<()> {
    let listener = TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind websocket echo listener {}", args.listen))?;

    tracing::info!(bind = %args.listen, "websocket echo demo ready");

    loop {
        let (stream, remote_addr) = listener.accept().await.context("websocket accept failed")?;
        tokio::spawn(async move {
            let Ok(mut websocket) = accept_async(stream).await else {
                tracing::warn!(%remote_addr, "websocket handshake failed");
                return;
            };
            while let Some(message) = websocket.next().await {
                match message {
                    Ok(Message::Binary(bytes)) => {
                        if websocket.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Text(text)) => {
                        if websocket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Ping(bytes)) => {
                        if websocket.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        let _ = websocket.send(Message::Close(frame)).await;
                        break;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, "websocket echo read failed");
                        break;
                    }
                }
            }
        });
    }
}

async fn run_tcp_echo(args: TcpEchoArgs) -> Result<()> {
    let listener = TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind tcp echo listener {}", args.listen))?;

    tracing::info!(bind = %args.listen, "tcp echo demo ready");

    loop {
        let (mut stream, remote_addr) =
            listener.accept().await.context("tcp echo accept failed")?;
        tokio::spawn(async move {
            let mut buffer = vec![0_u8; 16 * 1024];
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(read) => {
                        if let Err(error) = stream.write_all(&buffer[..read]).await {
                            tracing::warn!(?error, %remote_addr, "tcp echo write failed");
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(?error, %remote_addr, "tcp echo read failed");
                        break;
                    }
                }
            }
        });
    }
}

async fn run_udp_echo(args: UdpEchoArgs) -> Result<()> {
    let socket = UdpSocket::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind udp echo listener {}", args.listen))?;

    tracing::info!(bind = %args.listen, "udp echo demo ready");

    let mut buffer = vec![0_u8; 65_536];
    loop {
        let (size, peer_addr) = socket
            .recv_from(&mut buffer)
            .await
            .context("udp echo recv failed")?;
        socket
            .send_to(&buffer[..size], peer_addr)
            .await
            .with_context(|| format!("failed to echo udp payload to {}", peer_addr))?;
    }
}
