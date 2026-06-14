//! Linux-specific TCP/sysctl tuning profiles for Ubuntu and Debian gateways.
//!
//! Strategy factory pattern: each supported distro gets a dedicated tuning
//! strategy that knows the kernel version, default values, and safe upper
//! bounds.  At startup (or via `proxysss tune linux`) the hardware is
//! auto-detected and the best strategy is selected automatically.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

// ── workload profiles ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TcpTuneProfile {
    /// Reverse-proxy, CDN edge, mixed HTTP workloads
    Edge,
    /// Large-file transfer, backup, media streaming
    Bulk,
    /// Low-latency API, gaming, realtime
    Latency,
}

// ── supported distros ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinuxDistro {
    Auto,
    Ubuntu2204,
    Ubuntu2404,
    Ubuntu2604,
    Debian12,
    Debian13,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxDistroDetails {
    pub id: String,
    pub version_id: String,
    pub major_version: Option<u32>,
    pub detected: LinuxDistro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSocketTuneLevel {
    Disabled,
    PortableLinux,
    Ubuntu24Extreme,
    FutureLinuxExtreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeTunePlan {
    pub enabled: bool,
    pub os: String,
    pub distro: LinuxDistro,
    pub distro_id: String,
    pub version_id: String,
    pub major_version: Option<u32>,
    pub profile: TcpTuneProfile,
    pub socket_level: RuntimeSocketTuneLevel,
    pub enabled_features: Vec<String>,
    pub skipped_features: Vec<String>,
}

// ── hardware survey (auto-detected or user-overridden) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpTuneSurvey {
    pub profile: TcpTuneProfile,
    pub distro: LinuxDistro,
    pub memory_gb: u32,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub nic_gbps: u32,
    pub max_connections: u32,
    pub l3_cache_kb: u32,
    pub numa_nodes: u32,
    pub busy_poll_capable: bool,
}

impl Default for TcpTuneSurvey {
    fn default() -> Self {
        Self {
            profile: TcpTuneProfile::Edge,
            distro: LinuxDistro::Auto,
            memory_gb: 16,
            cpu_cores: 8,
            cpu_threads: 16,
            nic_gbps: 10,
            max_connections: 20_000,
            l3_cache_kb: 0,
            numa_nodes: 1,
            busy_poll_capable: true,
        }
    }
}

// ── CLI args ─────────────────────────────────────────────────────────────

#[derive(Args, Debug, Clone)]
pub struct LinuxTuneArgs {
    #[arg(long, value_enum, default_value_t = TcpTuneProfileArg::Edge)]
    pub profile: TcpTuneProfileArg,
    #[arg(long, value_enum, default_value_t = LinuxDistroArg::Auto)]
    pub distro: LinuxDistroArg,
    #[arg(long, default_value_t = 0)]
    pub memory_gb: u32,
    #[arg(long, default_value_t = 0)]
    pub cpu_cores: u32,
    #[arg(long, default_value_t = 0)]
    pub nic_gbps: u32,
    #[arg(long, default_value_t = 20000)]
    pub max_connections: u32,
    #[arg(long, default_value_t = false)]
    pub apply: bool,
    /// Bypass SSH/session safety checks and unsupported-key filtering.
    #[arg(long, default_value_t = false)]
    pub unsafe_apply: bool,
    /// Do not restore the previous sysctl profile if sysctl --system fails.
    #[arg(long, default_value_t = false)]
    pub no_rollback: bool,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TcpTuneProfileArg {
    Edge,
    Bulk,
    Latency,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LinuxDistroArg {
    Auto,
    Ubuntu2204,
    Ubuntu2404,
    Ubuntu2604,
    Debian12,
    Debian13,
}

impl From<TcpTuneProfileArg> for TcpTuneProfile {
    fn from(value: TcpTuneProfileArg) -> Self {
        match value {
            TcpTuneProfileArg::Edge => Self::Edge,
            TcpTuneProfileArg::Bulk => Self::Bulk,
            TcpTuneProfileArg::Latency => Self::Latency,
        }
    }
}

impl From<LinuxDistroArg> for LinuxDistro {
    fn from(value: LinuxDistroArg) -> Self {
        match value {
            LinuxDistroArg::Auto => Self::Auto,
            LinuxDistroArg::Ubuntu2204 => Self::Ubuntu2204,
            LinuxDistroArg::Ubuntu2404 => Self::Ubuntu2404,
            LinuxDistroArg::Ubuntu2604 => Self::Ubuntu2604,
            LinuxDistroArg::Debian12 => Self::Debian12,
            LinuxDistroArg::Debian13 => Self::Debian13,
        }
    }
}

// ── Hardware auto-detection ──────────────────────────────────────────────

pub fn detect_linux_distro() -> LinuxDistro {
    detect_linux_distro_details().detected
}

pub fn detect_linux_distro_details() -> LinuxDistroDetails {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let id = parse_os_release_value(&content, "ID");
    let version_id = parse_os_release_value(&content, "VERSION_ID");
    let detected = match (id.as_str(), version_id.as_str()) {
        ("ubuntu", "22.04") => LinuxDistro::Ubuntu2204,
        ("ubuntu", "24.04") => LinuxDistro::Ubuntu2404,
        ("ubuntu", "26.04") => LinuxDistro::Ubuntu2604,
        ("debian", "12") => LinuxDistro::Debian12,
        ("debian", "13") => LinuxDistro::Debian13,
        ("ubuntu", v) if v.starts_with("22.") => LinuxDistro::Ubuntu2204,
        ("ubuntu", v) if v.starts_with("24.") => LinuxDistro::Ubuntu2404,
        ("ubuntu", v) if v.starts_with("26.") => LinuxDistro::Ubuntu2604,
        ("debian", v) if v.starts_with("12") => LinuxDistro::Debian12,
        ("debian", v) if v.starts_with("13") => LinuxDistro::Debian13,
        _ => LinuxDistro::Auto,
    };
    LinuxDistroDetails {
        id,
        major_version: parse_major_version(&version_id),
        version_id,
        detected,
    }
}

fn parse_os_release_value(content: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| {
            line.trim_start_matches(&prefix)
                .trim_matches('\"')
                .to_string()
        })
        .unwrap_or_default()
}

fn parse_major_version(version_id: &str) -> Option<u32> {
    version_id
        .split(['.', '-'])
        .next()
        .and_then(|value| value.parse::<u32>().ok())
}

pub fn detect_cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

pub fn detect_memory_gb() -> u32 {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u32 = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return kb.div_ceil(1024 * 1024);
        }
    }
    8
}

