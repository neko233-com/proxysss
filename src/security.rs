use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use http::header::{HeaderMap, TRANSFER_ENCODING};
use http::{HeaderValue, StatusCode, Uri};
use url::Url;

use crate::config::{
    AdminAuthRateLimitConfig, DdosProtectionConfig, DomainRouteConfig, DynamicBlacklistConfig,
    HttpAccessControlConfig, KubernetesConfig, ReverseProxyRouteConfig, SecurityConfig,
    StreamAccessControlConfig, TcpListenerConfig, UdpListenerConfig,
};

#[derive(Clone)]
pub struct AdminAuthGuard {
    failures: std::sync::Arc<DashMap<String, AdminAuthFailureState>>,
}

#[derive(Clone)]
struct AdminAuthFailureState {
    count: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

impl Default for AdminAuthGuard {
    fn default() -> Self {
        Self {
            failures: std::sync::Arc::new(DashMap::new()),
        }
    }
}

impl AdminAuthGuard {
    pub fn key_for(remote_addr: SocketAddr) -> String {
        remote_addr.ip().to_string()
    }

    pub fn is_locked(&self, key: &str, config: &AdminAuthRateLimitConfig) -> bool {
        if !config.enabled {
            return false;
        }
        let Some(entry) = self.failures.get(key) else {
            return false;
        };
        entry
            .locked_until
            .is_some_and(|until| until > Instant::now())
    }

    pub fn record_failure(&self, key: &str, config: &AdminAuthRateLimitConfig) {
        if !config.enabled {
            return;
        }
        let now = Instant::now();
        let window = Duration::from_secs(config.window_secs.max(1));
        let lockout = Duration::from_secs(config.lockout_secs.max(1));
        let mut entry = self
            .failures
            .entry(key.to_string())
            .or_insert(AdminAuthFailureState {
                count: 0,
                window_start: now,
                locked_until: None,
            });
        if now.duration_since(entry.window_start) >= window {
            entry.window_start = now;
            entry.count = 0;
            entry.locked_until = None;
        }
        entry.count = entry.count.saturating_add(1);
        if entry.count >= config.max_failures.max(1) {
            entry.locked_until = Some(now + lockout);
        }
    }

    pub fn clear_failures(&self, key: &str) {
        self.failures.remove(key);
    }
}

pub fn admin_loopback_only_allows(remote_addr: SocketAddr, bind: &str) -> bool {
    if !bind.contains("127.0.0.1") && !bind.contains("localhost") && !bind.contains("::1") {
        return true;
    }
    remote_addr.ip().is_loopback()
}

pub fn reject_ambiguous_http1_request(
    headers: &HeaderMap,
    config: &SecurityConfig,
) -> Option<StatusCode> {
    if !config.reject_ambiguous_http1 {
        return None;
    }

    let content_length = headers.get("content-length");
    let transfer_encoding = headers.get(TRANSFER_ENCODING);
    if content_length.is_some() && transfer_encoding.is_some() {
        return Some(StatusCode::BAD_REQUEST);
    }

    if let Some(value) = transfer_encoding.and_then(|item| item.to_str().ok()) {
        let normalized = value.to_ascii_lowercase();
        if normalized.contains(',') {
            return Some(StatusCode::BAD_REQUEST);
        }
    }

    None
}

pub fn validate_route_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("route name cannot be empty"));
    }
    if trimmed.len() > 64 {
        return Err(anyhow!("route name must be at most 64 characters"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(anyhow!(
            "route name must contain only ASCII letters, digits, '-' or '_'"
        ));
    }
    Ok(())
}

pub fn validate_upstream_target(upstream: &str, security: &SecurityConfig) -> Result<()> {
    let trimmed = upstream.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("upstream cannot be empty"));
    }

    let url = if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("ws://")
        || trimmed.starts_with("wss://")
    {
        Url::parse(trimmed).context("invalid upstream url")?
    } else {
        Url::parse(&format!("http://{trimmed}")).context("invalid upstream host")?
    };

    if let Some(host) = url.host_str() {
        if security.block_ssrf_targets {
            reject_ssrf_host(host, security)?;
        }
    }

    if url.username() != "" || url.password().is_some() {
        return Err(anyhow!("upstream url must not embed credentials"));
    }

    Ok(())
}

