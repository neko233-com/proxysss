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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default)]
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
pub struct TlsConfig {
    #[serde(default)]
    pub mode: TlsMode,
    #[serde(default)]
    pub auto_https: AutoHttpsConfig,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TcpConfig {
    #[serde(default = "default_tcp_listeners")]
    pub listeners: Vec<TcpListenerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpListenerConfig {
    pub name: String,
    pub bind: String,
    #[serde(default)]
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
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
pub struct AdminConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_admin_bind")]
    pub bind: String,
    #[serde(default = "default_admin_username")]
    pub username: String,
    #[serde(default = "default_admin_password")]
    pub password: String,
    #[serde(default = "default_true")]
    pub expose_config: bool,
    #[serde(default = "default_true")]
    pub enable_write_ops: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_monitoring_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub hot_reload: HotReloadConfig,
    #[serde(default)]
    pub maintenance_state: MaintenanceStateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    #[serde(default = "default_true")]
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
    pub static_sites: Vec<StaticSiteConfig>,
    #[serde(default)]
    pub ftp: FtpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub http: HttpRateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessControlConfig {
    #[serde(default)]
    pub http: HttpAccessControlConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rate_limit_zone")]
    pub zone: String,
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
    pub strip_prefix: bool,
    #[serde(default, alias = "add_headers")]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default, alias = "remove_headers")]
    pub strip_headers: Vec<String>,
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
    pub strip_prefix: bool,
    #[serde(default, alias = "add_headers")]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default, alias = "remove_headers")]
    pub strip_headers: Vec<String>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    Zstd,
    Brotli,
    Gzip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseCacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_zone")]
    pub zone: String,
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
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
pub struct FtpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ftp_bind")]
    pub bind: String,
    #[serde(default = "default_ftp_upstream")]
    pub upstream: String,
    #[serde(default = "default_true")]
    pub native_control: bool,
    #[serde(default)]
    pub public_ip: String,
    #[serde(default = "default_ftp_passive_port_start")]
    pub passive_port_start: u16,
    #[serde(default = "default_ftp_passive_port_end")]
    pub passive_port_end: u16,
    #[serde(default = "default_true")]
    pub passive_hint: bool,
}

impl GatewayConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let mut visited = HashSet::new();
        let value = load_config_value_recursive(path, &mut visited)?;
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

        if self.include.enabled && self.include.files.is_empty() {
            errors.push("include.files cannot be empty when include.enabled=true".to_string());
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
            {
                errors.push(
                    "load_balance.active_health requires http_enabled or tcp_enabled".to_string(),
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
            TlsMode::AcmeManaged | TlsMode::AcmeExternal => {
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
                    if self.http.tls.mode == TlsMode::AcmeExternal
                        && self.http.tls.auto_https.client.trim().is_empty()
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
                        "http.tls.acme.domains cannot be empty when mode is acme_managed/acme_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.email.trim().is_empty() {
                    errors.push(
                        "http.tls.acme.email cannot be empty when mode is acme_managed/acme_external"
                            .to_string(),
                    );
                }
                if self.http.tls.mode == TlsMode::AcmeExternal
                    && self.http.tls.acme.client.trim().is_empty()
                {
                    errors.push(
                        "http.tls.acme.client cannot be empty when mode is acme_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.renew_interval_hours == 0 {
                    errors.push(
                        "http.tls.acme.renew_interval_hours must be greater than 0".to_string(),
                    );
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
            warnings.push("tls.mode=self_signed is for development or internal environments; use acme_managed or acme_external for public traffic".to_string());
        }

        if self.http.plain_bind.trim().is_empty() {
            warnings.push(
                "http.plain_bind is disabled; the welcome page will not be reachable on port 80 (nginx parity expects 0.0.0.0:80)".to_string(),
            );
        }

        warnings
    }

    pub fn render_default_yaml() -> String {
        let config = Self::default();
        serde_yaml::to_string(&config).unwrap_or_else(|_| "".to_string())
    }

    pub fn render_default_json() -> String {
        let config = Self::default();
        serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string())
    }

    fn normalize(&mut self, root_dir: &Path) {
        let root_dir = normalize_root_dir(root_dir);
        self.root_dir = root_dir.clone();
        self.normalize_domain_tls(&root_dir);
        self.apply_auto_https();

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
        for site in &mut self.services.static_sites {
            site.root = absolutize(&root_dir, &site.root);
        }

        self.script.cwd = Some(match &self.script.cwd {
            Some(cwd) => absolutize(&root_dir, cwd),
            None => root_dir.clone(),
        });

        normalize_vec_lowercase(&mut self.affinity.http.header_keys);
        normalize_vec_lowercase(&mut self.logging.redact_headers);
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

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::default(),
            auto_https: AutoHttpsConfig::default(),
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
            interval_secs: default_active_health_interval_secs(),
            timeout_ms: default_active_health_timeout_ms(),
            path: default_active_health_path(),
            expected_statuses: default_active_health_expected_statuses(),
            failure_threshold: default_active_health_failure_threshold(),
            success_threshold: default_active_health_success_threshold(),
            jitter_percent: 0,
            alert_webhooks: Vec::new(),
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

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            bind: default_admin_bind(),
            username: default_admin_username(),
            password: default_admin_password(),
            expose_config: default_true(),
            enable_write_ops: default_true(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            path: default_monitoring_path(),
        }
    }
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
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
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
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
            strip_prefix: false,
            set_headers: BTreeMap::new(),
            strip_headers: Vec::new(),
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

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        Self::Zstd
    }
}

impl Default for ResponseCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            zone: default_cache_zone(),
            ttl_secs: default_cache_ttl_secs(),
            statuses: default_cache_statuses(),
            max_body_bytes: default_cache_max_body_bytes(),
            allow_purge: default_true(),
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
        }
    }
}