pub fn detect_l3_cache_kb() -> u32 {
    for idx in 0..8 {
        let level_path = format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/level");
        let size_path = format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/size");
        if let Ok(lvl) = std::fs::read_to_string(&level_path) {
            if lvl.trim() == "3" {
                if let Ok(content) = std::fs::read_to_string(&size_path) {
                    let trimmed = content.trim();
                    if let Some(kb_str) = trimmed.strip_suffix('K') {
                        if let Ok(kb) = kb_str.parse::<u32>() {
                            return kb;
                        }
                    }
                    if let Some(mb_str) = trimmed.strip_suffix('M') {
                        if let Ok(mb) = mb_str.parse::<u32>() {
                            return mb * 1024;
                        }
                    }
                }
            }
        }
    }
    0
}

pub fn detect_numa_nodes() -> u32 {
    std::fs::read_to_string("/sys/devices/system/node/possible")
        .ok()
        .and_then(|s| {
            let parts: Vec<&str> = s.trim().split('-').collect();
            if parts.len() == 2 {
                let max: u32 = parts[1].parse().ok()?;
                Some(max + 1)
            } else {
                parts[0].parse::<u32>().ok().map(|_| 1u32)
            }
        })
        .unwrap_or(1)
}

pub fn detect_nic_gbps() -> u32 {
    let entries = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();
    for line in entries.lines().skip(2) {
        let iface = line.split(':').next().unwrap_or("").trim();
        if iface == "lo" || iface.is_empty() {
            continue;
        }
        let speed_path = format!("/sys/class/net/{iface}/speed");
        if let Ok(content) = std::fs::read_to_string(&speed_path) {
            if let Ok(mbps) = content.trim().parse::<u32>() {
                if mbps > 0 {
                    return mbps.div_ceil(1000);
                }
            }
        }
    }
    10
}

