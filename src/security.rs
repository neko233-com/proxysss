use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use http::header::{HeaderMap, TRANSFER_ENCODING};
use http::{HeaderValue, StatusCode, Uri};
use url::Url;

use crate::config::{
    AdminAuthRateLimitConfig, DdosProtectionConfig, DomainRouteConfig, DynamicBlacklistConfig,
    HttpAccessControlConfig, KubernetesConfig, ReverseProxyRouteConfig, SecurityConfig,
    StreamAccessControlConfig, StreamRouteConfig, TcpListenerConfig, UdpListenerConfig,
};
use crate::stream_endpoint::StreamEndpoint;

#[cfg(test)]
#[path = "security_blacklist_tests.rs"]
mod blacklist_tests;

// Store elapsed-time TTLs rather than adding untrusted seconds to Instant:
// u64::MAX is valid input but cannot be represented as a future Instant.
#[derive(Clone, Copy)]
struct Expiry {
    started_at: Instant,
    valid_for: Duration,
}

impl Expiry {
    fn new(now: Instant, ttl_secs: u64) -> Self {
        Self {
            started_at: now,
            valid_for: Duration::from_secs(ttl_secs.max(1)),
        }
    }

    fn remaining_secs(self, now: Instant) -> u64 {
        let remaining = self
            .valid_for
            .saturating_sub(now.saturating_duration_since(self.started_at));
        remaining
            .as_secs()
            .saturating_add(u64::from(remaining.subsec_nanos() != 0))
    }

    fn active(self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) < self.valid_for
    }
}

#[derive(Clone)]
pub struct AdminAuthGuard {
    failures: std::sync::Arc<DashMap<String, AdminAuthFailureState>>,
}

#[derive(Clone)]
struct AdminAuthFailureState {
    count: u32,
    window_start: Instant,
    locked_until: Option<Expiry>,
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
            .is_some_and(|until| until.active(Instant::now()))
    }

    pub fn record_failure(&self, key: &str, config: &AdminAuthRateLimitConfig) {
        if !config.enabled {
            return;
        }
        let now = Instant::now();
        let window = Duration::from_secs(config.window_secs.max(1));
        let mut entry = self
            .failures
            .entry(key.to_string())
            .or_insert(AdminAuthFailureState {
                count: 0,
                window_start: now,
                locked_until: None,
            });
        // Another authentication attempt may have locked this key after the
        // caller checked it. Do not erase or extend that active lockout.
        if entry.locked_until.is_some_and(|until| until.active(now)) {
            return;
        }
        if now.duration_since(entry.window_start) >= window {
            entry.window_start = now;
            entry.count = 0;
            entry.locked_until = None;
        }
        entry.count = entry.count.saturating_add(1);
        if entry.count >= config.max_failures.max(1) {
            entry.locked_until = Some(Expiry::new(now, config.lockout_secs));
        }
    }

    pub fn clear_failures(&self, key: &str) {
        self.failures.remove(key);
    }

    pub(crate) fn prune_expired(&self, now: Instant, config: &AdminAuthRateLimitConfig) {
        self.failures.retain(|_, state| {
            state.locked_until.is_some_and(|until| until.active(now))
                || now.saturating_duration_since(state.window_start).as_secs()
                    < config.window_secs.max(1)
        });
    }

    #[cfg(test)]
    pub(crate) fn tracked_entries(&self) -> usize {
        self.failures.len()
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
    let endpoint = StreamEndpoint::parse(upstream)?;
    if security.block_ssrf_targets {
        match endpoint {
            StreamEndpoint::Socket(address) => reject_ssrf_ip(address.ip(), security)?,
            StreamEndpoint::Host { name, .. } => reject_ssrf_host(name, security)?,
        }
    }
    Ok(())
}

