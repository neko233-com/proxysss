//! Control-plane notifications. Webhook credentials remain in environment variables.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::future::join_all;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::config::{NotificationChannel, NotificationChannelKind, NotificationConfig};

#[derive(Clone, Copy)]
pub enum NotificationEvent {
    CertificateFailed,
    CertificateReloadFailed,
    CertificateIssued,
    CertificateLoaded,
    CloudSyncFailed,
    CloudSyncSucceeded,
}

impl NotificationEvent {
    fn details(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::CertificateFailed => ("证书签发失败", "red", "certificate_issue_failed"),
            Self::CertificateReloadFailed => {
                ("证书重载失败", "orange", "certificate_reload_failed")
            }
            Self::CertificateIssued => ("证书签发完成", "blue", "certificate_issued"),
            Self::CertificateLoaded => ("证书已加载", "green", "certificate_loaded"),
            Self::CloudSyncFailed => ("云证书同步未完成", "orange", "cloud_sync_failed"),
            Self::CloudSyncSucceeded => ("云证书同步成功", "green", "cloud_sync_succeeded"),
        }
    }
}

pub async fn dispatch(config: &NotificationConfig, event: NotificationEvent, message: &str) {
    dispatch_with_links(config, event, message, &[]).await;
}

pub async fn dispatch_with_links(
    config: &NotificationConfig,
    event: NotificationEvent,
    message: &str,
    links: &[String],
) {
    if config.channels.is_empty() {
        return;
    }
    let client = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            tracing::warn!("failed to initialize notification client");
            return;
        }
    };
    let attempts = config.channels.iter().map(|channel| async {
        for attempt in 0..3 {
            if send_channel(&client, channel, event, message, links)
                .await
                .is_ok()
            {
                tracing::info!(channel = %channel.name, "notification delivered");
                return;
            }
            if attempt < 2 {
                tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
            }
        }
        // Never log webhook URLs, response bodies, signatures, or environment values.
        tracing::warn!(channel = %channel.name, "notification delivery failed after retries");
    });
    join_all(attempts).await;
}

async fn send_channel(
    client: &reqwest::Client,
    channel: &NotificationChannel,
    event: NotificationEvent,
    message: &str,
    links: &[String],
) -> Result<()> {
    let url = std::env::var(&channel.webhook_env)
        .context("notification webhook environment variable is unavailable")?;
    let parsed = url::Url::parse(&url).context("invalid notification webhook URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("open.feishu.cn")
        || !parsed.path().starts_with("/open-apis/bot/v2/hook/")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
    {
        return Err(anyhow!("unsupported notification webhook destination"));
    }
    let secret = if channel.signing_secret_env.is_empty() {
        None
    } else {
        Some(
            std::env::var(&channel.signing_secret_env)
                .context("notification signing secret environment variable is unavailable")?,
        )
    };
    match channel.kind {
        NotificationChannelKind::Feishu => {
            send_feishu(client, &url, secret.as_deref(), event, message, links).await
        }
    }
}

fn feishu_payload(
    event: NotificationEvent,
    message: &str,
    links: &[String],
    secret: Option<&str>,
    timestamp: u64,
) -> Result<serde_json::Value> {
    let (title, color, event_key) = event.details();
    let time = time::OffsetDateTime::from_unix_timestamp(i64::try_from(timestamp)?)
        .context("invalid notification timestamp")?;
    let utc = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        time.year(),
        u8::from(time.month()),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    );
    let mut payload = serde_json::json!({
        "msg_type": "interactive",
        "card": {
            "config": {"wide_screen_mode": true},
            "header": {"template": color, "title": {"tag": "plain_text", "content": format!("proxysss · {title}")}},
            "elements": [
                {"tag": "div", "text": {"tag": "plain_text", "content": message}},
                {"tag": "hr"},
                {"tag": "div", "fields": [
                    {"is_short": true, "text": {"tag": "lark_md", "content": format!("**事件**\n{title}")}},
                    {"is_short": true, "text": {"tag": "lark_md", "content": format!("**时间**\n{utc}")}}
                ]},
                {"tag": "note", "elements": [{"tag": "plain_text", "content": format!("搜索：proxysss SSL {event_key}")}]}
            ]
        }
    });
    if !links.is_empty() {
        let mut rendered = Vec::with_capacity(links.len());
        for link in links {
            let parsed = url::Url::parse(link).context("invalid certificate verification URL")?;
            if parsed.scheme() != "https"
                || !matches!(parsed.host(), Some(url::Host::Domain(_)))
                || parsed.as_str() != link
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed.port().is_some()
                || parsed.path() != "/"
                || parsed.query().is_some()
                || parsed.fragment().is_some()
            {
                return Err(anyhow!("invalid certificate verification URL"));
            }
            rendered.push(format!("[{link}]({link})"));
        }
        payload["card"]["elements"].as_array_mut().expect("card elements are an array").insert(3, serde_json::json!({"tag": "div", "text": {"tag": "lark_md", "content": format!("**HTTPS 核验**\n{}", rendered.join("\n"))}}));
    }
    if let Some(secret) = secret {
        let key = format!("{timestamp}\n{secret}");
        let mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
            .map_err(|_| anyhow!("invalid notification signing key"))?;
        payload["timestamp"] = timestamp.to_string().into();
        payload["sign"] = STANDARD.encode(mac.finalize().into_bytes()).into();
    }
    Ok(payload)
}

