use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_HTTP_PORT: u16 = 80;
pub const DEFAULT_HTTPS_PORT: u16 = 443;
pub const DEFAULT_CONFIG_FILE_NAME: &str = "proxysss.yaml";
pub const DEFAULT_SCRIPT_FILE_NAME: &str = "gateway.ts";
pub const DEFAULT_ADMIN_USERNAME: &str = "root";
pub const DEFAULT_ADMIN_PASSWORD: &str = "root";
pub const DEFAULT_ADMIN_BEARER_TOKEN: &str = "neko233";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default, skip_serializing_if = "IncludeConfig::is_disabled")]
    pub include: IncludeConfig,
    #[serde(default = "default_log_filter")]
    pub log_filter: String,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub tcp: TcpConfig,
    #[serde(default)]
    pub udp: UdpConfig,
    #[serde(default)]
    pub script: ScriptConfig,
    #[serde(default)]
    pub plugins: PluginsConfig,
    #[serde(default)]
    pub load_balance: LoadBalanceConfig,
    #[serde(default)]
    pub affinity: AffinityConfig,
    #[serde(default)]
    pub admin: AdminConfig,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub kubernetes: KubernetesConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub services: ServicesConfig,
    #[serde(skip)]
    pub root_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncludeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

impl IncludeConfig {
    fn is_disabled(&self) -> bool {
        !self.enabled && !self.required && self.files.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub format: LogFormat,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default)]
    pub filter: String,
    #[serde(default = "default_true")]
    pub access_log: bool,
    #[serde(default = "default_sample_rate")]
    pub access_sample_rate: f64,
    #[serde(default = "default_slow_request_ms")]
    pub slow_request_ms: u64,
    #[serde(default = "default_redact_headers")]
    pub redact_headers: Vec<String>,
    #[serde(default = "default_access_log_path")]
    pub access_log_path: PathBuf,
    #[serde(default = "default_error_log_path")]
    pub error_log_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Plain,
    #[default]
    Json,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default)]
    pub plain_bind: String,
    #[serde(default = "default_tls_gateway_bind")]
    pub tls_bind: String,
    #[serde(default = "default_tls_gateway_bind")]
    pub h3_bind: String,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_true")]
    pub allow_insecure_upstreams: bool,
    #[serde(default)]
    pub error_pages: HttpErrorPagesConfig,
    #[serde(default)]
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpErrorPagesConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub show_details: bool,
    #[serde(default)]
    pub pages: Vec<HttpErrorPageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpErrorPageConfig {
    pub status: u16,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub file_path: PathBuf,
    #[serde(default = "default_error_page_content_type")]
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDemandTlsConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Optional glob patterns (e.g. `*.example.com`) that may receive first-hit certificates.
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default = "default_on_demand_max_certs")]
    pub max_active_certs: usize,
    #[serde(default = "default_on_demand_rate_per_hour")]
    pub max_issues_per_hour: u32,
    #[serde(default)]
    pub ask_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub mode: TlsMode,
    #[serde(default)]
    pub auto_https: AutoHttpsConfig,
    #[serde(default)]
    pub on_demand: OnDemandTlsConfig,
    #[serde(default)]
    pub certificates: Vec<TlsCertificateConfig>,
    #[serde(default = "default_cert_path")]
    pub cert_path: PathBuf,
    #[serde(default = "default_key_path")]
    pub key_path: PathBuf,
    #[serde(default = "default_true")]
    pub generate_self_signed_if_missing: bool,
    #[serde(default = "default_server_name")]
    pub server_name: String,
    #[serde(default)]
    pub acme: AcmeExternalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoHttpsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub production: bool,
    #[serde(default = "default_acme_client")]
    pub client: String,
    #[serde(default)]
    pub challenge: AcmeChallengeType,
    #[serde(default = "default_acme_renew_hours")]
    pub renew_interval_hours: u64,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    #[default]
    SelfSigned,
    Manual,
    AcmeManaged,
    AcmeExternal,
    AcmeDnsExternal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeExternalConfig {
    #[serde(default = "default_acme_client")]
    pub client: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_acme_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default)]
    pub challenge: AcmeChallengeType,
    #[serde(default)]
    pub directory_production: bool,
    #[serde(default = "default_acme_renew_hours")]
    pub renew_interval_hours: u64,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub dns: AcmeDnsExternalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcmeDnsExternalConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub credentials: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCertificateConfig {
    #[serde(default)]
    pub domains: Vec<String>,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcmeChallengeType {
    #[default]
    TlsAlpn01,
    Http01,
    Dns01,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TcpConfig {
    #[serde(default = "default_tcp_listeners")]
    pub listeners: Vec<TcpListenerConfig>,
    #[serde(default)]
    pub stream_routes: Vec<StreamRouteConfig>,
}

/// Domain-aware TCP/TLS stream proxy routes (Redis, MySQL, PostgreSQL, MongoDB, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRouteConfig {
    pub name: String,
    #[serde(default)]
    pub domains: Vec<String>,
    pub listen: String,
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_weights: BTreeMap<String, u32>,
    /// Observability hint: redis, mysql, postgres, mongodb, etc.
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub tls_mode: StreamTlsMode,
    #[serde(default)]
    pub access_control: StreamAccessControlConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamTlsMode {
    /// Peek TLS ClientHello for SNI when present; otherwise use the route upstream.
    #[default]
    Auto,
    /// Always relay TLS without termination (ssl_preread style routing).
    Passthrough,
    /// Terminate TLS on the gateway edge (uses http.tls material for the matched domain).
    Terminate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamAccessControlConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, alias = "allowlist", alias = "whitelist")]
    pub allow: Vec<String>,
    #[serde(default, alias = "denylist", alias = "blacklist", alias = "blocklist")]
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpListenerConfig {
    pub name: String,
    pub bind: String,
    #[serde(default)]
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_weights: BTreeMap<String, u32>,
    /// Observability hint: game_tcp, mqtt, custom-binary, etc.
    #[serde(default)]
    pub protocol: String,
    /// Disable Nagle for latency-sensitive streams such as games and AI tool bridges.
    #[serde(default = "default_true")]
    pub nodelay: bool,
    #[serde(default = "default_stream_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdpConfig {
    #[serde(default = "default_udp_listeners")]
    pub listeners: Vec<UdpListenerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpListenerConfig {
    pub name: String,
    pub bind: String,
    #[serde(default)]
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_weights: BTreeMap<String, u32>,
    /// Observability hint: kcp, qcp, quic-game, voip, custom-datagram, etc.
    #[serde(default)]
    pub protocol: String,
    /// Idle UDP association TTL. KCP and game traffic should keep this above client heartbeat interval.
    #[serde(default = "default_udp_session_ttl_secs")]
    pub session_ttl_secs: u64,
    /// Per-listener association cap. 0 disables the cap.
    #[serde(default = "default_udp_max_associations")]
    pub max_associations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_script_entry")]
    pub entry: PathBuf,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_script_timeout_ms")]
    pub timeout_ms: u64,
    /// Hard memory ceiling (MiB) for the embedded QuickJS engine. A plugin that
    /// exceeds it triggers a JavaScript out-of-memory error, never an abort.
    #[serde(default = "default_script_memory_limit_mb")]
    pub memory_limit_mb: u64,
    /// Maximum JavaScript stack size (KiB) for the embedded engine.
    #[serde(default = "default_script_max_stack_size_kb")]
    pub max_stack_size_kb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_plugins_auto_load_dir")]
    pub auto_load_dir: PathBuf,
    #[serde(default = "default_plugin_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_true")]
    pub allow_admin_manage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoadBalanceConfig {
    #[serde(default)]
    pub algorithm: LoadBalanceAlgorithm,
    #[serde(default)]
    pub retries: RetryPolicyConfig,
    #[serde(default)]
    pub active_health: ActiveHealthConfig,
    #[serde(default)]
    pub passive_health: PassiveHealthConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceAlgorithm {
    #[default]
    Rendezvous,
    RoundRobin,
    LeastConnections,
    SourceHash,
    Weighted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_lb_max_retries")]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveHealthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_lb_fail_threshold")]
    pub fail_threshold: u32,
    #[serde(default = "default_lb_quarantine_secs")]
    pub quarantine_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveHealthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub http_enabled: bool,
    #[serde(default = "default_true")]
    pub tcp_enabled: bool,
    #[serde(default)]
    pub udp_enabled: bool,
    #[serde(default = "default_active_health_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "default_active_health_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_active_health_path")]
    pub path: String,
    #[serde(default = "default_active_health_expected_statuses")]
    pub expected_statuses: Vec<u16>,
    #[serde(default = "default_active_health_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_active_health_success_threshold")]
    pub success_threshold: u32,
    #[serde(default)]
    pub jitter_percent: u8,
    #[serde(default)]
    pub alert_webhooks: Vec<String>,
    #[serde(default = "default_active_health_udp_payload")]
    pub udp_payload: String,
    #[serde(default = "default_true")]
    pub udp_expect_response: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveHealthOverrideConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub expected_statuses: Option<Vec<u16>>,
    #[serde(default)]
    pub failure_threshold: Option<u32>,
    #[serde(default)]
    pub success_threshold: Option<u32>,
    #[serde(default)]
    pub jitter_percent: Option<u8>,
    #[serde(default)]
    pub alert_webhooks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_affinity_ttl")]
    pub sticky_ttl_secs: u64,
    #[serde(default = "default_true")]
    pub fallback_to_remote_addr: bool,
    #[serde(default)]
    pub http: HttpAffinityConfig,
    #[serde(default)]
    pub stream: StreamAffinityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAffinityConfig {
    #[serde(default = "default_http_affinity_query_keys")]
    pub query_keys: Vec<String>,
    #[serde(default = "default_http_affinity_header_keys")]
    pub header_keys: Vec<String>,
    #[serde(default = "default_http_affinity_cookie_keys")]
    pub cookie_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamAffinityConfig {
    #[serde(default = "default_stream_affinity_prefixes")]
    pub probe_prefixes: Vec<String>,
    #[serde(default = "default_stream_affinity_delimiters")]
    pub probe_delimiters: Vec<String>,
    #[serde(default = "default_stream_peek_bytes")]
    pub peek_bytes: usize,
    #[serde(default = "default_stream_peek_timeout_ms")]
    pub peek_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuthRateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_admin_auth_max_failures")]
    pub max_failures: u32,
    #[serde(default = "default_admin_auth_window_secs")]
    pub window_secs: u64,
    #[serde(default = "default_admin_auth_lockout_secs")]
    pub lockout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminHttpsConfig {
    /// Expose the full admin API on the main gateway HTTPS listener at `path_prefix`.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_admin_https_path_prefix")]
    pub path_prefix: String,
    /// When empty, any Host on the TLS listener may reach the HTTPS admin API.
    #[serde(default)]
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_admin_bind")]
    pub bind: String,
    #[serde(default = "default_admin_username")]
    pub username: String,
    #[serde(default = "default_admin_password")]
    pub password: String,
    #[serde(
        default = "default_admin_bearer_token",
        skip_serializing_if = "skip_default_admin_bearer_token"
    )]
    pub bearer_token: String,
    #[serde(default)]
    pub expose_config: bool,
    #[serde(default)]
    pub enable_write_ops: bool,
    #[serde(default = "default_true")]
    pub loopback_only: bool,
    #[serde(default)]
    pub https: AdminHttpsConfig,
    #[serde(default)]
    pub auth_rate_limit: AdminAuthRateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdosProtectionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ddos_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_ddos_window_secs")]
    pub window_secs: u64,
    #[serde(default = "default_ddos_ban_secs")]
    pub ban_secs: u64,
    #[serde(default)]
    pub burst: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicBlacklistConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_dynamic_blacklist_path")]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub validate_admin_mutations: bool,
    #[serde(default = "default_true")]
    pub block_ssrf_targets: bool,
    #[serde(default = "default_true")]
    pub reject_ambiguous_http1: bool,
    #[serde(default = "default_blocked_upstream_hosts")]
    pub blocked_upstream_hosts: Vec<String>,
    #[serde(default = "default_blocked_upstream_cidrs")]
    pub blocked_upstream_cidrs: Vec<String>,
    #[serde(default)]
    pub ddos: DdosProtectionConfig,
    /// MAC deny list (Linux L2 only; ignored on Windows/macOS).
    #[serde(default)]
    pub mac_deny: Vec<String>,
    #[serde(default)]
    pub dynamic_blacklist: DynamicBlacklistConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_k8s_namespace")]
    pub namespace: String,
    #[serde(default = "default_k8s_cluster_domain")]
    pub cluster_domain: String,
    #[serde(default)]
    pub mappings: Vec<KubernetesServiceMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesServiceMapping {
    pub name: String,
    pub service: String,
    #[serde(default = "default_k8s_service_port")]
    pub port: u16,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_route_path_prefix")]
    pub path_prefix: String,
    #[serde(default)]
    pub strip_prefix: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MonitoringFormat {
    #[default]
    Prometheus,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_monitoring_path")]
    pub path: String,
    #[serde(default)]
    pub format: MonitoringFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub hot_reload: HotReloadConfig,
    #[serde(default)]
    pub maintenance_state: MaintenanceStateConfig,
    #[serde(default)]
    pub watchdog: WatchdogConfig,
    #[serde(default)]
    pub performance: RuntimePerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePerformanceConfig {
    /// Default-on adaptive OS/runtime tuning. This never writes sysctl files;
    /// persistent host tuning stays explicit through `proxysss tune linux --apply`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub profile: RuntimePerformanceProfile,
    #[serde(default)]
    pub traffic_profile: RuntimePerformanceTrafficProfile,
    #[serde(default = "default_true")]
    pub adaptive_system: bool,
    #[serde(default = "default_true")]
    pub socket_extreme: bool,
    #[serde(default = "default_true")]
    pub log_on_start: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePerformanceProfile {
    #[default]
    Edge,
    Bulk,
    Latency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePerformanceTrafficProfile {
    #[default]
    Small,
    Balanced,
    Bulk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub restart_critical_tasks: bool,
    #[serde(default = "default_watchdog_restart_backoff_secs")]
    pub restart_backoff_secs: u64,
    #[serde(default = "default_watchdog_heartbeat_interval_secs")]
    pub heartbeat_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_reload_interval_ms")]
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceStateConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_maintenance_state_path")]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesConfig {
    #[serde(default)]
    pub reverse_proxy: ReverseProxyConfig,
    #[serde(default)]
    pub service_discovery: ServiceDiscoveryConfig,
    #[serde(default)]
    pub response_policy: HttpResponsePolicyConfig,
    #[serde(default)]
    pub cache_zones: Vec<CacheZoneConfig>,
    #[serde(default)]
    pub domain_routes: Vec<DomainRouteConfig>,
    #[serde(default)]
    pub access_control: AccessControlConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub webdav: WebDavConfig,
    #[serde(default)]
    pub filecloud: FileCloudConfig,
    #[serde(default)]
    pub static_sites: Vec<StaticSiteConfig>,
    #[serde(default)]
    pub ftp: FtpConfig,
    #[serde(default)]
    pub ai_proxy: crate::ai_proxy::AiProxyConfig,
}

/// Control-plane service discovery declarations.
///
/// Discovery is intentionally modeled as configuration metadata instead of a
/// data-plane lookup hook. Automation can poll Consul/etcd/Nacos, write the
/// resolved upstreams back into the single YAML file or admin API, and let the
/// hot reload path publish a fresh in-memory routing table.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceDiscoveryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_service_discovery_interval_secs")]
    pub interval_secs: u64,
    #[serde(default)]
    pub registries: Vec<ServiceRegistryConfig>,
    #[serde(default)]
    pub mappings: Vec<ServiceDiscoveryMappingConfig>,
}

/// A named registry endpoint that can be referenced by one or more mappings.
///
/// Credentials stay on this control-plane object so general config display
/// paths can redact them consistently with other secrets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistryConfig {
    pub name: String,
    #[serde(default)]
    pub provider: ServiceRegistryProvider,
    pub endpoint: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

/// Supported registry families for gateway automation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRegistryProvider {
    #[default]
    Consul,
    Etcd,
    Nacos,
}

/// Connects one discovered service name to one gateway upstream pool.
///
/// The mapping names the gateway object to refresh, but does not make request
/// forwarding depend on the registry. That keeps HTTP/TCP/UDP hot paths
/// deterministic and free of registry network I/O.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryMappingConfig {
    pub name: String,
    pub registry: String,
    pub service: String,
    #[serde(default)]
    pub target: ServiceDiscoveryTarget,
    pub target_name: String,
    #[serde(default)]
    pub scheme: String,
    #[serde(default)]
    pub port_name: String,
    #[serde(default)]
    pub metadata_filter: BTreeMap<String, String>,
}

/// Gateway object kinds that can receive discovered upstreams.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceDiscoveryTarget {
    #[default]
    ReverseProxyRoute,
    DomainRoute,
    TcpListener,
    UdpListener,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub http: HttpRateLimitConfig,
    #[serde(default)]
    pub stream: StreamRateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_zone")]
    pub zone: String,
    #[serde(default)]
    pub algorithm: RateLimitAlgorithm,
    #[serde(default = "default_rate_limit_requests")]
    pub connections: u32,
    #[serde(default = "default_rate_limit_window_ms")]
    pub window_ms: u64,
    #[serde(default)]
    pub burst: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessControlConfig {
    #[serde(default)]
    pub http: HttpAccessControlConfig,
    #[serde(default)]
    pub stream: StreamAccessControlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAccessControlConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, alias = "allowlist", alias = "whitelist")]
    pub allow: Vec<String>,
    #[serde(default, alias = "denylist", alias = "blacklist", alias = "blocklist")]
    pub deny: Vec<String>,
    #[serde(default = "default_access_control_status")]
    pub status: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitAlgorithm {
    #[default]
    FixedWindow,
    TokenBucket,
    LeakyBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_zone")]
    pub zone: String,
    #[serde(default)]
    pub algorithm: RateLimitAlgorithm,
    #[serde(default)]
    pub key: RateLimitKey,
    #[serde(default = "default_rate_limit_requests")]
    pub requests: u32,
    #[serde(default = "default_rate_limit_window_ms")]
    pub window_ms: u64,
    #[serde(default)]
    pub burst: u32,
    #[serde(default)]
    pub max_connections: u32,
    #[serde(default = "default_rate_limit_status")]
    pub status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitKey {
    #[default]
    RemoteAddr,
    Host,
    Header(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReverseProxyConfig {
    #[serde(default)]
    pub routes: Vec<ReverseProxyRouteConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpResponsePolicyConfig {
    #[serde(default)]
    pub compression: ResponseCompressionConfig,
    #[serde(default)]
    pub cache: ResponseCacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseProxyRouteConfig {
    pub name: String,
    #[serde(default = "default_route_path_prefix")]
    pub path_prefix: String,
    #[serde(default)]
    pub hosts: Vec<String>,
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_weights: BTreeMap<String, u32>,
    #[serde(default)]
    pub strip_prefix: bool,
    #[serde(default, alias = "add_headers")]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default, alias = "remove_headers")]
    pub strip_headers: Vec<String>,
    #[serde(default = "default_true")]
    pub forward_headers: bool,
    #[serde(default)]
    pub compression: ResponseCompressionConfig,
    #[serde(default)]
    pub cache: ResponseCacheConfig,
    #[serde(default)]
    pub rate_limit: HttpRateLimitConfig,
    #[serde(default)]
    pub active_health: ActiveHealthOverrideConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRouteConfig {
    pub name: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_route_path_prefix")]
    pub path_prefix: String,
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_weights: BTreeMap<String, u32>,
    #[serde(default)]
    pub strip_prefix: bool,
    #[serde(default, alias = "add_headers")]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default, alias = "remove_headers")]
    pub strip_headers: Vec<String>,
    #[serde(default = "default_true")]
    pub forward_headers: bool,
    #[serde(default)]
    pub compression: ResponseCompressionConfig,
    #[serde(default)]
    pub cache: ResponseCacheConfig,
    #[serde(default)]
    pub rate_limit: HttpRateLimitConfig,
    #[serde(default)]
    pub active_health: ActiveHealthOverrideConfig,
    #[serde(default)]
    pub ssl: DomainTlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCompressionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_compression_algorithms")]
    pub algorithms: Vec<CompressionAlgorithm>,
    #[serde(default = "default_compression_min_length")]
    pub min_length: usize,
    #[serde(default = "default_compression_types")]
    pub content_types: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    #[default]
    Zstd,
    Brotli,
    Gzip,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CacheBehavior {
    /// Honor origin Cache-Control for storage; use configured edge TTL when storing.
    #[default]
    RespectOrigin,
    /// Skip cache lookup and storage entirely.
    Bypass,
    /// Always fetch upstream; may still emit no-cache response headers.
    NoCache,
    /// Force edge TTL from config; ignore origin max-age for storage decisions.
    Override,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub behavior: CacheBehavior,
    #[serde(default = "default_cache_zone")]
    pub zone: String,
    #[serde(default)]
    pub key_prefix: String,
    #[serde(default)]
    pub vary_headers: Vec<String>,
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    /// Browser/client max-age override (0 = pass through origin Cache-Control).
    #[serde(default)]
    pub browser_ttl_secs: u64,
    #[serde(default)]
    pub stale_while_revalidate_secs: u64,
    #[serde(default)]
    pub stale_if_error_secs: u64,
    #[serde(default)]
    pub emit_cdn_cache_control: bool,
    #[serde(default = "default_cache_statuses")]
    pub statuses: Vec<u16>,
    #[serde(default = "default_cache_max_body_bytes")]
    pub max_body_bytes: usize,
    #[serde(default = "default_true")]
    pub allow_purge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheZoneConfig {
    pub name: String,
    #[serde(default = "default_cache_zone_max_entries")]
    pub max_entries: usize,
    #[serde(default)]
    pub disk_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainTlsConfig {
    #[serde(default, rename = "type")]
    pub mode: DomainTlsMode,
    #[serde(default)]
    pub is_auto_ssl: bool,
    #[serde(default)]
    pub cert_path: PathBuf,
    #[serde(default)]
    pub key_path: PathBuf,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DomainTlsMode {
    #[default]
    Inherit,
    Disabled,
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticSiteConfig {
    pub name: String,
    #[serde(default = "default_static_path_prefix")]
    pub path_prefix: String,
    #[serde(default = "default_static_root")]
    pub root: PathBuf,
    #[serde(default = "default_static_index_files")]
    pub index_files: Vec<String>,
    #[serde(default)]
    pub autoindex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_webdav_path_prefix")]
    pub path_prefix: String,
    #[serde(default = "default_webdav_root")]
    pub root: PathBuf,
    #[serde(default = "default_true")]
    pub allow_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCloudConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_filecloud_path_prefix")]
    pub path_prefix: String,
    #[serde(default = "default_filecloud_root")]
    pub root: PathBuf,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_filecloud_title")]
    pub title: String,
    #[serde(default = "default_true")]
    pub allow_upload: bool,
    #[serde(default = "default_true")]
    pub allow_delete: bool,
    #[serde(default = "default_true")]
    pub allow_mkdir: bool,
    #[serde(default = "default_true")]
    pub allow_move: bool,
    #[serde(default = "default_filecloud_max_upload_bytes")]
    pub max_upload_bytes: u64,
    #[serde(default = "default_filecloud_cdn_cache_secs")]
    pub cdn_cache_secs: u64,
    #[serde(default = "default_filecloud_session_ttl_secs")]
    pub session_ttl_secs: u64,
    #[serde(default)]
    pub require_auth_for_download: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ftp_bind")]
    pub bind: String,
    #[serde(default = "default_ftp_upstream", alias = "proxy_pass")]
    pub upstream: String,
    #[serde(default = "default_true")]
    pub native_control: bool,
    #[serde(default, alias = "pasv_address")]
    pub public_ip: String,
    #[serde(default = "default_ftp_passive_port_start", alias = "port_start")]
    pub passive_port_start: u16,
    #[serde(default = "default_ftp_passive_port_end", alias = "port_end")]
    pub passive_port_end: u16,
    #[serde(default = "default_true")]
    pub passive_hint: bool,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub command_allow: Vec<String>,
    #[serde(default)]
    pub command_deny: Vec<String>,
    #[serde(default)]
    pub transfer_allow: Vec<String>,
    #[serde(default)]
    pub transfer_deny: Vec<String>,
    #[serde(default)]
    pub user_policies: Vec<FtpUserPolicy>,
    #[serde(default = "default_ftp_proxy_timeout_ms")]
    pub proxy_timeout_ms: u64,
    #[serde(default)]
    pub max_login_attempts: u32,
    #[serde(default)]
    pub limit_rate: u64,
    #[serde(default = "default_true")]
    pub log_commands: bool,
    #[serde(default = "default_true")]
    pub log_transfers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpUserPolicy {
    pub user: String,
    #[serde(default)]
    pub command_allow: Vec<String>,
    #[serde(default)]
    pub command_deny: Vec<String>,
    #[serde(default)]
    pub transfer_allow: Vec<String>,
    #[serde(default)]
    pub transfer_deny: Vec<String>,
}

impl GatewayConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let value = load_config_value(path)?;
        let mut config: GatewayConfig = serde_yaml::from_value(value)
            .with_context(|| format!("failed to decode merged config {}", path.display()))?;

        config.normalize(path.parent().unwrap_or_else(|| Path::new(".")));
        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = Vec::<String>::new();

        if self.http.plain_bind.trim().is_empty()
            && self.http.tls_bind.trim().is_empty()
            && self.http.h3_bind.trim().is_empty()
        {
            errors.push(
                "at least one of http.plain_bind/http.tls_bind/http.h3_bind must be set"
                    .to_string(),
            );
        }

        validate_bind_optional("http.plain_bind", &self.http.plain_bind, &mut errors);
        validate_bind_optional("http.tls_bind", &self.http.tls_bind, &mut errors);
        validate_bind_optional("http.h3_bind", &self.http.h3_bind, &mut errors);

        if self.http.request_timeout_ms == 0 {
            errors.push("http.request_timeout_ms must be greater than 0".to_string());
        }

        for (index, page) in self.http.error_pages.pages.iter().enumerate() {
            if !(400..=599).contains(&page.status) {
                errors.push(format!(
                    "http.error_pages.pages.{index}.status must be in 400..=599"
                ));
            }
            if page.body.trim().is_empty() && page.file_path.as_os_str().is_empty() {
                errors.push(format!(
                    "http.error_pages.pages.{index} requires body or file_path"
                ));
            }
            if page.content_type.trim().is_empty() {
                errors.push(format!(
                    "http.error_pages.pages.{index}.content_type cannot be empty"
                ));
            }
        }

        if self.logging.access_sample_rate <= 0.0 || self.logging.access_sample_rate > 1.0 {
            errors.push("logging.access_sample_rate must be in (0, 1]".to_string());
        }

        if self.runtime.hot_reload.interval_ms < 200 {
            errors.push("runtime.hot_reload.interval_ms must be >= 200".to_string());
        }

        if self.runtime.maintenance_state.enabled
            && self.runtime.maintenance_state.path.as_os_str().is_empty()
        {
            errors.push("runtime.maintenance_state.path cannot be empty".to_string());
        }

        if self.runtime.watchdog.enabled {
            if self.runtime.watchdog.restart_backoff_secs == 0 {
                errors.push(
                    "runtime.watchdog.restart_backoff_secs must be greater than 0".to_string(),
                );
            }
            if self.runtime.watchdog.heartbeat_interval_secs == 0 {
                errors.push(
                    "runtime.watchdog.heartbeat_interval_secs must be greater than 0".to_string(),
                );
            }
        }

        if !self.include.is_disabled() {
            errors.push(
                "include is removed in v1.0: merge every referenced file into proxysss.yaml and delete the include block"
                    .to_string(),
            );
        }

        if self.script.enabled {
            if self.script.entry.as_os_str().is_empty() {
                errors.push("script.entry cannot be empty when script.enabled=true".to_string());
            }

            if self.script.timeout_ms == 0 {
                errors.push(
                    "script.timeout_ms must be greater than 0 when script.enabled=true".to_string(),
                );
            }
        }

        if self.load_balance.retries.enabled && self.load_balance.retries.max_retries > 16 {
            errors.push("load_balance.retries.max_retries must be <= 16".to_string());
        }

        if self.load_balance.passive_health.enabled {
            if self.load_balance.passive_health.fail_threshold == 0 {
                errors.push(
                    "load_balance.passive_health.fail_threshold must be greater than 0".to_string(),
                );
            }
            if self.load_balance.passive_health.quarantine_secs == 0 {
                errors.push(
                    "load_balance.passive_health.quarantine_secs must be greater than 0"
                        .to_string(),
                );
            }
        }

        if self.load_balance.active_health.enabled {
            if !self.load_balance.active_health.http_enabled
                && !self.load_balance.active_health.tcp_enabled
                && !self.load_balance.active_health.udp_enabled
            {
                errors.push(
                    "load_balance.active_health requires http_enabled, tcp_enabled, or udp_enabled"
                        .to_string(),
                );
            }
            if self.load_balance.active_health.interval_secs == 0 {
                errors.push(
                    "load_balance.active_health.interval_secs must be greater than 0".to_string(),
                );
            }
            if self.load_balance.active_health.timeout_ms < 100 {
                errors.push("load_balance.active_health.timeout_ms must be >= 100".to_string());
            }
            if self.load_balance.active_health.http_enabled
                && !self.load_balance.active_health.path.starts_with('/')
            {
                errors.push("load_balance.active_health.path must start with /".to_string());
            }
            if self.load_balance.active_health.http_enabled
                && self.load_balance.active_health.expected_statuses.is_empty()
            {
                errors.push(
                    "load_balance.active_health.expected_statuses cannot be empty".to_string(),
                );
            }
            if self.load_balance.active_health.failure_threshold == 0 {
                errors.push(
                    "load_balance.active_health.failure_threshold must be greater than 0"
                        .to_string(),
                );
            }
            if self.load_balance.active_health.success_threshold == 0 {
                errors.push(
                    "load_balance.active_health.success_threshold must be greater than 0"
                        .to_string(),
                );
            }
            if self.load_balance.active_health.jitter_percent > 100 {
                errors.push("load_balance.active_health.jitter_percent must be <= 100".to_string());
            }
            if self.load_balance.active_health.udp_enabled
                && self.load_balance.active_health.udp_payload.is_empty()
            {
                errors.push("load_balance.active_health.udp_payload cannot be empty".to_string());
            }
            for (index, webhook) in self
                .load_balance
                .active_health
                .alert_webhooks
                .iter()
                .enumerate()
            {
                if !looks_like_url(webhook) {
                    errors.push(format!(
                        "load_balance.active_health.alert_webhooks.{index} must be an http/https URL"
                    ));
                }
            }
        }

        if self.plugins.enabled {
            if !self.script.enabled {
                errors.push(
                    "plugins.enabled=true requires script.enabled=true because plugins run inside the TypeScript runtime".to_string(),
                );
            }
            if self.plugins.extensions.is_empty() {
                errors.push(
                    "plugins.extensions cannot be empty when plugins are enabled".to_string(),
                );
            }
            if self.plugins.auto_load_dir.as_os_str().is_empty() {
                errors.push(
                    "plugins.auto_load_dir cannot be empty when plugins are enabled".to_string(),
                );
            }
        }

        if self.services.rate_limit.http.enabled {
            validate_http_rate_limit_config(
                &self.services.rate_limit.http,
                "services.rate_limit.http",
                &mut errors,
            );
        }

        if self.services.rate_limit.stream.enabled {
            validate_stream_rate_limit_config(
                &self.services.rate_limit.stream,
                "services.rate_limit.stream",
                &mut errors,
            );
        }

        if self.services.service_discovery.enabled {
            validate_service_discovery_config(&self.services.service_discovery, &mut errors);
        }

        if self.services.access_control.http.enabled {
            if !(100..=599).contains(&self.services.access_control.http.status) {
                errors.push(
                    "services.access_control.http.status must be a valid HTTP status".to_string(),
                );
            }
            for (kind, entries) in [
                ("allow", &self.services.access_control.http.allow),
                ("deny", &self.services.access_control.http.deny),
            ] {
                for (index, entry) in entries.iter().enumerate() {
                    if entry.trim().is_empty() {
                        errors.push(format!(
                            "services.access_control.http.{kind}.{index} cannot be empty"
                        ));
                        continue;
                    }
                    if !is_valid_ip_rule(entry) {
                        errors.push(format!(
                            "services.access_control.http.{kind}.{index} must be an IP or CIDR block: {entry}"
                        ));
                    }
                }
            }
        }

        let mut route_names = HashSet::<String>::new();
        for route in &self.services.reverse_proxy.routes {
            if route.name.trim().is_empty() {
                errors.push("services.reverse_proxy.routes.name cannot be empty".to_string());
            }
            if !route_names.insert(route.name.clone()) {
                errors.push(format!("duplicate reverse proxy route name {}", route.name));
            }
            if route.path_prefix.trim().is_empty() || !route.path_prefix.starts_with('/') {
                errors.push(format!(
                    "services.reverse_proxy.routes.{}.path_prefix must start with /",
                    route.name
                ));
            }
            if route.upstream.trim().is_empty() {
                errors.push(format!(
                    "services.reverse_proxy.routes.{}.upstream cannot be empty",
                    route.name
                ));
            }
            if route.hosts.iter().any(|host| host.trim().is_empty()) {
                errors.push(format!(
                    "services.reverse_proxy.routes.{}.hosts cannot contain empty items",
                    route.name
                ));
            }
            validate_compression_config(
                &route.compression,
                &format!("services.reverse_proxy.routes.{}.compression", route.name),
                &mut errors,
            );
            validate_cache_config(
                &route.cache,
                &format!("services.reverse_proxy.routes.{}.cache", route.name),
                &mut errors,
            );
            validate_cache_zone_reference(
                &route.cache,
                &self.services.cache_zones,
                &format!("services.reverse_proxy.routes.{}.cache", route.name),
                &mut errors,
            );
            validate_http_rate_limit_config(
                &route.rate_limit,
                &format!("services.reverse_proxy.routes.{}.rate_limit", route.name),
                &mut errors,
            );
            validate_active_health_override(
                &route.active_health,
                &format!("services.reverse_proxy.routes.{}.active_health", route.name),
                &mut errors,
            );
        }

        validate_compression_config(
            &self.services.response_policy.compression,
            "services.response_policy.compression",
            &mut errors,
        );
        validate_cache_config(
            &self.services.response_policy.cache,
            "services.response_policy.cache",
            &mut errors,
        );
        validate_cache_zone_reference(
            &self.services.response_policy.cache,
            &self.services.cache_zones,
            "services.response_policy.cache",
            &mut errors,
        );

        let mut cache_zone_names = HashSet::<String>::new();
        for zone in &self.services.cache_zones {
            if zone.name.trim().is_empty() {
                errors.push("services.cache_zones.name cannot be empty".to_string());
            }
            if !cache_zone_names.insert(zone.name.clone()) {
                errors.push(format!("duplicate cache zone name {}", zone.name));
            }
            if zone.max_entries == 0 {
                errors.push(format!(
                    "services.cache_zones.{}.max_entries must be greater than 0",
                    zone.name
                ));
            }
        }

        if self.services.ai_proxy.enabled && self.services.ai_proxy.routes.is_empty() {
            errors.push(
                "services.ai_proxy.routes cannot be empty when ai_proxy is enabled".to_string(),
            );
        }
        let mut ai_route_names = HashSet::<String>::new();
        for route in &self.services.ai_proxy.routes {
            if route.name.trim().is_empty() {
                errors.push("services.ai_proxy.routes.name cannot be empty".to_string());
            }
            if !ai_route_names.insert(route.name.clone()) {
                errors.push(format!("duplicate ai proxy route name {}", route.name));
            }
            if route.path_prefix.trim().is_empty() || !route.path_prefix.starts_with('/') {
                errors.push(format!(
                    "services.ai_proxy.routes.{}.path_prefix must start with /",
                    route.name
                ));
            }
            if route.upstream.trim().is_empty() {
                errors.push(format!(
                    "services.ai_proxy.routes.{}.upstream cannot be empty",
                    route.name
                ));
            }
        }

        let mut domain_route_names = HashSet::<String>::new();
        for route in &self.services.domain_routes {
            if route.name.trim().is_empty() {
                errors.push("services.domain_routes.name cannot be empty".to_string());
            }
            if !domain_route_names.insert(route.name.clone()) {
                errors.push(format!("duplicate domain route name {}", route.name));
            }
            if route.domains.is_empty() {
                errors.push(format!(
                    "services.domain_routes.{}.domains cannot be empty",
                    route.name
                ));
            }
            if route.domains.iter().any(|host| host.trim().is_empty()) {
                errors.push(format!(
                    "services.domain_routes.{}.domains cannot contain empty items",
                    route.name
                ));
            }
            if route.path_prefix.trim().is_empty() || !route.path_prefix.starts_with('/') {
                errors.push(format!(
                    "services.domain_routes.{}.path_prefix must start with /",
                    route.name
                ));
            }
            if route.upstream.trim().is_empty() {
                errors.push(format!(
                    "services.domain_routes.{}.upstream cannot be empty",
                    route.name
                ));
            }
            validate_compression_config(
                &route.compression,
                &format!("services.domain_routes.{}.compression", route.name),
                &mut errors,
            );
            validate_cache_config(
                &route.cache,
                &format!("services.domain_routes.{}.cache", route.name),
                &mut errors,
            );
            validate_cache_zone_reference(
                &route.cache,
                &self.services.cache_zones,
                &format!("services.domain_routes.{}.cache", route.name),
                &mut errors,
            );
            validate_http_rate_limit_config(
                &route.rate_limit,
                &format!("services.domain_routes.{}.rate_limit", route.name),
                &mut errors,
            );
            validate_active_health_override(
                &route.active_health,
                &format!("services.domain_routes.{}.active_health", route.name),
                &mut errors,
            );
            match route.ssl.effective_mode() {
                DomainTlsMode::Manual => {
                    if route.ssl.cert_path.as_os_str().is_empty() {
                        errors.push(format!(
                            "services.domain_routes.{}.ssl.cert_path cannot be empty when ssl.type=manual",
                            route.name
                        ));
                    }
                    if route.ssl.key_path.as_os_str().is_empty() {
                        errors.push(format!(
                            "services.domain_routes.{}.ssl.key_path cannot be empty when ssl.type=manual",
                            route.name
                        ));
                    }
                }
                DomainTlsMode::Auto => {
                    if self.http.tls.auto_https.email.trim().is_empty()
                        && route.ssl.email.trim().is_empty()
                    {
                        errors.push(format!(
                            "services.domain_routes.{}.ssl.email or http.tls.auto_https.email is required when ssl.type=auto",
                            route.name
                        ));
                    }
                }
                DomainTlsMode::Disabled | DomainTlsMode::Inherit => {}
            }
        }

        if self.services.webdav.enabled {
            if self.services.webdav.path_prefix.trim().is_empty()
                || !self.services.webdav.path_prefix.starts_with('/')
            {
                errors.push("services.webdav.path_prefix must start with /".to_string());
            }
            if self.services.webdav.root.as_os_str().is_empty() {
                errors.push(
                    "services.webdav.root cannot be empty when webdav is enabled".to_string(),
                );
            }
        }

        if self.services.filecloud.enabled {
            if self.services.filecloud.path_prefix.trim().is_empty()
                || !self.services.filecloud.path_prefix.starts_with('/')
            {
                errors.push("services.filecloud.path_prefix must start with /".to_string());
            }
            if self.services.filecloud.root.as_os_str().is_empty() {
                errors.push(
                    "services.filecloud.root cannot be empty when filecloud is enabled".to_string(),
                );
            }
            if self.services.filecloud.password.trim().is_empty() {
                errors.push(
                    "services.filecloud.password cannot be empty when filecloud is enabled"
                        .to_string(),
                );
            }
            if self.services.filecloud.max_upload_bytes == 0 {
                errors
                    .push("services.filecloud.max_upload_bytes must be greater than 0".to_string());
            }
        }

        let mut static_names = HashSet::<String>::new();
        for site in &self.services.static_sites {
            if site.name.trim().is_empty() {
                errors.push("services.static_sites.name cannot be empty".to_string());
            }
            if !static_names.insert(site.name.clone()) {
                errors.push(format!("duplicate static site name {}", site.name));
            }
            if site.path_prefix.trim().is_empty() || !site.path_prefix.starts_with('/') {
                errors.push(format!(
                    "services.static_sites.{}.path_prefix must start with /",
                    site.name
                ));
            }
            if site.root.as_os_str().is_empty() {
                errors.push(format!(
                    "services.static_sites.{}.root cannot be empty",
                    site.name
                ));
            }
            if site
                .index_files
                .iter()
                .any(|item| item.contains('/') || item.contains('\\'))
            {
                errors.push(format!(
                    "services.static_sites.{}.index_files must contain file names only",
                    site.name
                ));
            }
        }

        if self.services.ftp.enabled {
            validate_bind_required("services.ftp.bind", &self.services.ftp.bind, &mut errors);
            if self.services.ftp.upstream.trim().is_empty() {
                errors
                    .push("services.ftp.upstream cannot be empty when ftp is enabled".to_string());
            }
            if self.services.ftp.native_control
                && self.services.ftp.passive_port_start > self.services.ftp.passive_port_end
            {
                errors.push(
                    "services.ftp.passive_port_start must be <= services.ftp.passive_port_end"
                        .to_string(),
                );
            }
            if !self.services.ftp.public_ip.trim().is_empty()
                && self.services.ftp.public_ip.parse::<IpAddr>().is_err()
            {
                errors.push("services.ftp.public_ip must be a valid IP address".to_string());
            }
        }

        if self.admin.enabled {
            validate_bind_optional("admin.bind", &self.admin.bind, &mut errors);
            if self.admin.username.trim().is_empty() {
                errors.push("admin.username cannot be empty when admin is enabled".to_string());
            }
            if self.admin.password.len() < 4 {
                errors.push(
                    "admin.password must be at least 4 characters when admin is enabled"
                        .to_string(),
                );
            }
            if self.admin.https.enabled {
                let prefix = normalize_admin_https_path_prefix(&self.admin.https.path_prefix);
                if prefix == "/" {
                    errors.push("admin.https.path_prefix cannot be /".to_string());
                }
                if !prefix.starts_with('/') {
                    errors.push("admin.https.path_prefix must start with /".to_string());
                }
                for reserved in ["/docs", "/healthz", "/metrics", "/filecloud"] {
                    if prefix == reserved || prefix.starts_with(&format!("{reserved}/")) {
                        errors.push(format!(
                            "admin.https.path_prefix cannot overlap reserved path {reserved}"
                        ));
                    }
                }
            }
        }

        if self.monitoring.enabled && !self.monitoring.path.starts_with('/') {
            errors.push("monitoring.path must start with '/'".to_string());
        }

        if self.affinity.sticky_ttl_secs == 0 {
            errors.push("affinity.sticky_ttl_secs must be greater than 0".to_string());
        }

        if self.affinity.stream.peek_bytes == 0 {
            errors.push("affinity.stream.peek_bytes must be greater than 0".to_string());
        }

        if self.affinity.stream.peek_timeout_ms == 0 {
            errors.push("affinity.stream.peek_timeout_ms must be greater than 0".to_string());
        }

        let mut stream_route_names = HashSet::<String>::new();
        for route in &self.tcp.stream_routes {
            if route.name.trim().is_empty() {
                errors.push("tcp.stream_routes[].name cannot be empty".to_string());
            }
            if !stream_route_names.insert(route.name.clone()) {
                errors.push(format!("duplicate tcp.stream_routes name {}", route.name));
            }
            if route.listen.trim().is_empty() {
                errors.push(format!(
                    "tcp.stream_routes.{}.listen cannot be empty",
                    route.name
                ));
            }
            if route.upstream.trim().is_empty() {
                errors.push(format!(
                    "tcp.stream_routes.{}.upstream cannot be empty",
                    route.name
                ));
            }
            validate_bind_required(
                &format!("tcp.stream_routes.{}.listen", route.name),
                &normalize_stream_listen(&route.listen),
                &mut errors,
            );
        }

        if self.http.tls.on_demand.enabled && self.http.tls.mode != TlsMode::AcmeManaged {
            errors
                .push("http.tls.on_demand.enabled requires http.tls.mode=acme_managed".to_string());
        }

        if self.security.ddos.enabled && self.security.ddos.max_connections == 0 {
            errors.push("security.ddos.max_connections must be greater than 0".to_string());
        }

        let mut tcp_names = HashSet::<String>::new();
        let mut tcp_binds = HashSet::<String>::new();
        for listener in &self.tcp.listeners {
            if listener.name.trim().is_empty() {
                errors.push("tcp listener name cannot be empty".to_string());
            }
            validate_bind_required(
                &format!("tcp.listeners.{}.bind", listener.name),
                &listener.bind,
                &mut errors,
            );
            if !tcp_names.insert(listener.name.clone()) {
                errors.push(format!("duplicate tcp listener name {}", listener.name));
            }
            if !tcp_binds.insert(listener.bind.clone()) {
                errors.push(format!("duplicate tcp listener bind {}", listener.bind));
            }
            if listener.upstream.trim().is_empty()
                && listener.upstreams.iter().all(|item| item.trim().is_empty())
                && !self.script.enabled
            {
                errors.push(format!(
                    "tcp.listeners.{} requires upstream/upstreams when script.enabled=false",
                    listener.name
                ));
            }
            if listener.connect_timeout_ms == 0 {
                errors.push(format!(
                    "tcp.listeners.{}.connect_timeout_ms must be greater than 0",
                    listener.name
                ));
            }
        }

        let mut udp_names = HashSet::<String>::new();
        let mut udp_binds = HashSet::<String>::new();
        for listener in &self.udp.listeners {
            if listener.name.trim().is_empty() {
                errors.push("udp listener name cannot be empty".to_string());
            }
            validate_bind_required(
                &format!("udp.listeners.{}.bind", listener.name),
                &listener.bind,
                &mut errors,
            );
            if !udp_names.insert(listener.name.clone()) {
                errors.push(format!("duplicate udp listener name {}", listener.name));
            }
            if !udp_binds.insert(listener.bind.clone()) {
                errors.push(format!("duplicate udp listener bind {}", listener.bind));
            }
            if listener.upstream.trim().is_empty()
                && listener.upstreams.iter().all(|item| item.trim().is_empty())
                && !self.script.enabled
            {
                errors.push(format!(
                    "udp.listeners.{} requires upstream/upstreams when script.enabled=false",
                    listener.name
                ));
            }
            if listener.session_ttl_secs == 0 {
                errors.push(format!(
                    "udp.listeners.{}.session_ttl_secs must be greater than 0",
                    listener.name
                ));
            }
        }

        match self.http.tls.mode {
            TlsMode::Manual => {
                if !self.http.tls.cert_path.exists() {
                    errors.push(format!(
                        "http.tls.cert_path does not exist: {}",
                        self.http.tls.cert_path.display()
                    ));
                }
                if !self.http.tls.key_path.exists() {
                    errors.push(format!(
                        "http.tls.key_path does not exist: {}",
                        self.http.tls.key_path.display()
                    ));
                }
            }
            TlsMode::AcmeManaged | TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal => {
                if self.http.tls.auto_https.enabled {
                    if self.http.tls.auto_https.domains.is_empty() {
                        errors.push(
                            "http.tls.auto_https.domains cannot be empty when auto_https.enabled=true"
                                .to_string(),
                        );
                    }
                    if self.http.tls.auto_https.email.trim().is_empty() {
                        errors.push(
                            "http.tls.auto_https.email cannot be empty when auto_https.enabled=true"
                                .to_string(),
                        );
                    }
                    if matches!(
                        self.http.tls.mode,
                        TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal
                    ) && self.http.tls.auto_https.client.trim().is_empty()
                    {
                        errors.push(
                            "http.tls.auto_https.client cannot be empty when auto_https.enabled=true"
                                .to_string(),
                        );
                    }
                    if self.http.tls.auto_https.renew_interval_hours == 0 {
                        errors.push(
                            "http.tls.auto_https.renew_interval_hours must be greater than 0"
                                .to_string(),
                        );
                    }
                }
                if self.http.tls.acme.domains.is_empty() {
                    errors.push(
                        "http.tls.acme.domains cannot be empty when mode is acme_managed/acme_external/acme_dns_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.email.trim().is_empty() {
                    errors.push(
                        "http.tls.acme.email cannot be empty when mode is acme_managed/acme_external/acme_dns_external"
                            .to_string(),
                    );
                }
                if matches!(
                    self.http.tls.mode,
                    TlsMode::AcmeExternal | TlsMode::AcmeDnsExternal
                ) && self.http.tls.acme.client.trim().is_empty()
                {
                    errors.push(
                        "http.tls.acme.client cannot be empty when mode is acme_external/acme_dns_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.renew_interval_hours == 0 {
                    errors.push(
                        "http.tls.acme.renew_interval_hours must be greater than 0".to_string(),
                    );
                }
                if self.uses_managed_dns01() {
                    self.validate_acme_dns_settings(&mut errors);
                }
                if self.http.tls.mode == TlsMode::AcmeDnsExternal {
                    if !self
                        .http
                        .tls
                        .acme
                        .domains
                        .iter()
                        .any(|domain| domain.trim().starts_with("*."))
                    {
                        errors.push(
                            "http.tls.acme.domains must include at least one wildcard domain when mode is acme_dns_external"
                                .to_string(),
                        );
                    }
                    if self.http.tls.acme.dns.provider.trim().is_empty() {
                        errors.push(
                            "http.tls.acme.dns.provider cannot be empty when mode is acme_dns_external"
                                .to_string(),
                        );
                    }
                    if self.http.tls.acme.dns.credentials.is_empty() {
                        errors.push(
                            "http.tls.acme.dns.credentials cannot be empty when mode is acme_dns_external"
                                .to_string(),
                        );
                    }
                    for (key, value) in &self.http.tls.acme.dns.credentials {
                        if key.trim().is_empty() {
                            errors.push(
                                "http.tls.acme.dns.credentials cannot contain empty environment variable names"
                                    .to_string(),
                            );
                        }
                        if value.trim().is_empty() {
                            errors.push(format!(
                                "http.tls.acme.dns.credentials.{key} cannot be empty when mode is acme_dns_external"
                            ));
                        }
                    }
                }
            }
            TlsMode::SelfSigned => {}
        }

        for (index, certificate) in self.http.tls.certificates.iter().enumerate() {
            if certificate.domains.is_empty() {
                errors.push(format!(
                    "http.tls.certificates.{index}.domains cannot be empty"
                ));
            }
            if certificate
                .domains
                .iter()
                .any(|domain| domain.trim().is_empty())
            {
                errors.push(format!(
                    "http.tls.certificates.{index}.domains cannot contain empty items"
                ));
            }
            if !certificate.cert_path.exists() {
                errors.push(format!(
                    "http.tls.certificates.{index}.cert_path does not exist: {}",
                    certificate.cert_path.display()
                ));
            }
            if !certificate.key_path.exists() {
                errors.push(format!(
                    "http.tls.certificates.{index}.key_path does not exist: {}",
                    certificate.key_path.display()
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "configuration validation failed:\n - {}",
                errors.join("\n - ")
            ))
        }
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.admin.enabled
            && self.admin.username == DEFAULT_ADMIN_USERNAME
            && self.admin.password == DEFAULT_ADMIN_PASSWORD
        {
            warnings.push("admin credentials are still default; change admin.username/admin.password before production".to_string());
        }

        if self.http.tls.mode == TlsMode::SelfSigned {
            warnings.push("tls.mode=self_signed is for development or internal environments; use acme_managed, acme_external, or acme_dns_external for public traffic".to_string());
        }

        if self.http.plain_bind.trim().is_empty() {
            warnings.push(
                "http.plain_bind is disabled; the welcome page will not be reachable on port 80 (nginx parity expects 0.0.0.0:80)".to_string(),
            );
        }

        if self.admin.enabled && self.admin.enable_write_ops && !self.admin.loopback_only {
            warnings.push(
                "admin.enable_write_ops is true while admin.loopback_only is false; keep the admin API on loopback for production".to_string(),
            );
        }

        if self.admin.enabled && self.admin.enable_write_ops && self.admin.expose_config {
            warnings.push(
                "admin.expose_config is enabled with write operations; disable config export on untrusted networks".to_string(),
            );
        }

        if self.admin.https.enabled && self.admin.enable_write_ops {
            warnings.push(
                "admin.https is enabled with write operations; bootstrap TLS/ACME on loopback first, then drive automation over HTTPS".to_string(),
            );
        }

        warnings
    }

    pub fn render_default_yaml() -> String {
        let config = Self::default();
        serde_yaml::to_string(&config).unwrap_or_else(|_| "".to_string())
    }

    fn normalize(&mut self, root_dir: &Path) {
        let root_dir = normalize_root_dir(root_dir);
        self.root_dir = root_dir.clone();
        self.normalize_domain_tls(&root_dir);
        self.apply_auto_https();
        self.normalize_acme_dns_config();
        self.admin.https.path_prefix =
            normalize_admin_https_path_prefix(&self.admin.https.path_prefix);
        normalize_vec_lowercase(&mut self.admin.https.hosts);

        if self.logging.filter.trim().is_empty() {
            self.logging.filter = default_filter_for_level(self.logging.level);
        }
        self.log_filter = self.logging.filter.clone();

        self.http.tls.cert_path = absolutize(&root_dir, &self.http.tls.cert_path);
        self.http.tls.key_path = absolutize(&root_dir, &self.http.tls.key_path);
        self.http.tls.acme.cache_dir = absolutize(&root_dir, &self.http.tls.acme.cache_dir);
        for page in &mut self.http.error_pages.pages {
            page.file_path = absolutize_if_not_empty(&root_dir, &page.file_path);
        }
        self.runtime.maintenance_state.path =
            absolutize(&root_dir, &self.runtime.maintenance_state.path);
        normalize_vec_lowercase(&mut self.http.tls.auto_https.domains);
        normalize_vec_lowercase(&mut self.services.response_policy.compression.content_types);
        self.services.response_policy.compression.algorithms =
            normalize_compression_algorithms(&self.services.response_policy.compression.algorithms);
        if self.services.response_policy.cache.zone.trim().is_empty() {
            self.services.response_policy.cache.zone = default_cache_zone();
        }
        if self.services.rate_limit.http.zone.trim().is_empty() {
            self.services.rate_limit.http.zone = default_rate_limit_zone();
        }
        for zone in &mut self.services.cache_zones {
            zone.disk_path = absolutize_if_not_empty(&root_dir, &zone.disk_path);
        }
        for certificate in &mut self.http.tls.certificates {
            normalize_vec_lowercase(&mut certificate.domains);
            certificate.cert_path = absolutize(&root_dir, &certificate.cert_path);
            certificate.key_path = absolutize(&root_dir, &certificate.key_path);
        }
        self.plugins.auto_load_dir = absolutize(&root_dir, &self.plugins.auto_load_dir);
        self.logging.access_log_path = absolutize(&root_dir, &self.logging.access_log_path);
        self.logging.error_log_path = absolutize(&root_dir, &self.logging.error_log_path);
        self.services.webdav.root = absolutize(&root_dir, &self.services.webdav.root);
        self.services.filecloud.root = absolutize(&root_dir, &self.services.filecloud.root);
        for site in &mut self.services.static_sites {
            site.root = absolutize(&root_dir, &site.root);
        }

        self.script.cwd = Some(match &self.script.cwd {
            Some(cwd) => absolutize(&root_dir, cwd),
            None => root_dir.clone(),
        });

        normalize_vec_lowercase(&mut self.affinity.http.header_keys);
        normalize_vec_lowercase(&mut self.logging.redact_headers);
        crate::security::apply_kubernetes_routes(
            &mut self.kubernetes,
            &mut self.services.domain_routes,
        );
    }

    fn normalize_domain_tls(&mut self, root_dir: &Path) {
        let mut auto_domains = BTreeMap::<String, String>::new();
        let mut certificates = Vec::<TlsCertificateConfig>::new();
        for route in &mut self.services.reverse_proxy.routes {
            normalize_vec_lowercase(&mut route.compression.content_types);
            route.compression.algorithms =
                normalize_compression_algorithms(&route.compression.algorithms);
            if route.cache.zone.trim().is_empty() {
                route.cache.zone = default_cache_zone();
            }
            if route.rate_limit.zone.trim().is_empty() {
                route.rate_limit.zone = default_rate_limit_zone();
            }
            normalize_active_health_override(&mut route.active_health);
        }
        for route in &mut self.services.domain_routes {
            normalize_vec_lowercase(&mut route.domains);
            normalize_vec_lowercase(&mut route.compression.content_types);
            route.compression.algorithms =
                normalize_compression_algorithms(&route.compression.algorithms);
            if route.cache.zone.trim().is_empty() {
                route.cache.zone = default_cache_zone();
            }
            if route.rate_limit.zone.trim().is_empty() {
                route.rate_limit.zone = default_rate_limit_zone();
            }
            normalize_active_health_override(&mut route.active_health);
            route.ssl.cert_path = absolutize_if_not_empty(root_dir, &route.ssl.cert_path);
            route.ssl.key_path = absolutize_if_not_empty(root_dir, &route.ssl.key_path);

            match route.ssl.effective_mode() {
                DomainTlsMode::Manual => {
                    let domains = if route.domains.is_empty() {
                        Vec::new()
                    } else {
                        route.domains.clone()
                    };
                    certificates.push(TlsCertificateConfig {
                        domains,
                        cert_path: route.ssl.cert_path.clone(),
                        key_path: route.ssl.key_path.clone(),
                    });
                }
                DomainTlsMode::Auto => {
                    for domain in &route.domains {
                        auto_domains
                            .entry(domain.clone())
                            .or_insert_with(|| route.ssl.email.clone());
                    }
                }
                DomainTlsMode::Disabled | DomainTlsMode::Inherit => {}
            }
        }
        self.http.tls.certificates.extend(certificates);

        if !auto_domains.is_empty() {
            self.http.tls.auto_https.enabled = true;
            for (domain, email) in auto_domains {
                if !self
                    .http
                    .tls
                    .auto_https
                    .domains
                    .iter()
                    .any(|item| item == &domain)
                {
                    self.http.tls.auto_https.domains.push(domain);
                }
                if self.http.tls.auto_https.email.trim().is_empty() && !email.trim().is_empty() {
                    self.http.tls.auto_https.email = email;
                }
            }
        }
    }

    pub fn uses_managed_dns01(&self) -> bool {
        self.http.tls.mode == TlsMode::AcmeManaged
            && self.http.tls.acme.challenge == AcmeChallengeType::Dns01
    }

    fn normalize_acme_dns_config(&mut self) {
        if self.http.tls.mode == TlsMode::AcmeDnsExternal {
            let provider = self.http.tls.acme.dns.provider.clone();
            if crate::acme::is_builtin_dns_provider(&provider) {
                self.http.tls.mode = TlsMode::AcmeManaged;
                self.http.tls.acme.challenge = AcmeChallengeType::Dns01;
            }
        }

        if self.uses_managed_dns01() {
            self.http.tls.acme.dns.provider =
                crate::acme::normalize_provider_id(&self.http.tls.acme.dns.provider);
        }
    }

    fn validate_acme_dns_settings(&self, errors: &mut Vec<String>) {
        if self.http.tls.acme.dns.provider.trim().is_empty() {
            errors.push(
                "http.tls.acme.dns.provider cannot be empty when using built-in DNS-01".to_string(),
            );
        } else if !crate::acme::is_builtin_dns_provider(&self.http.tls.acme.dns.provider) {
            errors.push(format!(
                "http.tls.acme.dns.provider '{}' is not a built-in DNS provider; supported: {}",
                self.http.tls.acme.dns.provider,
                crate::acme::list_builtin_dns_provider_ids().join(", ")
            ));
        }
        if crate::acme::normalize_provider_id(&self.http.tls.acme.dns.provider) != "manual"
            && self.http.tls.acme.dns.credentials.is_empty()
        {
            errors.push(
                "http.tls.acme.dns.credentials cannot be empty when using built-in DNS-01"
                    .to_string(),
            );
        }
        if crate::acme::normalize_provider_id(&self.http.tls.acme.dns.provider) == "manual" {
            return;
        }
        for (key, value) in &self.http.tls.acme.dns.credentials {
            if key.trim().is_empty() {
                errors.push(
                    "http.tls.acme.dns.credentials cannot contain empty credential names"
                        .to_string(),
                );
            }
            if value.trim().is_empty() {
                errors.push(format!(
                    "http.tls.acme.dns.credentials.{key} cannot be empty when using built-in DNS-01"
                ));
            }
        }
    }

    fn apply_auto_https(&mut self) {
        if !self.http.tls.auto_https.enabled {
            return;
        }

        self.http.tls.mode = TlsMode::AcmeManaged;
        self.http.tls.acme.domains = self.http.tls.auto_https.domains.clone();
        self.http.tls.acme.email = self.http.tls.auto_https.email.clone();
        self.http.tls.acme.client = self.http.tls.auto_https.client.clone();
        self.http.tls.acme.challenge = self.http.tls.auto_https.challenge;
        self.http.tls.acme.directory_production = self.http.tls.auto_https.production;
        self.http.tls.acme.renew_interval_hours = self.http.tls.auto_https.renew_interval_hours;
        self.http.tls.acme.extra_args = self.http.tls.auto_https.extra_args.clone();

        if let Some(primary_domain) = self.http.tls.auto_https.domains.first() {
            self.http.tls.server_name = primary_domain.clone();
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            config_version: default_config_version(),
            include: IncludeConfig::default(),
            log_filter: default_log_filter(),
            logging: LoggingConfig::default(),
            http: HttpConfig::default(),
            tcp: TcpConfig::default(),
            udp: UdpConfig::default(),
            script: ScriptConfig::default(),
            plugins: PluginsConfig::default(),
            load_balance: LoadBalanceConfig::default(),
            affinity: AffinityConfig::default(),
            admin: AdminConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            kubernetes: KubernetesConfig::default(),
            runtime: RuntimeConfig::default(),
            services: ServicesConfig::default(),
            root_dir: PathBuf::from("."),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::default(),
            level: LogLevel::default(),
            filter: default_log_filter(),
            access_log: default_true(),
            access_sample_rate: default_sample_rate(),
            slow_request_ms: default_slow_request_ms(),
            redact_headers: default_redact_headers(),
            access_log_path: default_access_log_path(),
            error_log_path: default_error_log_path(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            plain_bind: default_plain_gateway_bind(),
            tls_bind: default_tls_gateway_bind(),
            h3_bind: default_tls_gateway_bind(),
            request_timeout_ms: default_request_timeout_ms(),
            allow_insecure_upstreams: false,
            error_pages: HttpErrorPagesConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

impl Default for HttpErrorPagesConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            show_details: false,
            pages: Vec::new(),
        }
    }
}

impl Default for HttpErrorPageConfig {
    fn default() -> Self {
        Self {
            status: 404,
            body: String::new(),
            file_path: PathBuf::new(),
            content_type: default_error_page_content_type(),
        }
    }
}

impl Default for OnDemandTlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allow: Vec::new(),
            max_active_certs: default_on_demand_max_certs(),
            max_issues_per_hour: default_on_demand_rate_per_hour(),
            ask_url: String::new(),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::default(),
            auto_https: AutoHttpsConfig::default(),
            on_demand: OnDemandTlsConfig::default(),
            certificates: Vec::new(),
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            generate_self_signed_if_missing: default_true(),
            server_name: default_server_name(),
            acme: AcmeExternalConfig::default(),
        }
    }
}

impl Default for AutoHttpsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domains: Vec::new(),
            email: String::new(),
            production: false,
            client: default_acme_client(),
            challenge: AcmeChallengeType::default(),
            renew_interval_hours: default_acme_renew_hours(),
            extra_args: Vec::new(),
        }
    }
}

impl Default for AcmeExternalConfig {
    fn default() -> Self {
        Self {
            client: default_acme_client(),
            email: String::new(),
            domains: Vec::new(),
            cache_dir: default_acme_cache_dir(),
            challenge: AcmeChallengeType::default(),
            directory_production: false,
            renew_interval_hours: default_acme_renew_hours(),
            extra_args: Vec::new(),
            dns: AcmeDnsExternalConfig::default(),
        }
    }
}

impl Default for TlsCertificateConfig {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            cert_path: default_cert_path(),
            key_path: default_key_path(),
        }
    }
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            entry: default_script_entry(),
            cwd: Some(PathBuf::from(".")),
            env: BTreeMap::new(),
            timeout_ms: default_script_timeout_ms(),
            memory_limit_mb: default_script_memory_limit_mb(),
            max_stack_size_kb: default_script_max_stack_size_kb(),
        }
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_load_dir: default_plugins_auto_load_dir(),
            extensions: default_plugin_extensions(),
            allow_admin_manage: default_true(),
        }
    }
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_retries: default_lb_max_retries(),
        }
    }
}

impl Default for PassiveHealthConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            fail_threshold: default_lb_fail_threshold(),
            quarantine_secs: default_lb_quarantine_secs(),
        }
    }
}

impl Default for ActiveHealthConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            http_enabled: default_true(),
            tcp_enabled: default_true(),
            udp_enabled: false,
            interval_secs: default_active_health_interval_secs(),
            timeout_ms: default_active_health_timeout_ms(),
            path: default_active_health_path(),
            expected_statuses: default_active_health_expected_statuses(),
            failure_threshold: default_active_health_failure_threshold(),
            success_threshold: default_active_health_success_threshold(),
            jitter_percent: 0,
            alert_webhooks: Vec::new(),
            udp_payload: default_active_health_udp_payload(),
            udp_expect_response: default_true(),
        }
    }
}

