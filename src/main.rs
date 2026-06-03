mod bench;
mod config;
mod demo;
mod gateway;
mod install;
mod script;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use config::{GatewayConfig, LogFormat};
use reqwest::Method;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "proxysss")]
#[command(about = "Programmable Rust gateway with TS/JS routing scripts")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
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

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { config } => {
            let config_path = install::resolve_run_config_path(config)?;
            let gateway_config = GatewayConfig::load(&config_path)?;

            init_logging(&gateway_config.logging.filter, gateway_config.logging.format);
            for warning in gateway_config.warnings() {
                tracing::warn!(warning, "configuration warning");
            }

            tracing::info!(config = %config_path.display(), "starting gateway");
            gateway::Gateway::from_config(config_path, gateway_config).await?.run().await
        }
        Commands::CheckConfig { config } => {
            init_logging("info,proxysss=info", LogFormat::Plain);

            let config_path = install::resolve_run_config_path(config)?;
            let gateway_config = GatewayConfig::load(&config_path)?;
            println!("configuration check passed: {}", config_path.display());
            for warning in gateway_config.warnings() {
                println!("warning: {warning}");
            }
            Ok(())
        }
        Commands::Init { dir, overwrite } => {
            init_logging("info,proxysss=info", LogFormat::Plain);
            install::init_layout(dir, overwrite)
        }
        Commands::CertBootstrap { dir, overwrite } => {
            init_logging("info,proxysss=info", LogFormat::Plain);
            install::bootstrap_certs_in_dir(dir, overwrite)
        }
        Commands::Service { action } => match action {
            ServiceCommands::Install { config } => {
                init_logging("info,proxysss=info", LogFormat::Plain);
                install::install_service(config)
            }
            ServiceCommands::Uninstall => {
                init_logging("info,proxysss=info", LogFormat::Plain);
                install::uninstall_service()
            }
            ServiceCommands::Start => {
                init_logging("info,proxysss=info", LogFormat::Plain);
                install::start_service()
            }
            ServiceCommands::Stop => {
                init_logging("info,proxysss=info", LogFormat::Plain);
                install::stop_service()
            }
            ServiceCommands::Status => {
                init_logging("info,proxysss=info", LogFormat::Plain);
                install::service_status()
            }
        },
        Commands::Bench { protocol } => {
            init_logging("info,proxysss=info", LogFormat::Plain);
            bench::run(protocol).await
        }
        Commands::Demo { kind } => {
            init_logging("info,proxysss=info", LogFormat::Plain);
            demo::run(kind).await
        }
        Commands::PrintDefaultConfig { format } => {
            match format {
                ConfigOutputFormat::Yaml => print!("{}", config::GatewayConfig::render_default_yaml("deno")),
                ConfigOutputFormat::Json => print!("{}", config::GatewayConfig::render_default_json("deno")),
            }
            Ok(())
        }
        Commands::Plugin {
            action,
            config,
            admin_url,
            username,
            password,
        } => {
            init_logging("info,proxysss=info", LogFormat::Plain);
            let admin = resolve_admin_context(config, admin_url, username, password)?;
            let client = reqwest::Client::new();

            match action {
                PluginCommands::List => {
                    let payload = admin_request_json(&client, &admin, Method::GET, "/v1/plugins", None).await?;
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

fn normalize_plugin_module_path(value: &str) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with("file:") {
        return Ok(value.to_string());
    }

    let input = PathBuf::from(value);
    if input.is_absolute() {
        Ok(input.to_string_lossy().to_string())
    } else {
        Ok(std::env::current_dir()?.join(input).to_string_lossy().to_string())
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
        .or_else(|| loaded_config.as_ref().map(|cfg| format!("http://{}", cfg.admin.bind)))
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
    let payload: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));

    if !status.is_success() {
        return Err(anyhow::anyhow!("admin request failed with {}: {}", status, payload));
    }

    Ok(payload)
}

fn init_logging(filter: &str, format: LogFormat) {
    match format {
        LogFormat::Plain => {
            let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter.to_string()));

            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
        LogFormat::Json => {
            let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter.to_string()));

            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
    }
}
