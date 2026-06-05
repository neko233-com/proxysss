mod bench;
mod config;
mod demo;
mod gateway;
mod install;
mod script;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use config::{GatewayConfig, LogFormat, LoggingConfig};
use reqwest::Method;
use serde::Serialize;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[derive(Parser, Debug)]
#[command(name = "proxysss")]
#[command(about = "Programmable Rust gateway with TS/JS routing scripts")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Update {
        #[arg(long, default_value = "latest")]
        version: String,
        #[arg(long, default_value_t = false)]
        no_service_restart: bool,
        #[arg(long, default_value_t = false)]
        skip_init: bool,
    },
    SwitchVersion {
        version: String,
        #[arg(long, default_value_t = false)]
        allow_downgrade: bool,
        #[arg(long, default_value_t = false)]
        no_service_restart: bool,
        #[arg(long, default_value_t = false)]
        skip_init: bool,
    },
    Start,
    Stop,
    Enable {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Disable,
    Status,
    CheckConfig {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Init {
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    CertBootstrap {
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    Service {
        #[command(subcommand)]
        action: ServiceCommands,
    },
    Bench {
        #[command(subcommand)]
        protocol: bench::BenchCommand,
    },
    Demo {
        #[command(subcommand)]
        kind: demo::DemoCommand,
    },
    PrintDefaultConfig {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
        #[arg(long, global = true)]
        config: Option<PathBuf>,
    },
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        admin_url: Option<String>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConfigOutputFormat {
    Yaml,
    Json,
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    Show {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    Includes,
    WatchedScripts,
    Routes,
    ReloadPlan,
    NginxParity {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    Explain,
    Capabilities,
}

#[derive(Subcommand, Debug)]
enum ServiceCommands {
    Install {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Uninstall,
    Start,
    Stop,
    Status,
}

#[derive(Subcommand, Debug)]
enum PluginCommands {
    List,
    Load {
        #[arg(long)]
        name: String,
        #[arg(long)]
        module_path: String,
        #[arg(long, default_value_t = 0)]
        priority: i32,
        #[arg(long, default_value_t = true)]
        enabled: bool,
    },
    Unload {
        #[arg(long)]
        name: String,
    },
}

struct AdminClientContext {
    base_url: String,
    username: String,
    password: String,
}

const CAPABILITY_MATRIX: &[(&str, &str)] = &[
    (
        "http reverse proxy",
        "built-in services.reverse_proxy routes with host/path matching, upstream pools, and strip_prefix",
    ),
    ("https/http2 termination", "supported"),
    ("http3/quic", "supported"),
    ("websocket/ws/wss", "supported"),
    ("tcp stream proxy", "supported"),
    ("udp stream proxy", "supported"),
    (
        "static files",
        "built-in services.static_sites runtime for GET/HEAD, index files, and optional autoindex",
    ),
    ("ftp", "supported via services.ftp TCP passthrough"),
    (
        "webdav",
        "built-in services.webdav runtime for OPTIONS/PROPFIND/GET/HEAD/PUT/DELETE/MKCOL/COPY/MOVE",
    ),
    (
        "explicit sub-config",
        "supported via include.enabled + include.files",
    ),
    (
        "hot reload",
        "configuration, explicit includes, main script, and auto-loaded plugins are fingerprinted",
    ),
    (
        "logging levels",
        "debug/info/warn/error with info as default, debug reserved for internal diagnostics, file sinks at logs/access.log and logs/error.log",
    ),
    (
        "auto https",
        "proxysss YAML style http.tls.auto_https expands to ACME external certificate issue/renew",
    ),
    (
        "admin api/console",
        "supported on 127.0.0.1:7777 by default",
    ),
    (
        "agent install skill",
        "see skills/proxysss-install/SKILL.md",
    ),
];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ParityStatus {
    Supported,
    Partial,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct NginxParityItem {
    capability: &'static str,
    status: ParityStatus,
    evidence: &'static str,
    next_gap: &'static str,
}

const NGINX_PARITY_MATRIX: &[NginxParityItem] = &[
    NginxParityItem {
        capability: "default HTTP port 80",
        status: ParityStatus::Supported,
        evidence: "GatewayConfig default http.plain_bind=0.0.0.0:80",
        next_gap: "",
    },
    NginxParityItem {
        capability: "default HTTPS/HTTP3 port 443",
        status: ParityStatus::Supported,
        evidence: "GatewayConfig default http.tls_bind/http.h3_bind=0.0.0.0:443",
        next_gap: "",
    },
    NginxParityItem {
        capability: "declarative reverse proxy",
        status: ParityStatus::Supported,
        evidence: "services.reverse_proxy.routes with host/path/upstream pool matching",
        next_gap: "",
    },
    NginxParityItem {
        capability: "static file service",
        status: ParityStatus::Supported,
        evidence: "services.static_sites supports GET/HEAD, index files, autoindex",
        next_gap: "",
    },
    NginxParityItem {
        capability: "WebDAV",
        status: ParityStatus::Supported,
        evidence: "built-in OPTIONS/PROPFIND/GET/HEAD/PUT/DELETE/MKCOL/COPY/MOVE",
        next_gap: "",
    },
    NginxParityItem {
        capability: "TCP/UDP stream proxy",
        status: ParityStatus::Supported,
        evidence: "tcp.listeners and udp.listeners support YAML upstream/upstreams with optional script extension hooks",
        next_gap: "",
    },
    NginxParityItem {
        capability: "FTP",
        status: ParityStatus::Partial,
        evidence: "services.ftp TCP passthrough listener",
        next_gap: "native FTP control/passive data channel awareness",
    },
    NginxParityItem {
        capability: "TLS certificates",
        status: ParityStatus::Partial,
        evidence: "self_signed/manual/acme_external modes plus proxysss YAML auto_https sugar",
        next_gap: "first-class multi-cert/SNI certificate selection",
    },
    NginxParityItem {
        capability: "access/error logging",
        status: ParityStatus::Supported,
        evidence:
            "access tracing, debug/info/warn/error levels, logs/access.log and logs/error.log sinks",
        next_gap: "",
    },
    NginxParityItem {
        capability: "hot reload",
        status: ParityStatus::Supported,
        evidence: "reload fingerprint covers config, includes, main script, auto-loaded plugins",
        next_gap: "",
    },
    NginxParityItem {
        capability: "compression",
        status: ParityStatus::Missing,
        evidence: "no gzip/brotli response filter yet",
        next_gap: "add configurable gzip/brotli response compression",
    },
    NginxParityItem {
        capability: "cache/proxy cache",
        status: ParityStatus::Missing,
        evidence: "no on-disk or memory proxy cache yet",
        next_gap: "add proxy cache zones and cache key policy",
    },
    NginxParityItem {
        capability: "rate limiting",
        status: ParityStatus::Partial,
        evidence: "services.rate_limit.http fixed-window request limiter",
        next_gap: "add connection limiting and shared-zone style policies",
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run { config: None }) {
        Commands::Run { config } => {
            let config_path = install::resolve_run_config_path(config)?;
            let gateway_config = GatewayConfig::load(&config_path)?;

            init_logging(&gateway_config.logging, &gateway_config.root_dir)?;
            emit_startup_banner(&config_path, &gateway_config);
            for warning in gateway_config.warnings() {
                tracing::warn!(warning, "configuration warning");
            }

            tracing::info!(config = %config_path.display(), "starting gateway");
            gateway::Gateway::from_config(config_path, gateway_config)
                .await?
                .run()
                .await
        }
        Commands::Update {
            version,
            no_service_restart,
            skip_init,
        } => run_installer_command("update", &version, false, no_service_restart, skip_init),
        Commands::SwitchVersion {
            version,
            allow_downgrade,
            no_service_restart,
            skip_init,
        } => run_installer_command(
            "install",
            &version,
            allow_downgrade,
            no_service_restart,
            skip_init,
        ),
        Commands::Start => {
            init_cli_logging();
            install::start_service()
        }
        Commands::Stop => {
            init_cli_logging();
            install::stop_service()
        }
        Commands::Enable { config } => {
            init_cli_logging();
            install::install_service(config)
        }
        Commands::Disable => {
            init_cli_logging();
            install::uninstall_service()
        }
        Commands::Status => {
            init_cli_logging();
            install::service_status()
        }
        Commands::CheckConfig { config } => {
            init_cli_logging();

            let config_path = install::resolve_run_config_path(config)?;
            let gateway_config = GatewayConfig::load(&config_path)?;
            println!("configuration check passed: {}", config_path.display());
            for warning in gateway_config.warnings() {
                println!("warning: {warning}");
            }
            Ok(())
        }
        Commands::Init { dir, overwrite } => {
            init_cli_logging();
            install::init_layout(dir, overwrite)
        }
        Commands::CertBootstrap { dir, overwrite } => {
            init_cli_logging();
            install::bootstrap_certs_in_dir(dir, overwrite)
        }
        Commands::Service { action } => match action {
            ServiceCommands::Install { config } => {
                init_cli_logging();
                install::install_service(config)
            }
            ServiceCommands::Uninstall => {
                init_cli_logging();
                install::uninstall_service()
            }
            ServiceCommands::Start => {
                init_cli_logging();
                install::start_service()
            }
            ServiceCommands::Stop => {
                init_cli_logging();
                install::stop_service()
            }
            ServiceCommands::Status => {
                init_cli_logging();
                install::service_status()
            }
        },
        Commands::Bench { protocol } => {
            init_cli_logging();
            bench::run(protocol).await
        }
        Commands::Demo { kind } => {
            init_cli_logging();
            demo::run(kind).await
        }
        Commands::PrintDefaultConfig { format } => {
            match format {
                ConfigOutputFormat::Yaml => {
                    print!(
                        "{}",
                        config::GatewayConfig::render_default_yaml(&install::preferred_script_command())
                    )
                }
                ConfigOutputFormat::Json => {
                    print!(
                        "{}",
                        config::GatewayConfig::render_default_json(&install::preferred_script_command())
                    )
                }
            }
            Ok(())
        }
        Commands::Config { action, config } => {
            init_cli_logging();
            match action {
                ConfigCommands::Show { format } => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    match format {
                        ConfigOutputFormat::Yaml => {
                            print!("{}", serde_yaml::to_string(&gateway_config)?)
                        }
                        ConfigOutputFormat::Json => {
                            print!("{}", serde_json::to_string_pretty(&gateway_config)?)
                        }
                    }
                    Ok(())
                }
                ConfigCommands::Includes => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    println!("config: {}", config_path.display());
                    println!("include.enabled: {}", gateway_config.include.enabled);
                    println!("include.required: {}", gateway_config.include.required);
                    if gateway_config.include.files.is_empty() {
                        println!("include.files: []");
                    } else {
                        println!("include.files:");
                        for file in &gateway_config.include.files {
                            println!(" - {}", file.display());
                        }
                    }
                    Ok(())
                }
                ConfigCommands::WatchedScripts => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    let paths = gateway::watched_script_paths(&gateway_config);
                    println!("config: {}", config_path.display());
                    println!(
                        "hot_reload.enabled: {}",
                        gateway_config.runtime.hot_reload.enabled
                    );
                    if paths.is_empty() {
                        println!("watched_scripts: []");
                    } else {
                        println!("watched_scripts:");
                        for path in paths {
                            println!(" - {}", path.display());
                        }
                    }
                    Ok(())
                }
                ConfigCommands::Routes => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    print!("{}", render_route_topology(&gateway_config));
                    Ok(())
                }
                ConfigCommands::ReloadPlan => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    print!("{}", render_reload_plan(&gateway_config));
                    Ok(())
                }
                ConfigCommands::NginxParity { format } => {
                    match format {
                        ConfigOutputFormat::Yaml => {
                            print!("{}", serde_yaml::to_string(NGINX_PARITY_MATRIX)?)
                        }
                        ConfigOutputFormat::Json => {
                            print!("{}", serde_json::to_string_pretty(NGINX_PARITY_MATRIX)?)
                        }
                    }
                    Ok(())
                }
                ConfigCommands::Explain => {
                    let config_path = install::resolve_run_config_path(config)?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    print_config_explain(&config_path, &gateway_config);
                    Ok(())
                }
                ConfigCommands::Capabilities => {
                    print_capabilities();
                    Ok(())
                }
            }
        }
        Commands::Plugin {
            action,
            config,
            admin_url,
            username,
            password,
        } => {
            init_cli_logging();
            let admin = resolve_admin_context(config, admin_url, username, password)?;
            let client = reqwest::Client::new();

            match action {
                PluginCommands::List => {
                    let payload =
                        admin_request_json(&client, &admin, Method::GET, "/v1/plugins", None)
                            .await?;
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    Ok(())
                }
                PluginCommands::Load {
                    name,
                    module_path,
                    priority,
                    enabled,
                } => {
                    let module_path = normalize_plugin_module_path(&module_path)?;
                    let body = json!({
                        "name": name,
                        "module_path": module_path,
                        "priority": priority,
                        "enabled": enabled,
                        "config": serde_json::Value::Null,
                    });
                    let payload = admin_request_json(
                        &client,
                        &admin,
                        Method::POST,
                        "/v1/plugins/load",
                        Some(body),
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    Ok(())
                }
                PluginCommands::Unload { name } => {
                    let body = json!({ "name": name });
                    let payload = admin_request_json(
                        &client,
                        &admin,
                        Method::POST,
                        "/v1/plugins/unload",
                        Some(body),
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                    Ok(())
                }
            }
        }
    }
}

fn print_config_explain(config_path: &std::path::Path, config: &GatewayConfig) {
    println!("proxysss config summary");
    println!("config path       : {}", config_path.display());
    println!(
        "http plain        : {}",
        blank_as_disabled(&config.http.plain_bind)
    );
    println!(
        "https/http2       : {}",
        blank_as_disabled(&config.http.tls_bind)
    );
    println!(
        "http3             : {}",
        blank_as_disabled(&config.http.h3_bind)
    );
    println!(
        "admin             : {}",
        if config.admin.enabled {
            &config.admin.bind
        } else {
            "disabled"
        }
    );
    println!(
        "include           : enabled={}, required={}, files={}",
        config.include.enabled,
        config.include.required,
        config.include.files.len()
    );
    println!(
        "tcp listeners     : {}",
        config.tcp.listeners.len() + usize::from(config.services.ftp.enabled)
    );
    println!("udp listeners     : {}", config.udp.listeners.len());
    println!(
        "logging           : level={:?}, format={:?}, access_log={}, access_log_path={}, error_log_path={}",
        config.logging.level,
        config.logging.format,
        config.logging.access_log,
        config.logging.access_log_path.display(),
        config.logging.error_log_path.display()
    );
    println!(
        "auto https        : enabled={}, domains={}, production={}",
        config.http.tls.auto_https.enabled,
        if config.http.tls.auto_https.domains.is_empty() {
            "[]".to_string()
        } else {
            config.http.tls.auto_https.domains.join(",")
        },
        config.http.tls.auto_https.production
    );
    println!(
        "reverse proxy     : routes={}",
        config.services.reverse_proxy.routes.len()
    );
    println!(
        "rate limit        : http.enabled={}, requests={}, window_ms={}, burst={}",
        config.services.rate_limit.http.enabled,
        config.services.rate_limit.http.requests,
        config.services.rate_limit.http.window_ms,
        config.services.rate_limit.http.burst
    );
    println!("static sites      : {}", config.services.static_sites.len());
    println!(
        "webdav            : enabled={}, prefix={}, root={}",
        config.services.webdav.enabled,
        config.services.webdav.path_prefix,
        config.services.webdav.root.display()
    );
    println!(
        "ftp               : enabled={}, bind={}, upstream={}",
        config.services.ftp.enabled, config.services.ftp.bind, config.services.ftp.upstream
    );
    println!(
        "script            : enabled={}, command={}, args={}",
        config.script.enabled,
        config.script.command,
        config.script.args.join(" ")
    );
}

fn render_route_topology(config: &GatewayConfig) -> String {
    let mut output = String::new();
    output.push_str("proxysss route topology\n");

    output.push_str("[http.listeners]\n");
    output.push_str(&format!(
        "plain={}\n",
        blank_as_disabled(&config.http.plain_bind)
    ));
    output.push_str(&format!(
        "tls={}\n",
        blank_as_disabled(&config.http.tls_bind)
    ));
    output.push_str(&format!("h3={}\n", blank_as_disabled(&config.http.h3_bind)));

    output.push_str("[reverse_proxy]\n");
    if config.services.reverse_proxy.routes.is_empty() {
        output.push_str("none\n");
    } else {
        for route in &config.services.reverse_proxy.routes {
            output.push_str(&format!(
                "{} hosts={} path={} upstream={} upstreams={} strip_prefix={}\n",
                route.name,
                if route.hosts.is_empty() {
                    "*".to_string()
                } else {
                    route.hosts.join(",")
                },
                route.path_prefix,
                route.upstream,
                route.upstreams.len(),
                route.strip_prefix
            ));
        }
    }

    output.push_str("[rate_limit]\n");
    output.push_str(&format!(
        "http enabled={} requests={} window_ms={} burst={} status={}\n",
        config.services.rate_limit.http.enabled,
        config.services.rate_limit.http.requests,
        config.services.rate_limit.http.window_ms,
        config.services.rate_limit.http.burst,
        config.services.rate_limit.http.status
    ));

    output.push_str("[static_sites]\n");
    if config.services.static_sites.is_empty() {
        output.push_str("none\n");
    } else {
        for site in &config.services.static_sites {
            output.push_str(&format!(
                "{} path={} root={} autoindex={}\n",
                site.name,
                site.path_prefix,
                site.root.display(),
                site.autoindex
            ));
        }
    }

    output.push_str("[webdav]\n");
    if config.services.webdav.enabled {
        output.push_str(&format!(
            "path={} root={} allow_write={}\n",
            config.services.webdav.path_prefix,
            config.services.webdav.root.display(),
            config.services.webdav.allow_write
        ));
    } else {
        output.push_str("disabled\n");
    }

    output.push_str("[tcp]\n");
    if config.tcp.listeners.is_empty() && !config.services.ftp.enabled {
        output.push_str("none\n");
    } else {
        for listener in &config.tcp.listeners {
            output.push_str(&format!(
                "{} bind={} upstream={} upstreams={}\n",
                listener.name,
                listener.bind,
                blank_as_disabled(&listener.upstream),
                listener.upstreams.len()
            ));
        }
        if config.services.ftp.enabled {
            output.push_str(&format!(
                "ftp bind={} upstream={}\n",
                config.services.ftp.bind, config.services.ftp.upstream
            ));
        }
    }

    output.push_str("[udp]\n");
    if config.udp.listeners.is_empty() {
        output.push_str("none\n");
    } else {
        for listener in &config.udp.listeners {
            output.push_str(&format!(
                "{} bind={} upstream={} upstreams={}\n",
                listener.name,
                listener.bind,
                blank_as_disabled(&listener.upstream),
                listener.upstreams.len()
            ));
        }
    }

    output
}

fn render_reload_plan(config: &GatewayConfig) -> String {
    let mut output = String::new();
    output.push_str("proxysss reload plan\n");
    output.push_str(&format!(
        "hot_reload.enabled={}\n",
        config.runtime.hot_reload.enabled
    ));
    output.push_str(&format!(
        "hot_reload.interval_ms={}\n",
        config.runtime.hot_reload.interval_ms
    ));
    output.push_str("[hot_reload]\n");
    output.push_str("configuration values except listener identity\n");
    output.push_str("explicit include files from include.files\n");
    output.push_str("main extension script from script.args\n");
    output.push_str("auto-loaded plugin scripts from plugins.auto_load_dir\n");
    output.push_str("reverse_proxy routes\n");
    output.push_str("static_sites\n");
    output.push_str("webdav settings\n");
    output.push_str("ftp upstream when services.ftp listener identity is unchanged\n");

    output.push_str("[restart_required]\n");
    output.push_str("http.plain_bind/http.tls_bind/http.h3_bind\n");
    output.push_str("admin.enabled/admin.bind\n");
    output.push_str("tcp listener name/bind set\n");
    output.push_str("udp listener name/bind set\n");
    output.push_str("services.ftp.enabled/services.ftp.bind\n");
    output.push_str("http.tls.mode\n");
    output.push_str("logging.format/logging.filter/logging.level\n");
    output.push_str("logging.access_log_path/logging.error_log_path\n");

    output
}

static LOG_GUARDS: OnceLock<Vec<tracing_appender::non_blocking::WorkerGuard>> = OnceLock::new();

fn print_capabilities() {
    println!("proxysss capability matrix");
    for (name, status) in CAPABILITY_MATRIX {
        println!("{name:<25}: {status}");
    }
}

fn blank_as_disabled(value: &str) -> &str {
    if value.trim().is_empty() {
        "disabled"
    } else {
        value
    }
}

fn run_installer_command(
    action: &str,
    version: &str,
    allow_downgrade: bool,
    no_service_restart: bool,
    skip_init: bool,
) -> Result<()> {
    init_cli_logging();
    let script_url = match std::env::consts::OS {
        "windows" => {
            "https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1"
        }
        "linux" | "macos" => {
            "https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh"
        }
        os => return Err(anyhow::anyhow!("unsupported update os {os}")),
    };

    if std::env::consts::OS == "windows" {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "& ([ScriptBlock]::Create((irm '{}'))) -Action {} -Version {}{}{}{}",
                script_url,
                powershell_escape_arg(action),
                powershell_escape_arg(version),
                if allow_downgrade {
                    " -AllowDowngrade"
                } else {
                    ""
                },
                if no_service_restart {
                    " -NoServiceRestart"
                } else {
                    ""
                },
                if skip_init { " -SkipInit" } else { "" },
            ),
        ]);
        return run_inherited(command, "run Windows installer");
    }

    let mut command = Command::new("sh");
    command.arg("-c").arg(format!(
        "curl -fsSL '{}' | bash -s -- --action {} --version {}{}{}{}",
        script_url,
        sh_escape_arg(action),
        sh_escape_arg(version),
        if allow_downgrade {
            " --allow-downgrade"
        } else {
            ""
        },
        if no_service_restart {
            " --no-service-restart"
        } else {
            ""
        },
        if skip_init { " --skip-init" } else { "" },
    ));
    run_inherited(command, "run Unix installer")
}

fn powershell_escape_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn sh_escape_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

fn run_inherited(mut command: Command, description: &str) -> Result<()> {
    let status = command
        .status()
        .map_err(|error| anyhow::anyhow!("failed to {description}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{description} failed with {status}"))
    }
}

fn normalize_plugin_module_path(value: &str) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with("file:") {
        return Ok(value.to_string());
    }

    let input = PathBuf::from(value);
    if input.is_absolute() {
        Ok(input.to_string_lossy().to_string())
    } else {
        Ok(std::env::current_dir()?
            .join(input)
            .to_string_lossy()
            .to_string())
    }
}