impl Default for AffinityConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            sticky_ttl_secs: default_affinity_ttl(),
            fallback_to_remote_addr: default_true(),
            http: HttpAffinityConfig::default(),
            stream: StreamAffinityConfig::default(),
        }
    }
}

impl Default for HttpAffinityConfig {
    fn default() -> Self {
        Self {
            query_keys: default_http_affinity_query_keys(),
            header_keys: default_http_affinity_header_keys(),
            cookie_keys: default_http_affinity_cookie_keys(),
        }
    }
}

impl Default for StreamAffinityConfig {
    fn default() -> Self {
        Self {
            probe_prefixes: default_stream_affinity_prefixes(),
            probe_delimiters: default_stream_affinity_delimiters(),
            peek_bytes: default_stream_peek_bytes(),
            peek_timeout_ms: default_stream_peek_timeout_ms(),
        }
    }
}

impl Default for AdminAuthRateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_failures: default_admin_auth_max_failures(),
            window_secs: default_admin_auth_window_secs(),
            lockout_secs: default_admin_auth_lockout_secs(),
        }
    }
}

impl Default for AdminHttpsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path_prefix: default_admin_https_path_prefix(),
            hosts: Vec::new(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            bind: default_admin_bind(),
            username: default_admin_username(),
            password: default_admin_password(),
            bearer_token: default_admin_bearer_token(),
            expose_config: false,
            enable_write_ops: false,
            loopback_only: default_true(),
            https: AdminHttpsConfig::default(),
            auth_rate_limit: AdminAuthRateLimitConfig::default(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            validate_admin_mutations: default_true(),
            block_ssrf_targets: default_true(),
            reject_ambiguous_http1: default_true(),
            blocked_upstream_hosts: default_blocked_upstream_hosts(),
            blocked_upstream_cidrs: default_blocked_upstream_cidrs(),
            ddos: DdosProtectionConfig::default(),
            mac_deny: Vec::new(),
            dynamic_blacklist: DynamicBlacklistConfig::default(),
        }
    }
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            namespace: default_k8s_namespace(),
            cluster_domain: default_k8s_cluster_domain(),
            mappings: Vec::new(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            path: default_monitoring_path(),
            format: MonitoringFormat::default(),
        }
    }
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: default_reload_interval_ms(),
        }
    }
}

