use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::config::ScriptConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpContext {
    pub request_id: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub scheme: String,
    pub version: String,
    pub remote_addr: String,
    pub player_id: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamContext {
    pub request_id: String,
    pub listener: String,
    pub protocol: String,
    pub remote_addr: String,
    pub player_id: Option<String>,
    pub first_packet_preview: Option<String>,
    pub payload_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub upstream: String,
    #[serde(default)]
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub affinity_key: Option<String>,
    #[serde(default)]
    pub rewrite_path: Option<String>,
    #[serde(default)]
    pub set_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub strip_headers: Vec<String>,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPluginSpec {
    pub name: String,
    pub module_path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPluginInfo {
    pub name: String,
    pub module_path: String,
    pub priority: i32,
    pub enabled: bool,
    #[serde(default)]
    pub loaded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ScriptRequest<T>
where
    T: Serialize,
{
    id: String,
    kind: String,
    listener: Option<String>,
    ctx: T,
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptResponse {
    id: String,
    ok: bool,
    #[serde(default)]
    route: Option<RouteDecision>,
    #[serde(default)]
    plugins: Option<Vec<ScriptPluginInfo>>,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

pub struct ScriptRuntime {
    writer: Arc<Mutex<ChildStdin>>,
    pending: Arc<DashMap<String, oneshot::Sender<Result<ScriptResponse>>>>,
    timeout: Duration,
}

impl ScriptRuntime {
    pub fn spawn(config: &ScriptConfig) -> Result<Self> {
        let mut command = Command::new(&config.command);
        command.args(&config.args);

        if let Some(cwd) = &config.cwd {
            command.current_dir(cwd);
        }

        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn script runtime {}", config.command))?;

        let stdin = child
            .stdin
            .take()
            .context("script runtime stdin unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("script runtime stdout unavailable")?;
        let pending = Arc::new(DashMap::<String, oneshot::Sender<Result<ScriptResponse>>>::new());
        let pending_reader = pending.clone();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<ScriptResponse>(&line) {
                            Ok(response) => {
                                if let Some((_, sender)) = pending_reader.remove(&response.id) {
                                    let _ = sender.send(Ok(response));
                                }
                            }
                            Err(error) => {
                                tracing::warn!(?error, line, "failed to parse script response");
                            }
                        }
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(error) => {
                        tracing::warn!(?error, "script stdout reader failed");
                        break;
                    }
                }
            }

            let pending_ids: Vec<String> = pending_reader
                .iter()
                .map(|entry| entry.key().clone())
                .collect();
            for id in pending_ids {
                if let Some((_, sender)) = pending_reader.remove(&id) {
                    let _ = sender.send(Err(anyhow!("script runtime closed before responding")));
                }
            }
        });

        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => tracing::warn!(?status, "script runtime exited"),
                Err(error) => tracing::warn!(?error, "failed waiting for script runtime exit"),
            }
        });

        Ok(Self {
            writer: Arc::new(Mutex::new(stdin)),
            pending,
            timeout: Duration::from_millis(config.timeout_ms.max(1)),
        })
    }

    pub async fn route_http(&self, ctx: HttpContext) -> Result<RouteDecision> {
        self.call("http", None, ctx).await?.into_route_result()
    }

    pub async fn route_tcp(&self, ctx: StreamContext) -> Result<RouteDecision> {
        self.call("tcp", Some(ctx.listener.clone()), ctx)
            .await?
            .into_route_result()
    }

    pub async fn route_udp(&self, ctx: StreamContext) -> Result<RouteDecision> {
        self.call("udp", Some(ctx.listener.clone()), ctx)
            .await?
            .into_route_result()
    }

    pub async fn list_plugins(&self) -> Result<Vec<ScriptPluginInfo>> {
        self.call("plugin_list", None, json!({}))
            .await?
            .into_plugins_result()
    }

    pub async fn load_plugin(&self, spec: ScriptPluginSpec) -> Result<Value> {
        self.call("plugin_load", None, spec)
            .await?
            .into_data_result()
    }

    pub async fn unload_plugin(&self, name: &str) -> Result<Value> {
        self.call("plugin_unload", None, json!({ "name": name }))
            .await?
            .into_data_result()
    }

    async fn call<T>(&self, kind: &str, listener: Option<String>, ctx: T) -> Result<ScriptResponse>
    where
        T: Serialize,
    {
        let id = Uuid::new_v4().to_string();
        let request = ScriptRequest {
            id: id.clone(),
            kind: kind.to_string(),
            listener,
            ctx,
        };
        let payload =
            serde_json::to_string(&request).context("failed to serialize script request")?;
        let (sender, receiver) = oneshot::channel();
        self.pending.insert(id.clone(), sender);

        {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(payload.as_bytes())
                .await
                .context("failed to write script request")?;
            writer
                .write_all(b"\n")
                .await
                .context("failed to write script newline")?;
            writer
                .flush()
                .await
                .context("failed to flush script request")?;
        }

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("script response channel dropped")),
            Err(_) => {
                self.pending.remove(&id);
                Err(anyhow!("script routing timed out after {:?}", self.timeout))
            }
        }
    }
}

impl ScriptResponse {
    fn into_route_result(self) -> Result<RouteDecision> {
        self.ensure_ok()?;
        self.route
            .ok_or_else(|| anyhow!("script returned ok without route"))
    }

    fn into_plugins_result(self) -> Result<Vec<ScriptPluginInfo>> {
        self.ensure_ok()?;
        if let Some(plugins) = self.plugins {
            return Ok(plugins);
        }

        if let Some(data) = self.data {
            if let Some(plugins_value) = data.get("plugins") {
                let plugins =
                    serde_json::from_value::<Vec<ScriptPluginInfo>>(plugins_value.clone())
                        .context("failed to decode plugin list from script data")?;
                return Ok(plugins);
            }
        }

        Err(anyhow!("script returned ok without plugin list"))
    }

    fn into_data_result(self) -> Result<Value> {
        self.ensure_ok()?;
        Ok(self.data.unwrap_or_else(|| json!({ "ok": true })))
    }

    fn ensure_ok(&self) -> Result<()> {
        if self.ok {
            Ok(())
        } else {
            Err(anyhow!(self
                .error
                .clone()
                .unwrap_or_else(|| "script rejected request".to_string())))
        }
    }
}

fn default_true() -> bool {
    true
}
