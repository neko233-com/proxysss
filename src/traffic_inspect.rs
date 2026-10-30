//! On-demand, local control-plane traffic sampling; no data-plane instrumentation.
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::GatewayConfig;

#[derive(Debug, Deserialize)]
struct Process {
    pid: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Snapshot {
    uptime_seconds: u64,
    http_requests: u64,
    domain_requests: BTreeMap<String, u64>,
    websocket_sessions_active: u64,
    process: Process,
}

#[derive(Debug, Serialize)]
struct DomainRate {
    host: String,
    requests: u64,
    requests_per_second: f64,
}

#[derive(Debug, Serialize)]
struct Report {
    interval_seconds: f64,
    http_requests: u64,
    http_requests_per_second: f64,
    websocket_sessions_active: u64,
    domains: Vec<DomainRate>,
    note: &'static str,
}

fn compare(before: &Snapshot, after: &Snapshot, elapsed: Duration) -> Result<Report> {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0
        || before.process.pid.is_none()
        || before.process.pid != after.process.pid
        || after.uptime_seconds < before.uptime_seconds
        || after.http_requests < before.http_requests
    {
        anyhow::bail!("gateway restarted or counters reset during sampling; retry");
    }
    let mut domains = Vec::new();
    for (host, count) in &after.domain_requests {
        if let Some(previous) = before.domain_requests.get(host) {
            let Some(delta) = count.checked_sub(*previous) else {
                anyhow::bail!("domain counters reset during sampling; retry");
            };
            domains.push(DomainRate {
                host: host.clone(),
                requests: delta,
                requests_per_second: delta as f64 / seconds,
            });
        }
    }
    domains.sort_by(|a, b| {
        b.requests
            .cmp(&a.requests)
            .then_with(|| a.host.cmp(&b.host))
    });
    let requests = after.http_requests - before.http_requests;
    Ok(Report { interval_seconds: seconds, http_requests: requests,
        http_requests_per_second: requests as f64 / seconds,
        websocket_sessions_active: after.websocket_sessions_active, domains,
        note: "Only requests reaching this gateway; CDN edge traffic and cache hit rate are not included. Domains added or removed during sampling are omitted." })
}

async fn read_json(mut response: reqwest::Response) -> Result<serde_json::Value> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len().saturating_add(chunk.len()) > 1024 * 1024 {
            anyhow::bail!("local statistics response exceeds 1 MiB");
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).context("invalid local response")
}

async fn snapshot(client: &reqwest::Client, url: &str, token: &str) -> Result<Snapshot> {
    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context("local admin request failed")?;
    let response = response
        .error_for_status()
        .context("local admin rejected statistics request")?;
    let value = read_json(response).await?;
    serde_json::from_value(value.get("stats").cloned().unwrap_or(value))
        .context("invalid local statistics fields")
}

