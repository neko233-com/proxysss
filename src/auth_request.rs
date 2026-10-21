//! 有界异步授权子请求。脚本只声明策略，网络 I/O 留在 Tokio 请求任务内。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use http::{HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use url::Url;

const MAX_IN_FLIGHT: usize = 128;
const MAX_HEADER_BYTES: usize = 16 * 1024;

/// 只接受完整 HTTP(S) URL；不从访客请求隐式复制参数、头或正文。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthRequestConfig {
    pub url: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_failure_status")]
    pub failure_status: u16,
    #[serde(default)]
    pub status_map: BTreeMap<u16, u16>,
}

fn default_timeout_ms() -> u64 {
    3_000
}

fn default_failure_status() -> u16 {
    503
}

impl AuthRequestConfig {
    pub fn validate(&self) -> Result<()> {
        let url = Url::parse(&self.url)
            .map_err(|_| anyhow::anyhow!("auth_request.url must be an absolute HTTP(S) URL"))?;
        anyhow::ensure!(
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
                && self.url.len() <= 4096,
            "auth_request.url must be a bounded HTTP(S) URL without credentials or fragment"
        );
        anyhow::ensure!(
            (1..=30_000).contains(&self.timeout_ms),
            "auth_request.timeout_ms must be in 1..=30000"
        );
        anyhow::ensure!(
            (400..=599).contains(&self.failure_status),
            "auth_request.failure_status must deny with HTTP 400..=599"
        );
        anyhow::ensure!(
            self.status_map.len() <= 300
                && self.status_map.iter().all(|(source, target)| {
                    (300..=599).contains(source) && (400..=599).contains(target)
                }),
            "auth_request.status_map may only map non-success statuses to HTTP 400..=599"
        );
        Ok(())
    }

    fn failure(&self) -> StatusCode {
        StatusCode::from_u16(self.failure_status)
            .ok()
            .filter(|status| status.is_client_error() || status.is_server_error())
            .unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
    }
}

pub enum AuthRequestOutcome {
    Allowed,
    Denied {
        status: StatusCode,
        authenticate: Option<HeaderValue>,
    },
}

/// 一个 gateway 生命周期共享限额，配置重载不能重置活动请求配额。
#[derive(Clone)]
pub struct AuthRequestExecutor {
    client: reqwest::Client,
    permits: Arc<Semaphore>,
}

