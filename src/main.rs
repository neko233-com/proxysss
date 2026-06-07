mod bench;
mod config;
mod demo;
mod gateway;
mod install;
mod script;
mod ts_transpile;

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use config::{GatewayConfig, LogFormat, LoggingConfig, DEFAULT_ADMIN_BEARER_TOKEN};
use reqwest::Method;
use serde::Serialize;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[derive(Parser, Debug)]
#[command(name = "proxysss")]
#[command(about = "Programmable Rust gateway with TS/JS routing scripts")]
#[command(version)]
struct Cli {
    #[arg(
        short = 'c',
        long = "config",
        visible_alias = "config-file",
        global = true
    )]
    config_file: Option<PathBuf>,
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
    Start {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Stop {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Restart {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Enable {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Disable,
    Status {
        #[arg(long)]
        config: Option<PathBuf>,
    },
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
    Script {
        #[command(subcommand)]
        action: ScriptCommands,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Token {
        #[command(subcommand)]
        action: TokenCommands,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Tune {
        #[command(subcommand)]
        action: TuneCommands,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConfigOutputFormat {
    Yaml,
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    Show {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    CreateTemplate {
        #[arg(value_enum)]
        kind: ConfigTemplateKind,
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    Includes,
    WatchedScripts,
    Routes,
    ReloadPlan,
    NginxParity {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    CaddyParity {
        #[arg(long, value_enum, default_value_t = ConfigOutputFormat::Yaml)]
        format: ConfigOutputFormat,
    },
    Explain,
    Capabilities,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConfigTemplateKind {
    Full,
    Http,
    Tcp,
    Udp,
    StaticSite,
    Webdav,
    Script,
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

#[derive(Subcommand, Debug)]
enum ScriptCommands {
    RunFile {
        path: PathBuf,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Eval {
        code: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum TokenCommands {
    Show,
    Set { value: Option<String> },
}

#[derive(Subcommand, Debug)]
enum TuneCommands {
    Tcp,
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
    (
        "domain-centric reverse proxy",
        "services.domain_routes treats domain as the primary HTTP reverse proxy unit with per-domain ssl/compression/cache",
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
    (
        "ftp",
        "services.ftp now proxies the control channel and rewrites passive data channels through a local port pool",
    ),
    (
        "webdav",
        "built-in services.webdav runtime for OPTIONS/PROPFIND/GET/HEAD/PUT/DELETE/MKCOL/COPY/MOVE",
    ),
    (
        "single yaml config",
        "recommended and enforced: keep runtime settings in one proxysss.yaml file and use -config/--config/-c only to point at a different YAML path",
    ),
    (
        "hot reload",
        "the main YAML config, the main script, and auto-loaded plugins are fingerprinted",
    ),
    (
        "forwarded headers",
        "x-real-ip, x-forwarded-for, x-forwarded-host, x-forwarded-proto, and forwarded are injected on upstream requests",
    ),
    (
        "logging levels",
        "debug/info/warn/error with info as default, debug reserved for internal diagnostics, file sinks at logs/access.log and logs/error.log",
    ),
    (
        "plugin sidecar config",
        "auto-loaded plugins may read <name>.plugin.yaml/.yml for enabled/priority/config without external runtime",
    ),
    (
        "ai api compatibility",
        "generic HTTP proxying works for OpenAI-compatible/New API style traffic; optional default-off plugin templates add host/path rewrite and audit headers",
    ),
    (
        "auto https",
        "proxysss YAML style http.tls.auto_https expands to managed ACME HTTP-01 issue/renew without external binaries",
    ),
    (
        "multi-cert sni",
        "http.tls.certificates and services.domain_routes[*].ssl manual mode select certs by SNI hostname",
    ),
    (
        "zstd/gzip/brotli compression",
        "services.response_policy plus per-route overrides negotiate zstd/br/gzip for matching responses",
    ),
    (
        "ip allow/deny blacklist",
        "services.access_control.http supports built-in allow/deny IP and CIDR filtering",
    ),
    (
        "http cache",
        "services.response_policy/routes support shared cache zones, optional disk-backed entries, and PURGE for active invalidation",
    ),
    (
        "active health checks",
        "load_balance.active_health performs periodic HTTP/TCP upstream probes and exposes results plus manual drain state in /v1/upstreams and the admin dashboard",
    ),
    (
        "manual upstream drain",
        "admin API and the 7777 dashboard can take individual upstreams offline or restore them without changing config",
    ),
    (
        "tcp tuning assistant",
        "proxysss tune tcp launches an interactive Linux-oriented sysctl tuning flow based on workload and hardware",
    ),
    (
        "admin api/console",
        "supported on 127.0.0.1:7777 by default",
    ),
    (
        "cluster automation api",
        "admin API supports bearer-token authenticated /v1/domain-routes/upsert, /v1/reverse-proxy-routes/upsert, /v1/tcp-listeners/upsert, and /v1/udp-listeners/upsert writes persisted back into proxysss.yaml and reloaded in process",
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

#[derive(Debug, Clone, Copy, Serialize)]
struct CaddyFeatureItem {
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
        evidence: "services.ftp proxies the control channel and rewrites both passive and active data channels through a configurable local port pool",
        next_gap: "richer FTP policy controls and command-aware observability",
    },
    NginxParityItem {
        capability: "TLS certificates",
        status: ParityStatus::Supported,
        evidence: "self_signed/manual/acme_managed/acme_external plus http.tls.certificates and domain-route manual SSL for multi-cert SNI",
        next_gap: "",
    },
    NginxParityItem {
        capability: "self-contained auto ssl",
        status: ParityStatus::Partial,
        evidence: "auto https now uses managed ACME HTTP-01/TLS-ALPN-01 issue/renew without acme.sh; acme_external remains available for legacy flows",
        next_gap: "add richer account/provider selection and DNS challenge support",
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
        evidence: "reload fingerprint covers the main YAML config, the main script, and auto-loaded plugins",
        next_gap: "",
    },
    NginxParityItem {
        capability: "compression",
        status: ParityStatus::Partial,
        evidence: "services.response_policy plus route-level overrides provide configurable zstd/brotli/gzip response compression",
        next_gap: "extend response policy into cache zones and protocol-specific tuning surfaces",
    },
    NginxParityItem {
        capability: "IP allow/deny / blacklist",
        status: ParityStatus::Supported,
        evidence: "services.access_control.http provides built-in IP/CIDR allow and deny lists",
        next_gap: "",
    },
    NginxParityItem {
        capability: "cache/proxy cache",
        status: ParityStatus::Partial,
        evidence: "services.response_policy/routes provide shared cache zones, disk-backed cache files, and PURGE-based invalidation",
        next_gap: "add background revalidation and more advanced cache key/variant controls",
    },
    NginxParityItem {
        capability: "active health checks",
        status: ParityStatus::Supported,
        evidence: "load_balance.active_health probes reverse proxy HTTP upstreams and stream TCP upstreams on a schedule and feeds runtime health state into selection and admin APIs",
        next_gap: "",
    },
    NginxParityItem {
        capability: "rate limiting",
        status: ParityStatus::Partial,
        evidence: "services.rate_limit.http plus route-level overrides provide fixed-window shared-zone request limiting and concurrent connection caps",
        next_gap: "add leaky-bucket/token-bucket shaping and stream-layer shared policies",
    },
    NginxParityItem {
        capability: "forwarding header semantics",
        status: ParityStatus::Supported,
        evidence:
            "proxy layer injects x-real-ip, x-forwarded-for, x-forwarded-host, x-forwarded-proto, and forwarded",
        next_gap: "",
    },
    NginxParityItem {
        capability: "ai api passthrough",
        status: ParityStatus::Supported,
        evidence:
            "generic reverse proxy routes handle bearer/api-key headers and optional plugin rewrite/audit hooks for OpenAI-compatible and New API traffic",
        next_gap: "",
    },
    NginxParityItem {
        capability: "plugin sidecar configuration",
        status: ParityStatus::Supported,
        evidence:
            "auto-loaded plugins read <name>.plugin.yaml/.yml for enabled/priority/config while remaining default-off",
        next_gap: "",
    },
];

const CADDY_FEATURE_MATRIX: &[CaddyFeatureItem] = &[
    CaddyFeatureItem {
        capability: "automatic HTTPS",
        status: ParityStatus::Supported,
        evidence: "http.tls.auto_https now expands to managed ACME HTTP-01/TLS-ALPN-01 issuance/renewal without external binaries",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "automatic HTTP to HTTPS redirects",
        status: ParityStatus::Supported,
        evidence: "plain HTTP requests for TLS-managed domains are automatically 308 redirected to HTTPS except ACME challenge paths",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "admin API and hot reload",
        status: ParityStatus::Supported,
        evidence: "admin API on 127.0.0.1:7777 plus config/script/plugin hot reload fingerprinting",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "file server",
        status: ParityStatus::Supported,
        evidence: "services.static_sites handles file serving, index files, and autoindex",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "reverse proxy",
        status: ParityStatus::Supported,
        evidence: "services.reverse_proxy and services.domain_routes provide matcher-based reverse proxying",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "response encoding",
        status: ParityStatus::Supported,
        evidence: "services.response_policy and per-route overrides negotiate zstd/br/gzip",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "request matchers and header manipulation",
        status: ParityStatus::Supported,
        evidence: "host/path matchers, header strip/set, and access control rules are built into the route layer",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "Ubuntu/Debian-friendly TCP tuning",
        status: ParityStatus::Supported,
        evidence: "proxysss tune tcp emits an interactive Linux sysctl profile targeting /etc/sysctl.d/99-proxysss-tcp.conf",
        next_gap: "",
    },
    CaddyFeatureItem {
        capability: "on-demand TLS",
        status: ParityStatus::Missing,
        evidence: "certificate issuance currently uses configured domain sets rather than first-hit on-demand policy",
        next_gap: "add policy-gated on-demand issuance and storage controls",
    },
    CaddyFeatureItem {
        capability: "active upstream health checks",
        status: ParityStatus::Supported,
        evidence: "load_balance.active_health periodically probes HTTP and TCP upstreams and surfaces the result plus manual drain state in the admin API/dashboard",
        next_gap: "",
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = Cli::parse_from(normalize_cli_args(std::env::args_os()));
    let global_config = cli.config_file.clone();

    match cli.command.unwrap_or(Commands::Run { config: None }) {
        Commands::Run { config } => {
            let config_path =
                install::resolve_run_config_path(merge_config_arg(global_config.clone(), config))?;
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
        Commands::Start { config } => {
            init_cli_logging();
            install::start_background(merge_config_arg(global_config.clone(), config))
        }
        Commands::Stop { config } => {
            init_cli_logging();
            install::stop_background(merge_config_arg(global_config.clone(), config))
        }
        Commands::Restart { config } => {
            init_cli_logging();
            install::restart_background(merge_config_arg(global_config.clone(), config))
        }
        Commands::Enable { config } => {
            init_cli_logging();
            install::install_service(merge_config_arg(global_config.clone(), config))
        }
        Commands::Disable => {
            init_cli_logging();
            install::uninstall_service()
        }
        Commands::Status { config } => {
            init_cli_logging();
            install::background_status(merge_config_arg(global_config.clone(), config))
        }
        Commands::CheckConfig { config } => {
            init_cli_logging();

            let config_path =
                install::resolve_run_config_path(merge_config_arg(global_config.clone(), config))?;
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
                    print!("{}", config::GatewayConfig::render_default_yaml())
                }
            }
            Ok(())
        }
        Commands::Config { action, config } => {
            init_cli_logging();
            let command_config = merge_config_arg(global_config.clone(), config);
            match action {
                ConfigCommands::CreateTemplate {
                    kind,
                    output,
                    overwrite,
                } => {
                    write_config_template(kind, &output, overwrite)?;
                    println!(
                        "wrote {} template to {}",
                        config_template_name(kind),
                        output.display()
                    );
                    Ok(())
                }
                ConfigCommands::Show { format } => {
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    match format {
                        ConfigOutputFormat::Yaml => {
                            print!("{}", render_redacted_config_yaml(&gateway_config)?)
                        }
                    }
                    Ok(())
                }
                ConfigCommands::Includes => {
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
                    println!("config: {}", config_path.display());
                    println!("include: unsupported");
                    println!("recommendation: keep all runtime settings in a single YAML file");
                    Ok(())
                }
                ConfigCommands::WatchedScripts => {
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
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
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    print!("{}", render_route_topology(&gateway_config));
                    Ok(())
                }
                ConfigCommands::ReloadPlan => {
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
                    let gateway_config = GatewayConfig::load(&config_path)?;
                    print!("{}", render_reload_plan(&gateway_config));
                    Ok(())
                }
                ConfigCommands::NginxParity { format } => {
                    match format {
                        ConfigOutputFormat::Yaml => {
                            print!("{}", serde_yaml::to_string(NGINX_PARITY_MATRIX)?)
                        }
                    }
                    Ok(())
                }
                ConfigCommands::CaddyParity { format } => {
                    match format {
                        ConfigOutputFormat::Yaml => {
                            print!("{}", serde_yaml::to_string(CADDY_FEATURE_MATRIX)?)
                        }
                    }
                    Ok(())
                }
                ConfigCommands::Explain => {
                    let config_path = install::resolve_run_config_path(command_config.clone())?;
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
            let admin = resolve_admin_context(
                merge_config_arg(global_config.clone(), config),
                admin_url,
                username,
                password,
            )?;
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
        Commands::Script { action, config } => {
            init_cli_logging();
            match action {
                ScriptCommands::RunFile { path, args } => run_script_runtime(
                    merge_config_arg(global_config.clone(), config),
                    ScriptInvocation::File(path),
                    args,
                ),
                ScriptCommands::Eval { code, args } => run_script_runtime(
                    merge_config_arg(global_config.clone(), config),
                    ScriptInvocation::Snippet(code),
                    args,
                ),
            }
        }
        Commands::Token { action, config } => {
            init_cli_logging();
            match action {
                TokenCommands::Show => {
                    show_admin_token(merge_config_arg(global_config.clone(), config))
                }
                TokenCommands::Set { value } => {
                    set_admin_token(merge_config_arg(global_config.clone(), config), value)
                }
            }
        }
        Commands::Tune { action } => {
            init_cli_logging();
            match action {
                TuneCommands::Tcp => run_interactive_tcp_tune(),
            }
        }
    }
}

fn merge_config_arg(
    global_config: Option<PathBuf>,
    local_config: Option<PathBuf>,
) -> Option<PathBuf> {
    local_config.or(global_config)
}

fn show_admin_token(config: Option<PathBuf>) -> Result<()> {
    let config_path = install::resolve_run_config_path(config)?;
    let token = if config_path.exists() {
        GatewayConfig::load(&config_path)?.admin.bearer_token
    } else {
        DEFAULT_ADMIN_BEARER_TOKEN.to_string()
    };

    println!("config: {}", config_path.display());
    println!("admin.bearer_token: {}", token);
    Ok(())
}

fn set_admin_token(config: Option<PathBuf>, value: Option<String>) -> Result<()> {
    let config_path = install::resolve_run_config_path(config)?;
    let token = value.unwrap_or_else(|| DEFAULT_ADMIN_BEARER_TOKEN.to_string());
    let raw = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?
    } else {
        GatewayConfig::render_default_yaml()
    };
    let updated = render_config_with_admin_token(&raw, &token)?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&config_path, updated)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    println!("updated admin.bearer_token in {}", config_path.display());
    println!(
        "note: if proxysss is running with hot reload enabled, the token change is applied from the YAML update"
    );
    Ok(())
}

fn render_redacted_config_yaml(config: &GatewayConfig) -> Result<String> {
    let mut value =
        serde_yaml::to_value(config).context("failed to serialize config for display")?;
    if let Some(admin) = value
        .get_mut("admin")
        .and_then(|item| item.as_mapping_mut())
    {
        admin.insert(
            serde_yaml::Value::String("password".to_string()),
            serde_yaml::Value::String("***".to_string()),
        );
        admin.insert(
            serde_yaml::Value::String("bearer_token".to_string()),
            serde_yaml::Value::String("***".to_string()),
        );
    }
    serde_yaml::to_string(&value).context("failed to render redacted config")
}

fn render_config_with_admin_token(original: &str, token: &str) -> Result<String> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_str(original).context("failed to parse existing YAML config")?;
    let root = value
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("top-level YAML config must be a mapping"))?;

    let admin_key = serde_yaml::Value::String("admin".to_string());
    if !root.contains_key(&admin_key) {
        root.insert(
            admin_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let admin = root
        .get_mut(&admin_key)
        .and_then(|item| item.as_mapping_mut())
        .ok_or_else(|| anyhow::anyhow!("admin must be a mapping"))?;

    let token_key = serde_yaml::Value::String("bearer_token".to_string());
    if token == DEFAULT_ADMIN_BEARER_TOKEN {
        admin.remove(&token_key);
    } else {
        admin.insert(token_key, serde_yaml::Value::String(token.to_string()));
    }

    serde_yaml::to_string(&value).context("failed to render YAML with admin token")
}

fn normalize_cli_args<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .map(|arg| {
            if arg == "-config" {
                OsString::from("--config")
            } else {
                arg
            }
        })
        .collect()
}

enum ScriptInvocation {
    File(PathBuf),
    Snippet(String),
}

#[derive(Clone, Copy)]
enum TcpTuneProfile {
    Edge,
    Bulk,
    Latency,
}

struct TcpTuneSurvey {
    profile: TcpTuneProfile,
    memory_gb: u32,
    cpu_cores: u32,
    nic_gbps: u32,
    max_connections: u32,
}

fn run_interactive_tcp_tune() -> Result<()> {
    println!("proxysss tcp tune interactive");
    println!("target os       : {}", std::env::consts::OS);
    println!("target distro   : Ubuntu/Debian first");

    let survey = TcpTuneSurvey {
        profile: prompt_profile()?,
        memory_gb: prompt_u32("RAM (GiB)", 16)?,
        cpu_cores: prompt_u32("CPU cores", 8)?,
        nic_gbps: prompt_u32("NIC speed (Gbps)", 10)?,
        max_connections: prompt_u32("Peak concurrent connections", 20000)?,
    };

    let profile_name = match survey.profile {
        TcpTuneProfile::Edge => "edge",
        TcpTuneProfile::Bulk => "bulk",
        TcpTuneProfile::Latency => "latency",
    };
    let content = render_linux_tcp_sysctl_profile(&survey);

    println!("profile         : {profile_name}");
    println!("generated sysctl :");
    println!("{content}");

    if std::env::consts::OS != "linux" {
        println!("linux-specific apply is not available on this platform; copy the profile to an Ubuntu/Debian host and load it with sysctl.");
        return Ok(());
    }

    if !prompt_yes_no("Write/apply this profile now", false)? {
        return Ok(());
    }

    let target = PathBuf::from("/etc/sysctl.d/99-proxysss-tcp.conf");
    match fs::write(&target, &content) {
        Ok(()) => {
            let status = Command::new("sysctl").arg("--system").status();
            match status {
                Ok(status) if status.success() => {
                    println!("applied sysctl profile at {}", target.display());
                }
                Ok(status) => {
                    println!(
                        "wrote {}, but sysctl --system exited with status {}",
                        target.display(),
                        status
                    );
                }
                Err(error) => {
                    println!(
                        "wrote {}, but failed to run sysctl --system: {}",
                        target.display(),
                        error
                    );
                }
            }
            Ok(())
        }
        Err(error) => {
            let fallback = PathBuf::from("proxysss-tcp.sysctl.conf");
            fs::write(&fallback, &content)
                .with_context(|| format!("failed to write {}", fallback.display()))?;
            println!("could not write {}: {}", target.display(), error);
            println!(
                "wrote fallback profile to {}. on Ubuntu/Debian apply with: sudo cp {} {} && sudo sysctl --system",
                fallback.display(),
                fallback.display(),
                target.display()
            );
            Ok(())
        }
    }
}

fn prompt_profile() -> Result<TcpTuneProfile> {
    println!("workload profile: 1=edge reverse proxy, 2=bulk transfer, 3=latency sensitive API");
    let value = prompt_string("Profile", "1")?;
    Ok(match value.trim() {
        "2" | "bulk" => TcpTuneProfile::Bulk,
        "3" | "latency" => TcpTuneProfile::Latency,
        _ => TcpTuneProfile::Edge,
    })
}

fn prompt_u32(label: &str, default: u32) -> Result<u32> {
    let raw = prompt_string(label, &default.to_string())?;
    raw.trim()
        .parse::<u32>()
        .with_context(|| format!("invalid numeric value for {label}"))
}

fn prompt_yes_no(label: &str, default: bool) -> Result<bool> {
    let default_text = if default { "Y" } else { "N" };
    let raw = prompt_string(label, default_text)?;
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(default);
    }
    Ok(matches!(normalized.as_str(), "y" | "yes" | "1" | "true"))
}

fn prompt_string(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush().context("failed flushing stdout")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("failed reading stdin")?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn render_linux_tcp_sysctl_profile(survey: &TcpTuneSurvey) -> String {
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

    format!(
        "# proxysss linux tcp tuning\n# profile={} memory_gb={} cpu_cores={} nic_gbps={} max_connections={}\nnet.core.somaxconn={}\nnet.ipv4.tcp_max_syn_backlog={}\nnet.core.netdev_max_backlog={}\nnet.core.rmem_max={}\nnet.core.wmem_max={}\nnet.ipv4.tcp_rmem=4096 87380 {}\nnet.ipv4.tcp_wmem=4096 65536 {}\nnet.ipv4.ip_local_port_range=10240 65535\nnet.ipv4.tcp_fin_timeout={}\nnet.ipv4.tcp_tw_reuse=1\nnet.ipv4.tcp_keepalive_time={}\nnet.ipv4.tcp_keepalive_intvl=15\nnet.ipv4.tcp_keepalive_probes=5\nnet.ipv4.tcp_mtu_probing=1\nnet.ipv4.tcp_fastopen=3\nnet.ipv4.tcp_slow_start_after_idle=0\nnet.ipv4.tcp_window_scaling=1\nnet.core.default_qdisc=fq\nnet.ipv4.tcp_congestion_control=bbr\nnet.core.busy_poll={}\nnet.core.busy_read={}\n",
        match survey.profile {
            TcpTuneProfile::Edge => "edge",
            TcpTuneProfile::Bulk => "bulk",
            TcpTuneProfile::Latency => "latency",
        },
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
        "config model      : single YAML file (default proxysss.yaml, custom via -config/--config/-c)"
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
        "auto https        : enabled={}, mode={:?}, domains={}, production={}",
        config.http.tls.auto_https.enabled,
        config.http.tls.mode,
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
        "active health     : enabled={}, http_enabled={}, tcp_enabled={}, path={}, interval_secs={}, timeout_ms={}, expected_statuses={}",
        config.load_balance.active_health.enabled,
        config.load_balance.active_health.http_enabled,
        config.load_balance.active_health.tcp_enabled,
        config.load_balance.active_health.path,
        config.load_balance.active_health.interval_secs,
        config.load_balance.active_health.timeout_ms,
        config
            .load_balance
            .active_health
            .expected_statuses
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "error pages       : enabled={}, show_details={}, custom_pages={}",
        config.http.error_pages.enabled,
        config.http.error_pages.show_details,
        config.http.error_pages.pages.len()
    );
    println!(
        "maintenance state : enabled={}, path={}",
        config.runtime.maintenance_state.enabled,
        config.runtime.maintenance_state.path.display()
    );
    println!(
        "domain routes     : routes={}",
        config.services.domain_routes.len()
    );
    println!(
        "rate limit        : http.enabled={}, zone={}, requests={}, window_ms={}, burst={}, max_connections={}",
        config.services.rate_limit.http.enabled,
        config.services.rate_limit.http.zone,
        config.services.rate_limit.http.requests,
        config.services.rate_limit.http.window_ms,
        config.services.rate_limit.http.burst
        ,config.services.rate_limit.http.max_connections
    );
    println!(
        "access control    : http.enabled={}, allow={}, deny={}, status={}",
        config.services.access_control.http.enabled,
        config.services.access_control.http.allow.len(),
        config.services.access_control.http.deny.len(),
        config.services.access_control.http.status
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
        "script            : enabled={}, entry={}, timeout_ms={}, memory_mb={}, stack_kb={}",
        config.script.enabled,
        config.script.entry.display(),
        config.script.timeout_ms,
        config.script.memory_limit_mb,
        config.script.max_stack_size_kb
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
                "{} hosts={} path={} upstream={} upstreams={} strip_prefix={} active_health={}\n",
                route.name,
                if route.hosts.is_empty() {
                    "*".to_string()
                } else {
                    route.hosts.join(",")
                },
                route.path_prefix,
                route.upstream,
                route.upstreams.len(),
                route.strip_prefix,
                config.load_balance.active_health.enabled
            ));
        }

        output.push_str("[domain_routes]\n");
        if config.services.domain_routes.is_empty() {
            output.push_str("none\n");
        } else {
            for route in &config.services.domain_routes {
                let backend_pool = route_backend_pool(&route.upstream, &route.upstreams).join(",");
                output.push_str(&format!(
                    "{} domains={} primary_domain={} path={} upstream={} upstreams={} backend_pool={} strip_prefix={} ssl={:?} auto_ssl={} compression={} cache={} cache_zone={} rate_limit={} rate_limit_zone={} max_connections={}\n",
                    route.name,
                    route.domains.join(","),
                    route.domains.first().map(String::as_str).unwrap_or("-"),
                    route.path_prefix,
                    route.upstream,
                    route.upstreams.len(),
                    backend_pool,
                    route.strip_prefix,
                    route.ssl.mode,
                    route.ssl.is_auto_ssl,
                    route.compression.enabled,
                    route.cache.enabled,
                    route.cache.zone,
                    route.rate_limit.enabled,
                    route.rate_limit.zone,
                    route.rate_limit.max_connections
                ));
            }
        }
    }

    output.push_str("[rate_limit]\n");
    output.push_str(&format!(
        "http enabled={} zone={} requests={} window_ms={} burst={} max_connections={} status={}\n",
        config.services.rate_limit.http.enabled,
        config.services.rate_limit.http.zone,
        config.services.rate_limit.http.requests,
        config.services.rate_limit.http.window_ms,
        config.services.rate_limit.http.burst,
        config.services.rate_limit.http.max_connections,
        config.services.rate_limit.http.status
    ));
    output.push_str("[active_health]\n");
    output.push_str(&format!(
        "enabled={} http_enabled={} tcp_enabled={} path={} interval_secs={} timeout_ms={} expected_statuses={}\n",
        config.load_balance.active_health.enabled,
        config.load_balance.active_health.http_enabled,
        config.load_balance.active_health.tcp_enabled,
        config.load_balance.active_health.path,
        config.load_balance.active_health.interval_secs,
        config.load_balance.active_health.timeout_ms,
        config
            .load_balance
            .active_health
            .expected_statuses
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    ));
    output.push_str("[access_control]\n");
    output.push_str(&format!(
        "http enabled={} allow={} deny={} status={}\n",
        config.services.access_control.http.enabled,
        config.services.access_control.http.allow.join(","),
        config.services.access_control.http.deny.join(","),
        config.services.access_control.http.status
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
                "ftp bind={} upstream={} native_control={} public_ip={} passive_ports={}-{}\n",
                config.services.ftp.bind,
                config.services.ftp.upstream,
                config.services.ftp.native_control,
                if config.services.ftp.public_ip.is_empty() {
                    "auto".to_string()
                } else {
                    config.services.ftp.public_ip.clone()
                },
                config.services.ftp.passive_port_start,
                config.services.ftp.passive_port_end
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
    output.push_str("the main proxysss.yaml file\n");
    output.push_str("main extension script from script.entry\n");
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

fn route_backend_pool(primary: &str, extras: &[String]) -> Vec<String> {
    let mut backends = Vec::new();
    if !primary.trim().is_empty() {
        backends.push(primary.to_string());
    }
    for upstream in extras {
        if upstream.trim().is_empty() || backends.iter().any(|item| item == upstream) {
            continue;
        }
        backends.push(upstream.clone());
    }
    backends
}

fn config_template_name(kind: ConfigTemplateKind) -> &'static str {
    match kind {
        ConfigTemplateKind::Full => "full",
        ConfigTemplateKind::Http => "http",
        ConfigTemplateKind::Tcp => "tcp",
        ConfigTemplateKind::Udp => "udp",
        ConfigTemplateKind::StaticSite => "static-site",
        ConfigTemplateKind::Webdav => "webdav",
        ConfigTemplateKind::Script => "script",
    }
}

fn render_config_template(kind: ConfigTemplateKind) -> &'static str {
    match kind {
        ConfigTemplateKind::Full => {
            "config_version: 1\nhttp:\n  plain_bind: 0.0.0.0:80\n  tls_bind: 0.0.0.0:443\n  h3_bind: 0.0.0.0:443\n  error_pages:\n    enabled: true\n    pages:\n      - status: 404\n        content_type: text/html; charset=utf-8\n        body: |\n          <html><body><h1>{{status}} {{reason}}</h1><p>proxysss could not match this route.</p></body></html>\n  tls:\n    auto_https:\n      enabled: true\n      email: admin@example.com\nload_balance:\n  active_health:\n    enabled: true\n    http_enabled: true\n    tcp_enabled: true\n    path: /healthz\n    failure_threshold: 2\n    success_threshold: 2\nruntime:\n  maintenance_state:\n    enabled: true\n    path: ./runtime/maintenance-state.json\nservices:\n  domain_routes:\n    - name: app\n      domains: [example.com, www.example.com]\n      path_prefix: /\n      upstream: http://127.0.0.1:9000\n      compression:\n        enabled: true\n      cache:\n        enabled: true\n        ttl_secs: 30\n      active_health:\n        path: /healthz\n"
        }
        ConfigTemplateKind::Http => {
            "http:\n  plain_bind: 0.0.0.0:80\n  tls_bind: 0.0.0.0:443\n  h3_bind: 0.0.0.0:443\nservices:\n  domain_routes:\n    - name: api\n      domains: [api.example.com]\n      path_prefix: /api\n      upstream: http://127.0.0.1:8080\n      upstreams:\n        - http://127.0.0.1:8080\n        - http://127.0.0.1:8081\n      strip_prefix: true\n      ssl:\n        type: auto\n      active_health:\n        path: /readyz\n        failure_threshold: 2\n        success_threshold: 2\n"
        }
        ConfigTemplateKind::Tcp => {
            "tcp:\n  listeners:\n    - name: game-tcp\n      bind: 0.0.0.0:7000\n      upstream: 127.0.0.1:9000\n      upstreams:\n        - 127.0.0.1:9000\n        - 127.0.0.1:9001\n"
        }
        ConfigTemplateKind::Udp => {
            "udp:\n  listeners:\n    - name: realtime\n      bind: 0.0.0.0:7001\n      upstreams:\n        - 127.0.0.1:9100\n        - 127.0.0.1:9101\n"
        }
        ConfigTemplateKind::StaticSite => {
            "services:\n  static_sites:\n    - name: public\n      path_prefix: /assets\n      root: ./public\n      index_files: [index.html, index.htm]\n      autoindex: false\n"
        }
        ConfigTemplateKind::Webdav => {
            "services:\n  webdav:\n    enabled: true\n    path_prefix: /dav\n    root: ./webdav\n    allow_write: true\n"
        }
        ConfigTemplateKind::Script => {
            "script:\n  enabled: true\n  entry: gateway.ts\n  cwd: .\n  timeout_ms: 500\n  memory_limit_mb: 64\n  max_stack_size_kb: 512\nplugins:\n  enabled: false\n"
        }
    }
}

fn write_config_template(kind: ConfigTemplateKind, output: &Path, overwrite: bool) -> Result<()> {
    if output.exists() && !overwrite {
        return Err(anyhow::anyhow!(
            "{} already exists; pass --overwrite to replace it",
            output.display()
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(output, render_config_template(kind))
        .with_context(|| format!("failed to write {}", output.display()))
}

fn run_script_runtime(
    config: Option<PathBuf>,
    invocation: ScriptInvocation,
    extra_args: Vec<String>,
) -> Result<()> {
    let config_path = install::resolve_run_config_path(config)?;
    let gateway_config = if config_path.exists() {
        GatewayConfig::load(&config_path)?
    } else {
        let root_dir = std::env::current_dir().context("failed to resolve current directory")?;
        let mut config = GatewayConfig {
            root_dir: root_dir.clone(),
            ..GatewayConfig::default()
        };
        config.script.cwd = Some(config.root_dir.clone());
        config
    };

    if !extra_args.is_empty() {
        eprintln!(
            "note: extra script args are ignored by the embedded TypeScript engine: {:?}",
            extra_args
        );
    }

    let mut env = gateway::default_script_env(&gateway_config);
    for (key, value) in &gateway_config.script.env {
        env.insert(key.clone(), value.clone());
    }
    let timeout = std::time::Duration::from_millis(
        gateway_config
            .script
            .timeout_ms
            .saturating_mul(10)
            .max(2000),
    );

    let (script_path, temp_script_path) = match invocation {
        ScriptInvocation::File(path) => (path, None),
        ScriptInvocation::Snippet(code) => {
            let temp_name = format!(
                "proxysss-script-eval-{}.ts",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
            let temp_path = std::env::temp_dir().join(temp_name);
            fs::write(&temp_path, code)
                .with_context(|| format!("failed to write {}", temp_path.display()))?;
            (temp_path.clone(), Some(temp_path))
        }
    };

    let result = script::run_module_file(&script_path, &env, timeout);

    if let Some(path) = temp_script_path {
        let _ = fs::remove_file(path);
    }

    result
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
            "single yaml config",
            "hot reload",
            "forwarded headers",
            "logging levels",
            "plugin sidecar config",
            "ai api compatibility",
            "auto https",
            "active health checks",
            "manual upstream drain",
            "ip allow/deny blacklist",
            "admin api/console",
            "cluster automation api",
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
                compression: crate::config::ResponseCompressionConfig::default(),
                cache: crate::config::ResponseCacheConfig::default(),
                rate_limit: crate::config::HttpRateLimitConfig::default(),
                active_health: crate::config::ActiveHealthOverrideConfig::default(),
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
        config.services.access_control.http.enabled = true;
        config
            .services
            .access_control
            .http
            .deny
            .push("203.0.113.0/24".to_string());
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
        assert!(topology.contains("[access_control]"));
        assert!(topology.contains("deny=203.0.113.0/24"));
        assert!(topology.contains("chat bind=0.0.0.0:7000 upstream=127.0.0.1:9000"));
        assert!(topology.contains("realtime bind=0.0.0.0:7001 upstream=disabled upstreams=1"));
        assert!(topology.contains("ftp bind=0.0.0.0:21"));
    }

    #[test]
    fn reload_plan_lists_hot_reload_and_restart_boundaries() {
        let plan = render_reload_plan(&GatewayConfig::default());
        assert!(plan.contains("reverse_proxy routes"));
        assert!(plan.contains("auto-loaded plugin scripts"));
        assert!(plan.contains("the main proxysss.yaml file"));
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
    fn config_templates_cover_http_and_stream_learning_paths() {
        assert!(render_config_template(ConfigTemplateKind::Http).contains("services:"));
        assert!(render_config_template(ConfigTemplateKind::Http).contains("domain_routes"));
        assert!(render_config_template(ConfigTemplateKind::Tcp).contains("tcp:"));
        assert!(render_config_template(ConfigTemplateKind::Udp).contains("udp:"));
        assert!(render_config_template(ConfigTemplateKind::Script).contains("script:"));
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
            "forwarding header semantics",
            "ai api passthrough",
            "plugin sidecar configuration",
            "compression",
            "active health checks",
            "IP allow/deny / blacklist",
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
    }

    #[test]
    fn caddy_feature_matrix_tracks_useful_surface_and_gaps() {
        for required in [
            "automatic HTTPS",
            "automatic HTTP to HTTPS redirects",
            "admin API and hot reload",
            "file server",
            "reverse proxy",
            "response encoding",
            "active upstream health checks",
            "request matchers and header manipulation",
            "Ubuntu/Debian-friendly TCP tuning",
        ] {
            assert!(
                CADDY_FEATURE_MATRIX
                    .iter()
                    .any(|item| item.capability == required),
                "missing caddy feature item: {required}"
            );
        }

        assert!(CADDY_FEATURE_MATRIX
            .iter()
            .any(|item| item.status == ParityStatus::Supported));
        assert!(CADDY_FEATURE_MATRIX
            .iter()
            .any(|item| item.status == ParityStatus::Missing));
    }

    #[test]
    fn merge_config_arg_prefers_command_local_value() {
        let global = Some(PathBuf::from("global.yaml"));
        let local = Some(PathBuf::from("local.yaml"));
        assert_eq!(
            merge_config_arg(global, local),
            Some(PathBuf::from("local.yaml"))
        );
    }

    #[test]
    fn merge_config_arg_falls_back_to_global_value() {
        let global = Some(PathBuf::from("global.yaml"));
        assert_eq!(
            merge_config_arg(global, None),
            Some(PathBuf::from("global.yaml"))
        );
    }

    #[test]
    fn normalize_cli_args_rewrites_single_dash_config_flag() {
        let args = normalize_cli_args([
            OsString::from("proxysss"),
            OsString::from("-config"),
            OsString::from("custom.yaml"),
        ]);
        assert_eq!(args[1], OsString::from("--config"));
        assert_eq!(args[2], OsString::from("custom.yaml"));
    }

    #[test]
    fn route_backend_pool_keeps_primary_and_deduplicates_extras() {
        let pool = route_backend_pool(
            "http://127.0.0.1:9000",
            &[
                "http://127.0.0.1:9000".to_string(),
                "http://127.0.0.1:9001".to_string(),
            ],
        );
        assert_eq!(
            pool,
            vec![
                "http://127.0.0.1:9000".to_string(),
                "http://127.0.0.1:9001".to_string()
            ]
        );
    }

    #[test]
    fn render_config_with_admin_token_omits_default_token() {
        let rendered = render_config_with_admin_token(
            "admin:\n  bind: 127.0.0.1:7777\n",
            DEFAULT_ADMIN_BEARER_TOKEN,
        )
        .expect("render token config");
        assert!(!rendered.contains("bearer_token"));
        assert!(!rendered.contains(DEFAULT_ADMIN_BEARER_TOKEN));
    }

    #[test]
    fn render_config_with_admin_token_sets_custom_token() {
        let rendered =
            render_config_with_admin_token("admin:\n  bind: 127.0.0.1:7777\n", "cluster-secret")
                .expect("render token config");
        assert!(rendered.contains("bearer_token: cluster-secret"));
    }

    #[test]
    fn render_redacted_config_yaml_masks_admin_secrets() {
        let mut config = GatewayConfig::default();
        config.admin.password = "super-secret".to_string();
        config.admin.bearer_token = "cluster-secret".to_string();

        let rendered = render_redacted_config_yaml(&config).expect("render redacted yaml");
        assert!(
            rendered.contains("password: '***'")
                || rendered.contains("password: \"***\"")
                || rendered.contains("password: '***'")
        );
        assert!(
            rendered.contains("bearer_token: '***'")
                || rendered.contains("bearer_token: \"***\"")
                || rendered.contains("bearer_token: ***")
        );
        assert!(!rendered.contains("super-secret"));
        assert!(!rendered.contains("cluster-secret"));
    }
}