fn resolve_admin_context(
    config: Option<PathBuf>,
    admin_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<AdminClientContext> {
    let loaded_config = if admin_url.is_none() || username.is_none() || password.is_none() {
        let config_path = install::resolve_run_config_path(config)?;
        Some(GatewayConfig::load(&config_path)?)
    } else {
        None
    };

    let base_url = admin_url
        .or_else(|| {
            loaded_config
                .as_ref()
                .map(|cfg| format!("http://{}", cfg.admin.bind))
        })
        .ok_or_else(|| anyhow::anyhow!("admin url not provided and config unavailable"))?;

    let username = username
        .or_else(|| loaded_config.as_ref().map(|cfg| cfg.admin.username.clone()))
        .ok_or_else(|| anyhow::anyhow!("admin username not provided and config unavailable"))?;

    let password = password
        .or_else(|| loaded_config.as_ref().map(|cfg| cfg.admin.password.clone()))
        .ok_or_else(|| anyhow::anyhow!("admin password not provided and config unavailable"))?;

    Ok(AdminClientContext {
        base_url: normalize_admin_base_url(&base_url),
        username,
        password,
    })
}

fn normalize_admin_base_url(value: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") {
        value.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", value.trim_end_matches('/'))
    }
}

async fn admin_request_json(
    client: &reqwest::Client,
    admin: &AdminClientContext,
    method: Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let url = format!("{}{}", admin.base_url, path);
    let mut request = client
        .request(method, &url)
        .basic_auth(admin.username.clone(), Some(admin.password.clone()))
        .header(reqwest::header::CONTENT_TYPE, "application/json");

    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    let payload: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "admin request failed with {}: {}",
            status,
            payload
        ));
    }

    Ok(payload)
}

