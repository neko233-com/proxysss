use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub windows: DesktopPerformanceConfig,
    #[serde(default)]
    pub macos: DesktopPerformanceConfig,
}

/// Per-socket settings; zero retains the OS default. Never changes host state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopPerformanceConfig {
    pub enabled: bool,
    pub tcp_keepalive_secs: u32,
    pub tcp_keepalive_interval_secs: u32,
    pub tcp_send_buffer_bytes: usize,
    pub tcp_receive_buffer_bytes: usize,
    /// Shared listener only; per-peer associations retain native defaults.
    pub udp_buffer_bytes: usize,
    /// macOS only; zero preserves the kernel default.
    pub tcp_notsent_lowat_bytes: usize,
}

impl Default for DesktopPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tcp_keepalive_secs: 60,
            tcp_keepalive_interval_secs: 15,
            tcp_send_buffer_bytes: 0,
            tcp_receive_buffer_bytes: 0,
            udp_buffer_bytes: 256 * 1024,
            tcp_notsent_lowat_bytes: 0,
        }
    }
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

impl Default for RuntimePerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            profile: RuntimePerformanceProfile::default(),
            traffic_profile: RuntimePerformanceTrafficProfile::default(),
            adaptive_system: default_true(),
            socket_extreme: default_true(),
            log_on_start: default_true(),
            windows: DesktopPerformanceConfig::default(),
            macos: DesktopPerformanceConfig::default(),
        }
    }
}