impl Default for MaintenanceStateConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            path: default_maintenance_state_path(),
        }
    }
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            restart_critical_tasks: default_true(),
            restart_backoff_secs: default_watchdog_restart_backoff_secs(),
            heartbeat_interval_secs: default_watchdog_heartbeat_interval_secs(),
        }
    }
}

impl Default for RuntimePerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            profile: RuntimePerformanceProfile::default(),
            traffic_profile: RuntimePerformanceTrafficProfile::default(),
            adaptive_system: default_true(),
            socket_extreme: default_true(),
            log_on_start: default_true(),
        }
    }
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path_prefix: default_webdav_path_prefix(),
            root: default_webdav_root(),
            allow_write: default_true(),
        }
    }
}

impl Default for FileCloudConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path_prefix: default_filecloud_path_prefix(),
            root: default_filecloud_root(),
            password: String::new(),
            title: default_filecloud_title(),
            allow_upload: default_true(),
            allow_delete: default_true(),
            allow_mkdir: default_true(),
            allow_move: default_true(),
            max_upload_bytes: default_filecloud_max_upload_bytes(),
            cdn_cache_secs: default_filecloud_cdn_cache_secs(),
            session_ttl_secs: default_filecloud_session_ttl_secs(),
            require_auth_for_download: false,
        }
    }
}