pub async fn run(config: &GatewayConfig, interval: u64) -> Result<()> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10));
    let addr = config.admin.bind.parse::<SocketAddr>().ok();
    let (client, url, token) = if config.admin.enabled
        && addr.is_some_and(|a| a.ip().is_loopback())
        && !config.admin.bearer_token.is_empty()
    {
        (
            builder.build()?,
            format!("http://{}/v1/stats", addr.unwrap()),
            config.admin.bearer_token.clone(),
        )
    } else {
        let monitor = &config.admin.monitor;
        let host =
            monitor.hosts.iter().find(|h| !h.contains('*')).context(
                "traffic requires an exact monitor host when loopback admin is disabled",
            )?;
        let addr: SocketAddr = config
            .http
            .tls_bind
            .parse()
            .context("traffic requires a numeric HTTPS listener")?;
        if !monitor.enabled
            || monitor.password.is_empty()
            || !(addr.ip().is_loopback() || addr.ip().is_unspecified())
        {
            anyhow::bail!("traffic requires an enabled local HTTPS monitor listener");
        }
        if interval.saturating_add(20) >= monitor.session_ttl_secs {
            anyhow::bail!(
                "sampling interval is too long for the configured monitor session lifetime"
            );
        }
        let loopback = if addr.is_ipv4() {
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
        } else {
            std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
        };
        let host_url = reqwest::Url::parse(&format!("https://{host}:{}/", addr.port()))
            .context("invalid monitor host")?;
        if host_url.host_str() != Some(host.as_str())
            || host_url.path() != "/"
            || host_url.query().is_some()
            || host_url.fragment().is_some()
            || !host_url.username().is_empty()
        {
            anyhow::bail!("invalid exact monitor host");
        }
        builder = builder.resolve(host, SocketAddr::new(loopback, addr.port()));
        let client = builder.build()?;
        let base = format!(
            "https://{host}:{}{}",
            addr.port(),
            monitor.path_prefix.trim_end_matches('/')
        );
        let response = client
            .post(format!("{base}/v1/login"))
            .json(&serde_json::json!({"password": monitor.password}))
            .send()
            .await?
            .error_for_status()
            .context("local monitor login failed")?;
        let login = read_json(response).await?;
        let token = login
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .context("local monitor did not return a session token")?
            .to_owned();
        (client, format!("{base}/v1/stats"), token)
    };
    let before = snapshot(&client, &url, &token).await?;
    let started = Instant::now();
    tokio::time::sleep(Duration::from_secs(interval)).await;
    let after = snapshot(&client, &url, &token).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&compare(&before, &after, started.elapsed())?)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(pid: u32, uptime: u64, total: u64, domains: &[(&str, u64)]) -> Snapshot {
        Snapshot {
            process: Process { pid: Some(pid) },
            uptime_seconds: uptime,
            http_requests: total,
            domain_requests: domains.iter().map(|(h, c)| (h.to_string(), *c)).collect(),
            websocket_sessions_active: 3,
        }
    }
    #[test]
    fn rates_sort_and_ignore_new_domains() {
        let a = sample(1, 10, 100, &[("a", 10), ("b", 5)]);
        let b = sample(1, 12, 110, &[("a", 12), ("b", 11), ("new", 1)]);
        let report = compare(&a, &b, Duration::from_secs(2)).unwrap();
        assert_eq!(report.http_requests_per_second, 5.0);
        assert_eq!(report.domains.len(), 2);
        assert_eq!(report.domains[0].host, "b");
        assert_eq!(report.domains[0].requests_per_second, 3.0);
    }
    #[test]
    fn reset_and_restart_are_not_reported_as_valid_rates() {
        let a = sample(1, 10, 100, &[("a", 10)]);
        assert!(compare(
            &a,
            &sample(2, 11, 101, &[("a", 11)]),
            Duration::from_secs(1)
        )
        .is_err());
        assert!(compare(&a, &sample(1, 11, 101, &[("a", 1)]), Duration::from_secs(1)).is_err());
    }

    #[tokio::test]
    async fn samples_the_real_authenticated_admin_api() -> Result<()> {
        use crate::verify::harness::{
            cleanup, reserve_port, spawn_gateway, temp_root, wait_http_ok, write_config,
        };
        let root = temp_root("proxysss-traffic-inspect");
        let gateway_port = reserve_port().await?;
        let admin_port = reserve_port().await?;
        let yaml = format!(
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
runtime:
  hot_reload:
    enabled: false
load_balance:
  active_health:
    enabled: false
monitoring:
  domain_stats_hosts: [example.test]
admin:
  enabled: true
  bind: 127.0.0.1:{admin_port}
  bearer_token: traffic-test-token
"#
        );
        let config_path = write_config(&root, &yaml)?;
        let (_gateway, _runner) = spawn_gateway(config_path).await?;
        wait_http_ok(&format!("http://127.0.0.1:{admin_port}/healthz")).await?;
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{admin_port}/v1/stats");
        assert!(snapshot(&client, &url, "wrong").await.is_err());
        let before = snapshot(&client, &url, "traffic-test-token").await?;
        client
            .get(format!("http://127.0.0.1:{gateway_port}/"))
            .header("host", "example.test")
            .send()
            .await?
            .error_for_status()?;
        let after = snapshot(&client, &url, "traffic-test-token").await?;
        let report = compare(&before, &after, Duration::from_secs(1))?;
        assert_eq!(
            report
                .domains
                .iter()
                .find(|d| d.host == "example.test")
                .unwrap()
                .requests,
            1
        );
        cleanup(&root);
        Ok(())
    }
}