fn load_config_value_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<serde_yaml::Value> {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical.clone()) {
        return Err(anyhow!(
            "configuration include cycle detected at {}",
            canonical.display()
        ));
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let mut base = parse_value_by_extension(&raw, path)?;
    let include = read_include_config(&base)?;

    if include.enabled {
        let root_dir = path.parent().unwrap_or_else(|| Path::new("."));
        for child in include.files {
            let child_path = absolutize(root_dir, &child);
            if !child_path.exists() {
                if include.required {
                    return Err(anyhow!(
                        "required include file does not exist: {}",
                        child_path.display()
                    ));
                }
                continue;
            }

            let child_value = load_config_value_recursive(&child_path, visited)?;
            merge_value(&mut base, child_value);
        }
    }

    visited.remove(&canonical);
    Ok(base)
}

fn parse_value_by_extension(raw: &str, path: &Path) -> Result<serde_yaml::Value> {
    let raw = raw.trim_start_matches('\u{feff}');
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match ext.as_deref() {
        Some("json") => {
            let json: serde_json::Value = serde_json::from_str(raw)
                .with_context(|| format!("failed to parse json config {}", path.display()))?;
            serde_yaml::to_value(json)
                .with_context(|| format!("failed to convert json config {}", path.display()))
        }
        _ => serde_yaml::from_str(raw)
            .with_context(|| format!("failed to parse yaml config {}", path.display())),
    }
}

fn read_include_config(value: &serde_yaml::Value) -> Result<IncludeConfig> {
    let include = value
        .get("include")
        .cloned()
        .unwrap_or(serde_yaml::Value::Null);
    Ok(serde_yaml::from_value(include).unwrap_or_default())
}

fn merge_value(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
    match (base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => merge_value(base_value, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
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

fn default_tcp_listeners() -> Vec<TcpListenerConfig> {
    Vec::new()
}

fn default_udp_listeners() -> Vec<UdpListenerConfig> {
    Vec::new()
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

fn default_monitoring_path() -> String {
    "/metrics".to_string()
}

fn default_admin_username() -> String {
    DEFAULT_ADMIN_USERNAME.to_string()
}

fn default_admin_password() -> String {
    DEFAULT_ADMIN_PASSWORD.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_admin_credentials_are_root_root() {
        let config = GatewayConfig::default();
        assert_eq!(config.admin.username, DEFAULT_ADMIN_USERNAME);
        assert_eq!(config.admin.password, DEFAULT_ADMIN_PASSWORD);
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
        assert_eq!(config.load_balance.active_health.interval_secs, 10);
        assert_eq!(config.load_balance.active_health.timeout_ms, 2000);
        assert_eq!(config.load_balance.active_health.path, "/healthz");
        assert_eq!(config.load_balance.passive_health.fail_threshold, 3);
        assert_eq!(config.load_balance.passive_health.quarantine_secs, 15);
    }

    #[test]
    fn validate_rejects_active_health_without_any_probe_kind() {
        let mut config = GatewayConfig::default();
        config.load_balance.active_health.enabled = true;
        config.load_balance.active_health.http_enabled = false;
        config.load_balance.active_health.tcp_enabled = false;

        let error = config
            .validate()
            .expect_err("expected invalid active health protocol selection");
        assert!(error
            .to_string()
            .contains("load_balance.active_health requires http_enabled or tcp_enabled"));
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
                strip_prefix: false,
                set_headers: BTreeMap::new(),
                strip_headers: Vec::new(),
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
        });

        let error = config
            .validate()
            .expect_err("expected invalid listener without upstream");
        assert!(error
            .to_string()
            .contains("requires upstream/upstreams when script.enabled=false"));
    }

    #[test]
    fn explicit_include_merges_child_config() {
        let base_dir =
            std::env::temp_dir().join(format!("proxysss-include-test-{}", std::process::id()));
        let conf_dir = base_dir.join("conf.d");
        fs::create_dir_all(&conf_dir).expect("create temp config dir");
        let base = base_dir.join("proxysss.yaml");
        let child = conf_dir.join("admin.yaml");

        fs::write(
            &base,
            "include:\n  enabled: true\n  required: true\n  files:\n    - ./conf.d/admin.yaml\n",
        )
        .expect("write base config");
        fs::write(&child, "admin:\n  bind: 127.0.0.1:7778\n").expect("write child config");

        let config = GatewayConfig::load(&base).expect("load merged config");
        assert_eq!(config.admin.bind, "127.0.0.1:7778");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn include_enabled_requires_files() {
        let mut config = GatewayConfig::default();
        config.include.enabled = true;
        config.include.files.clear();
        let error = config.validate().expect_err("expected invalid include");
        assert!(error.to_string().contains("include.files"));
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
}