pub fn validate_domain_route_mutation(
    route: &DomainRouteConfig,
    security: &SecurityConfig,
) -> Result<()> {
    if !security.validate_admin_mutations {
        return Ok(());
    }
    validate_route_name(&route.name)?;
    validate_upstream_target(&route.upstream, security)?;
    for upstream in &route.upstreams {
        validate_upstream_target(upstream, security)?;
    }
    for domain in &route.domains {
        validate_domain_name(domain)?;
    }
    if !route.path_prefix.starts_with('/') {
        return Err(anyhow!("path_prefix must start with '/'"));
    }
    Ok(())
}

pub fn validate_reverse_proxy_route_mutation(
    route: &ReverseProxyRouteConfig,
    security: &SecurityConfig,
) -> Result<()> {
    if !security.validate_admin_mutations {
        return Ok(());
    }
    validate_route_name(&route.name)?;
    validate_upstream_target(&route.upstream, security)?;
    for upstream in &route.upstreams {
        validate_upstream_target(upstream, security)?;
    }
    for host in &route.hosts {
        if !host.is_empty() && !host.contains('*') {
            validate_domain_name(host)?;
        }
    }
    if !route.path_prefix.starts_with('/') {
        return Err(anyhow!("path_prefix must start with '/'"));
    }
    Ok(())
}

pub fn validate_tcp_listener_mutation(
    listener: &TcpListenerConfig,
    security: &SecurityConfig,
) -> Result<()> {
    if !security.validate_admin_mutations {
        return Ok(());
    }
    validate_route_name(&listener.name)?;
    if !listener.upstream.trim().is_empty() {
        validate_stream_upstream(&listener.upstream, security)?;
    }
    for upstream in &listener.upstreams {
        validate_stream_upstream(upstream, security)?;
    }
    Ok(())
}

pub fn validate_udp_listener_mutation(
    listener: &UdpListenerConfig,
    security: &SecurityConfig,
) -> Result<()> {
    if !security.validate_admin_mutations {
        return Ok(());
    }
    validate_route_name(&listener.name)?;
    if !listener.upstream.trim().is_empty() {
        validate_stream_upstream(&listener.upstream, security)?;
    }
    for upstream in &listener.upstreams {
        validate_stream_upstream(upstream, security)?;
    }
    Ok(())
}

fn validate_stream_upstream(upstream: &str, security: &SecurityConfig) -> Result<()> {
    let trimmed = upstream.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("stream upstream cannot be empty"));
    }
    if let Ok(socket) = trimmed.parse::<SocketAddr>() {
        if security.block_ssrf_targets {
            reject_ssrf_ip(socket.ip(), security)?;
        }
        return Ok(());
    }
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        if security.block_ssrf_targets {
            reject_ssrf_ip(ip, security)?;
        }
        return Ok(());
    }
    if security.block_ssrf_targets {
        reject_ssrf_host(trimmed, security)?;
    }
    Ok(())
}

pub fn validate_domains(domains: &[String]) -> Result<()> {
    if domains.is_empty() {
        return Err(anyhow!("domains cannot be empty"));
    }
    for domain in domains {
        validate_domain_name(domain)?;
    }
    Ok(())
}

fn validate_domain_name(domain: &str) -> Result<()> {
    let trimmed = domain.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Err(anyhow!("domain cannot be empty"));
    }
    if trimmed.len() > 253 {
        return Err(anyhow!("domain is too long"));
    }
    if trimmed.chars().any(|ch| ch.is_whitespace() || ch == '/') {
        return Err(anyhow!("domain contains invalid characters"));
    }
    Ok(())
}

fn reject_ssrf_host(host: &str, security: &SecurityConfig) -> Result<()> {
    let normalized = host.trim().trim_matches(['[', ']']).to_ascii_lowercase();
    if security
        .blocked_upstream_hosts
        .iter()
        .any(|item| item.eq_ignore_ascii_case(&normalized))
    {
        return Err(anyhow!(
            "upstream host {host} is blocked by security policy"
        ));
    }
    if let Ok(ip) = normalized.parse::<IpAddr>() {
        reject_ssrf_ip(ip, security)?;
    }
    Ok(())
}

fn reject_ssrf_ip(ip: IpAddr, security: &SecurityConfig) -> Result<()> {
    if !security.block_ssrf_targets {
        return Ok(());
    }
    if is_metadata_or_private_ip(ip) {
        return Err(anyhow!("upstream ip {ip} is blocked by SSRF policy"));
    }
    for cidr in &security.blocked_upstream_cidrs {
        if ip_matches_cidr(ip, cidr) {
            return Err(anyhow!("upstream ip {ip} matches blocked cidr {cidr}"));
        }
    }
    Ok(())
}