impl Default for StaticSiteConfig {
    fn default() -> Self {
        Self {
            name: "public".to_string(),
            path_prefix: default_static_path_prefix(),
            root: default_static_root(),
            index_files: default_static_index_files(),
            autoindex: false,
        }
    }
}

impl Default for ReverseProxyRouteConfig {
    fn default() -> Self {
        Self {
            name: "api".to_string(),
            path_prefix: default_route_path_prefix(),
            hosts: Vec::new(),
            upstream: "http://127.0.0.1:8080".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            forward_headers: true,
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
        }
    }
}

impl Default for DomainRouteConfig {
    fn default() -> Self {
        Self {
            name: "app".to_string(),
            domains: vec!["example.com".to_string()],
            path_prefix: default_route_path_prefix(),
            upstream: "http://127.0.0.1:8080".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
            forward_headers: true,
            compression: ResponseCompressionConfig::default(),
            cache: ResponseCacheConfig::default(),
            rate_limit: HttpRateLimitConfig::default(),
            active_health: ActiveHealthOverrideConfig::default(),
            ssl: DomainTlsConfig::default(),
        }
    }
}

impl Default for ResponseCompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithms: default_compression_algorithms(),
            min_length: default_compression_min_length(),
            content_types: default_compression_types(),
        }
    }
}