async fn send_feishu(
    client: &reqwest::Client,
    url: &str,
    secret: Option<&str>,
    event: NotificationEvent,
    message: &str,
    links: &[String],
) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before epoch")?
        .as_secs();
    let payload = feishu_payload(event, message, links, secret, timestamp)?;
    let mut response = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(reqwest::Error::without_url)?;
    if !response.status().is_success() {
        return Err(anyhow!("notification HTTP error"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(reqwest::Error::without_url)?
    {
        if body.len().saturating_add(chunk.len()) > 16 * 1024 {
            return Err(anyhow!("notification response too large"));
        }
        body.extend_from_slice(&chunk);
    }
    let response: serde_json::Value =
        serde_json::from_slice(&body).context("invalid notification response")?;
    if response.get("code").and_then(|value| value.as_i64()) != Some(0) {
        return Err(anyhow!("notification service rejected message"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feishu_payload_uses_optional_signature() -> Result<()> {
        let plain = feishu_payload(
            NotificationEvent::CertificateLoaded,
            "renewed",
            &[],
            None,
            1_600_000_000,
        )?;
        assert_eq!(plain["msg_type"], "interactive");
        assert_eq!(plain["card"]["elements"][0]["text"]["content"], "renewed");
        assert_eq!(plain["card"]["header"]["template"], "green");
        assert!(plain["card"]["elements"][3]["elements"][0]["content"]
            .as_str()
            .unwrap()
            .contains("certificate_loaded"));
        assert!(plain.get("sign").is_none());
        let signed = feishu_payload(
            NotificationEvent::CloudSyncFailed,
            "renewed",
            &[],
            Some("test-secret"),
            1_600_000_000,
        )?;
        assert_eq!(signed["card"]["header"]["template"], "orange");
        assert_eq!(signed["timestamp"], "1600000000");
        assert_eq!(
            signed["sign"],
            "BwqgZGo8lGLJAUAc/kiJR0TDtnLVnsITtszoHeRuXV0="
        );
        let card = feishu_payload(
            NotificationEvent::CloudSyncSucceeded,
            "证书 ID：cert-example\n证书域名：*.example.com",
            &["https://cdn.example.com/".into()],
            None,
            1_600_000_000,
        )?;
        assert!(card["card"]["elements"][3]["text"]["content"]
            .as_str()
            .unwrap()
            .contains("[https://cdn.example.com/](https://cdn.example.com/)"));
        assert!(feishu_payload(
            NotificationEvent::CloudSyncSucceeded,
            "test",
            &["http://example.com/".into()],
            None,
            1
        )
        .is_err());
        Ok(())
    }

    #[tokio::test]
    async fn feishu_delivery_checks_service_response_and_keeps_channels_independent() -> Result<()>
    {
        use http_body_util::{BodyExt, Full};
        use hyper::{service::service_fn, Request, Response};
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let service =
                        service_fn(|request: Request<hyper::body::Incoming>| async move {
                            let good = request.uri().path() == "/ok";
                            let bytes = request.into_body().collect().await.unwrap().to_bytes();
                            let payload: serde_json::Value =
                                serde_json::from_slice(&bytes).unwrap();
                            assert_eq!(
                                payload["card"]["elements"][0]["text"]["content"],
                                "certificate loaded"
                            );
                            let result = if good {
                                r#"{"code":0}"#
                            } else {
                                r#"{"code":19021}"#
                            };
                            Ok::<_, std::convert::Infallible>(Response::new(Full::new(
                                bytes::Bytes::from_static(result.as_bytes()),
                            )))
                        });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        let client = reqwest::Client::new();
        let good_url = format!("http://{address}/ok");
        let failed_url = format!("http://{address}/failed");
        let (first, second) = tokio::join!(
            send_feishu(
                &client,
                &good_url,
                None,
                NotificationEvent::CertificateLoaded,
                "certificate loaded",
                &[]
            ),
            send_feishu(
                &client,
                &failed_url,
                None,
                NotificationEvent::CertificateLoaded,
                "certificate loaded",
                &[]
            ),
        );
        server.abort();
        assert!(first.is_ok());
        assert!(second.is_err());
        Ok(())
    }
}