fn init_cli_logging() {
    let logging = LoggingConfig {
        access_log_path: PathBuf::new(),
        error_log_path: PathBuf::new(),
        ..LoggingConfig::default()
    };
    let _ = init_logging(&logging, Path::new("."));
}

fn init_logging(logging: &LoggingConfig, root_dir: &Path) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(logging.filter.as_str()));

    let mut guards = Vec::new();
    let use_access_file = logging.access_log && !logging.access_log_path.as_os_str().is_empty();
    let use_error_file = !logging.error_log_path.as_os_str().is_empty();

    let access_writer = if use_access_file {
        Some(open_non_blocking_log_writer(
            &logging.access_log_path,
            root_dir,
            &mut guards,
        )?)
    } else {
        None
    };
    let error_writer = if use_error_file {
        Some(open_non_blocking_log_writer(
            &logging.error_log_path,
            root_dir,
            &mut guards,
        )?)
    } else {
        None
    };

    let access_filter =
        tracing_subscriber::filter::filter_fn(|metadata| metadata.target() == "access");
    let error_filter = tracing_subscriber::filter::filter_fn(|metadata| {
        metadata.level() == &tracing::Level::WARN || metadata.level() == &tracing::Level::ERROR
    });

    match (logging.format, access_writer, error_writer) {
        (LogFormat::Plain, Some(access), Some(error)) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(access)
                        .with_filter(access_filter),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(error)
                        .with_filter(error_filter),
                )
                .init();
        }
        (LogFormat::Plain, Some(access), None) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(access)
                        .with_filter(access_filter),
                )
                .init();
        }
        (LogFormat::Plain, None, Some(error)) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(error)
                        .with_filter(error_filter),
                )
                .init();
        }
        (LogFormat::Plain, None, None) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
        (LogFormat::Json, Some(access), Some(error)) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(access)
                        .with_filter(access_filter),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(error)
                        .with_filter(error_filter),
                )
                .init();
        }
        (LogFormat::Json, Some(access), None) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(access)
                        .with_filter(access_filter),
                )
                .init();
        }
        (LogFormat::Json, None, Some(error)) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(error)
                        .with_filter(error_filter),
                )
                .init();
        }
        (LogFormat::Json, None, None) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
    }

    let _ = LOG_GUARDS.set(guards);
    Ok(())
}

