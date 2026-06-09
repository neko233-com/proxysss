//! Linux-specific TCP/sysctl tuning profiles for Ubuntu and Debian gateways.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TcpTuneProfile {
    Edge,
    Bulk,
    Latency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinuxDistro {
    Auto,
    Ubuntu2204,
    Ubuntu2404,
    Debian12,
    Debian13,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpTuneSurvey {
    pub profile: TcpTuneProfile,
    pub distro: LinuxDistro,
    pub memory_gb: u32,
    pub cpu_cores: u32,
    pub nic_gbps: u32,
    pub max_connections: u32,
}

impl Default for TcpTuneSurvey {
    fn default() -> Self {
        Self {
            profile: TcpTuneProfile::Edge,
            distro: LinuxDistro::Auto,
            memory_gb: 16,
            cpu_cores: 8,
            nic_gbps: 10,
            max_connections: 20_000,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct LinuxTuneArgs {
    #[arg(long, value_enum, default_value_t = TcpTuneProfileArg::Edge)]
    pub profile: TcpTuneProfileArg,
    #[arg(long, value_enum, default_value_t = LinuxDistroArg::Auto)]
    pub distro: LinuxDistroArg,
    #[arg(long, default_value_t = 16)]
    pub memory_gb: u32,
    #[arg(long, default_value_t = 8)]
    pub cpu_cores: u32,
    #[arg(long, default_value_t = 10)]
    pub nic_gbps: u32,
    #[arg(long, default_value_t = 20000)]
    pub max_connections: u32,
    #[arg(long, default_value_t = false)]
    pub apply: bool,
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
            LinuxDistroArg::Debian12 => Self::Debian12,
            LinuxDistroArg::Debian13 => Self::Debian13,
        }
    }
}

pub fn detect_linux_distro() -> LinuxDistro {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let id = parse_os_release_value(&content, "ID");
    let version_id = parse_os_release_value(&content, "VERSION_ID");

    match (id.as_str(), version_id.as_str()) {
        ("ubuntu", "22.04") => LinuxDistro::Ubuntu2204,
        ("ubuntu", "24.04") => LinuxDistro::Ubuntu2404,
        ("debian", "12") => LinuxDistro::Debian12,
        ("debian", "13") => LinuxDistro::Debian13,
        ("ubuntu", _) if version_id.starts_with("22.") => LinuxDistro::Ubuntu2204,
        ("ubuntu", _) if version_id.starts_with("24.") => LinuxDistro::Ubuntu2404,
        ("debian", _) if version_id.starts_with("12") => LinuxDistro::Debian12,
        ("debian", _) if version_id.starts_with("13") => LinuxDistro::Debian13,
        _ => LinuxDistro::Auto,
    }
}

fn parse_os_release_value(content: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| {
            line.trim_start_matches(&prefix)
                .trim_matches('"')
                .to_string()
        })
        .unwrap_or_default()
}

pub fn render_linux_tcp_sysctl_profile(survey: &TcpTuneSurvey) -> String {
    let connection_factor = survey.max_connections.max(1024);
    let somaxconn = (connection_factor / 4).clamp(4096, 65_535);
    let syn_backlog = (connection_factor / 2).clamp(4096, 262_144);
    let netdev_backlog = (survey.nic_gbps.saturating_mul(8192)).clamp(8192, 262_144);
    let memory_factor = survey.memory_gb.max(4) * 1024 * 1024;
    let rmem_max = (memory_factor * 2).clamp(16 * 1024 * 1024, 256 * 1024 * 1024);
    let wmem_max = rmem_max;
    let fin_timeout = match survey.profile {
        TcpTuneProfile::Latency => 10,
        TcpTuneProfile::Bulk => 20,
        TcpTuneProfile::Edge => 15,
    };
    let keepalive_time = match survey.profile {
        TcpTuneProfile::Latency => 60,
        TcpTuneProfile::Bulk => 120,
        TcpTuneProfile::Edge => 90,
    };
    let busy_poll = if matches!(survey.profile, TcpTuneProfile::Latency) {
        50
    } else {
        0
    };

    let distro = if survey.distro == LinuxDistro::Auto {
        detect_linux_distro()
    } else {
        survey.distro
    };

    let (distro_label, extra_lines) = distro_overrides(distro, survey);

    format!(
        "# proxysss linux tcp tuning\n# profile={} distro={} memory_gb={} cpu_cores={} nic_gbps={} max_connections={}\nnet.core.somaxconn={}\nnet.ipv4.tcp_max_syn_backlog={}\nnet.core.netdev_max_backlog={}\nnet.core.rmem_max={}\nnet.core.wmem_max={}\nnet.ipv4.tcp_rmem=4096 87380 {}\nnet.ipv4.tcp_wmem=4096 65536 {}\nnet.ipv4.ip_local_port_range=10240 65535\nnet.ipv4.tcp_fin_timeout={}\nnet.ipv4.tcp_tw_reuse=1\nnet.ipv4.tcp_keepalive_time={}\nnet.ipv4.tcp_keepalive_intvl=15\nnet.ipv4.tcp_keepalive_probes=5\nnet.ipv4.tcp_mtu_probing=1\nnet.ipv4.tcp_fastopen=3\nnet.ipv4.tcp_slow_start_after_idle=0\nnet.ipv4.tcp_window_scaling=1\nnet.core.default_qdisc=fq\nnet.ipv4.tcp_congestion_control=bbr\nnet.core.busy_poll={}\nnet.core.busy_read={}\n{extra_lines}",
        match survey.profile {
            TcpTuneProfile::Edge => "edge",
            TcpTuneProfile::Bulk => "bulk",
            TcpTuneProfile::Latency => "latency",
        },
        distro_label,
        survey.memory_gb,
        survey.cpu_cores,
        survey.nic_gbps,
        survey.max_connections,
        somaxconn,
        syn_backlog,
        netdev_backlog,
        rmem_max,
        wmem_max,
        rmem_max,
        wmem_max,
        fin_timeout,
        keepalive_time,
        busy_poll,
        busy_poll,
    )
}

fn distro_overrides(distro: LinuxDistro, survey: &TcpTuneSurvey) -> (&'static str, String) {
    match distro {
        LinuxDistro::Ubuntu2204 => (
            "ubuntu-22.04",
            "# Ubuntu 22.04 LTS: kernel 5.15+, fq + bbr stable\nvm.swappiness=10\n".to_string(),
        ),
        LinuxDistro::Ubuntu2404 => (
            "ubuntu-24.04",
            "# Ubuntu 24.04 LTS: enable busy polling for edge latency when profile allows\nnet.core.netdev_budget=600\nnet.core.netdev_budget_usecs=8000\n".to_string(),
        ),
        LinuxDistro::Debian12 => (
            "debian-12",
            "# Debian 12 (bookworm): conservative backlog for mixed workloads\nnet.core.netdev_max_backlog=16384\n".to_string(),
        ),
        LinuxDistro::Debian13 => (
            "debian-13",
            "# Debian 13 (trixie): align with newer default qdisc\nnet.core.default_qdisc=fq_codel\n".to_string(),
        ),
        LinuxDistro::Auto => (
            "auto",
            if survey.cpu_cores >= 16 {
                "net.core.netdev_budget=600\n".to_string()
            } else {
                String::new()
            },
        ),
    }
}

pub fn run_linux_tune(args: LinuxTuneArgs) -> Result<()> {
    let survey = TcpTuneSurvey {
        profile: args.profile.into(),
        distro: args.distro.into(),
        memory_gb: args.memory_gb,
        cpu_cores: args.cpu_cores,
        nic_gbps: args.nic_gbps,
        max_connections: args.max_connections,
    };

    let content = render_linux_tcp_sysctl_profile(&survey);
    println!("proxysss linux tune");
    println!("target os       : {}", std::env::consts::OS);
    println!("distro          : {:?}", survey.distro);
    println!("profile         : {:?}", survey.profile);
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

    let target = PathBuf::from("/etc/sysctl.d/99-proxysss-tcp.conf");
    std::fs::write(&target, &content)
        .with_context(|| format!("failed writing {}", target.display()))?;
    let status = std::process::Command::new("sysctl")
        .arg("--system")
        .status()
        .context("failed running sysctl --system")?;
    if !status.success() {
        anyhow::bail!("sysctl --system exited with {status}");
    }
    println!("applied {}", target.display());
    Ok(())
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
    }

    #[test]
    fn render_includes_debian_12_backlog_override() {
        let survey = TcpTuneSurvey {
            distro: LinuxDistro::Debian12,
            ..TcpTuneSurvey::default()
        };
        let rendered = render_linux_tcp_sysctl_profile(&survey);
        assert!(rendered.contains("debian-12"));
        assert!(rendered.contains("netdev_max_backlog=16384"));
    }
}