impl Default for ResponseCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            behavior: CacheBehavior::default(),
            zone: default_cache_zone(),
            key_prefix: String::new(),
            vary_headers: Vec::new(),
            ttl_secs: default_cache_ttl_secs(),
            browser_ttl_secs: 0,
            stale_while_revalidate_secs: 0,
            stale_if_error_secs: 0,
            emit_cdn_cache_control: false,
            statuses: default_cache_statuses(),
            max_body_bytes: default_cache_max_body_bytes(),
            allow_purge: default_true(),
        }
    }
}

impl Default for DdosProtectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_connections: default_ddos_max_connections(),
            window_secs: default_ddos_window_secs(),
            ban_secs: default_ddos_ban_secs(),
            burst: 0,
        }
    }
}

impl Default for DynamicBlacklistConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: default_dynamic_blacklist_path(),
        }
    }
}

impl Default for StreamRateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            zone: default_rate_limit_zone(),
            algorithm: RateLimitAlgorithm::default(),
            connections: default_rate_limit_requests(),
            window_ms: default_rate_limit_window_ms(),
            burst: 0,
        }
    }
}

impl Default for CacheZoneConfig {
    fn default() -> Self {
        Self {
            name: default_cache_zone(),
            max_entries: default_cache_zone_max_entries(),
            disk_path: PathBuf::new(),
        }
    }
}

impl Default for DomainTlsConfig {
    fn default() -> Self {
        Self {
            mode: DomainTlsMode::default(),
            is_auto_ssl: false,
            cert_path: PathBuf::new(),
            key_path: PathBuf::new(),
            email: String::new(),
        }
    }
}

impl DomainTlsConfig {
    pub fn effective_mode(&self) -> DomainTlsMode {
        if self.is_auto_ssl {
            DomainTlsMode::Auto
        } else {
            self.mode
        }
    }
}

impl Default for HttpRateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            zone: default_rate_limit_zone(),
            algorithm: RateLimitAlgorithm::default(),
            key: RateLimitKey::default(),
            requests: default_rate_limit_requests(),
            window_ms: default_rate_limit_window_ms(),
            burst: 0,
            max_connections: 0,
            status: default_rate_limit_status(),
        }
    }
}

impl Default for HttpAccessControlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allow: Vec::new(),
            deny: Vec::new(),
            status: default_access_control_status(),
        }
    }
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_ftp_bind(),
            upstream: default_ftp_upstream(),
            native_control: default_true(),
            public_ip: String::new(),
            passive_port_start: default_ftp_passive_port_start(),
            passive_port_end: default_ftp_passive_port_end(),
            passive_hint: default_true(),
            allow: Vec::new(),
            deny: Vec::new(),
            command_allow: Vec::new(),
            command_deny: Vec::new(),
            transfer_allow: Vec::new(),
            transfer_deny: Vec::new(),
            user_policies: Vec::new(),
            proxy_timeout_ms: default_ftp_proxy_timeout_ms(),
            max_login_attempts: 0,
            limit_rate: 0,
            log_commands: default_true(),
            log_transfers: default_true(),
        }
    }
}

