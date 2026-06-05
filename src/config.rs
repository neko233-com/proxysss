use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::net::SocketAddr;
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
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub mode: TlsMode,
    #[serde(default)]
    pub auto_https: AutoHttpsConfig,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    #[serde(default = "default_script_command")]
    pub command: String,
    #[serde(default = "default_script_args")]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default = "default_script_timeout_ms")]
    pub timeout_ms: u64,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_reload_interval_ms")]
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesConfig {
    #[serde(default)]
    pub reverse_proxy: ReverseProxyConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRateLimitConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub key: RateLimitKey,
    #[serde(default = "default_rate_limit_requests")]
    pub requests: u32,
    #[serde(default = "default_rate_limit_window_ms")]
    pub window_ms: u64,
    #[serde(default)]
    pub burst: u32,
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
    #[serde(default)]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub strip_headers: Vec<String>,
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

        if self.logging.access_sample_rate <= 0.0 || self.logging.access_sample_rate > 1.0 {
            errors.push("logging.access_sample_rate must be in (0, 1]".to_string());
        }

        if self.runtime.hot_reload.interval_ms < 200 {
            errors.push("runtime.hot_reload.interval_ms must be >= 200".to_string());
        }

        if self.include.enabled && self.include.files.is_empty() {
            errors.push("include.files cannot be empty when include.enabled=true".to_string());
        }

        if self.script.command.trim().is_empty() {
            errors.push("script.command cannot be empty".to_string());
        }

        if self.script.timeout_ms == 0 {
            errors.push("script.timeout_ms must be greater than 0".to_string());
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

        if self.plugins.enabled {
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
            if self.services.rate_limit.http.requests == 0 {
                errors.push("services.rate_limit.http.requests must be greater than 0".to_string());
            }
            if self.services.rate_limit.http.window_ms < 100 {
                errors.push("services.rate_limit.http.window_ms must be >= 100".to_string());
            }
            if !(100..=599).contains(&self.services.rate_limit.http.status) {
                errors.push(
                    "services.rate_limit.http.status must be a valid HTTP status".to_string(),
                );
            }
            if let RateLimitKey::Header(name) = &self.services.rate_limit.http.key {
                if name.trim().is_empty() {
                    errors.push(
                        "services.rate_limit.http.key header name cannot be empty".to_string(),
                    );
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
            TlsMode::AcmeExternal => {
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
                    if self.http.tls.auto_https.client.trim().is_empty() {
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
                        "http.tls.acme.domains cannot be empty when mode is acme_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.email.trim().is_empty() {
                    errors.push(
                        "http.tls.acme.email cannot be empty when mode is acme_external"
                            .to_string(),
                    );
                }
                if self.http.tls.acme.client.trim().is_empty() {
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
            warnings.push("tls.mode=self_signed is for development or internal environments; use acme_external for public traffic".to_string());
        }

        if self.http.plain_bind.trim().is_empty() {
            warnings.push(
                "http.plain_bind is disabled; the welcome page will not be reachable on port 80 (nginx parity expects 0.0.0.0:80)".to_string(),
            );
        }

        warnings
    }

    pub fn render_default_yaml(script_command: &str) -> String {
        let mut config = Self::default();
        config.script.command = script_command.to_string();
        serde_yaml::to_string(&config).unwrap_or_else(|_| "".to_string())
    }

    pub fn render_default_json(script_command: &str) -> String {
        let mut config = Self::default();
        config.script.command = script_command.to_string();
        serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string())
    }

    fn normalize(&mut self, root_dir: &Path) {
        let root_dir = normalize_root_dir(root_dir);
        self.root_dir = root_dir.clone();
        self.apply_auto_https();

        if self.logging.filter.trim().is_empty() {
            self.logging.filter = default_filter_for_level(self.logging.level);
        }
        self.log_filter = self.logging.filter.clone();

        self.http.tls.cert_path = absolutize(&root_dir, &self.http.tls.cert_path);
        self.http.tls.key_path = absolutize(&root_dir, &self.http.tls.key_path);
        self.http.tls.acme.cache_dir = absolutize(&root_dir, &self.http.tls.acme.cache_dir);
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

    fn apply_auto_https(&mut self) {
        if !self.http.tls.auto_https.enabled {
            return;
        }

        self.http.tls.mode = TlsMode::AcmeExternal;
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
            tls: TlsConfig::default(),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: TlsMode::default(),
            auto_https: AutoHttpsConfig::default(),
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

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            command: default_script_command(),
            args: default_script_args(),
            cwd: Some(PathBuf::from(".")),
            timeout_ms: default_script_timeout_ms(),
        }
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
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
        }
    }
}

impl Default for HttpRateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key: RateLimitKey::default(),
            requests: default_rate_limit_requests(),
            window_ms: default_rate_limit_window_ms(),
            burst: 0,
            status: default_rate_limit_status(),
        }
    }
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_ftp_bind(),
            upstream: default_ftp_upstream(),
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

fn default_script_command() -> String {
    "deno".to_string()
}

fn default_script_args() -> Vec<String> {
    vec![
        "run".to_string(),
        "-A".to_string(),
        DEFAULT_SCRIPT_FILE_NAME.to_string(),
    ]
}

fn default_script_timeout_ms() -> u64 {
    500
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

fn default_rate_limit_requests() -> u32 {
    60
}

fn default_rate_limit_window_ms() -> u64 {
    60_000
}

fn default_rate_limit_status() -> u16 {
    429
}

fn default_ftp_bind() -> String {
    "0.0.0.0:21".to_string()
}

fn default_ftp_upstream() -> String {
    "127.0.0.1:2121".to_string()
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
        assert_eq!(config.load_balance.passive_health.fail_threshold, 3);
        assert_eq!(config.load_balance.passive_health.quarantine_secs, 15);
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
    fn auto_https_expands_to_acme_external_config() {
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
        assert_eq!(config.http.tls.mode, TlsMode::AcmeExternal);
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