pub fn auto_fill_survey(survey: &mut TcpTuneSurvey) {
    if survey.cpu_cores == 0 {
        survey.cpu_cores = detect_cpu_cores();
    }
    if survey.cpu_threads == 0 {
        survey.cpu_threads = survey.cpu_cores;
    }
    if survey.memory_gb == 0 {
        survey.memory_gb = detect_memory_gb();
    }
    if survey.nic_gbps == 0 {
        survey.nic_gbps = detect_nic_gbps();
    }
    if survey.l3_cache_kb == 0 {
        survey.l3_cache_kb = detect_l3_cache_kb();
    }
    if survey.numa_nodes == 0 {
        survey.numa_nodes = detect_numa_nodes();
    }
    if survey.distro == LinuxDistro::Auto {
        survey.distro = detect_linux_distro();
    }
}

// ── Strategy factory ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SysctlParams {
    pub entries: BTreeMap<String, String>,
    pub comments: Vec<String>,
}

impl SysctlParams {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            comments: Vec::new(),
        }
    }

    fn set(&mut self, key: &str, value: impl std::fmt::Display) {
        self.entries.insert(key.to_string(), value.to_string());
    }

    fn comment(&mut self, text: &str) {
        self.comments.push(text.to_string());
    }

    pub fn render(&self) -> String {
        let mut out = String::with_capacity(4096);
        for c in &self.comments {
            out.push_str("# ");
            out.push_str(c);
            out.push('\n');
        }
        for (k, v) in &self.entries {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
        out
    }
}

pub trait TuneStrategy: Send + Sync {
    fn distro_label(&self) -> &'static str;
    fn apply(&self, survey: &TcpTuneSurvey, params: &mut SysctlParams);
}

fn apply_common_tcp_params(survey: &TcpTuneSurvey, p: &mut SysctlParams) {
    let conn = survey.max_connections.max(1024);
    let somaxconn = (conn / 4).clamp(4096, 65_535);
    let syn_backlog = (conn / 2).clamp(4096, 262_144);
    let netdev_backlog = (survey.nic_gbps.saturating_mul(8192)).clamp(8192, 262_144);
    let mem = survey.memory_gb.max(4) as u64 * 1024 * 1024;
    let rmem = (mem * 2).clamp(32 * 1024 * 1024, 256 * 1024 * 1024) as u32;
    let wmem = rmem;

    let (fin_timeout, keepalive, busy_poll) = match survey.profile {
        TcpTuneProfile::Latency => (10, 60, 50),
        TcpTuneProfile::Bulk => (20, 120, 0),
        TcpTuneProfile::Edge => (15, 90, 0),
    };

    p.comment(&format!(
        "proxysss linux tcp tuning - profile={:?} cores={} mem={}G nic={}G",
        survey.profile, survey.cpu_cores, survey.memory_gb, survey.nic_gbps
    ));

    p.set("net.core.somaxconn", somaxconn);
    p.set("net.ipv4.tcp_max_syn_backlog", syn_backlog);
    p.set("net.core.netdev_max_backlog", netdev_backlog);
    p.set("net.core.rmem_max", rmem);
    p.set("net.core.wmem_max", wmem);
    p.set("net.ipv4.tcp_rmem", format!("4096 87380 {rmem}"));
    p.set("net.ipv4.tcp_wmem", format!("4096 262144 {wmem}"));
    p.set("net.ipv4.ip_local_port_range", "10240 65535");
    p.set("net.ipv4.tcp_fin_timeout", fin_timeout);
    p.set("net.ipv4.tcp_tw_reuse", 1);
    p.set("net.ipv4.tcp_keepalive_time", keepalive);
    p.set("net.ipv4.tcp_keepalive_intvl", 15);
    p.set("net.ipv4.tcp_keepalive_probes", 5);
    p.set("net.ipv4.tcp_mtu_probing", 1);
    p.set("net.ipv4.tcp_fastopen", 3);
    p.set("net.ipv4.tcp_slow_start_after_idle", 0);
    p.set("net.ipv4.tcp_window_scaling", 1);
    p.set("net.core.default_qdisc", "fq");
    p.set("net.ipv4.tcp_congestion_control", "bbr");
    p.set("net.core.busy_poll", busy_poll);
    p.set("net.core.busy_read", busy_poll);

    let tw_buckets = if survey.max_connections > 50_000 {
        4_000_000
    } else {
        2_000_000
    };
    p.set("net.ipv4.tcp_max_tw_buckets", tw_buckets);

    // File-descriptor ceilings for very high concurrent socket counts
    // (100k-1M connections as a super-scale gateway). fs.nr_open raises the
    // per-process hard cap that bounds `ulimit -n`; fs.file-max is the
    // system-wide open-file ceiling. Pair with a matching systemd LimitNOFILE.
    p.set("fs.nr_open", 12_000_000);
    p.set("fs.file-max", 12_000_000);
}