fn legacy_config_issues(value: &serde_yaml::Value) -> Vec<String> {
    let mut issues = Vec::new();

    if let Some(include) = value.get("include") {
        let enabled = include
            .get("enabled")
            .and_then(serde_yaml::Value::as_bool)
            .unwrap_or(false);
        let files = include
            .get("files")
            .and_then(serde_yaml::Value::as_sequence)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_yaml::Value::as_str)
                    .filter(|path| !path.is_empty())
                    .count()
            })
            .unwrap_or(0);
        if enabled || files > 0 {
            issues.push(
                "include is removed in v1.0: merge every referenced file into proxysss.yaml and delete the include block"
                    .to_string(),
            );
        }
    }

    if let Some(script) = value.get("script") {
        if script.get("command").is_some() {
            issues.push(
                "script.command (external Deno/Node runtime) is removed: use the embedded engine with script.enabled=true and script.entry=gateway.ts; run `proxysss init --overwrite` to regenerate"
                    .to_string(),
            );
        }
        if script.get("args").is_some() && script.get("entry").is_none() {
            issues.push(
                "script.args without script.entry indicates a legacy external runtime: switch to script.entry=gateway.ts and remove script.command/script.args"
                    .to_string(),
            );
        }
    }

    issues
}

fn load_config_value(path: &Path) -> Result<serde_yaml::Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let base = parse_value_by_extension(&raw, path)?;
    let legacy_issues = legacy_config_issues(&base);
    if !legacy_issues.is_empty() {
        return Err(anyhow!(
            "legacy config in {} is incompatible with proxysss v1.0:\n - {}",
            path.display(),
            legacy_issues.join("\n - ")
        ));
    }
    Ok(base)
}

fn parse_value_by_extension(raw: &str, path: &Path) -> Result<serde_yaml::Value> {
    let raw = raw.trim_start_matches('\u{feff}');
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match ext.as_deref() {
        Some("json") => Err(anyhow!(
            "JSON config files are unsupported; use YAML and keep the default name proxysss.yaml or pass -config/--config/-c"
        )),
        _ => serde_yaml::from_str(raw)
            .with_context(|| format!("failed to parse yaml config {}", path.display())),
    }
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    normalize_path_lexically(&path)
}

fn absolutize_if_not_empty(root: &Path, path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::new()
    } else {
        absolutize(root, path)
    }
}

fn normalize_root_dir(root_dir: &Path) -> PathBuf {
    let path = if root_dir.is_absolute() {
        root_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root_dir)
    };
    normalize_path_lexically(&path)
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn normalize_vec_lowercase(values: &mut [String]) {
    for value in values {
        *value = value.to_ascii_lowercase();
    }
}

fn validate_bind_optional(label: &str, bind: &str, errors: &mut Vec<String>) {
    if bind.trim().is_empty() {
        return;
    }
    if bind.parse::<SocketAddr>().is_err() {
        errors.push(format!("{label} is not a valid socket address: {bind}"));
    }
}

fn validate_bind_required(label: &str, bind: &str, errors: &mut Vec<String>) {
    if bind.trim().is_empty() {
        errors.push(format!("{label} cannot be empty"));
        return;
    }
    validate_bind_optional(label, bind, errors);
}

fn default_config_version() -> u32 {
    1
}

fn default_log_filter() -> String {
    "info,proxysss=info".to_string()
}

fn default_filter_for_level(level: LogLevel) -> String {
    match level {
        LogLevel::Debug => "debug,proxysss=debug".to_string(),
        LogLevel::Info => "info,proxysss=info".to_string(),
        LogLevel::Warn => "warn,proxysss=warn".to_string(),
        LogLevel::Error => "error,proxysss=error".to_string(),
    }
}

fn default_plain_gateway_bind() -> String {
    format!("0.0.0.0:{DEFAULT_HTTP_PORT}")
}

fn default_tls_gateway_bind() -> String {
    format!("0.0.0.0:{DEFAULT_HTTPS_PORT}")
}

fn default_request_timeout_ms() -> u64 {
    15_000
}

fn default_true() -> bool {
    true
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_slow_request_ms() -> u64 {
    300
}

fn default_redact_headers() -> Vec<String> {
    vec![
        "authorization".to_string(),
        "cookie".to_string(),
        "set-cookie".to_string(),
    ]
}

fn default_access_log_path() -> PathBuf {
    PathBuf::from("logs/access.log")
}

fn default_error_log_path() -> PathBuf {
    PathBuf::from("logs/error.log")
}

fn default_cert_path() -> PathBuf {
    PathBuf::from("certs/proxysss-cert.pem")
}

fn default_key_path() -> PathBuf {
    PathBuf::from("certs/proxysss-key.pem")
}

fn default_server_name() -> String {
    "gateway.local".to_string()
}

fn default_acme_client() -> String {
    "acme.sh".to_string()
}

fn default_acme_cache_dir() -> PathBuf {
    PathBuf::from("certs/acme-cache")
}

fn default_acme_renew_hours() -> u64 {
    12
}

fn default_on_demand_max_certs() -> usize {
    100
}

fn default_on_demand_rate_per_hour() -> u32 {
    30
}

fn default_ddos_max_connections() -> u32 {
    50
}

fn default_ddos_window_secs() -> u64 {
    10
}

fn default_ddos_ban_secs() -> u64 {
    300
}

fn default_dynamic_blacklist_path() -> PathBuf {
    PathBuf::from("runtime/dynamic-blacklist.json")
}

fn default_tcp_listeners() -> Vec<TcpListenerConfig> {
    Vec::new()
}

fn default_udp_listeners() -> Vec<UdpListenerConfig> {
    Vec::new()
}

fn default_stream_connect_timeout_ms() -> u64 {
    3_000
}

fn default_udp_session_ttl_secs() -> u64 {
    180
}

fn default_udp_max_associations() -> usize {
    262_144
}

fn default_script_entry() -> PathBuf {
    PathBuf::from(DEFAULT_SCRIPT_FILE_NAME)
}

fn default_script_timeout_ms() -> u64 {
    500
}

fn default_script_memory_limit_mb() -> u64 {
    64
}

fn default_script_max_stack_size_kb() -> u64 {
    512
}

fn default_plugins_auto_load_dir() -> PathBuf {
    PathBuf::from("plugins")
}

fn default_plugin_extensions() -> Vec<String> {
    vec![
        "ts".to_string(),
        "js".to_string(),
        "mjs".to_string(),
        "cjs".to_string(),
    ]
}

fn default_affinity_ttl() -> u64 {
    3600
}

fn default_http_affinity_query_keys() -> Vec<String> {
    vec!["playerId".to_string(), "pid".to_string(), "uid".to_string()]
}

fn default_http_affinity_header_keys() -> Vec<String> {
    vec!["x-player-id".to_string(), "x-uid".to_string()]
}

fn default_http_affinity_cookie_keys() -> Vec<String> {
    vec!["playerId".to_string(), "pid".to_string()]
}

fn default_stream_affinity_prefixes() -> Vec<String> {
    vec![
        "playerId=".to_string(),
        "pid=".to_string(),
        "uid=".to_string(),
    ]
}

fn default_stream_affinity_delimiters() -> Vec<String> {
    vec![
        "|".to_string(),
        ";".to_string(),
        ",".to_string(),
        "\n".to_string(),
        "\r".to_string(),
        " ".to_string(),
    ]
}

fn default_stream_peek_bytes() -> usize {
    256
}

fn default_stream_peek_timeout_ms() -> u64 {
    5
}

fn default_admin_bind() -> String {
    "127.0.0.1:7777".to_string()
}

fn default_admin_https_path_prefix() -> String {
    "/_proxysss/admin".to_string()
}

pub fn normalize_admin_https_path_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return default_admin_https_path_prefix();
    }
    let with_slash = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    with_slash.trim_end_matches('/').to_string()
}

fn default_admin_auth_max_failures() -> u32 {
    8
}

fn default_admin_auth_window_secs() -> u64 {
    300
}

fn default_admin_auth_lockout_secs() -> u64 {
    900
}

fn default_blocked_upstream_hosts() -> Vec<String> {
    vec![
        "metadata.google.internal".to_string(),
        "169.254.169.254".to_string(),
    ]
}

fn default_blocked_upstream_cidrs() -> Vec<String> {
    vec!["169.254.169.254/32".to_string()]
}

fn default_k8s_namespace() -> String {
    "default".to_string()
}

fn default_k8s_cluster_domain() -> String {
    "cluster.local".to_string()
}

fn default_k8s_service_port() -> u16 {
    80
}

fn default_monitoring_path() -> String {
    "/metrics".to_string()
}

fn default_admin_username() -> String {
    DEFAULT_ADMIN_USERNAME.to_string()
}

fn default_admin_password() -> String {
    DEFAULT_ADMIN_PASSWORD.to_string()
}

fn default_admin_bearer_token() -> String {
    DEFAULT_ADMIN_BEARER_TOKEN.to_string()
}

fn skip_default_admin_bearer_token(token: &String) -> bool {
    token == DEFAULT_ADMIN_BEARER_TOKEN
}

fn default_lb_max_retries() -> u32 {
    2
}

fn default_lb_fail_threshold() -> u32 {
    3
}

fn default_lb_quarantine_secs() -> u64 {
    15
}

fn default_maintenance_state_path() -> PathBuf {
    PathBuf::from("runtime/maintenance-state.json")
}

fn default_active_health_interval_secs() -> u64 {
    10
}

fn default_active_health_timeout_ms() -> u64 {
    2000
}

fn default_active_health_path() -> String {
    "/healthz".to_string()
}

fn default_error_page_content_type() -> String {
    "text/html; charset=utf-8".to_string()
}

fn default_active_health_expected_statuses() -> Vec<u16> {
    vec![200, 204]
}

fn default_active_health_failure_threshold() -> u32 {
    2
}

fn default_active_health_success_threshold() -> u32 {
    2
}

fn default_active_health_udp_payload() -> String {
    "proxysss-health".to_string()
}

fn default_watchdog_restart_backoff_secs() -> u64 {
    2
}

fn default_watchdog_heartbeat_interval_secs() -> u64 {
    30
}

fn normalize_active_health_override(config: &mut ActiveHealthOverrideConfig) {
    if let Some(path) = &mut config.path {
        *path = path.trim().to_string();
    }
    config
        .alert_webhooks
        .retain(|value| !value.trim().is_empty());
}

fn default_reload_interval_ms() -> u64 {
    1500
}

fn default_webdav_path_prefix() -> String {
    "/dav".to_string()
}

fn default_webdav_root() -> PathBuf {
    PathBuf::from("webdav")
}

fn default_filecloud_path_prefix() -> String {
    "/filecloud".to_string()
}

fn default_filecloud_root() -> PathBuf {
    PathBuf::from("filecloud-data")
}

fn default_filecloud_title() -> String {
    "FileCloud".to_string()
}

fn default_filecloud_max_upload_bytes() -> u64 {
    512 * 1024 * 1024
}

fn default_filecloud_cdn_cache_secs() -> u64 {
    86_400
}

fn default_filecloud_session_ttl_secs() -> u64 {
    86_400 * 7
}

fn default_static_path_prefix() -> String {
    "/public".to_string()
}

fn default_static_root() -> PathBuf {
    PathBuf::from("public")
}

fn default_static_index_files() -> Vec<String> {
    vec!["index.html".to_string(), "index.htm".to_string()]
}

fn default_route_path_prefix() -> String {
    "/".to_string()
}

fn default_compression_min_length() -> usize {
    1024
}

fn default_compression_algorithms() -> Vec<CompressionAlgorithm> {
    vec![
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::Brotli,
        CompressionAlgorithm::Gzip,
    ]
}

fn default_compression_types() -> Vec<String> {
    vec![
        "text/".to_string(),
        "application/json".to_string(),
        "application/javascript".to_string(),
        "application/xml".to_string(),
        "image/svg+xml".to_string(),
    ]
}

/// Normalize `6379` → `0.0.0.0:6379` for stream route listen values.
pub fn normalize_stream_listen(listen: &str) -> String {
    let trimmed = listen.trim();
    if trimmed.contains(':') {
        trimmed.to_string()
    } else if let Ok(port) = trimmed.parse::<u16>() {
        format!("0.0.0.0:{port}")
    } else {
        trimmed.to_string()
    }
}

/// Glob-style domain match (`*.example.com`, exact hostnames).
pub fn domain_matches_pattern(domain: &str, pattern: &str) -> bool {
    let domain = domain.trim().to_ascii_lowercase();
    let pattern = pattern.trim().to_ascii_lowercase();
    if pattern.is_empty() {
        return false;
    }
    if pattern == domain {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return domain.ends_with(suffix) && domain.len() > suffix.len();
    }
    false
}

pub fn on_demand_domain_allowed(config: &OnDemandTlsConfig, domain: &str) -> bool {
    if !config.enabled {
        return false;
    }
    if config.allow.is_empty() {
        return false;
    }
    config
        .allow
        .iter()
        .any(|pattern| domain_matches_pattern(domain, pattern))
}

fn default_cache_ttl_secs() -> u64 {
    30
}

fn default_cache_zone() -> String {
    "default".to_string()
}

fn default_cache_zone_max_entries() -> usize {
    4096
}

fn default_cache_statuses() -> Vec<u16> {
    vec![200, 203, 204, 301, 302, 404]
}

fn default_cache_max_body_bytes() -> usize {
    2 * 1024 * 1024
}

fn normalize_compression_algorithms(
    algorithms: &[CompressionAlgorithm],
) -> Vec<CompressionAlgorithm> {
    let mut normalized = Vec::new();
    for algorithm in algorithms {
        if !normalized.contains(algorithm) {
            normalized.push(*algorithm);
        }
    }
    if normalized.is_empty() {
        default_compression_algorithms()
    } else {
        normalized
    }
}

fn validate_compression_config(
    config: &ResponseCompressionConfig,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if !config.enabled {
        return;
    }
    if config.algorithms.is_empty() {
        errors.push(format!("{prefix}.algorithms cannot be empty"));
    }
    if config.min_length == 0 {
        errors.push(format!("{prefix}.min_length must be greater than 0"));
    }
    if config.content_types.is_empty() {
        errors.push(format!("{prefix}.content_types cannot be empty"));
    }
}

fn validate_cache_config(config: &ResponseCacheConfig, prefix: &str, errors: &mut Vec<String>) {
    if !config.enabled {
        return;
    }
    if config.zone.trim().is_empty() {
        errors.push(format!("{prefix}.zone cannot be empty"));
    }
    if config.ttl_secs == 0 {
        errors.push(format!("{prefix}.ttl_secs must be greater than 0"));
    }
    if config.statuses.is_empty() {
        errors.push(format!("{prefix}.statuses cannot be empty"));
    }
    if config.max_body_bytes == 0 {
        errors.push(format!("{prefix}.max_body_bytes must be greater than 0"));
    }
}

fn validate_cache_zone_reference(
    config: &ResponseCacheConfig,
    zones: &[CacheZoneConfig],
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if !config.enabled || config.zone == default_cache_zone() {
        return;
    }
    if !zones.iter().any(|zone| zone.name == config.zone) {
        errors.push(format!(
            "{prefix}.zone references undefined cache zone {}",
            config.zone
        ));
    }
}