fn is_metadata_or_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets() == [169, 254, 169, 254]
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

fn ip_matches_cidr(ip: IpAddr, cidr: &str) -> bool {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let Ok(network) = network.parse::<IpAddr>() else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    match (ip, network) {
        (IpAddr::V4(ip), IpAddr::V4(network)) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            let ip_bits = u32::from_be_bytes(ip.octets());
            let network_bits = u32::from_be_bytes(network.octets());
            (ip_bits & mask) == (network_bits & mask)
        }
        (IpAddr::V6(ip), IpAddr::V6(network)) if prefix <= 128 => {
            let ip_bits = u128::from_be_bytes(ip.octets());
            let network_bits = u128::from_be_bytes(network.octets());
            let shift = 128 - prefix;
            let mask = if shift == 128 { 0 } else { u128::MAX << shift };
            (ip_bits & mask) == (network_bits & mask)
        }
        _ => false,
    }
}

#[derive(Clone)]
struct DdosWindowState {
    count: u32,
    window_start: Instant,
    banned_until: Option<Instant>,
}

#[derive(Clone, Default)]
pub struct DdosGuard {
    windows: std::sync::Arc<DashMap<String, DdosWindowState>>,
}

impl DdosGuard {
    pub fn check_and_record(&self, ip: IpAddr, config: &DdosProtectionConfig) -> Option<u64> {
        if !config.enabled {
            return None;
        }
        let key = ip.to_string();
        let now = Instant::now();
        let window = Duration::from_secs(config.window_secs.max(1));
        let ban = Duration::from_secs(config.ban_secs.max(1));
        let limit = config.max_connections.saturating_add(config.burst);

        let mut entry = self.windows.entry(key).or_insert(DdosWindowState {
            count: 0,
            window_start: now,
            banned_until: None,
        });

        if entry.banned_until.is_some_and(|until| until > now) {
            return Some(
                entry
                    .banned_until
                    .unwrap()
                    .duration_since(now)
                    .as_secs()
                    .max(1),
            );
        }

        if now.duration_since(entry.window_start) >= window {
            entry.window_start = now;
            entry.count = 0;
            entry.banned_until = None;
        }

        entry.count = entry.count.saturating_add(1);
        if entry.count > limit {
            entry.banned_until = Some(now + ban);
            return Some(ban.as_secs().max(1));
        }

        None
    }

    pub fn ban_ip(&self, ip: IpAddr, ban_secs: u64) {
        let now = Instant::now();
        let ban = Duration::from_secs(ban_secs.max(1));
        self.windows.insert(
            ip.to_string(),
            DdosWindowState {
                count: 0,
                window_start: now,
                banned_until: Some(now + ban),
            },
        );
    }

    pub fn unban_ip(&self, ip: IpAddr) {
        self.windows.remove(&ip.to_string());
    }
}

#[derive(Clone, Default)]
pub struct DynamicBlacklist {
    entries: std::sync::Arc<DashMap<String, Instant>>,
}

impl DynamicBlacklist {
    pub fn load_from_disk(config: &DynamicBlacklistConfig) -> Self {
        let blacklist = Self::default();
        if !config.enabled {
            return blacklist;
        }
        if let Ok(bytes) = std::fs::read(&config.path) {
            if let Ok(items) = serde_json::from_slice::<Vec<String>>(&bytes) {
                let ban = Duration::from_secs(3600);
                let until = Instant::now() + ban;
                for item in items {
                    blacklist.entries.insert(item, until);
                }
            }
        }
        blacklist
    }