struct Ubuntu2404Strategy;
impl TuneStrategy for Ubuntu2404Strategy {
    fn distro_label(&self) -> &'static str {
        "ubuntu-24.04"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        p.comment("Ubuntu 24.04 LTS: kernel 6.8+, aggressive edge/gateway tuning");
        let budget = if survey.cpu_cores >= 8 { 900 } else { 600 };
        let budget_usecs = if survey.cpu_cores >= 8 { 12000 } else { 8000 };
        p.set("net.core.netdev_budget", budget);
        p.set("net.core.netdev_budget_usecs", budget_usecs);
        p.set("net.ipv4.tcp_no_metrics_save", 1);
        p.set("net.ipv4.tcp_sack", 1);
        p.set("net.ipv4.tcp_dsack", 1);
        p.set("net.ipv4.tcp_timestamps", 1);
        p.set("net.ipv4.tcp_ecn", 0);
        p.set("net.ipv4.tcp_frto", 2);
        p.set("vm.swappiness", 10);
        p.set("vm.dirty_ratio", 10);
        p.set("vm.dirty_background_ratio", 3);
        p.set("net.ipv4.tcp_abort_on_overflow", 0);
        p.set(
            "net.ipv4.tcp_max_orphans",
            (survey.memory_gb * 16384).max(65536),
        );
        p.set("net.core.optmem_max", 131072);
        if matches!(survey.profile, TcpTuneProfile::Latency) {
            p.set("net.ipv4.tcp_thin_linear_timeouts", 1);
            p.set("net.ipv4.tcp_thin_dupack", 1);
        }
    }
}

struct Ubuntu2204Strategy;
impl TuneStrategy for Ubuntu2204Strategy {
    fn distro_label(&self) -> &'static str {
        "ubuntu-22.04"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        p.comment("Ubuntu 22.04 LTS: kernel 5.15+, fq + bbr stable");
        p.set("vm.swappiness", 10);
        p.set("net.ipv4.tcp_abort_on_overflow", 0);
    }
}

struct Ubuntu2604Strategy;
impl TuneStrategy for Ubuntu2604Strategy {
    fn distro_label(&self) -> &'static str {
        "ubuntu-26.04"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        p.comment("Ubuntu 26.04 LTS+: highest throughput ceiling");
        p.set("net.core.netdev_budget", 900);
        p.set("net.core.netdev_budget_usecs", 12000);
    }
}