fn open_non_blocking_log_writer(
    path: &Path,
    root_dir: &Path,
    guards: &mut Vec<tracing_appender::non_blocking::WorkerGuard>,
) -> Result<tracing_appender::non_blocking::NonBlocking> {
    let writer = open_log_writer(path, root_dir)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);
    guards.push(guard);
    Ok(non_blocking)
}

fn open_log_writer(path: &Path, root_dir: &Path) -> Result<std::fs::File> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root_dir.join(path)
    };
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating log directory {}", parent.display()))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&resolved)
        .with_context(|| format!("failed opening log file {}", resolved.display()))
}

fn emit_startup_banner(config_path: &std::path::Path, config: &GatewayConfig) {
    let version = env!("CARGO_PKG_VERSION");
    let plain_bind = if config.http.plain_bind.trim().is_empty() {
        "disabled".to_string()
    } else {
        format!("http://{}", config.http.plain_bind)
    };
    let tls_bind = if config.http.tls_bind.trim().is_empty() {
        "disabled".to_string()
    } else {
        format!("https://{}", config.http.tls_bind)
    };
    let h3_bind = if config.http.h3_bind.trim().is_empty() {
        "disabled".to_string()
    } else {
        format!("h3://{}", config.http.h3_bind)
    };
    let admin_bind = if config.admin.enabled {
        format!("http://{}", config.admin.bind)
    } else {
        "disabled".to_string()
    };

    if matches!(config.logging.format, LogFormat::Plain) {
        println!(
            r#"
██████╗ ██████╗  ██████╗ ██╗  ██╗██╗   ██╗███████╗███████╗
██╔══██╗██╔══██╗██╔═══██╗╚██╗██╔╝╚██╗ ██╔╝██╔════╝██╔════╝
██████╔╝██████╔╝██║   ██║ ╚███╔╝  ╚████╔╝ ███████╗███████╗
██╔═══╝ ██╔══██╗██║   ██║ ██╔██╗   ╚██╔╝  ╚════██║╚════██║
██║     ██║  ██║╚██████╔╝██╔╝ ██╗   ██║   ███████║███████║
╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚══════╝

proxysss v{version}
config  : {config}
http    : {plain_bind}
https   : {tls_bind}
http/3  : {h3_bind}
admin   : {admin_bind}

open    : welcome page on /, admin console on {admin_bind}
support : http / https / http3 / tcp / udp / ws / wss
"#,
            version = version,
            config = config_path.display(),
            plain_bind = plain_bind,
            tls_bind = tls_bind,
            h3_bind = h3_bind,
            admin_bind = admin_bind,
        );
    }

    tracing::info!(
        version,
        config = %config_path.display(),
        plain_bind = %plain_bind,
        tls_bind = %tls_bind,
        h3_bind = %h3_bind,
        admin_bind = %admin_bind,
        "startup banner emitted"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix_tracks_nginx_parity_surface() {
        let required = [
            "http reverse proxy",
            "https/http2 termination",
            "http3/quic",
            "websocket/ws/wss",
            "tcp stream proxy",
            "udp stream proxy",
            "static files",
            "ftp",
            "webdav",
            "explicit sub-config",
            "hot reload",
            "logging levels",
            "auto https",
            "admin api/console",
            "agent install skill",
        ];

        for item in required {
            assert!(
                CAPABILITY_MATRIX.iter().any(|(name, _)| *name == item),
                "missing capability matrix item: {item}"
            );
        }
    }

    #[test]
    fn capability_matrix_mentions_default_admin_port() {
        let admin = CAPABILITY_MATRIX
            .iter()
            .find(|(name, _)| *name == "admin api/console")
            .expect("admin capability exists");

        assert!(admin.1.contains("7777"));
    }

    #[test]
    fn route_topology_lists_agent_relevant_entries() {
        let mut config = GatewayConfig::default();
        config
            .services
            .reverse_proxy
            .routes
            .push(crate::config::ReverseProxyRouteConfig {
                name: "api".to_string(),
                path_prefix: "/api".to_string(),
                hosts: vec!["api.example.com".to_string()],
                upstream: "http://127.0.0.1:8080".to_string(),
                upstreams: vec!["http://127.0.0.1:8080".to_string()],
                strip_prefix: true,
                set_headers: std::collections::BTreeMap::new(),
                strip_headers: Vec::new(),
            });
        config
            .services
            .static_sites
            .push(crate::config::StaticSiteConfig {
                name: "public".to_string(),
                path_prefix: "/assets".to_string(),
                root: "public".into(),
                index_files: vec!["index.html".to_string()],
                autoindex: false,
            });
        config.services.webdav.enabled = true;
        config.services.ftp.enabled = true;
        config.tcp.listeners.push(crate::config::TcpListenerConfig {
            name: "chat".to_string(),
            bind: "0.0.0.0:7000".to_string(),
            upstream: "127.0.0.1:9000".to_string(),
            upstreams: vec!["127.0.0.1:9001".to_string()],
        });
        config.udp.listeners.push(crate::config::UdpListenerConfig {
            name: "realtime".to_string(),
            bind: "0.0.0.0:7001".to_string(),
            upstream: String::new(),
            upstreams: vec!["127.0.0.1:9100".to_string()],
        });

        let topology = render_route_topology(&config);
        assert!(topology.contains("plain=0.0.0.0:80"));
        assert!(topology.contains("tls=0.0.0.0:443"));
        assert!(topology.contains("api hosts=api.example.com path=/api"));
        assert!(topology.contains("public path=/assets"));
        assert!(topology.contains("[webdav]"));
        assert!(topology.contains("chat bind=0.0.0.0:7000 upstream=127.0.0.1:9000"));
        assert!(topology.contains("realtime bind=0.0.0.0:7001 upstream=disabled upstreams=1"));
        assert!(topology.contains("ftp bind=0.0.0.0:21"));
    }

    #[test]
    fn reload_plan_lists_hot_reload_and_restart_boundaries() {
        let plan = render_reload_plan(&GatewayConfig::default());
        assert!(plan.contains("reverse_proxy routes"));
        assert!(plan.contains("auto-loaded plugin scripts"));
        assert!(plan.contains("services.ftp.enabled/services.ftp.bind"));
        assert!(plan.contains("http.plain_bind/http.tls_bind/http.h3_bind"));
        assert!(plan.contains("logging.access_log_path/logging.error_log_path"));
        let restart_section = plan
            .split("[restart_required]")
            .nth(1)
            .expect("restart section");
        assert!(restart_section.contains("logging.access_log_path/logging.error_log_path"));
    }

    #[test]
    fn capability_matrix_mentions_auto_https() {
        assert!(CAPABILITY_MATRIX
            .iter()
            .any(|(name, status)| *name == "auto https" && status.contains("auto_https")));
    }

    #[test]
    fn open_log_writer_creates_parent_directory() {
        let root =
            std::env::temp_dir().join(format!("proxysss-log-writer-test-{}", std::process::id()));
        let log_path = root.join("logs").join("access.log");
        let _ = std::fs::remove_dir_all(&root);

        open_log_writer(&log_path, &root).expect("open log writer");
        assert!(log_path.exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nginx_parity_matrix_tracks_supported_and_gap_items() {
        for required in [
            "declarative reverse proxy",
            "static file service",
            "WebDAV",
            "hot reload",
            "compression",
            "cache/proxy cache",
            "rate limiting",
        ] {
            assert!(
                NGINX_PARITY_MATRIX
                    .iter()
                    .any(|item| item.capability == required),
                "missing nginx parity item: {required}"
            );
        }

        assert!(NGINX_PARITY_MATRIX
            .iter()
            .any(|item| item.status == ParityStatus::Partial));
        assert!(NGINX_PARITY_MATRIX
            .iter()
            .any(|item| item.status == ParityStatus::Missing));
    }
}
