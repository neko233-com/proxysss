//! Process-local platform tuning. All work happens at startup or socket creation,
//! never per HTTP request, relay chunk or datagram.
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::sync::OnceLock;
use std::time::Duration;

use crate::config::{DesktopPerformanceConfig, RuntimePerformanceConfig};
use crate::linux_tune::{RuntimeSocketTuneLevel, RuntimeTunePlan, TcpTuneProfile};

pub fn validate(config: &RuntimePerformanceConfig, errors: &mut Vec<String>) {
    for (os, settings) in [("windows", &config.windows), ("macos", &config.macos)] {
        for (name, value) in [
            ("tcp_send_buffer_bytes", settings.tcp_send_buffer_bytes),
            (
                "tcp_receive_buffer_bytes",
                settings.tcp_receive_buffer_bytes,
            ),
            ("udp_buffer_bytes", settings.udp_buffer_bytes),
            ("tcp_notsent_lowat_bytes", settings.tcp_notsent_lowat_bytes),
        ] {
            if value > 16 * 1024 * 1024 {
                errors.push(format!("runtime.performance.{os}.{name} must be 0..16777216; zero keeps the OS default"));
            }
        }
        if settings.tcp_keepalive_secs > 86400
            || settings.tcp_keepalive_interval_secs == 0
            || settings.tcp_keepalive_interval_secs > 3600
        {
            errors.push(format!("runtime.performance.{os}: keepalive idle must be 0..86400 seconds and interval 1..3600 seconds"));
        }
    }
    if config.windows.tcp_notsent_lowat_bytes != 0 {
        errors.push("runtime.performance.windows.tcp_notsent_lowat_bytes must be 0; TCP_NOTSENT_LOWAT is only implemented on macOS/Linux".into());
    }
}

pub fn settings_for<'a>(
    config: &'a RuntimePerformanceConfig,
    os: &str,
) -> Option<&'a DesktopPerformanceConfig> {
    let settings = match os {
        "windows" => &config.windows,
        "macos" => &config.macos,
        _ => return None,
    };
    (config.enabled && config.adaptive_system && settings.enabled).then_some(settings)
}

fn io_backend(os: &str) -> &'static str {
    match os {
        "windows" => "IOCP",
        "macos" => "kqueue",
        "linux" => "epoll",
        _ => "portable",
    }
}

pub fn plan(config: &RuntimePerformanceConfig) -> RuntimeTunePlan {
    let profile = match config.profile {
        crate::config::RuntimePerformanceProfile::Edge => TcpTuneProfile::Edge,
        crate::config::RuntimePerformanceProfile::Bulk => TcpTuneProfile::Bulk,
        crate::config::RuntimePerformanceProfile::Latency => TcpTuneProfile::Latency,
    };
    let mut plan = crate::linux_tune::build_runtime_tune_plan(
        config.enabled,
        config.adaptive_system,
        config.socket_extreme,
        profile,
    );
    if matches!(plan.os.as_str(), "windows" | "macos") {
        plan.version_id = native_version();
        plan.skipped_features.clear();
        if let Some(settings) = settings_for(config, &plan.os) {
            plan.socket_level = if plan.os == "windows" {
                RuntimeSocketTuneLevel::WindowsIocp
            } else {
                RuntimeSocketTuneLevel::MacosKqueue
            };
            plan.enabled_features.push(format!(
                "{}: Tokio/Mio native I/O, conservative scheduler; no Linux CPU pinning",
                io_backend(&plan.os)
            ));
            if settings.tcp_send_buffer_bytes == 0 && settings.tcp_receive_buffer_bytes == 0 {
                plan.enabled_features.push("TCP send/receive buffers retain OS autotuning; no per-connection multi-MiB reservation".into());
            }
            probe_settings(settings, &mut plan);
            plan.skipped_features.push("Linux SO_REUSEPORT fanout, QUICKACK, sysctl, splice/sendfile strategies are not transplanted to this OS".into());
        } else {
            plan.skipped_features.push(format!("socket adaptation disabled by runtime.performance.enabled/adaptive_system/{}.enabled", plan.os));
        }
        if config.socket_extreme {
            plan.skipped_features.push(
                "socket_extreme is Linux-only; desktop settings remain explicitly bounded".into(),
            );
        }
    }
    plan
}