struct Debian12Strategy;
impl TuneStrategy for Debian12Strategy {
    fn distro_label(&self) -> &'static str {
        "debian-12"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        p.comment("Debian 12 (bookworm): kernel 6.1+, gateway-optimized");
        p.set("net.core.netdev_max_backlog", 32768);
        p.set("net.ipv4.tcp_no_metrics_save", 1);
        p.set("net.ipv4.tcp_sack", 1);
        p.set("net.ipv4.tcp_dsack", 1);
        p.set("net.ipv4.tcp_timestamps", 1);
        p.set("net.ipv4.tcp_ecn", 0);
        p.set("vm.swappiness", 10);
        p.set("vm.dirty_ratio", 10);
        p.set("vm.dirty_background_ratio", 3);
    }
}

struct Debian13Strategy;
impl TuneStrategy for Debian13Strategy {
    fn distro_label(&self) -> &'static str {
        "debian-13"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        p.comment("Debian 13 (trixie): newer default qdisc");
        p.set("net.core.default_qdisc", "fq_codel");
    }
}

struct AutoStrategy;
impl TuneStrategy for AutoStrategy {
    fn distro_label(&self) -> &'static str {
        "auto"
    }
    fn apply(&self, survey: &TcpTuneSurvey, p: &mut SysctlParams) {
        apply_common_tcp_params(survey, p);
        if survey.cpu_cores >= 16 {
            p.set("net.core.netdev_budget", 600);
        }
    }
}

fn strategy_for_distro(distro: LinuxDistro) -> Box<dyn TuneStrategy> {
    match distro {
        LinuxDistro::Ubuntu2204 => Box::new(Ubuntu2204Strategy),
        LinuxDistro::Ubuntu2404 => Box::new(Ubuntu2404Strategy),
        LinuxDistro::Ubuntu2604 => Box::new(Ubuntu2604Strategy),
        LinuxDistro::Debian12 => Box::new(Debian12Strategy),
        LinuxDistro::Debian13 => Box::new(Debian13Strategy),
        LinuxDistro::Auto => Box::new(AutoStrategy),
    }
}

// ── Public API ───────────────────────────────────────────────────────────

pub fn render_linux_tcp_sysctl_profile(survey: &TcpTuneSurvey) -> String {
    let distro = if survey.distro == LinuxDistro::Auto {
        detect_linux_distro()
    } else {
        survey.distro
    };
    let strategy = strategy_for_distro(distro);
    let mut params = SysctlParams::new();
    params.comment(&format!("strategy={}", strategy.distro_label()));
    strategy.apply(survey, &mut params);
    params.render()
}

#[allow(dead_code)]
pub fn render_auto_detected_sysctl_profile(profile: TcpTuneProfile) -> (TcpTuneSurvey, String) {
    let mut survey = TcpTuneSurvey {
        profile,
        distro: LinuxDistro::Auto,
        ..TcpTuneSurvey::default()
    };
    auto_fill_survey(&mut survey);

    let distro = if survey.distro == LinuxDistro::Auto {
        detect_linux_distro()
    } else {
        survey.distro
    };
    survey.distro = distro;

    let strategy = strategy_for_distro(distro);
    let mut params = SysctlParams::new();
    params.comment(&format!("strategy={}", strategy.distro_label()));
    strategy.apply(&survey, &mut params);
    (survey, params.render())
}

