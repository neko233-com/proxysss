use std::collections::HashSet;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

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
    #[serde(skip)]
    pub root_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub format: LogFormat,
    #[serde(default = "default_log_filter")]
    pub filter: String,
    #[serde(default = "default_true")]
    pub access_log: bool,
    #[serde(default = "default_sample_rate")]
    pub access_sample_rate: f64,
    #[serde(default = "default_slow_request_ms")]
    pub slow_request_ms: u64,
    #[serde(default = "default_redact_headers")]
    pub redact_headers: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Plain,
    #[default]
    Json,
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

impl GatewayConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;

        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        let mut config = match ext.as_deref() {
            Some("json") => parse_json_then_yaml(&raw, path)?,
            _ => parse_yaml_then_json(&raw, path)?,
        };

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
        self.root_dir = root_dir.to_path_buf();

        if self.logging.filter.trim().is_empty() {
            self.logging.filter = self.log_filter.clone();
        }
        self.log_filter = self.logging.filter.clone();

        self.http.tls.cert_path = absolutize(root_dir, &self.http.tls.cert_path);
        self.http.tls.key_path = absolutize(root_dir, &self.http.tls.key_path);
        self.http.tls.acme.cache_dir = absolutize(root_dir, &self.http.tls.acme.cache_dir);
        self.plugins.auto_load_dir = absolutize(root_dir, &self.plugins.auto_load_dir);

        self.script.cwd = Some(match &self.script.cwd {
            Some(cwd) => absolutize(root_dir, cwd),
            None => root_dir.to_path_buf(),
        });

        normalize_vec_lowercase(&mut self.affinity.http.header_keys);
        normalize_vec_lowercase(&mut self.logging.redact_headers);
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            config_version: default_config_version(),
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
            root_dir: PathBuf::from("."),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::default(),
            filter: default_log_filter(),
            access_log: default_true(),
            access_sample_rate: default_sample_rate(),
            slow_request_ms: default_slow_request_ms(),
            redact_headers: default_redact_headers(),
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
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            generate_self_signed_if_missing: default_true(),
            server_name: default_server_name(),
            acme: AcmeExternalConfig::default(),
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

fn parse_yaml_then_json(raw: &str, path: &Path) -> Result<GatewayConfig> {
    serde_yaml::from_str(raw)
        .or_else(|_| serde_json::from_str(raw))
        .with_context(|| format!("failed to parse config as yaml/json: {}", path.display()))
}

fn parse_json_then_yaml(raw: &str, path: &Path) -> Result<GatewayConfig> {
    serde_json::from_str(raw)
        .or_else(|_| serde_yaml::from_str(raw))
        .with_context(|| format!("failed to parse config as json/yaml: {}", path.display()))
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
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
    vec![TcpListenerConfig {
        name: "game-login".to_string(),
        bind: "0.0.0.0:26379".to_string(),
    }]
}

fn default_udp_listeners() -> Vec<UdpListenerConfig> {
    vec![UdpListenerConfig {
        name: "game-realtime".to_string(),
        bind: "0.0.0.0:2053".to_string(),
    }]
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
    "127.0.0.1:7778".to_string()
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
    fn default_ports_are_nginx_and_caddy_replacement_ports() {
        let config = GatewayConfig::default();
        assert_eq!(config.http.plain_bind, "0.0.0.0:80");
        assert_eq!(config.http.tls_bind, "0.0.0.0:443");
        assert_eq!(config.http.h3_bind, "0.0.0.0:443");
        assert_eq!(config.admin.bind, "127.0.0.1:7778");
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