pub fn report(config: &RuntimePerformanceConfig) -> serde_json::Value {
    serde_json::json!({
        "plan": plan(config),
        "io_backend": io_backend(std::env::consts::OS),
        "available_cpu_threads": std::thread::available_parallelism().map(usize::from).unwrap_or(1),
        "gateway_runtime_workers": 1,
        "windows": config.windows,
        "macos": config.macos,
        "activation": "启动时应用；runtime.performance 配置变更需要重启。探测只证明当前内核接受参数，不证明吞吐提升。",
        "recommendation": "默认保留 TCP 自动调节；UDP 缓冲只在建 socket 时设置。先用真实混合负载比较吞吐、尾延迟和内存，再增大缓冲。",
        "host_changes": "无注册表、netsh、sysctl、路由、防火墙或系统电源设置写入；Linux 持久调优仍需显式 tune linux --apply。"
    })
}

// Probe unbound sockets: no listener, external connection, or host mutation.
fn probe_settings(settings: &DesktopPerformanceConfig, plan: &mut RuntimeTunePlan) {
    use socket2::{Domain, Protocol, Socket, Type};
    for (kind, protocol) in [(Type::STREAM, Protocol::TCP), (Type::DGRAM, Protocol::UDP)] {
        match Socket::new(Domain::IPV4, kind, Some(protocol)) {
            Ok(socket) => {
                let mut record = |name: &'static str, result: std::io::Result<()>| match result {
                    Ok(()) => plan
                        .enabled_features
                        .push(format!("socket probe accepted: {name}")),
                    Err(error) => plan.skipped_features.push(format!(
                        "socket probe rejected: {name}: {error}; retain OS fallback"
                    )),
                };
                if kind == Type::STREAM {
                    apply_tcp_options(&socket, settings, &mut record);
                } else {
                    apply_udp_options(&socket, settings, &mut record);
                }
                if kind == Type::DGRAM && settings.udp_buffer_bytes > 0 {
                    if let (Ok(receive), Ok(send)) =
                        (socket.recv_buffer_size(), socket.send_buffer_size())
                    {
                        plan.enabled_features.push(format!("UDP buffer requested={} effective_receive={receive} effective_send={send}", settings.udp_buffer_bytes));
                    }
                }
            }
            Err(error) => plan
                .skipped_features
                .push(format!("socket capability probe unavailable: {error}")),
        }
    }
}

fn apply_tcp_options(
    socket: &socket2::Socket,
    settings: &DesktopPerformanceConfig,
    record: &mut impl FnMut(&'static str, std::io::Result<()>),
) {
    if settings.tcp_keepalive_secs > 0 {
        let keepalive = socket2::TcpKeepalive::new()
            .with_time(Duration::from_secs(u64::from(settings.tcp_keepalive_secs)))
            .with_interval(Duration::from_secs(u64::from(
                settings.tcp_keepalive_interval_secs,
            )));
        record("TCP keepalive", socket.set_tcp_keepalive(&keepalive));
    }
    if settings.tcp_send_buffer_bytes > 0 {
        record(
            "TCP SO_SNDBUF",
            socket.set_send_buffer_size(settings.tcp_send_buffer_bytes),
        );
    }
    if settings.tcp_receive_buffer_bytes > 0 {
        record(
            "TCP SO_RCVBUF",
            socket.set_recv_buffer_size(settings.tcp_receive_buffer_bytes),
        );
    }
    #[cfg(target_os = "macos")]
    if settings.tcp_notsent_lowat_bytes > 0 {
        use std::os::fd::AsRawFd;
        let value = settings.tcp_notsent_lowat_bytes as libc::c_uint;
        let result = unsafe {
            libc::setsockopt(
                socket.as_raw_fd(),
                libc::IPPROTO_TCP,
                // Apple XNU bsd/netinet/tcp.h; libc does not export this on Darwin.
                // https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet/tcp.h
                0x201,
                &value as *const _ as *const libc::c_void,
                std::mem::size_of_val(&value) as libc::socklen_t,
            )
        };
        record(
            "macOS TCP_NOTSENT_LOWAT",
            if result == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            },
        );
    }
}