pub fn build_runtime_tune_plan(
    enabled: bool,
    adaptive_system: bool,
    socket_extreme: bool,
    profile: TcpTuneProfile,
) -> RuntimeTunePlan {
    let os = std::env::consts::OS.to_string();
    if !enabled {
        return RuntimeTunePlan {
            enabled: false,
            os,
            distro: LinuxDistro::Auto,
            distro_id: String::new(),
            version_id: String::new(),
            major_version: None,
            profile,
            socket_level: RuntimeSocketTuneLevel::Disabled,
            enabled_features: Vec::new(),
            skipped_features: vec!["runtime.performance.enabled=false".to_string()],
        };
    }

    if os != "linux" {
        return RuntimeTunePlan {
            enabled: true,
            os,
            distro: LinuxDistro::Auto,
            distro_id: String::new(),
            version_id: String::new(),
            major_version: None,
            profile,
            socket_level: RuntimeSocketTuneLevel::Disabled,
            enabled_features: Vec::new(),
            skipped_features: vec![
                "non-Linux host; Linux socket/sysctl strategy skipped".to_string()
            ],
        };
    }

    let details = detect_linux_distro_details();
    let mut enabled_features = Vec::new();
    let mut skipped_features = Vec::new();
    let mut socket_level = RuntimeSocketTuneLevel::PortableLinux;

    if adaptive_system {
        enabled_features.push("Linux TCP_NODELAY/TCP_QUICKACK runtime socket tuning".to_string());
        enabled_features.push("large per-stream send/receive socket buffers".to_string());
    } else {
        socket_level = RuntimeSocketTuneLevel::Disabled;
        skipped_features.push("runtime.performance.adaptive_system=false".to_string());
    }

    if adaptive_system && socket_extreme {
        match details.detected {
            LinuxDistro::Ubuntu2404 => {
                socket_level = RuntimeSocketTuneLevel::Ubuntu24Extreme;
                enabled_features.push(
                    "Ubuntu 24.x extreme socket path: TCP_NOTSENT_LOWAT + TCP_USER_TIMEOUT"
                        .to_string(),
                );
            }
            LinuxDistro::Ubuntu2604 => {
                socket_level = RuntimeSocketTuneLevel::FutureLinuxExtreme;
                enabled_features.push(
                    "Ubuntu 26.x+ future extreme socket path: TCP_NOTSENT_LOWAT + TCP_USER_TIMEOUT"
                        .to_string(),
                );
            }
            LinuxDistro::Ubuntu2204 => {
                skipped_features.push(
                    "Ubuntu 22.x detected; keeping portable Linux path because kernel defaults are older"
                        .to_string(),
                );
            }
            LinuxDistro::Debian12 | LinuxDistro::Debian13 => {
                skipped_features.push(
                    "Debian detected; keeping portable Linux path until distro-specific extreme policy is proven"
                        .to_string(),
                );
            }
            LinuxDistro::Auto => {
                skipped_features
                    .push("unknown Linux distro/version; keeping portable Linux path".to_string());
            }
        }
    } else if !socket_extreme {
        skipped_features.push("runtime.performance.socket_extreme=false".to_string());
    }

    RuntimeTunePlan {
        enabled: true,
        os,
        distro: details.detected,
        distro_id: details.id,
        version_id: details.version_id,
        major_version: details.major_version,
        profile,
        socket_level,
        enabled_features,
        skipped_features,
    }
}

pub fn run_linux_tune(args: LinuxTuneArgs) -> Result<()> {
    let mut survey = TcpTuneSurvey {
        profile: args.profile.into(),
        distro: args.distro.into(),
        memory_gb: args.memory_gb,
        cpu_cores: args.cpu_cores,
        cpu_threads: 0,
        nic_gbps: args.nic_gbps,
        max_connections: args.max_connections,
        l3_cache_kb: 0,
        numa_nodes: 0,
        busy_poll_capable: true,
    };

    auto_fill_survey(&mut survey);

    let distro = if survey.distro == LinuxDistro::Auto {
        detect_linux_distro()
    } else {
        survey.distro
    };
    survey.distro = distro;

    let strategy = strategy_for_distro(distro);
    let mut params = SysctlParams::new();
    params.comment(&format!("strategy={}", strategy.distro_label()));
    strategy.apply(&survey, &mut params);
    let content = params.render();

    println!("proxysss linux tune (hardware-aware)");
    println!("target os       : {}", std::env::consts::OS);
    println!(
        "distro          : {} ({})",
        strategy.distro_label(),
        distro_label(distro)
    );
    println!("profile         : {:?}", survey.profile);
    println!("cpu cores       : {}", survey.cpu_cores);
    println!("memory (GiB)    : {}", survey.memory_gb);
    println!("NIC (Gbps)      : {}", survey.nic_gbps);
    println!("L3 cache (KB)   : {}", survey.l3_cache_kb);
    println!("NUMA nodes      : {}", survey.numa_nodes);
    println!("max connections : {}", survey.max_connections);
    println!("generated sysctl:\n{content}");

    if let Some(output) = args.output {
        std::fs::write(&output, &content)
            .with_context(|| format!("failed writing {}", output.display()))?;
        println!("wrote {}", output.display());
    }

    if !args.apply {
        println!("dry-run only; pass --apply to write /etc/sysctl.d/99-proxysss-tcp.conf on Linux");
        return Ok(());
    }

    if std::env::consts::OS != "linux" {
        anyhow::bail!("--apply is only supported on Linux hosts");
    }

    safe_apply_sysctl_profile(&content, args.unsafe_apply, !args.no_rollback)?;
    Ok(())
}