    pub fn persist(&self, config: &DynamicBlacklistConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }
        if let Some(parent) = config.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let now = Instant::now();
        let active = self
            .entries
            .iter()
            .filter(|entry| entry.value() > &now)
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        std::fs::write(
            &config.path,
            serde_json::to_vec_pretty(&active).context("failed to serialize blacklist")?,
        )
        .with_context(|| format!("failed to write {}", config.path.display()))?;
        Ok(())
    }

    pub fn add(&self, ip: IpAddr, ban_secs: u64) {
        let until = Instant::now() + Duration::from_secs(ban_secs.max(1));
        self.entries.insert(ip.to_string(), until);
    }

    pub fn remove(&self, ip: IpAddr) {
        self.entries.remove(&ip.to_string());
    }

    pub fn is_blocked(&self, ip: IpAddr) -> bool {
        let Some(until) = self.entries.get(&ip.to_string()) else {
            return false;
        };
        if *until <= Instant::now() {
            drop(until);
            self.entries.remove(&ip.to_string());
            return false;
        }
        true
    }

    pub fn list_active(&self) -> Vec<String> {
        let now = Instant::now();
        self.entries
            .iter()
            .filter(|entry| *entry.value() > now)
            .map(|entry| entry.key().clone())
            .collect()
    }
}

pub fn ip_access_is_denied(config: &HttpAccessControlConfig, ip: IpAddr) -> Option<String> {
    if !config.enabled {
        return None;
    }
    access_list_denies(ip, &config.allow, &config.deny)
}

pub fn stream_access_is_denied(config: &StreamAccessControlConfig, ip: IpAddr) -> Option<String> {
    if !config.enabled {
        return None;
    }
    access_list_denies(ip, &config.allow, &config.deny)
}

fn access_list_denies(ip: IpAddr, allow: &[String], deny: &[String]) -> Option<String> {
    if !allow.is_empty() && !allow.iter().any(|entry| ip_matches_rule(ip, entry)) {
        return Some(ip.to_string());
    }
    if deny.iter().any(|entry| ip_matches_rule(ip, entry)) {
        return Some(ip.to_string());
    }
    None
}

pub fn ip_matches_rule(ip: IpAddr, rule: &str) -> bool {
    let Some((base, prefix)) = parse_ip_rule(rule) else {
        return false;
    };
    match (ip, base) {
        (IpAddr::V4(ip), IpAddr::V4(base)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix))
            };
            let ip_bits = u32::from_be_bytes(ip.octets());
            let base_bits = u32::from_be_bytes(base.octets());
            (ip_bits & mask) == (base_bits & mask)
        }
        (IpAddr::V6(ip), IpAddr::V6(base)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - u128::from(prefix))
            };
            let ip_bits = u128::from_be_bytes(ip.octets());
            let base_bits = u128::from_be_bytes(base.octets());
            (ip_bits & mask) == (base_bits & mask)
        }
        _ => false,
    }
}

fn parse_ip_rule(rule: &str) -> Option<(IpAddr, u8)> {
    let trimmed = rule.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((network, prefix)) = trimmed.split_once('/') {
        let base = network.parse::<IpAddr>().ok()?;
        let prefix = prefix.parse::<u8>().ok()?;
        return Some((base, prefix));
    }
    trimmed.parse::<IpAddr>().ok().map(|ip| (ip, 128))
}

pub fn apply_kubernetes_routes(
    config: &mut KubernetesConfig,
    domain_routes: &mut Vec<DomainRouteConfig>,
) {
    if !config.enabled {
        return;
    }

    let namespace = if config.namespace.trim().is_empty() {
        "default".to_string()
    } else {
        config.namespace.trim().to_string()
    };
    let cluster_domain = if config.cluster_domain.trim().is_empty() {
        "cluster.local".to_string()
    } else {
        config.cluster_domain.trim().trim_matches('.').to_string()
    };

    for mapping in &config.mappings {
        if mapping.service.trim().is_empty() {
            continue;
        }
        let port = mapping.port.max(1);
        let upstream = format!(
            "http://{}.{}.svc.{}:{}",
            mapping.service.trim(),
            namespace,
            cluster_domain,
            port
        );
        let route = DomainRouteConfig {
            name: mapping.name.clone(),
            domains: mapping.domains.clone(),
            path_prefix: if mapping.path_prefix.trim().is_empty() {
                "/".to_string()
            } else {
                mapping.path_prefix.clone()
            },
            upstream,
            upstreams: Vec::new(),
            upstream_weights: Default::default(),
            strip_prefix: mapping.strip_prefix,
            set_headers: Default::default(),
            strip_headers: Vec::new(),
            compression: Default::default(),
            cache: Default::default(),
            rate_limit: Default::default(),
            active_health: Default::default(),
            ssl: Default::default(),
        };

        if let Some(existing) = domain_routes
            .iter_mut()
            .find(|item| item.name == route.name)
        {
            *existing = route;
        } else {
            domain_routes.push(route);
        }
    }
}

pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|item| !item.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    let temp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|item| item.to_str())
            .unwrap_or("proxysss.yaml")
    ));
    std::fs::write(&temp_path, content)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    std::fs::rename(&temp_path, path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))?;
    Ok(())
}

pub fn sanitize_header_value(value: &str) -> Result<HeaderValue> {
    if value.chars().any(|ch| ch.is_control()) {
        return Err(anyhow!("header value contains control characters"));
    }
    HeaderValue::from_str(value).map_err(|error| anyhow!("invalid header value: {error}"))
}

pub fn request_uri_is_safe(uri: &Uri) -> bool {
    !uri.path().contains("//") && !uri.path().contains("\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_route_name_rejects_empty_and_symbols() {
        assert!(validate_route_name("api").is_ok());
        assert!(validate_route_name("api_v2").is_ok());
        assert!(validate_route_name("").is_err());
        assert!(validate_route_name("../x").is_err());
    }

    #[test]
    fn ssrf_blocks_metadata_ip_in_mutations() {
        let security = SecurityConfig {
            validate_admin_mutations: true,
            block_ssrf_targets: true,
            ..SecurityConfig::default()
        };
        assert!(
            validate_upstream_target("http://169.254.169.254/latest/meta-data", &security).is_err()
        );
        assert!(validate_upstream_target("http://127.0.0.1:8080", &security).is_err());
        assert!(validate_upstream_target("http://api.internal.example:8080", &security).is_ok());
    }

    #[test]
    fn admin_auth_guard_locks_after_failures() {
        let guard = AdminAuthGuard::default();
        let config = AdminAuthRateLimitConfig {
            enabled: true,
            max_failures: 2,
            window_secs: 60,
            lockout_secs: 30,
        };
        let key = "127.0.0.1".to_string();
        guard.record_failure(&key, &config);
        assert!(!guard.is_locked(&key, &config));
        guard.record_failure(&key, &config);
        assert!(guard.is_locked(&key, &config));
    }

    #[test]
    fn ambiguous_http1_request_is_rejected_when_enabled() {
        let mut headers = HeaderMap::new();
        headers.insert("content-length", HeaderValue::from_static("10"));
        headers.insert(TRANSFER_ENCODING, HeaderValue::from_static("chunked"));
        let security = SecurityConfig {
            reject_ambiguous_http1: true,
            ..SecurityConfig::default()
        };
        assert_eq!(
            reject_ambiguous_http1_request(&headers, &security),
            Some(StatusCode::BAD_REQUEST)
        );
    }

    #[test]
    fn ddos_guard_bans_after_threshold() {
        let guard = DdosGuard::default();
        let config = DdosProtectionConfig {
            enabled: true,
            max_connections: 2,
            window_secs: 60,
            ban_secs: 120,
            burst: 0,
        };
        let ip = "203.0.113.5".parse().expect("ip");
        assert!(guard.check_and_record(ip, &config).is_none());
        assert!(guard.check_and_record(ip, &config).is_none());
        assert!(guard.check_and_record(ip, &config).is_some());
        assert!(guard.check_and_record(ip, &config).is_some());
    }

    #[test]
    fn stream_access_control_honors_cidr_deny() {
        let config = StreamAccessControlConfig {
            enabled: true,
            allow: Vec::new(),
            deny: vec!["203.0.113.0/24".to_string()],
        };
        assert!(stream_access_is_denied(&config, "203.0.113.10".parse().expect("ip")).is_some());
        assert!(stream_access_is_denied(&config, "198.51.100.1".parse().expect("ip")).is_none());
    }

    #[test]
    fn kubernetes_mapping_expands_into_domain_routes() {
        let mut routes = Vec::new();
        let mut k8s = KubernetesConfig {
            enabled: true,
            namespace: "prod".to_string(),
            cluster_domain: "cluster.local".to_string(),
            mappings: vec![crate::config::KubernetesServiceMapping {
                name: "api".to_string(),
                service: "api-svc".to_string(),
                port: 8080,
                domains: vec!["api.example.com".to_string()],
                path_prefix: "/".to_string(),
                strip_prefix: false,
            }],
        };
        apply_kubernetes_routes(&mut k8s, &mut routes);
        assert_eq!(routes.len(), 1);
        assert!(routes[0]
            .upstream
            .contains("api-svc.prod.svc.cluster.local:8080"));
    }
}