fn validate_http_rate_limit_config(
    config: &HttpRateLimitConfig,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if !config.enabled {
        return;
    }
    if config.zone.trim().is_empty() {
        errors.push(format!("{prefix}.zone cannot be empty"));
    }
    if config.requests == 0 {
        errors.push(format!("{prefix}.requests must be greater than 0"));
    }
    if config.window_ms < 100 {
        errors.push(format!("{prefix}.window_ms must be >= 100"));
    }
    if !(100..=599).contains(&config.status) {
        errors.push(format!("{prefix}.status must be a valid HTTP status"));
    }
    if let RateLimitKey::Header(name) = &config.key {
        if name.trim().is_empty() {
            errors.push(format!("{prefix}.key header name cannot be empty"));
        }
    }
}

fn validate_stream_rate_limit_config(
    config: &StreamRateLimitConfig,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if !config.enabled {
        return;
    }
    if config.zone.trim().is_empty() {
        errors.push(format!("{prefix}.zone cannot be empty"));
    }
    if config.connections == 0 {
        errors.push(format!("{prefix}.connections must be greater than 0"));
    }
    if config.window_ms < 100 {
        errors.push(format!("{prefix}.window_ms must be >= 100"));
    }
}

/// Validate only the control-plane contract for service discovery.
///
/// This deliberately checks registry/mapping shape and references, not live
/// registry reachability. A config check must stay offline-friendly and safe in
/// Docker/CI where Consul, etcd, or Nacos may not be running.
fn validate_service_discovery_config(config: &ServiceDiscoveryConfig, errors: &mut Vec<String>) {
    if config.interval_secs == 0 {
        errors.push("services.service_discovery.interval_secs must be greater than 0".to_string());
    }
    if config.registries.is_empty() {
        errors
            .push("services.service_discovery.registries cannot be empty when enabled".to_string());
    }
    if config.mappings.is_empty() {
        errors.push("services.service_discovery.mappings cannot be empty when enabled".to_string());
    }

    let mut registry_names = HashSet::<String>::new();
    for (index, registry) in config.registries.iter().enumerate() {
        let name = registry.name.trim();
        if name.is_empty() {
            errors.push(format!(
                "services.service_discovery.registries.{index}.name cannot be empty"
            ));
        } else if !registry_names.insert(name.to_string()) {
            errors.push(format!(
                "services.service_discovery.registries.{index}.name duplicates registry {name}"
            ));
        }
        if !looks_like_url(&registry.endpoint) {
            errors.push(format!(
                "services.service_discovery.registries.{index}.endpoint must be an http/https URL"
            ));
        }
    }

    for (index, mapping) in config.mappings.iter().enumerate() {
        if mapping.name.trim().is_empty() {
            errors.push(format!(
                "services.service_discovery.mappings.{index}.name cannot be empty"
            ));
        }
        if mapping.registry.trim().is_empty() {
            errors.push(format!(
                "services.service_discovery.mappings.{index}.registry cannot be empty"
            ));
        } else if !registry_names.contains(mapping.registry.trim()) {
            errors.push(format!(
                "services.service_discovery.mappings.{index}.registry references unknown registry {}",
                mapping.registry
            ));
        }
        if mapping.service.trim().is_empty() {
            errors.push(format!(
                "services.service_discovery.mappings.{index}.service cannot be empty"
            ));
        }
        if mapping.target_name.trim().is_empty() {
            errors.push(format!(
                "services.service_discovery.mappings.{index}.target_name cannot be empty"
            ));
        }
    }
}

fn validate_active_health_override(
    config: &ActiveHealthOverrideConfig,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if let Some(path) = &config.path {
        if !path.starts_with('/') {
            errors.push(format!("{prefix}.path must start with /"));
        }
    }
    if let Some(timeout_ms) = config.timeout_ms {
        if timeout_ms < 100 {
            errors.push(format!("{prefix}.timeout_ms must be >= 100"));
        }
    }
    if let Some(statuses) = &config.expected_statuses {
        if statuses.is_empty() {
            errors.push(format!("{prefix}.expected_statuses cannot be empty"));
        }
    }
    if let Some(value) = config.failure_threshold {
        if value == 0 {
            errors.push(format!("{prefix}.failure_threshold must be greater than 0"));
        }
    }
    if let Some(value) = config.success_threshold {
        if value == 0 {
            errors.push(format!("{prefix}.success_threshold must be greater than 0"));
        }
    }
    if let Some(value) = config.jitter_percent {
        if value > 100 {
            errors.push(format!("{prefix}.jitter_percent must be <= 100"));
        }
    }
    for (index, webhook) in config.alert_webhooks.iter().enumerate() {
        if !looks_like_url(webhook) {
            errors.push(format!(
                "{prefix}.alert_webhooks.{index} must be an http/https URL"
            ));
        }
    }
}

fn looks_like_url(value: &str) -> bool {
    matches!(value.trim(), v if v.starts_with("http://") || v.starts_with("https://"))
}

fn default_rate_limit_requests() -> u32 {
    60
}

fn default_service_discovery_interval_secs() -> u64 {
    15
}

fn default_rate_limit_zone() -> String {
    "default".to_string()
}

fn default_rate_limit_window_ms() -> u64 {
    60_000
}

fn default_rate_limit_status() -> u16 {
    429
}

fn default_access_control_status() -> u16 {
    403
}

fn is_valid_ip_rule(value: &str) -> bool {
    parse_ip_rule(value).is_some()
}