fn safe_apply_sysctl_profile(content: &str, unsafe_apply: bool, rollback: bool) -> Result<()> {
    if !unsafe_apply {
        print_sysctl_safety_preflight();
    } else {
        println!("warning: --unsafe-apply bypasses SSH/session safety filtering");
    }

    let target = PathBuf::from("/etc/sysctl.d/99-proxysss-tcp.conf");
    let backup = backup_existing_sysctl_profile(&target)?;
    let filtered = if unsafe_apply {
        content.to_string()
    } else {
        filter_supported_sysctl_profile(content)
    };

    std::fs::write(&target, &filtered)
        .with_context(|| format!("failed writing {}", target.display()))?;

    let status = Command::new("sysctl")
        .arg("--system")
        .status()
        .context("failed running sysctl --system")?;
    if status.success() {
        println!("applied {}", target.display());
        if let Some(path) = backup {
            println!("previous profile backup: {}", path.display());
        }
        return Ok(());
    }

    if rollback {
        restore_sysctl_profile(&target, backup.as_ref())?;
        let _ = Command::new("sysctl").arg("--system").status();
        anyhow::bail!("sysctl --system exited with {status}; restored previous profile");
    }

    anyhow::bail!("sysctl --system exited with {status}; rollback disabled");
}

fn print_sysctl_safety_preflight() {
    println!("safety preflight : enabled");
    println!("ssh guard        : does not modify sshd, firewall, routing, rp_filter, or port 22");
    println!("sysctl guard     : unsupported keys and unavailable congestion controls are skipped");
    println!(
        "rollback guard   : previous /etc/sysctl.d/99-proxysss-tcp.conf is restored if apply fails"
    );
    if std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_CLIENT").is_some() {
        println!("ssh session      : detected; using conservative live sysctl apply path");
    }
}

fn backup_existing_sysctl_profile(target: &std::path::Path) -> Result<Option<PathBuf>> {
    if !target.exists() {
        return Ok(None);
    }
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let backup = target.with_extension(format!("conf.bak-{suffix}"));
    std::fs::copy(target, &backup).with_context(|| {
        format!(
            "failed backing up {} to {}",
            target.display(),
            backup.display()
        )
    })?;
    Ok(Some(backup))
}

fn restore_sysctl_profile(target: &std::path::Path, backup: Option<&PathBuf>) -> Result<()> {
    if let Some(backup) = backup {
        std::fs::copy(backup, target).with_context(|| {
            format!(
                "failed restoring {} from {}",
                target.display(),
                backup.display()
            )
        })?;
    } else if target.exists() {
        std::fs::remove_file(target)
            .with_context(|| format!("failed removing {}", target.display()))?;
    }
    Ok(())
}

fn filter_supported_sysctl_profile(content: &str) -> String {
    let mut out = String::with_capacity(content.len() + 512);
    out.push_str("# safety: generated through proxysss guarded sysctl apply\n");
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            out.push_str("# skipped malformed sysctl line: ");
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if !sysctl_key_exists(key) {
            out.push_str("# skipped unsupported sysctl key: ");
            out.push_str(key);
            out.push('\n');
            continue;
        }
        if key == "net.ipv4.tcp_congestion_control" && !congestion_control_available(value) {
            out.push_str("# skipped unavailable congestion control: ");
            out.push_str(value);
            out.push('\n');
            continue;
        }
        if key == "net.ipv4.tcp_abort_on_overflow" && value != "0" {
            out.push_str("# skipped unsafe tcp_abort_on_overflow value: ");
            out.push_str(value);
            out.push('\n');
            continue;
        }
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }
    out
}