pub fn validate_stream_route_mutation(
    route: &StreamRouteConfig,
    security: &SecurityConfig,
) -> Result<()> {
    if !security.validate_admin_mutations {
        return Ok(());
    }
    validate_route_name(&route.name)?;
    validate_stream_upstream(&route.upstream, security)?;
    for upstream in &route.upstreams {
        validate_stream_upstream(upstream, security)?;
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
    let normalized = host
        .trim()
        .trim_matches(['[', ']'])
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if security
        .blocked_upstream_hosts
        .iter()
        .any(|item| item.trim_end_matches('.').eq_ignore_ascii_case(&normalized))
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
    banned_until: Option<Expiry>,
}

#[derive(Clone, Default)]
pub struct DdosGuard {
    windows: std::sync::Arc<DashMap<IpAddr, DdosWindowState>>,
}

impl DdosGuard {
    pub fn check_and_record(&self, ip: IpAddr, config: &DdosProtectionConfig) -> Option<u64> {
        if !config.enabled {
            return None;
        }
        let now = Instant::now();
        let window = Duration::from_secs(config.window_secs.max(1));
        let limit = config.max_connections.saturating_add(config.burst);

        let mut entry = self.windows.entry(ip).or_insert(DdosWindowState {
            count: 0,
            window_start: now,
            banned_until: None,
        });

        if let Some(until) = entry.banned_until.filter(|until| until.active(now)) {
            return Some(until.remaining_secs(now));
        }

        if now.duration_since(entry.window_start) >= window {
            entry.window_start = now;
            entry.count = 0;
            entry.banned_until = None;
        }

        entry.count = entry.count.saturating_add(1);
        if entry.count > limit {
            entry.banned_until = Some(Expiry::new(now, config.ban_secs));
            return Some(config.ban_secs.max(1));
        }

        None
    }

    fn ban_ip(&self, ip: IpAddr, until: Expiry) {
        let now = Instant::now();
        self.windows.insert(
            ip,
            DdosWindowState {
                count: 0,
                window_start: now,
                banned_until: Some(until),
            },
        );
    }

    pub fn unban_ip(&self, ip: IpAddr) {
        self.windows.remove(&ip);
    }

    pub(crate) fn prune_expired(&self, now: Instant, config: &DdosProtectionConfig) {
        self.windows.retain(|_, state| {
            state.banned_until.is_some_and(|until| until.active(now))
                || now.saturating_duration_since(state.window_start).as_secs()
                    < config.window_secs.max(1)
        });
    }

    #[cfg(test)]
    pub(crate) fn tracked_entries(&self) -> usize {
        self.windows.len()
    }
}

#[derive(Clone, Copy)]
struct BlacklistEntry {
    expiry: Expiry,
    created_at: Duration,
    ttl_secs: u64,
}

impl BlacklistEntry {
    fn new(ban_secs: u64) -> Self {
        Self {
            expiry: Expiry::new(Instant::now(), ban_secs),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
            ttl_secs: ban_secs.max(1),
        }
    }

    fn stored(self, ip: IpAddr) -> StoredBan {
        StoredBan {
            ip,
            created_at_unix_secs: self.created_at.as_secs(),
            created_at_nanos: self.created_at.subsec_nanos(),
            ttl_secs: self.ttl_secs,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BlacklistSnapshot {
    version: u32,
    entries: Vec<StoredBan>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBan {
    ip: IpAddr,
    created_at_unix_secs: u64,
    #[serde(default)]
    created_at_nanos: u32,
    ttl_secs: u64,
}

#[derive(Clone, Copy)]
pub enum BlacklistMutation {
    Add { ip: IpAddr, ban_secs: u64 },
    Remove { ip: IpAddr },
}

#[derive(Clone)]
pub struct DynamicBlacklist {
    entries: std::sync::Arc<DashMap<IpAddr, BlacklistEntry>>,
    writer: std::sync::Arc<std::sync::Mutex<()>>,
    worker_permit: std::sync::Arc<tokio::sync::Semaphore>,
}

impl Default for DynamicBlacklist {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            writer: Default::default(),
            worker_permit: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}

impl DynamicBlacklist {
    pub fn load_from_disk(config: &DynamicBlacklistConfig) -> Result<Self> {
        let blacklist = Self::default();
        if !config.enabled {
            return Ok(blacklist);
        }
        let bytes = match std::fs::read(&config.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::symlink_metadata(&config.path) {
                    Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {
                        return Ok(blacklist)
                    }
                    _ => {
                        return Err(error).with_context(|| {
                            format!(
                                "blacklist path exists but cannot be read: {}",
                                config.path.display()
                            )
                        })
                    }
                }
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read blacklist {}", config.path.display()))
            }
        };
        let snapshot: BlacklistSnapshot = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "invalid blacklist snapshot {}; expected version 1 with timestamps and TTLs",
                config.path.display()
            )
        })?;
        if snapshot.version != 1 {
            anyhow::bail!(
                "unsupported blacklist snapshot version {}",
                snapshot.version
            );
        }
        let wall_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before Unix epoch")?;
        let now = Instant::now();
        let mut seen = std::collections::HashSet::new();
        for stored in snapshot.entries {
            if stored.created_at_nanos >= 1_000_000_000
                || stored.ttl_secs == 0
                || !seen.insert(stored.ip)
            {
                anyhow::bail!("invalid or duplicate blacklist entry for {}", stored.ip);
            }
            let created_at = Duration::new(stored.created_at_unix_secs, stored.created_at_nanos);
            let remaining = Duration::from_secs(stored.ttl_secs)
                .saturating_sub(wall_now.saturating_sub(created_at));
            if remaining.is_zero() {
                continue;
            }
            blacklist.entries.insert(
                stored.ip,
                BlacklistEntry {
                    expiry: Expiry {
                        started_at: now,
                        valid_for: remaining,
                    },
                    created_at,
                    ttl_secs: stored.ttl_secs,
                },
            );
        }
        Ok(blacklist)
    }

    // At most one blocking writer per blacklist. The permit travels into the
    // worker so disconnecting a caller cannot admit a second unfinished writer.
    pub async fn apply_admin(
        &self,
        config: DynamicBlacklistConfig,
        ddos: DdosGuard,
        mutation: BlacklistMutation,
    ) -> Result<()> {
        let permit = self
            .worker_permit
            .clone()
            .acquire_owned()
            .await
            .context("blacklist writer closed")?;
        let blacklist = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            blacklist.apply_admin_sync(&config, &ddos, mutation)
        })
        .await
        .context("blacklist writer failed")?
    }

    fn apply_admin_sync(
        &self,
        config: &DynamicBlacklistConfig,
        ddos: &DdosGuard,
        mutation: BlacklistMutation,
    ) -> Result<()> {
        let _writer = self
            .writer
            .lock()
            .map_err(|_| anyhow!("blacklist writer lock poisoned"))?;
        let (ip, replacement) = match mutation {
            BlacklistMutation::Add { ip, ban_secs } => (ip, Some(BlacklistEntry::new(ban_secs))),
            BlacklistMutation::Remove { ip } => (ip, None),
        };
        let now = Instant::now();
        if config.enabled {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock is before Unix epoch")?;
            let mut entries = self
                .entries
                .iter()
                .filter(|entry| *entry.key() != ip && entry.value().expiry.active(now))
                .map(|entry| entry.value().stored(*entry.key()))
                .collect::<Vec<_>>();
            if let Some(entry) = replacement {
                entries.push(entry.stored(ip));
            }
            entries.sort_unstable_by_key(|entry| entry.ip);
            let bytes = serde_json::to_vec_pretty(&BlacklistSnapshot {
                version: 1,
                entries,
            })
            .context("failed to serialize blacklist")?;
            write_blacklist_snapshot(&config.path, &bytes)?;
        }
        // Rename is the commit boundary. Nothing observable in memory changes
        // before it succeeds. Automatic bans never acquire the writer lock.
        if let Some(entry) = replacement {
            self.entries.insert(ip, entry);
            ddos.ban_ip(ip, entry.expiry);
        } else {
            self.entries.remove(&ip);
            ddos.unban_ip(ip);
        }
        Ok(())
    }

    pub fn add(&self, ip: IpAddr, ban_secs: u64) {
        self.entries.insert(ip, BlacklistEntry::new(ban_secs));
    }

    pub fn is_blocked(&self, ip: IpAddr) -> bool {
        // A lookup never deletes: an old reader must not remove a replacement
        // ban after releasing its shard guard. The control loop retires stale
        // entries while holding the same shard guard as its expiry check.
        self.entries
            .get(&ip)
            .is_some_and(|entry| entry.expiry.active(Instant::now()))
    }

    pub fn list_active(&self) -> Vec<String> {
        let now = Instant::now();
        self.entries
            .iter()
            .filter(|entry| entry.value().expiry.active(now))
            .map(|entry| entry.key().to_string())
            .collect()
    }

    pub(crate) fn prune_expired(&self, now: Instant) {
        self.entries.retain(|_, entry| entry.expiry.active(now));
    }

    #[cfg(test)]
    pub(crate) fn tracked_entries(&self) -> usize {
        self.entries.len()
    }
}