fn parse_ip_rule(value: &str) -> Option<(IpAddr, u8)> {
    let value = value.trim();
    let (ip, prefix) = match value.split_once('/') {
        Some((addr, prefix)) => {
            let ip = addr.trim().parse::<IpAddr>().ok()?;
            let prefix = prefix.trim().parse::<u8>().ok()?;
            (ip, prefix)
        }
        None => {
            let ip = value.parse::<IpAddr>().ok()?;
            let prefix = match ip {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            (ip, prefix)
        }
    };

    match ip {
        IpAddr::V4(_) if prefix <= 32 => Some((ip, prefix)),
        IpAddr::V6(_) if prefix <= 128 => Some((ip, prefix)),
        _ => None,
    }
}

fn default_ftp_bind() -> String {
    "0.0.0.0:21".to_string()
}

fn default_ftp_upstream() -> String {
    "127.0.0.1:2121".to_string()
}

fn default_ftp_passive_port_start() -> u16 {
    50_000
}

fn default_ftp_passive_port_end() -> u16 {
    50_100
}

fn default_ftp_proxy_timeout_ms() -> u64 {
    60_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_admin_credentials_are_root_root() {
        let config = GatewayConfig::default();
        assert_eq!(config.admin.username, DEFAULT_ADMIN_USERNAME);
        assert_eq!(config.admin.password, DEFAULT_ADMIN_PASSWORD);
        assert_eq!(config.admin.bearer_token, DEFAULT_ADMIN_BEARER_TOKEN);
    }

    #[test]
    fn default_yaml_does_not_expose_default_bearer_token() {
        let yaml = GatewayConfig::render_default_yaml();
        assert!(!yaml.contains("bearer_token"));
        assert!(!yaml.contains(DEFAULT_ADMIN_BEARER_TOKEN));
    }

    #[test]
    fn warnings_include_default_admin_credentials() {
        let config = GatewayConfig::default();
        let warnings = config.warnings();
        assert!(warnings
            .iter()
            .any(|item| item.contains("admin credentials are still default")));
    }

    #[test]
    fn warnings_include_disabled_plain_bind() {
        let mut config = GatewayConfig::default();
        config.http.plain_bind.clear();
        let warnings = config.warnings();
        assert!(warnings
            .iter()
            .any(|item| item.contains("http.plain_bind is disabled")));
    }

    #[test]
    fn load_balance_defaults_are_applied() {
        let config = GatewayConfig::default();
        assert!(matches!(
            config.load_balance.algorithm,
            LoadBalanceAlgorithm::Rendezvous
        ));
        assert_eq!(config.load_balance.retries.max_retries, 2);
        assert!(config.load_balance.active_health.enabled);
        assert!(config.load_balance.active_health.http_enabled);
        assert!(config.load_balance.active_health.tcp_enabled);
        assert!(!config.load_balance.active_health.udp_enabled);
        assert_eq!(config.load_balance.active_health.interval_secs, 10);
        assert_eq!(config.load_balance.active_health.timeout_ms, 2000);
        assert_eq!(config.load_balance.active_health.path, "/healthz");
        assert_eq!(
            config.load_balance.active_health.udp_payload,
            "proxysss-health"
        );
        assert!(config.load_balance.active_health.udp_expect_response);
        assert_eq!(config.load_balance.passive_health.fail_threshold, 3);
        assert_eq!(config.load_balance.passive_health.quarantine_secs, 15);
        assert!(config.runtime.watchdog.enabled);
        assert!(config.runtime.watchdog.restart_critical_tasks);
        assert_eq!(config.runtime.watchdog.restart_backoff_secs, 2);
        assert_eq!(config.runtime.watchdog.heartbeat_interval_secs, 30);
        assert!(!config.runtime.hot_reload.enabled);
        assert_eq!(config.runtime.hot_reload.interval_ms, 1500);
        assert!(config.runtime.performance.enabled);
        assert!(config.runtime.performance.adaptive_system);
        assert!(config.runtime.performance.socket_extreme);
        assert!(config.runtime.performance.log_on_start);
        assert_eq!(
            config.runtime.performance.profile,
            RuntimePerformanceProfile::Edge
        );
        assert_eq!(
            config.runtime.performance.traffic_profile,
            RuntimePerformanceTrafficProfile::Small
        );
    }

    #[test]
    fn default_yaml_exposes_adaptive_runtime_performance() {
        let yaml = GatewayConfig::render_default_yaml();
        assert!(yaml.contains("hot_reload:"));
        assert!(yaml.contains("enabled: false"));
        assert!(yaml.contains("performance:"));
        assert!(yaml.contains("traffic_profile: small"));
        assert!(yaml.contains("adaptive_system: true"));
        assert!(yaml.contains("socket_extreme: true"));
    }

    #[test]
    fn validate_rejects_active_health_without_any_probe_kind() {
        let mut config = GatewayConfig::default();
        config.load_balance.active_health.enabled = true;
        config.load_balance.active_health.http_enabled = false;
        config.load_balance.active_health.tcp_enabled = false;
        config.load_balance.active_health.udp_enabled = false;

        let error = config
            .validate()
            .expect_err("expected invalid active health protocol selection");
        assert!(error.to_string().contains(
            "load_balance.active_health requires http_enabled, tcp_enabled, or udp_enabled"
        ));
    }

    #[test]
    fn default_ports_match_nginx_takeover_goals() {
        let config = GatewayConfig::default();
        assert_eq!(config.http.plain_bind, "0.0.0.0:80");
        assert_eq!(config.http.tls_bind, "0.0.0.0:443");
        assert_eq!(config.http.h3_bind, "0.0.0.0:443");
        assert_eq!(config.admin.bind, "127.0.0.1:7777");
        assert!(config.tcp.listeners.is_empty());
        assert!(config.udp.listeners.is_empty());
    }

    #[test]
    fn default_logging_level_is_info() {
        let config = GatewayConfig::default();
        assert_eq!(config.logging.level, LogLevel::Info);
        assert_eq!(config.logging.filter, "info,proxysss=info");
    }

    #[test]
    fn default_log_paths_match_nginx_style_layout() {
        let config = GatewayConfig::default();
        assert_eq!(
            config.logging.access_log_path,
            PathBuf::from("logs/access.log")
        );
        assert_eq!(
            config.logging.error_log_path,
            PathBuf::from("logs/error.log")
        );
    }

    #[test]
    fn logging_level_derives_filter_when_filter_is_omitted() {
        let base_dir =
            std::env::temp_dir().join(format!("proxysss-log-level-test-{}", std::process::id()));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "logging:\n  level: warn\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(config.logging.level, LogLevel::Warn);
        assert_eq!(config.logging.filter, "warn,proxysss=warn");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn config_loader_accepts_utf8_bom_yaml() {
        let base_dir =
            std::env::temp_dir().join(format!("proxysss-bom-yaml-test-{}", std::process::id()));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "\u{feff}plugins:\n  enabled: false\nadmin:\n  bind: 127.0.0.1:17777\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load bom config");
        assert_eq!(config.admin.bind, "127.0.0.1:17777");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn auto_https_expands_to_managed_acme_config() {
        let base_dir =
            std::env::temp_dir().join(format!("proxysss-auto-https-test-{}", std::process::id()));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "http:\n  tls:\n    auto_https:\n      enabled: true\n      domains: [example.com, www.example.com]\n      email: admin@example.com\n      production: true\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(config.http.tls.mode, TlsMode::AcmeManaged);
        assert_eq!(config.http.tls.server_name, "example.com");
        assert_eq!(
            config.http.tls.acme.domains,
            vec!["example.com".to_string(), "www.example.com".to_string()]
        );
        assert_eq!(config.http.tls.acme.email, "admin@example.com");
        assert!(config.http.tls.acme.directory_production);

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn auto_https_requires_domains_and_email() {
        let mut config = GatewayConfig::default();
        config.http.tls.auto_https.enabled = true;
        config.apply_auto_https();

        let error = config.validate().expect_err("expected invalid auto_https");
        let message = error.to_string();
        assert!(message.contains("http.tls.auto_https.domains"));
        assert!(message.contains("http.tls.auto_https.email"));
    }

    #[test]
    fn domain_auto_ssl_expands_into_global_auto_https() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-domain-auto-ssl-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "services:\n  domain_routes:\n    - name: app\n      domains: [example.com, www.example.com]\n      path_prefix: /\n      upstream: http://127.0.0.1:9000\n      ssl:\n        type: auto\n        email: admin@example.com\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert!(config.http.tls.auto_https.enabled);
        assert_eq!(config.http.tls.mode, TlsMode::AcmeManaged);
        assert_eq!(config.http.tls.auto_https.email, "admin@example.com");
        assert!(config
            .http
            .tls
            .auto_https
            .domains
            .contains(&"example.com".to_string()));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn acme_dns_external_requires_dns_provider_credentials_and_wildcard_domain() {
        let mut config = GatewayConfig::default();
        config.http.tls.mode = TlsMode::AcmeDnsExternal;
        config.http.tls.acme.email = "admin@example.com".to_string();
        config.http.tls.acme.domains = vec!["example.com".to_string()];

        let error = config
            .validate()
            .expect_err("expected invalid acme dns config");
        let message = error.to_string();
        assert!(message.contains("http.tls.acme.domains must include at least one wildcard"));
        assert!(message.contains("http.tls.acme.dns.provider"));
        assert!(message.contains("http.tls.acme.dns.credentials"));
    }

    #[test]
    fn acme_managed_dns01_accepts_builtin_provider_credentials() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-acme-managed-dns01-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "http:\n  tls:\n    mode: acme_managed\n    generate_self_signed_if_missing: false\n    acme:\n      email: admin@example.com\n      challenge: dns01\n      domains: [example.com, '*.example.com']\n      dns:\n        provider: aliyun_cn\n        credentials:\n          access_key_id: key\n          access_key_secret: secret\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(config.http.tls.mode, TlsMode::AcmeManaged);
        assert_eq!(config.http.tls.acme.challenge, AcmeChallengeType::Dns01);
        assert_eq!(config.http.tls.acme.dns.provider, "aliyun_cn");
        assert!(config.uses_managed_dns01());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn acme_dns_external_accepts_acme_sh_dns_provider_credentials() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-acme-dns-external-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "http:\n  tls:\n    mode: acme_dns_external\n    generate_self_signed_if_missing: false\n    acme:\n      client: acme.sh\n      email: admin@example.com\n      domains: [example.com, '*.example.com']\n      dns:\n        provider: dns_cf\n        credentials:\n          CF_Token: secret-token\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(config.http.tls.mode, TlsMode::AcmeManaged);
        assert_eq!(config.http.tls.acme.challenge, AcmeChallengeType::Dns01);
        assert_eq!(config.http.tls.acme.dns.provider, "cloudflare");
        assert_eq!(
            config
                .http
                .tls
                .acme
                .dns
                .credentials
                .get("CF_Token")
                .map(String::as_str),
            Some("secret-token")
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn domain_manual_ssl_paths_are_absolutized_into_sni_certificates() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-domain-manual-ssl-test-{}",
            std::process::id()
        ));
        let cert_dir = base_dir.join("certs");
        fs::create_dir_all(&cert_dir).expect("create temp config dir");
        fs::write(cert_dir.join("edge.pem"), "test").expect("write cert");
        fs::write(cert_dir.join("edge.key"), "test").expect("write key");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "services:\n  domain_routes:\n    - name: edge\n      domains: [edge.example.com]\n      path_prefix: /\n      upstream: http://127.0.0.1:9000\n      ssl:\n        type: manual\n        cert_path: ./certs/edge.pem\n        key_path: ./certs/edge.key\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(config.http.tls.certificates.len(), 1);
        assert!(config.http.tls.certificates[0].cert_path.is_absolute());
        assert!(config.http.tls.certificates[0].key_path.is_absolute());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn access_control_accepts_blacklist_alias_and_cidr_rules() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-access-control-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "services:\n  access_control:\n    http:\n      enabled: true\n      blacklist: [203.0.113.0/24, 2001:db8::/32]\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert_eq!(
            config.services.access_control.http.deny,
            vec!["203.0.113.0/24".to_string(), "2001:db8::/32".to_string()]
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn service_discovery_accepts_registry_mappings() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-service-discovery-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.yaml");
        fs::write(
            &config_path,
            "services:\n  service_discovery:\n    enabled: true\n    interval_secs: 15\n    registries:\n      - name: consul-main\n        provider: consul\n        endpoint: http://127.0.0.1:8500\n      - name: nacos-main\n        provider: nacos\n        endpoint: http://127.0.0.1:8848\n      - name: etcd-main\n        provider: etcd\n        endpoint: http://127.0.0.1:2379\n    mappings:\n      - name: api-from-consul\n        registry: consul-main\n        service: billing-api\n        target: reverse_proxy_route\n        target_name: api\n        scheme: http\nplugins:\n  enabled: false\n",
        )
        .expect("write config");

        let config = GatewayConfig::load(&config_path).expect("load config");
        assert!(config.services.service_discovery.enabled);
        assert_eq!(config.services.service_discovery.registries.len(), 3);
        assert_eq!(
            config.services.service_discovery.mappings[0].target,
            ServiceDiscoveryTarget::ReverseProxyRoute
        );

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn validate_rejects_unknown_service_discovery_registry() {
        let mut config = GatewayConfig::default();
        config.services.service_discovery.enabled = true;
        config
            .services
            .service_discovery
            .registries
            .push(ServiceRegistryConfig {
                name: "consul-main".to_string(),
                provider: ServiceRegistryProvider::Consul,
                endpoint: "http://127.0.0.1:8500".to_string(),
                namespace: String::new(),
                group: String::new(),
                token: String::new(),
                username: String::new(),
                password: String::new(),
            });
        config
            .services
            .service_discovery
            .mappings
            .push(ServiceDiscoveryMappingConfig {
                name: "bad".to_string(),
                registry: "missing".to_string(),
                service: "billing-api".to_string(),
                target: ServiceDiscoveryTarget::ReverseProxyRoute,
                target_name: "api".to_string(),
                scheme: "http".to_string(),
                port_name: String::new(),
                metadata_filter: BTreeMap::new(),
            });

        let error = config
            .validate()
            .expect_err("expected unknown registry validation error");
        assert!(error
            .to_string()
            .contains("services.service_discovery.mappings.0.registry"));
    }

    #[test]
    fn validate_rejects_invalid_access_control_rule() {
        let mut config = GatewayConfig::default();
        config.services.access_control.http.enabled = true;
        config
            .services
            .access_control
            .http
            .deny
            .push("203.0.113.1/99".to_string());

        let error = config
            .validate()
            .expect_err("expected invalid access control rule");
        assert!(error
            .to_string()
            .contains("services.access_control.http.deny.0"));
    }

    #[test]
    fn validate_rejects_static_index_paths() {
        let mut config = GatewayConfig::default();
        config.services.static_sites.push(StaticSiteConfig {
            name: "bad".to_string(),
            path_prefix: "/bad".to_string(),
            root: PathBuf::from("public"),
            index_files: vec!["../index.html".to_string()],
            autoindex: false,
        });

        let error = config
            .validate()
            .expect_err("expected invalid static index");
        assert!(error.to_string().contains("index_files"));
    }

    #[test]
    fn validate_rejects_reverse_proxy_route_without_upstream() {
        let mut config = GatewayConfig::default();
        config
            .services
            .reverse_proxy
            .routes
            .push(ReverseProxyRouteConfig {
                name: "api".to_string(),
                path_prefix: "/api".to_string(),
                hosts: Vec::new(),
                upstream: String::new(),
                upstreams: Vec::new(),
                upstream_weights: BTreeMap::new(),
                strip_prefix: false,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                forward_headers: true,
                compression: ResponseCompressionConfig::default(),
                cache: ResponseCacheConfig::default(),
                rate_limit: HttpRateLimitConfig::default(),
                active_health: ActiveHealthOverrideConfig::default(),
            });

        let error = config
            .validate()
            .expect_err("expected invalid reverse proxy route");
        assert!(error.to_string().contains("upstream"));
    }

    #[test]
    fn validate_rejects_ai_proxy_without_valid_routes() {
        let mut config = GatewayConfig::default();
        config.services.ai_proxy.enabled = true;
        config
            .services
            .ai_proxy
            .routes
            .push(crate::ai_proxy::AiProxyRouteConfig {
                name: "new-api".to_string(),
                provider: "new-api".to_string(),
                match_host: "ai.example.com".to_string(),
                path_prefix: "v1".to_string(),
                upstream: String::new(),
                rewrite_base_path: "/v1".to_string(),
                add_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
                forward_headers: true,
                emit_metadata_headers: true,
            });

        let error = config
            .validate()
            .expect_err("expected invalid ai proxy route");
        let error = error.to_string();
        assert!(error.contains("services.ai_proxy.routes.new-api.path_prefix"));
        assert!(error.contains("services.ai_proxy.routes.new-api.upstream"));
    }

    #[test]
    fn validate_rejects_invalid_rate_limit_window() {
        let mut config = GatewayConfig::default();
        config.services.rate_limit.http.enabled = true;
        config.services.rate_limit.http.window_ms = 1;

        let error = config
            .validate()
            .expect_err("expected invalid rate limit window");
        assert!(error.to_string().contains("window_ms"));
    }

    #[test]
    fn validate_rejects_plugins_without_script_runtime() {
        let mut config = GatewayConfig::default();
        config.plugins.enabled = true;

        let error = config
            .validate()
            .expect_err("expected invalid plugin/script relation");
        assert!(error
            .to_string()
            .contains("plugins.enabled=true requires script.enabled=true"));
    }

    #[test]
    fn validate_rejects_stream_listener_without_yaml_or_script_route() {
        let mut config = GatewayConfig::default();
        config.tcp.listeners.push(TcpListenerConfig {
            name: "game".to_string(),
            bind: "0.0.0.0:7000".to_string(),
            upstream: String::new(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: String::new(),
            nodelay: true,
            connect_timeout_ms: 3_000,
        });

        let error = config
            .validate()
            .expect_err("expected invalid listener without upstream");
        assert!(error
            .to_string()
            .contains("requires upstream/upstreams when script.enabled=false"));
    }

    #[test]
    fn stream_listener_defaults_are_game_ready() {
        let yaml = r#"
tcp:
  listeners:
    - name: game-tcp
      bind: 127.0.0.1:7000
      upstream: 127.0.0.1:9000
udp:
  listeners:
    - name: game-kcp
      bind: 127.0.0.1:7001
      upstream: 127.0.0.1:9001
"#;
        let config: GatewayConfig = serde_yaml::from_str(yaml).expect("config yaml");

        assert!(config.tcp.listeners[0].nodelay);
        assert_eq!(config.tcp.listeners[0].connect_timeout_ms, 3_000);
        assert_eq!(config.udp.listeners[0].session_ttl_secs, 180);
        assert_eq!(config.udp.listeners[0].max_associations, 262_144);
    }

    #[test]
    fn validate_rejects_invalid_stream_listener_tuning() {
        let mut config = GatewayConfig::default();
        config.tcp.listeners.push(TcpListenerConfig {
            name: "game-tcp".to_string(),
            bind: "127.0.0.1:7000".to_string(),
            upstream: "127.0.0.1:9000".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "game_tcp".to_string(),
            nodelay: true,
            connect_timeout_ms: 0,
        });
        config.udp.listeners.push(UdpListenerConfig {
            name: "game-kcp".to_string(),
            bind: "127.0.0.1:7001".to_string(),
            upstream: "127.0.0.1:9001".to_string(),
            upstreams: Vec::new(),
            upstream_weights: BTreeMap::new(),
            protocol: "kcp".to_string(),
            session_ttl_secs: 0,
            max_associations: 1,
        });

        let error = config
            .validate()
            .expect_err("expected invalid stream listener tuning");
        let message = error.to_string();
        assert!(message.contains("connect_timeout_ms"));
        assert!(message.contains("session_ttl_secs"));
    }

    #[test]
    fn explicit_include_merges_child_config() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-include-unsupported-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let base = base_dir.join("proxysss.yaml");

        fs::write(
            &base,
            "include:\n  enabled: true\n  required: true\n  files:\n    - ./conf.d/admin.yaml\n",
        )
        .expect("write base config");

        let error = GatewayConfig::load(&base).expect_err("include should be rejected");
        assert!(error.to_string().contains("legacy config"));
        assert!(error.to_string().contains("include is removed"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn legacy_external_script_runtime_is_rejected() {
        let base_dir = std::env::temp_dir().join(format!(
            "proxysss-legacy-script-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let base = base_dir.join("proxysss.yaml");

        fs::write(
            &base,
            "script:\n  command: /usr/bin/deno\n  args: [run, -A, gateway.ts]\n",
        )
        .expect("write legacy script config");

        let error = GatewayConfig::load(&base).expect_err("legacy script runtime should fail");
        assert!(error.to_string().contains("script.command"));
        assert!(error.to_string().contains("embedded engine"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn include_enabled_requires_files() {
        let mut config = GatewayConfig::default();
        config.include.enabled = true;
        config
            .include
            .files
            .push(PathBuf::from("conf.d/extra.yaml"));
        let error = config.validate().expect_err("expected unsupported include");
        assert!(error.to_string().contains("include is removed"));
    }

    #[test]
    fn json_config_files_are_rejected() {
        let base_dir =
            std::env::temp_dir().join(format!("proxysss-json-config-test-{}", std::process::id()));
        fs::create_dir_all(&base_dir).expect("create temp config dir");
        let config_path = base_dir.join("proxysss.json");
        fs::write(&config_path, "{\"plugins\":{\"enabled\":false}}").expect("write json config");

        let error = GatewayConfig::load(&config_path).expect_err("json config should fail");
        assert!(error
            .to_string()
            .contains("JSON config files are unsupported"));

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn validate_rejects_excessive_retry_count() {
        let mut config = GatewayConfig::default();
        config.load_balance.retries.max_retries = 32;
        let error = config.validate().expect_err("expected invalid retries");
        assert!(error
            .to_string()
            .contains("load_balance.retries.max_retries"));
    }

    #[test]
    fn validate_rejects_invalid_passive_health_threshold() {
        let mut config = GatewayConfig::default();
        config.load_balance.passive_health.fail_threshold = 0;
        let error = config
            .validate()
            .expect_err("expected invalid passive health threshold");
        assert!(error
            .to_string()
            .contains("load_balance.passive_health.fail_threshold"));
    }

    #[test]
    fn validate_rejects_invalid_active_health_path() {
        let mut config = GatewayConfig::default();
        config.load_balance.active_health.enabled = true;
        config.load_balance.active_health.path = "healthz".to_string();

        let error = config
            .validate()
            .expect_err("expected invalid active health path");
        assert!(error
            .to_string()
            .contains("load_balance.active_health.path"));
    }

    #[test]
    fn root_password_can_be_overridden() {
        let mut config = GatewayConfig::default();
        config.admin.username = "ops-admin".to_string();
        config.admin.password = "super-secret-password".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_ports_are_nginx_replacement_ports() {
        let config = GatewayConfig::default();
        assert_eq!(config.http.plain_bind, "0.0.0.0:80");
        assert_eq!(config.http.tls_bind, "0.0.0.0:443");
        assert_eq!(config.http.h3_bind, "0.0.0.0:443");
        assert_eq!(config.admin.bind, "127.0.0.1:7777");
    }

    #[test]
    fn monitoring_path_is_configurable_and_validated() {
        let mut config = GatewayConfig::default();
        assert_eq!(config.monitoring.path, "/metrics");
        config.monitoring.path = "metrics".to_string();
        let error = config
            .validate()
            .expect_err("expected invalid monitoring path");
        assert!(error.to_string().contains("monitoring.path"));
    }

    #[test]
    fn on_demand_tls_requires_acme_managed_mode() {
        let mut config = GatewayConfig::default();
        config.http.tls.on_demand.enabled = true;
        let error = config
            .validate()
            .expect_err("expected on-demand tls validation error");
        assert!(error
            .to_string()
            .contains("http.tls.on_demand.enabled requires http.tls.mode=acme_managed"));
    }

    #[test]
    fn domain_pattern_matching_supports_wildcards() {
        assert!(domain_matches_pattern("api.example.com", "*.example.com"));
        assert!(!domain_matches_pattern("example.com", "*.example.com"));
    }

    #[test]
    fn normalize_stream_listen_expands_port_only_values() {
        assert_eq!(normalize_stream_listen("6379"), "0.0.0.0:6379");
    }
}