fn apply_udp_options(
    socket: &socket2::Socket,
    settings: &DesktopPerformanceConfig,
    record: &mut impl FnMut(&'static str, std::io::Result<()>),
) {
    if settings.udp_buffer_bytes > 0 {
        record(
            "UDP SO_RCVBUF",
            socket.set_recv_buffer_size(settings.udp_buffer_bytes),
        );
        record(
            "UDP SO_SNDBUF",
            socket.set_send_buffer_size(settings.udp_buffer_bytes),
        );
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
static DESKTOP_SETTINGS: OnceLock<Option<DesktopPerformanceConfig>> = OnceLock::new();

pub fn configure(config: &RuntimePerformanceConfig) {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let _ = DESKTOP_SETTINGS.set(settings_for(config, std::env::consts::OS).cloned());
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = config;
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn socket_result(name: &'static str, result: std::io::Result<()>) {
    // A bounded set of option names, once-only logging; no lock on forwarding.
    static REPORTED: OnceLock<dashmap::DashSet<&'static str>> = OnceLock::new();
    if let Err(error) = result {
        if REPORTED.get_or_init(dashmap::DashSet::new).insert(name) {
            tracing::warn!(option = name, %error, "platform socket tuning failed; retaining OS fallback");
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn tune_tcp(stream: &tokio::net::TcpStream) {
    if let Some(Some(settings)) = DESKTOP_SETTINGS.get() {
        let socket = socket2::SockRef::from(stream);
        apply_tcp_options(&socket, settings, &mut socket_result);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn tune_udp(socket: &tokio::net::UdpSocket) {
    if let Some(Some(settings)) = DESKTOP_SETTINGS.get() {
        let socket = socket2::SockRef::from(socket);
        apply_udp_options(&socket, settings, &mut socket_result);
    }
}

fn native_version() -> String {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "ver"])
            .creation_flags(0x08000000)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|version| !version.is_empty())
            .unwrap_or_else(|| "unknown Windows version".into())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|version| !version.is_empty())
            .unwrap_or_else(|| "unknown macOS version".into())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    "unknown".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_switches_are_independent_and_bounded() {
        let mut config = RuntimePerformanceConfig::default();
        assert!(settings_for(&config, "windows").is_some());
        assert!(settings_for(&config, "macos").is_some());
        config.windows.enabled = false;
        assert!(settings_for(&config, "windows").is_none());
        assert!(settings_for(&config, "macos").is_some());
        config.enabled = false;
        assert!(settings_for(&config, "macos").is_none());
        config.macos.udp_buffer_bytes = 17 * 1024 * 1024;
        let mut errors = Vec::new();
        validate(&config, &mut errors);
        assert!(errors
            .iter()
            .any(|error| error.contains("macos.udp_buffer_bytes")));
    }

    #[test]
    fn zero_options_preserve_native_socket_defaults() {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .unwrap();
        let before = (
            socket.recv_buffer_size().unwrap(),
            socket.send_buffer_size().unwrap(),
            socket.keepalive().unwrap(),
        );
        let settings = DesktopPerformanceConfig {
            tcp_keepalive_secs: 0,
            udp_buffer_bytes: 0,
            ..Default::default()
        };
        apply_tcp_options(&socket, &settings, &mut |_, _| {
            panic!("disabled option must not call setsockopt")
        });
        assert_eq!(
            before,
            (
                socket.recv_buffer_size().unwrap(),
                socket.send_buffer_size().unwrap(),
                socket.keepalive().unwrap()
            )
        );
    }

    #[test]
    fn native_socket_settings_are_applied_and_read_back() {
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .unwrap();
        apply_tcp_options(
            &socket,
            &DesktopPerformanceConfig::default(),
            &mut |name, result| result.unwrap_or_else(|error| panic!("{name}: {error}")),
        );
        assert!(socket.keepalive().unwrap());
        let udp = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        apply_udp_options(
            &udp,
            &DesktopPerformanceConfig::default(),
            &mut |name, result| result.unwrap_or_else(|error| panic!("{name}: {error}")),
        );
        assert!(udp.recv_buffer_size().unwrap() >= 8192);
        assert!(udp.send_buffer_size().unwrap() >= 8192);
    }
}