fn write_blacklist_snapshot(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    let temp = parent.join(format!(".proxysss-blacklist-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .with_context(|| format!("failed to create {}", temp.display()))?;
    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = TempFile(temp.clone());
    let written = (|| -> Result<()> {
        file.write_all(bytes)
            .with_context(|| format!("failed to write {}", temp.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync {}", temp.display()))?;
        Ok(())
    })();
    drop(file);
    written?;
    std::fs::rename(&temp, path)
        .with_context(|| format!("failed to replace blacklist {}", path.display()))?;
    Ok(())
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
    let ip = match (ip, base) {
        // Accept IPv4 rules for dual-stack peers while preserving IPv6 CIDRs
        // such as ::/0 and ::ffff:192.0.2.0/120 for the same mapped peer.
        (IpAddr::V6(value), IpAddr::V4(_)) => value.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
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
        let bits = if base.is_ipv4() { 32 } else { 128 };
        return (prefix <= bits).then_some((base, prefix));
    }
    trimmed
        .parse::<IpAddr>()
        .ok()
        .map(|ip| (ip, if ip.is_ipv4() { 32 } else { 128 }))
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
            forward_headers: true,
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
    #[test]
    fn exact_ipv4_acl_and_invalid_prefixes_do_not_shift_out_of_range() {
        let mapped_peer = "::ffff:192.0.2.7".parse().unwrap();
        assert!(ip_matches_rule(mapped_peer, "::/0"));
        assert!(ip_matches_rule(mapped_peer, "::ffff:192.0.2.0/120"));
        assert!(!ip_matches_rule(mapped_peer, "::ffff:192.0.3.0/120"));
        let ip = "192.0.2.10".parse().unwrap();
        assert!(super::ip_matches_rule(ip, "192.0.2.10"));
        assert!(!super::ip_matches_rule(ip, "192.0.2.11"));
        assert!(super::ip_matches_rule(
            "::ffff:192.0.2.10".parse().unwrap(),
            "192.0.2.10"
        ));
        for rule in ["192.0.2.0/33", "192.0.2.0/128", "::/129"] {
            assert!(!super::ip_matches_rule(ip, rule));
        }
        assert!(super::ip_matches_rule(ip, "0.0.0.0/0"));
    }
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
    fn untrusted_maximum_ban_durations_do_not_overflow_or_clear_active_lockouts() {
        let ip = "203.0.113.42".parse().unwrap();
        let ddos = DdosGuard::default();
        let config = DdosProtectionConfig {
            enabled: true,
            max_connections: 1,
            burst: 0,
            window_secs: 1,
            ban_secs: u64::MAX,
        };
        assert!(ddos.check_and_record(ip, &config).is_none());
        assert_eq!(ddos.check_and_record(ip, &config), Some(u64::MAX));
        ddos.ban_ip(ip, Expiry::new(Instant::now(), u64::MAX));
        ddos.prune_expired(Instant::now() + Duration::from_secs(5), &config);
        assert!(ddos.check_and_record(ip, &config).is_some());

        let auth = AdminAuthGuard::default();
        let config = AdminAuthRateLimitConfig {
            enabled: true,
            max_failures: 1,
            window_secs: 1,
            lockout_secs: u64::MAX,
        };
        auth.record_failure("admin", &config);
        let locked_at = {
            let mut state = auth.failures.get_mut("admin").unwrap();
            state.window_start = Instant::now() - Duration::from_secs(5);
            state.locked_until.unwrap().started_at
        };
        auth.record_failure("admin", &config);
        assert_eq!(
            auth.failures
                .get("admin")
                .unwrap()
                .locked_until
                .unwrap()
                .started_at,
            locked_at
        );
        auth.prune_expired(Instant::now() + Duration::from_secs(5), &config);
        assert!(auth.is_locked("admin", &config));

        let blacklist = DynamicBlacklist::default();
        blacklist.add(ip, u64::MAX);
        blacklist.prune_expired(Instant::now() + Duration::from_secs(5));
        assert!(blacklist.is_blocked(ip));
    }

    #[test]
    fn security_retirement_keeps_live_windows_and_bans_without_future_visits() {
        let ddos = DdosGuard::default();
        let config = DdosProtectionConfig {
            enabled: true,
            max_connections: 2,
            burst: 0,
            window_secs: 10,
            ban_secs: 30,
        };
        let short = "203.0.113.10".parse().unwrap();
        let long = "2001:db8::10".parse().unwrap();
        assert!(ddos.check_and_record(short, &config).is_none());
        ddos.ban_ip(long, Expiry::new(Instant::now(), 30));
        ddos.prune_expired(Instant::now(), &config);
        assert!(ddos.check_and_record(short, &config).is_none());
        assert!(
            ddos.check_and_record(short, &config).is_some(),
            "sweeping must not reset a live rate window"
        );
        let observed = Instant::now();
        ddos.prune_expired(observed + Duration::from_secs(11), &config);
        assert_eq!(ddos.tracked_entries(), 2, "bans outlive the rate window");
        ddos.prune_expired(observed + Duration::from_secs(31), &config);
        assert_eq!(ddos.tracked_entries(), 0);

        let auth = AdminAuthGuard::default();
        let auth_config = AdminAuthRateLimitConfig {
            enabled: true,
            max_failures: 2,
            window_secs: 10,
            lockout_secs: 30,
        };
        auth.record_failure("single", &auth_config);
        auth.record_failure("locked", &auth_config);
        auth.record_failure("locked", &auth_config);
        auth.prune_expired(Instant::now() + Duration::from_secs(11), &auth_config);
        assert_eq!(auth.tracked_entries(), 1);
        assert!(auth.is_locked("locked", &auth_config));
        auth.prune_expired(Instant::now() + Duration::from_secs(31), &auth_config);
        assert_eq!(auth.tracked_entries(), 0);

        let blacklist = DynamicBlacklist::default();
        blacklist.add(short, 0);
        blacklist.add(long, 30);
        let started = blacklist.entries.get(&short).unwrap().expiry.started_at;
        blacklist.prune_expired(started + Duration::from_millis(999));
        assert_eq!(blacklist.tracked_entries(), 2);
        blacklist.prune_expired(started + Duration::from_secs(1));
        assert_eq!(blacklist.tracked_entries(), 1);
        assert!(blacklist.is_blocked(long));
        blacklist.prune_expired(Instant::now() + Duration::from_secs(31));
        assert!(blacklist.list_active().is_empty());
        assert_eq!(blacklist.tracked_entries(), 0);
    }

    #[test]
    fn concurrent_blacklist_retirement_never_removes_a_replacement_ban() {
        let blacklist = DynamicBlacklist::default();
        let ip = "2001:db8::9".parse().unwrap();
        let mut expired = BlacklistEntry::new(1);
        expired.expiry.started_at = Instant::now() - Duration::from_secs(2);
        blacklist.entries.insert(ip, expired);
        assert!(!blacklist.is_blocked(ip));
        let observed = Instant::now();
        blacklist.add(ip, u64::MAX);
        blacklist.prune_expired(observed);
        assert!(blacklist.is_blocked(ip));
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let blacklist = &blacklist;
                scope.spawn(move || {
                    for _ in 0..2000 {
                        blacklist.prune_expired(Instant::now());
                    }
                });
            }
            for _ in 0..2000 {
                blacklist.add(ip, u64::MAX);
                assert!(blacklist.is_blocked(ip));
            }
        });
        assert!(blacklist.is_blocked(ip));
        assert_eq!(blacklist.tracked_entries(), 1);
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