fn sysctl_key_exists(key: &str) -> bool {
    Command::new("sysctl")
        .args(["-n", key])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn congestion_control_available(value: &str) -> bool {
    Command::new("sysctl")
        .args(["-n", "net.ipv4.tcp_available_congestion_control"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|available| available.split_whitespace().any(|item| item == value))
        .unwrap_or(false)
}

fn distro_label(d: LinuxDistro) -> &'static str {
    match d {
        LinuxDistro::Auto => "auto-detected",
        LinuxDistro::Ubuntu2204 => "Ubuntu 22.04 LTS",
        LinuxDistro::Ubuntu2404 => "Ubuntu 24.04 LTS",
        LinuxDistro::Ubuntu2604 => "Ubuntu 26.04 LTS",
        LinuxDistro::Debian12 => "Debian 12 (bookworm)",
        LinuxDistro::Debian13 => "Debian 13 (trixie)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_distro_hint_for_ubuntu_24() {
        let survey = TcpTuneSurvey {
            distro: LinuxDistro::Ubuntu2404,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("ubuntu-24.04"));
        assert!(rendered.contains("netdev_budget"));
        assert!(rendered.contains("tcp_max_tw_buckets"));
        assert!(rendered.contains("tcp_no_metrics_save"));
    }

    #[test]
    fn render_includes_distro_hint_for_ubuntu_26() {
        let survey = TcpTuneSurvey {
            distro: LinuxDistro::Ubuntu2604,
            max_connections: 200_000,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("ubuntu-26.04"));
        assert!(rendered.contains("tcp_max_tw_buckets"));
    }

    #[test]
    fn render_includes_debian_12_backlog_override() {
        let survey = TcpTuneSurvey {
            distro: LinuxDistro::Debian12,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("debian-12"));
        assert!(rendered.contains("netdev_max_backlog=32768"));
        assert!(rendered.contains("tcp_max_tw_buckets"));
    }

    #[test]
    fn strategy_factory_returns_correct_labels() {
        let distros = [
            LinuxDistro::Ubuntu2204,
            LinuxDistro::Ubuntu2404,
            LinuxDistro::Ubuntu2604,
            LinuxDistro::Debian12,
            LinuxDistro::Debian13,
            LinuxDistro::Auto,
        ];
        for d in distros {
            let s = strategy_for_distro(d);
            assert!(!s.distro_label().is_empty());
        }
    }

    #[test]
    fn latency_profile_enables_busy_poll() {
        let survey = TcpTuneSurvey {
            profile: TcpTuneProfile::Latency,
            distro: LinuxDistro::Ubuntu2404,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("busy_poll=50"));
        assert!(rendered.contains("busy_read=50"));
    }

    #[test]
    fn high_core_count_scales_budget() {
        let survey = TcpTuneSurvey {
            distro: LinuxDistro::Ubuntu2404,
            cpu_cores: 16,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("netdev_budget=900"));
    }

    #[test]
    fn disabled_runtime_plan_records_skip_reason() {
        let plan = build_runtime_tune_plan(false, true, true, TcpTuneProfile::Edge);
        assert!(!plan.enabled);
        assert_eq!(plan.socket_level, RuntimeSocketTuneLevel::Disabled);
        assert!(plan
            .skipped_features
            .iter()
            .any(|item| item.contains("enabled=false")));
    }

    #[test]
    fn parse_major_version_accepts_ubuntu_style_versions() {
        assert_eq!(parse_major_version("24.04"), Some(24));
        assert_eq!(parse_major_version("22"), Some(22));
        assert_eq!(parse_major_version("rolling"), None);
    }
}