impl AuthRequestExecutor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .use_rustls_tls()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .pool_max_idle_per_host(16)
                .build()?,
            permits: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
        })
    }

    pub async fn authorize(&self, policy: &AuthRequestConfig) -> AuthRequestOutcome {
        let deny = || AuthRequestOutcome::Denied {
            status: policy.failure(),
            authenticate: None,
        };
        if policy.validate().is_err() {
            return deny();
        }
        let Ok(_permit) = self.permits.try_acquire() else {
            return deny();
        };
        // 不创建后台任务；超时、原请求取消或响应返回均释放 permit 和网络 future。
        let response = tokio::time::timeout(
            Duration::from_millis(policy.timeout_ms),
            self.client.get(&policy.url).send(),
        )
        .await;
        let Ok(Ok(response)) = response else {
            return deny();
        };
        if response
            .headers()
            .iter()
            .map(|(name, value)| name.as_str().len() + value.as_bytes().len())
            .sum::<usize>()
            > MAX_HEADER_BYTES
        {
            return deny();
        }
        let status = response.status();
        if status.is_success() {
            return AuthRequestOutcome::Allowed;
        }
        let denied_status = policy
            .status_map
            .get(&status.as_u16())
            .and_then(|value| StatusCode::from_u16(*value).ok())
            .unwrap_or_else(|| {
                if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                    status
                } else {
                    policy.failure()
                }
            });
        AuthRequestOutcome::Denied {
            status: denied_status,
            authenticate: (denied_status == StatusCode::UNAUTHORIZED)
                .then(|| {
                    response
                        .headers()
                        .get(http::header::WWW_AUTHENTICATE)
                        .cloned()
                })
                .flatten(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn reply_once(status: u16, headers: &str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/check", listener.local_addr().unwrap());
        let reply = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Length: 0\r\nConnection: close\r\n{headers}\r\n"
        );
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let mut chunk = [0; 1024];
                let size = stream.read(&mut chunk).await.unwrap();
                assert!(size > 0 && request.len() < 16384);
                request.extend_from_slice(&chunk[..size]);
            }
            stream.write_all(reply.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        (url, task)
    }

    #[tokio::test]
    async fn auth_request_statuses_do_not_follow_redirects_and_only_allow_2xx() {
        let executor = AuthRequestExecutor::new().unwrap();
        for (source, expected) in [
            (200, 0),
            (204, 0),
            (401, 401),
            (403, 403),
            (404, 404),
            (302, 503),
            (500, 503),
        ] {
            let (url, task) = reply_once(
                source,
                "Location: http://127.0.0.1:1/never\r\nWWW-Authenticate: Basic realm=fixture\r\n",
            )
            .await;
            let policy = AuthRequestConfig {
                url,
                timeout_ms: 1000,
                failure_status: 503,
                status_map: BTreeMap::from([(404, 404)]),
            };
            match executor.authorize(&policy).await {
                AuthRequestOutcome::Allowed => assert_eq!(expected, 0),
                AuthRequestOutcome::Denied {
                    status,
                    authenticate,
                } => {
                    assert_eq!(status.as_u16(), expected);
                    assert_eq!(authenticate.is_some(), expected == 401);
                }
            }
            let request = task.await.unwrap();
            assert!(request.starts_with("GET /check HTTP/1.1\r\n"));
            let request = request.to_ascii_lowercase();
            for forbidden in [
                "authorization:",
                "cookie:",
                "x-forwarded-for:",
                "x-internal-token:",
            ] {
                assert!(!request.contains(forbidden));
            }
            assert_eq!(executor.permits.available_permits(), MAX_IN_FLIGHT);
        }
    }

    #[tokio::test]
    async fn auth_request_timeout_cancellation_capacity_and_header_limits_fail_closed() {
        let executor = AuthRequestExecutor::new().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut policy = AuthRequestConfig {
            url: format!("http://{}/check", listener.local_addr().unwrap()),
            timeout_ms: 20,
            failure_status: 503,
            status_map: BTreeMap::new(),
        };
        let delayed = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(10)).await;
        });
        assert!(matches!(
            executor.authorize(&policy).await,
            AuthRequestOutcome::Denied {
                status: StatusCode::SERVICE_UNAVAILABLE,
                ..
            }
        ));
        assert_eq!(executor.permits.available_permits(), MAX_IN_FLIGHT);
        delayed.abort();
        let held = executor
            .permits
            .clone()
            .acquire_many_owned(MAX_IN_FLIGHT as u32)
            .await
            .unwrap();
        assert!(matches!(
            executor.authorize(&policy).await,
            AuthRequestOutcome::Denied {
                status: StatusCode::SERVICE_UNAVAILABLE,
                ..
            }
        ));
        drop(held);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        policy.url = format!("http://{}/check", listener.local_addr().unwrap());
        policy.timeout_ms = 30000;
        assert!(
            tokio::time::timeout(Duration::from_millis(20), executor.authorize(&policy))
                .await
                .is_err()
        );
        assert_eq!(executor.permits.available_permits(), MAX_IN_FLIGHT);
        let (url, task) = reply_once(
            200,
            &format!("X-Large: {}\r\n", "x".repeat(MAX_HEADER_BYTES + 1)),
        )
        .await;
        policy.url = url;
        assert!(matches!(
            executor.authorize(&policy).await,
            AuthRequestOutcome::Denied {
                status: StatusCode::SERVICE_UNAVAILABLE,
                ..
            }
        ));
        task.await.unwrap();
    }

    #[test]
    fn auth_request_configuration_cannot_allow_failure_or_log_credentials() {
        let good: AuthRequestConfig = serde_json::from_value(serde_json::json!({
            "url": "http://127.0.0.1:12345/check",
            "status_map": {"404": 404}
        }))
        .unwrap();
        assert!(good.validate().is_ok());
        for value in [
            serde_json::json!({"url":"file:///etc/passwd"}),
            serde_json::json!({"url":"http://synthetic:password@localhost/check"}),
            serde_json::json!({"url":"http://localhost/check#fragment"}),
            serde_json::json!({"url":"http://localhost/check","timeout_ms":0}),
            serde_json::json!({"url":"http://localhost/check","failure_status":200}),
            serde_json::json!({"url":"http://localhost/check","status_map":{"404":200}}),
            serde_json::json!({"url":"http://localhost/check","status_map":{"200":403}}),
        ] {
            let policy: AuthRequestConfig = serde_json::from_value(value).unwrap();
            assert!(policy.validate().is_err());
        }
    }
}
