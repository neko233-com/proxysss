//! Embedded TypeScript/JavaScript script engine for proxysss.
//!
//! proxysss does **not** depend on any external `deno`, `node`, or `tsc`
//! toolchain. Gateway/plugin scripts are authored in TypeScript, transpiled to
//! plain JavaScript in-process (see [`crate::ts_transpile`]), and executed by a
//! statically-linked QuickJS engine (`rquickjs`).
//!
//! Design goals (see `specs/embedded-ts-runtime.md`):
//!
//! * **Single binary** — the JS engine is compiled into proxysss; there is no
//!   sidecar process and no IPC.
//! * **Absolute isolation** — a buggy plugin (throw, infinite loop, runaway
//!   memory) can never affect normal proxy traffic. Native/YAML routing always
//!   runs before scripts; every plugin hook is bounded by a hard timeout
//!   (interrupt handler) and a memory limit, and every failing invocation is
//!   reported to the error log while the pipeline continues.
//! * **Full control of input/output** — pipeline orchestration (priority
//!   ordering, merge/normalize, fallback) lives in Rust, not JavaScript.
//!
//! The public [`ScriptRuntime`] API is unchanged from the previous
//! external-runtime implementation so that `gateway.rs` call sites are
//! untouched.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context as _, Result};
use rquickjs::function::This;
use rquickjs::{CatchResultExt, Context, Ctx, Function, Module, Object, Value};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tokio::sync::oneshot;

use crate::config::ScriptConfig;
use crate::ts_transpile::transpile_module;

// ---------------------------------------------------------------------------
// Public data model (serde shapes preserved for compatibility).
// ---------------------------------------------------------------------------

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
    pub config: JsonValue,
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

/// Partial route returned by a plugin hook; merged/normalized in Rust.
#[derive(Debug, Clone, Default, Deserialize)]
struct PartialRoute {
    #[serde(default)]
    upstream: Option<String>,
    #[serde(default)]
    upstreams: Option<Vec<String>>,
    #[serde(default)]
    affinity_key: Option<String>,
    #[serde(default)]
    rewrite_path: Option<String>,
    #[serde(default)]
    set_headers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    strip_headers: Option<Vec<String>>,
    #[serde(default)]
    status: Option<u16>,
    #[serde(default)]
    content_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Worker protocol.
// ---------------------------------------------------------------------------

enum ScriptCall {
    Http(HttpContext),
    Stream {
        kind: StreamKind,
        ctx: StreamContext,
    },
    ListPlugins,
    LoadPlugin(ScriptPluginSpec),
    UnloadPlugin(String),
}

#[derive(Clone, Copy)]
enum StreamKind {
    Tcp,
    Udp,
}

impl StreamKind {
    fn as_str(self) -> &'static str {
        match self {
            StreamKind::Tcp => "tcp",
            StreamKind::Udp => "udp",
        }
    }
}

enum ScriptOutcome {
    Route(RouteDecision),
    Plugins(Vec<ScriptPluginInfo>),
    Data(JsonValue),
}

struct Job {
    call: ScriptCall,
    responder: oneshot::Sender<Result<ScriptOutcome>>,
}

/// A shared, lock-free per-call deadline used by the QuickJS interrupt handler
/// to enforce a hard timeout on any single script invocation.
struct Deadline {
    base: Instant,
    limit_ns: AtomicU64,
}

impl Deadline {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            limit_ns: AtomicU64::new(u64::MAX),
        }
    }

    fn arm(&self, duration: Duration) {
        let now = self.base.elapsed().as_nanos() as u64;
        self.limit_ns.store(
            now.saturating_add(duration.as_nanos() as u64),
            Ordering::Relaxed,
        );
    }

    fn disarm(&self) {
        self.limit_ns.store(u64::MAX, Ordering::Relaxed);
    }

    fn expired(&self) -> bool {
        (self.base.elapsed().as_nanos() as u64) > self.limit_ns.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Public runtime handle.
// ---------------------------------------------------------------------------

/// Async handle to the embedded script engine. All JavaScript executes on a
/// single dedicated worker thread that owns the QuickJS runtime; this handle
/// only sends jobs to it.
pub struct ScriptRuntime {
    sender: std::sync::mpsc::Sender<Job>,
    timeout: Duration,
}

impl ScriptRuntime {
    /// Start the embedded engine worker thread, load the optional main gateway
    /// script, and return a handle. Plugins are loaded subsequently via
    /// [`ScriptRuntime::load_plugin`].
    pub fn spawn(config: &ScriptConfig, runtime_env: &BTreeMap<String, String>) -> Result<Self> {
        let (job_tx, job_rx) = std::sync::mpsc::channel::<Job>();
        let (start_tx, start_rx) = std::sync::mpsc::channel::<Result<()>>();

        let timeout = Duration::from_millis(config.timeout_ms.max(1));
        let memory_limit = (config.memory_limit_mb.saturating_mul(1024 * 1024)) as usize;
        let stack_size = (config.max_stack_size_kb.saturating_mul(1024)) as usize;

        let entry = resolve_entry_path(config);
        let entry_name = entry
            .as_ref()
            .and_then(|path| path.file_stem().and_then(|stem| stem.to_str()))
            .unwrap_or("gateway")
            .to_string();
        let mut env = runtime_env.clone();
        for (key, value) in &config.env {
            env.insert(key.clone(), value.clone());
        }

        std::thread::Builder::new()
            .name("proxysss-script".to_string())
            .spawn(move || {
                worker_main(WorkerSetup {
                    job_rx,
                    start_tx,
                    memory_limit,
                    stack_size,
                    timeout,
                    entry,
                    entry_name,
                    env,
                });
            })
            .context("failed to spawn embedded script engine thread")?;

        match start_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                sender: job_tx,
                timeout,
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(anyhow!("embedded script engine failed to start")),
        }
    }

    pub async fn route_http(&self, ctx: HttpContext) -> Result<RouteDecision> {
        match self.call(ScriptCall::Http(ctx)).await? {
            ScriptOutcome::Route(route) => Ok(route),
            _ => Err(anyhow!(
                "script engine returned an unexpected response for http routing"
            )),
        }
    }

    pub async fn route_tcp(&self, ctx: StreamContext) -> Result<RouteDecision> {
        self.route_stream(StreamKind::Tcp, ctx).await
    }

    pub async fn route_udp(&self, ctx: StreamContext) -> Result<RouteDecision> {
        self.route_stream(StreamKind::Udp, ctx).await
    }

    async fn route_stream(&self, kind: StreamKind, ctx: StreamContext) -> Result<RouteDecision> {
        match self.call(ScriptCall::Stream { kind, ctx }).await? {
            ScriptOutcome::Route(route) => Ok(route),
            _ => Err(anyhow!(
                "script engine returned an unexpected response for stream routing"
            )),
        }
    }

    pub async fn list_plugins(&self) -> Result<Vec<ScriptPluginInfo>> {
        match self.call(ScriptCall::ListPlugins).await? {
            ScriptOutcome::Plugins(plugins) => Ok(plugins),
            _ => Err(anyhow!(
                "script engine returned an unexpected response for plugin list"
            )),
        }
    }

    pub async fn load_plugin(&self, spec: ScriptPluginSpec) -> Result<JsonValue> {
        match self.call(ScriptCall::LoadPlugin(spec)).await? {
            ScriptOutcome::Data(value) => Ok(value),
            _ => Err(anyhow!(
                "script engine returned an unexpected response for plugin load"
            )),
        }
    }

    pub async fn unload_plugin(&self, name: &str) -> Result<JsonValue> {
        match self
            .call(ScriptCall::UnloadPlugin(name.to_string()))
            .await?
        {
            ScriptOutcome::Data(value) => Ok(value),
            _ => Err(anyhow!(
                "script engine returned an unexpected response for plugin unload"
            )),
        }
    }

    async fn call(&self, call: ScriptCall) -> Result<ScriptOutcome> {
        let (responder, receiver) = oneshot::channel();
        self.sender
            .send(Job { call, responder })
            .map_err(|_| anyhow!("embedded script engine is no longer running"))?;

        // The worker enforces its own per-hook timeout via the interrupt
        // handler; this outer timeout is a safety net for the whole request.
        let outer = self.timeout.saturating_mul(4).max(Duration::from_secs(1));
        match tokio::time::timeout(outer, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("script engine dropped the response channel")),
            Err(_) => Err(anyhow!("script engine call timed out after {:?}", outer)),
        }
    }
}

fn resolve_entry_path(config: &ScriptConfig) -> Option<PathBuf> {
    if config.entry.as_os_str().is_empty() {
        return None;
    }
    let entry = if config.entry.is_absolute() {
        config.entry.clone()
    } else if let Some(cwd) = &config.cwd {
        cwd.join(&config.entry)
    } else {
        config.entry.clone()
    };
    entry.exists().then_some(entry)
}

// ---------------------------------------------------------------------------
// Worker thread: owns the QuickJS runtime and all plugin state.
// ---------------------------------------------------------------------------

struct WorkerSetup {
    job_rx: std::sync::mpsc::Receiver<Job>,
    start_tx: std::sync::mpsc::Sender<Result<()>>,
    memory_limit: usize,
    stack_size: usize,
    timeout: Duration,
    entry: Option<PathBuf>,
    entry_name: String,
    env: BTreeMap<String, String>,
}

struct LoadedPlugin<'js> {
    name: String,
    module_path: String,
    priority: i32,
    enabled: bool,
    loaded_at: String,
    object: Object<'js>,
}

/// Engine state and helpers bound to a single `ctx.with` scope.
struct Engine<'a, 'js> {
    ctx: &'a Ctx<'js>,
    deadline: &'a Deadline,
    timeout: Duration,
}

fn worker_main(setup: WorkerSetup) {
    let runtime = match rquickjs::Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            let _ = setup
                .start_tx
                .send(Err(anyhow!("failed to create QuickJS runtime: {error}")));
            return;
        }
    };

    if setup.memory_limit > 0 {
        runtime.set_memory_limit(setup.memory_limit);
    }
    if setup.stack_size > 0 {
        runtime.set_max_stack_size(setup.stack_size);
    }

    let deadline = Arc::new(Deadline::new());
    let interrupt_deadline = deadline.clone();
    runtime.set_interrupt_handler(Some(Box::new(move || interrupt_deadline.expired())));

    let context = match Context::full(&runtime) {
        Ok(ctx) => ctx,
        Err(error) => {
            let _ = setup
                .start_tx
                .send(Err(anyhow!("failed to create QuickJS context: {error}")));
            return;
        }
    };

    let _ = setup.start_tx.send(Ok(()));

    let WorkerSetup {
        job_rx,
        timeout,
        entry,
        entry_name,
        env,
        ..
    } = setup;

    context.with(|ctx| {
        let engine = Engine {
            ctx: &ctx,
            deadline: deadline.as_ref(),
            timeout,
        };

        if let Err(error) = engine.install_globals(&env) {
            tracing::error!(target: "proxysss::script", %error, "failed to install script globals");
        }

        let mut plugins: Vec<LoadedPlugin> = Vec::new();
        let mut fallback: Option<LoadedPlugin> = None;
        let mut module_counter: u64 = 0;

        if let Some(entry_path) = &entry {
            module_counter += 1;
            match engine.load_plugin_object(module_counter, &entry_name, entry_path) {
                Ok(object) => {
                    let _ = engine.run_lifecycle_hook(
                        &object,
                        "init_worker",
                        &json!({ "spec": { "name": entry_name, "module_path": entry_path.to_string_lossy() } }),
                    );
                    fallback = Some(LoadedPlugin {
                        name: entry_name.clone(),
                        module_path: entry_path.to_string_lossy().to_string(),
                        priority: i32::MIN / 2,
                        enabled: true,
                        loaded_at: now_iso(),
                        object,
                    });
                    tracing::info!(target: "proxysss::script", script = %entry_name, "loaded main gateway script into embedded engine");
                }
                Err(error) => {
                    tracing::error!(target: "proxysss::script", script = %entry_name, %error, "failed to load main gateway script; continuing with YAML-only routing");
                }
            }
        }

        while let Ok(job) = job_rx.recv() {
            let outcome = engine.handle_call(&mut plugins, &mut fallback, &mut module_counter, job.call);
            let _ = job.responder.send(outcome);
        }

        for plugin in plugins.drain(..) {
            let _ = engine.run_lifecycle_hook(&plugin.object, "onDispose", &json!({}));
        }
        if let Some(plugin) = fallback.take() {
            let _ = engine.run_lifecycle_hook(&plugin.object, "onDispose", &json!({}));
        }
    });
}

impl<'a, 'js> Engine<'a, 'js> {
    fn handle_call(
        &self,
        plugins: &mut Vec<LoadedPlugin<'js>>,
        fallback: &mut Option<LoadedPlugin<'js>>,
        module_counter: &mut u64,
        call: ScriptCall,
    ) -> Result<ScriptOutcome> {
        match call {
            ScriptCall::Http(http) => {
                let message = json!({
                    "id": http.request_id,
                    "kind": "http",
                    "listener": JsonValue::Null,
                    "ctx": serde_json::to_value(&http).unwrap_or(JsonValue::Null),
                });
                let route = self.run_http_pipeline(plugins, fallback, &message)?;
                Ok(ScriptOutcome::Route(route))
            }
            ScriptCall::Stream { kind, ctx: stream } => {
                let message = json!({
                    "id": stream.request_id,
                    "kind": kind.as_str(),
                    "listener": stream.listener,
                    "ctx": serde_json::to_value(&stream).unwrap_or(JsonValue::Null),
                });
                let route = self.run_stream_pipeline(plugins, fallback, kind, &message)?;
                Ok(ScriptOutcome::Route(route))
            }
            ScriptCall::ListPlugins => {
                let mut infos: Vec<ScriptPluginInfo> = plugins.iter().map(plugin_info).collect();
                if let Some(plugin) = fallback {
                    infos.push(plugin_info(plugin));
                }
                Ok(ScriptOutcome::Plugins(infos))
            }
            ScriptCall::LoadPlugin(spec) => Ok(ScriptOutcome::Data(self.load_plugin_record(
                plugins,
                module_counter,
                spec,
            )?)),
            ScriptCall::UnloadPlugin(name) => Ok(ScriptOutcome::Data(
                self.unload_plugin_record(plugins, &name)?,
            )),
        }
    }

    fn run_http_pipeline(
        &self,
        plugins: &[LoadedPlugin<'js>],
        fallback: &Option<LoadedPlugin<'js>>,
        message: &JsonValue,
    ) -> Result<RouteDecision> {
        let order = ordered_indices(plugins);
        let mut route: Option<PartialRoute> = None;

        for &index in &order {
            self.invoke_route_hook(&plugins[index], "access", message, &mut route);
        }
        for &index in &order {
            self.invoke_route_hook(&plugins[index], "balancer", message, &mut route);
        }
        if route.is_none() {
            if let Some(plugin) = fallback {
                self.invoke_route_hook(plugin, "access", message, &mut route);
            }
        }

        let decision = normalize_route(route.unwrap_or_default())
            .context("no plugin produced an http route and no fallback gateway script is loaded")?;
        self.run_log_hooks(plugins, fallback, message, &decision);
        Ok(decision)
    }

    fn run_stream_pipeline(
        &self,
        plugins: &[LoadedPlugin<'js>],
        fallback: &Option<LoadedPlugin<'js>>,
        kind: StreamKind,
        message: &JsonValue,
    ) -> Result<RouteDecision> {
        let order = ordered_indices(plugins);
        let mut route: Option<PartialRoute> = None;

        for &index in &order {
            self.invoke_route_hook(&plugins[index], "preread", message, &mut route);
        }
        if route.is_none() {
            if let Some(plugin) = fallback {
                self.invoke_route_hook(plugin, "preread", message, &mut route);
            }
        }

        let decision = normalize_route(route.unwrap_or_default()).with_context(|| {
            format!(
                "no plugin produced a {} route and no fallback gateway script is loaded",
                kind.as_str()
            )
        })?;
        self.run_log_hooks(plugins, fallback, message, &decision);
        Ok(decision)
    }

    fn invoke_route_hook(
        &self,
        plugin: &LoadedPlugin<'js>,
        hook: &str,
        message: &JsonValue,
        route: &mut Option<PartialRoute>,
    ) {
        let current = match route {
            Some(partial) => partial_to_json(partial),
            None => JsonValue::Null,
        };
        match self.call_route_hook(plugin, hook, message, &current) {
            Ok(Some(next)) => merge_route(route, next),
            Ok(None) => {}
            Err(error) => {
                tracing::error!(
                    target: "proxysss::script",
                    plugin = %plugin.name,
                    hook,
                    %error,
                    "plugin hook failed; skipping its decision and continuing the pipeline"
                );
            }
        }
    }

    fn call_route_hook(
        &self,
        plugin: &LoadedPlugin<'js>,
        hook: &str,
        message: &JsonValue,
        current: &JsonValue,
    ) -> Result<Option<PartialRoute>> {
        let Some(function) = plugin
            .object
            .get::<_, Option<Function>>(hook)
            .catch(self.ctx)
            .map_err(to_anyhow)?
        else {
            return Ok(None);
        };

        let message_js = json_to_js(self.ctx, message)?;
        let current_js = json_to_js(self.ctx, current)?;

        self.deadline.arm(self.timeout);
        let result: rquickjs::Result<Value> =
            function.call((This(plugin.object.clone()), message_js, current_js));
        self.deadline.disarm();

        let value = result.catch(self.ctx).map_err(to_anyhow)?;
        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }
        if value.as_promise().is_some() {
            // Async hooks are not awaited inside the synchronous pipeline; a
            // pending promise is treated as "no decision". Hooks should be sync.
            return Ok(None);
        }

        let json = js_to_json(value)?;
        let partial: PartialRoute =
            serde_json::from_value(json).context("plugin route decision had an invalid shape")?;
        Ok(Some(partial))
    }

    fn run_log_hooks(
        &self,
        plugins: &[LoadedPlugin<'js>],
        fallback: &Option<LoadedPlugin<'js>>,
        message: &JsonValue,
        decision: &RouteDecision,
    ) {
        let payload = json!({
            "message": message,
            "route": serde_json::to_value(decision).unwrap_or(JsonValue::Null),
        });
        for &index in &ordered_indices(plugins) {
            self.call_log_hook(&plugins[index], &payload);
        }
        if let Some(plugin) = fallback {
            self.call_log_hook(plugin, &payload);
        }
    }

    fn call_log_hook(&self, plugin: &LoadedPlugin<'js>, payload: &JsonValue) {
        let function = match plugin.object.get::<_, Option<Function>>("log") {
            Ok(Some(function)) => function,
            Ok(None) => return,
            Err(error) => {
                tracing::error!(target: "proxysss::script", plugin = %plugin.name, %error, "failed to read log hook");
                return;
            }
        };
        let payload_js = match json_to_js(self.ctx, payload) {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(target: "proxysss::script", plugin = %plugin.name, %error, "failed to marshal log payload");
                return;
            }
        };
        self.deadline.arm(self.timeout);
        let result: rquickjs::Result<Value> =
            function.call((This(plugin.object.clone()), payload_js));
        self.deadline.disarm();
        if let Err(error) = result.catch(self.ctx) {
            tracing::error!(target: "proxysss::script", plugin = %plugin.name, error = %error, "log hook failed");
        }
    }

    fn load_plugin_record(
        &self,
        plugins: &mut Vec<LoadedPlugin<'js>>,
        module_counter: &mut u64,
        spec: ScriptPluginSpec,
    ) -> Result<JsonValue> {
        if spec.name.trim().is_empty() {
            return Err(anyhow!("plugin name is required"));
        }
        if spec.module_path.trim().is_empty() {
            return Err(anyhow!("plugin module_path is required"));
        }

        let path = PathBuf::from(&spec.module_path);
        *module_counter += 1;
        let object = self.load_plugin_object(*module_counter, &spec.name, &path)?;

        let priority = read_meta_i32(&object, "priority").unwrap_or(spec.priority);
        let enabled = read_meta_bool(&object, "enabled").unwrap_or(spec.enabled);
        let loaded_at = now_iso();

        self.run_lifecycle_hook(
            &object,
            "init_worker",
            &json!({ "spec": serde_json::to_value(&spec).unwrap_or(JsonValue::Null) }),
        )?;

        if let Some(existing) = plugins.iter().position(|plugin| plugin.name == spec.name) {
            let old = plugins.remove(existing);
            let _ = self.run_lifecycle_hook(&old.object, "onDispose", &json!({}));
        }

        let info = json!({
            "name": spec.name,
            "module_path": spec.module_path,
            "priority": priority,
            "enabled": enabled,
            "loaded_at": loaded_at,
        });

        plugins.push(LoadedPlugin {
            name: spec.name,
            module_path: spec.module_path,
            priority,
            enabled,
            loaded_at,
            object,
        });

        Ok(info)
    }

    fn unload_plugin_record(
        &self,
        plugins: &mut Vec<LoadedPlugin<'js>>,
        name: &str,
    ) -> Result<JsonValue> {
        let Some(index) = plugins.iter().position(|plugin| plugin.name == name) else {
            return Err(anyhow!("plugin not found: {name}"));
        };
        let plugin = plugins.remove(index);
        let _ = self.run_lifecycle_hook(&plugin.object, "onDispose", &json!({}));
        Ok(json!({ "name": name, "unloaded": true }))
    }

    fn load_plugin_object(
        &self,
        module_id: u64,
        name: &str,
        path: &std::path::Path,
    ) -> Result<Object<'js>> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read script {}", path.display()))?;
        let filename = path.to_string_lossy().to_string();
        let js = transpile_module(&filename, &source)?;
        let module_name = format!("proxysss:plugin:{module_id}:{name}");

        self.deadline.arm(self.timeout.saturating_mul(4));
        let result = (|| -> Result<Object<'js>> {
            let declared = Module::declare(self.ctx.clone(), module_name.into_bytes(), js)
                .catch(self.ctx)
                .map_err(to_anyhow)?;
            // Synchronous modules (no top-level await) are fully evaluated by
            // `eval()`; we must not drive the runtime job queue here because the
            // surrounding `ctx.with` already holds the runtime borrow.
            let (evaluated, _promise) = declared.eval().catch(self.ctx).map_err(to_anyhow)?;
            let default: Value = evaluated
                .get("default")
                .catch(self.ctx)
                .map_err(to_anyhow)?;
            default
                .into_object()
                .ok_or_else(|| anyhow!("script {} must `export default` an object", path.display()))
        })();
        self.deadline.disarm();
        result
    }

    fn run_lifecycle_hook(&self, object: &Object<'js>, hook: &str, arg: &JsonValue) -> Result<()> {
        let Some(function) = object
            .get::<_, Option<Function>>(hook)
            .catch(self.ctx)
            .map_err(to_anyhow)?
        else {
            return Ok(());
        };
        let arg_js = json_to_js(self.ctx, arg)?;
        self.deadline.arm(self.timeout);
        let result: rquickjs::Result<Value> = function.call((This(object.clone()), arg_js));
        self.deadline.disarm();
        if let Err(error) = result.catch(self.ctx) {
            tracing::error!(target: "proxysss::script", hook, error = %error, "lifecycle hook failed");
        }
        Ok(())
    }

    fn install_globals(&self, env: &BTreeMap<String, String>) -> Result<()> {
        let log = Function::new(
            self.ctx.clone(),
            |level: String, message: String| match level.as_str() {
                "error" => tracing::error!(target: "proxysss::script", "{message}"),
                "warn" => tracing::warn!(target: "proxysss::script", "{message}"),
                "debug" => tracing::debug!(target: "proxysss::script", "{message}"),
                _ => tracing::info!(target: "proxysss::script", "{message}"),
            },
        )
        .catch(self.ctx)
        .map_err(to_anyhow)?;
        self.ctx
            .globals()
            .set("__proxysss_log", log)
            .catch(self.ctx)
            .map_err(to_anyhow)?;

        let env_value = json_to_js(
            self.ctx,
            &serde_json::to_value(env).unwrap_or(JsonValue::Null),
        )?;
        self.ctx
            .globals()
            .set("PROXYSSS_ENV", env_value)
            .catch(self.ctx)
            .map_err(to_anyhow)?;

        self.ctx
            .eval::<(), _>(CONSOLE_JS)
            .catch(self.ctx)
            .map_err(to_anyhow)?;
        Ok(())
    }
}

/// Evaluate a single TypeScript/JavaScript module file in a throwaway embedded
/// engine. Used by `proxysss script run-file` / `script eval` to validate
/// scripts without any external runtime; `console.*` output goes to stdout.
pub fn run_module_file(
    path: &std::path::Path,
    env: &BTreeMap<String, String>,
    timeout: Duration,
) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read script {}", path.display()))?;
    let filename = path.to_string_lossy().to_string();
    let js = transpile_module(&filename, &source)?;

    let runtime = rquickjs::Runtime::new().map_err(to_anyhow)?;
    runtime.set_memory_limit(64 * 1024 * 1024);
    let deadline = Arc::new(Deadline::new());
    let interrupt = deadline.clone();
    runtime.set_interrupt_handler(Some(Box::new(move || interrupt.expired())));
    deadline.arm(timeout.max(Duration::from_millis(1)));

    let context = Context::full(&runtime).map_err(to_anyhow)?;
    let mut outcome: Result<()> = Ok(());
    context.with(|ctx| {
        outcome = (|| -> Result<()> {
            let log = Function::new(ctx.clone(), |level: String, message: String| {
                if level == "error" {
                    eprintln!("{message}");
                } else {
                    println!("{message}");
                }
            })
            .catch(&ctx)
            .map_err(to_anyhow)?;
            ctx.globals()
                .set("__proxysss_log", log)
                .catch(&ctx)
                .map_err(to_anyhow)?;
            let env_value =
                json_to_js(&ctx, &serde_json::to_value(env).unwrap_or(JsonValue::Null))?;
            ctx.globals()
                .set("PROXYSSS_ENV", env_value)
                .catch(&ctx)
                .map_err(to_anyhow)?;
            ctx.eval::<(), _>(CONSOLE_JS)
                .catch(&ctx)
                .map_err(to_anyhow)?;

            let declared = Module::declare(ctx.clone(), b"proxysss:eval".to_vec(), js)
                .catch(&ctx)
                .map_err(to_anyhow)?;
            let (_evaluated, _promise) = declared.eval().catch(&ctx).map_err(to_anyhow)?;
            Ok(())
        })();
    });
    deadline.disarm();
    outcome
}

fn plugin_info(plugin: &LoadedPlugin) -> ScriptPluginInfo {
    ScriptPluginInfo {
        name: plugin.name.clone(),
        module_path: plugin.module_path.clone(),
        priority: plugin.priority,
        enabled: plugin.enabled,
        loaded_at: Some(plugin.loaded_at.clone()),
    }
}

fn ordered_indices(plugins: &[LoadedPlugin]) -> Vec<usize> {
    let mut order: Vec<usize> = plugins
        .iter()
        .enumerate()
        .filter(|(_, plugin)| plugin.enabled)
        .map(|(index, _)| index)
        .collect();
    order.sort_by(|&a, &b| plugins[b].priority.cmp(&plugins[a].priority));
    order
}

// ---------------------------------------------------------------------------
// Globals JS, helpers.
// ---------------------------------------------------------------------------

const CONSOLE_JS: &str = r#"
globalThis.console = (() => {
  const fmt = (args) => args.map((a) => (typeof a === "string" ? a : (() => { try { return JSON.stringify(a); } catch (_) { return String(a); } })())).join(" ");
  const emit = (level) => (...args) => globalThis.__proxysss_log(level, fmt(args));
  return { log: emit("info"), info: emit("info"), warn: emit("warn"), error: emit("error"), debug: emit("debug") };
})();
"#;

fn read_meta_i32(object: &Object, key: &str) -> Option<i32> {
    object.get::<_, Option<i32>>(key).ok().flatten()
}

fn read_meta_bool(object: &Object, key: &str) -> Option<bool> {
    object.get::<_, Option<bool>>(key).ok().flatten()
}

fn to_anyhow<E: std::fmt::Display>(error: E) -> anyhow::Error {
    anyhow!("{error}")
}

fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("epoch-ms:{millis}")
}

// ---------------------------------------------------------------------------
// Route merge / normalize (Rust-owned pipeline semantics).
// ---------------------------------------------------------------------------

fn merge_route(current: &mut Option<PartialRoute>, next: PartialRoute) {
    match current {
        None => *current = Some(next),
        Some(cur) => {
            if next.upstream.is_some() {
                cur.upstream = next.upstream;
            }
            if next.upstreams.is_some() {
                cur.upstreams = next.upstreams;
            }
            if next.affinity_key.is_some() {
                cur.affinity_key = next.affinity_key;
            }
            if next.rewrite_path.is_some() {
                cur.rewrite_path = next.rewrite_path;
            }
            if let Some(headers) = next.set_headers {
                let target = cur.set_headers.get_or_insert_with(BTreeMap::new);
                for (key, value) in headers {
                    target.insert(key, value);
                }
            }
            if let Some(strip) = next.strip_headers {
                let target = cur.strip_headers.get_or_insert_with(Vec::new);
                for item in strip {
                    if !target.contains(&item) {
                        target.push(item);
                    }
                }
            }
            if next.status.is_some() {
                cur.status = next.status;
            }
            if next.content_type.is_some() {
                cur.content_type = next.content_type;
            }
        }
    }
}

fn normalize_route(partial: PartialRoute) -> Result<RouteDecision> {
    let upstream = partial
        .upstream
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("route.upstream is required"))?;

    let mut upstreams: Vec<String> = partial
        .upstreams
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    if !upstreams.is_empty() && !upstreams.contains(&upstream) {
        upstreams.insert(0, upstream.clone());
    }

    Ok(RouteDecision {
        upstream,
        upstreams,
        affinity_key: partial.affinity_key,
        rewrite_path: partial.rewrite_path,
        set_headers: partial.set_headers.unwrap_or_default(),
        strip_headers: partial.strip_headers.unwrap_or_default(),
        status: partial.status,
        content_type: partial.content_type,
    })
}

fn partial_to_json(partial: &PartialRoute) -> JsonValue {
    let mut map = serde_json::Map::new();
    if let Some(value) = &partial.upstream {
        map.insert("upstream".into(), json!(value));
    }
    if let Some(value) = &partial.upstreams {
        map.insert("upstreams".into(), json!(value));
    }
    if let Some(value) = &partial.affinity_key {
        map.insert("affinity_key".into(), json!(value));
    }
    if let Some(value) = &partial.rewrite_path {
        map.insert("rewrite_path".into(), json!(value));
    }
    if let Some(value) = &partial.set_headers {
        map.insert("set_headers".into(), json!(value));
    }
    if let Some(value) = &partial.strip_headers {
        map.insert("strip_headers".into(), json!(value));
    }
    if let Some(value) = partial.status {
        map.insert("status".into(), json!(value));
    }
    if let Some(value) = &partial.content_type {
        map.insert("content_type".into(), json!(value));
    }
    JsonValue::Object(map)
}

// ---------------------------------------------------------------------------
// JSON <-> JS value conversion.
// ---------------------------------------------------------------------------

fn json_to_js<'js>(ctx: &Ctx<'js>, value: &JsonValue) -> Result<Value<'js>> {
    use rquickjs::{Array, IntoJs};
    match value {
        JsonValue::Null => Ok(Value::new_null(ctx.clone())),
        JsonValue::Bool(boolean) => Ok(Value::new_bool(ctx.clone(), *boolean)),
        JsonValue::Number(number) => {
            if let Some(int) = number.as_i64() {
                if let Ok(small) = i32::try_from(int) {
                    return Ok(Value::new_int(ctx.clone(), small));
                }
                return Ok(Value::new_float(ctx.clone(), int as f64));
            }
            Ok(Value::new_float(
                ctx.clone(),
                number.as_f64().unwrap_or(0.0),
            ))
        }
        JsonValue::String(text) => text.as_str().into_js(ctx).catch(ctx).map_err(to_anyhow),
        JsonValue::Array(items) => {
            let array = Array::new(ctx.clone()).catch(ctx).map_err(to_anyhow)?;
            for (index, item) in items.iter().enumerate() {
                let js = json_to_js(ctx, item)?;
                array.set(index, js).catch(ctx).map_err(to_anyhow)?;
            }
            Ok(array.into_value())
        }
        JsonValue::Object(map) => {
            let object = Object::new(ctx.clone()).catch(ctx).map_err(to_anyhow)?;
            for (key, item) in map {
                let js = json_to_js(ctx, item)?;
                object.set(key.as_str(), js).catch(ctx).map_err(to_anyhow)?;
            }
            Ok(object.into_value())
        }
    }
}

fn js_to_json(value: Value) -> Result<JsonValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(JsonValue::Null);
    }
    if let Some(boolean) = value.as_bool() {
        return Ok(JsonValue::Bool(boolean));
    }
    if let Some(int) = value.as_int() {
        return Ok(JsonValue::Number(int.into()));
    }
    if let Some(float) = value.as_float() {
        return Ok(serde_json::Number::from_f64(float)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null));
    }
    if let Some(string) = value.as_string() {
        let text = string.to_string().map_err(to_anyhow)?;
        return Ok(JsonValue::String(text));
    }
    if let Some(array) = value.as_array() {
        let mut items = Vec::with_capacity(array.len());
        for index in 0..array.len() {
            let element: Value = array.get(index).map_err(to_anyhow)?;
            items.push(js_to_json(element)?);
        }
        return Ok(JsonValue::Array(items));
    }
    if let Some(object) = value.as_object() {
        let mut map = serde_json::Map::new();
        for entry in object.props::<String, Value>() {
            let (key, element) = entry.map_err(to_anyhow)?;
            map.insert(key, js_to_json(element)?);
        }
        return Ok(JsonValue::Object(map));
    }
    Ok(JsonValue::Null)
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("proxysss-script-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &std::path::Path, name: &str, source: &str) -> String {
        let path = dir.join(name);
        std::fs::write(&path, source).unwrap();
        path.to_string_lossy().to_string()
    }

    fn base_config(dir: &std::path::Path, entry: &str) -> ScriptConfig {
        ScriptConfig {
            enabled: true,
            entry: PathBuf::from(entry),
            cwd: Some(dir.to_path_buf()),
            timeout_ms: 2000,
            ..ScriptConfig::default()
        }
    }

    fn http(path: &str, player: Option<&str>) -> HttpContext {
        HttpContext {
            request_id: "req".into(),
            host: "example.com".into(),
            method: "GET".into(),
            path: path.into(),
            query: None,
            scheme: "http".into(),
            version: "1.1".into(),
            remote_addr: "127.0.0.1:1".into(),
            player_id: player.map(|p| p.to_string()),
            headers: BTreeMap::new(),
            body_len: 0,
        }
    }

    #[tokio::test]
    async fn fallback_script_routes_http() {
        let dir = temp_dir();
        write(
            &dir,
            "gateway.ts",
            r#"
            export default {
                name: "gateway",
                access(message: { ctx: { path?: string } }) {
                    if (message.ctx.path === "/healthz") {
                        return { upstream: "proxysss://healthz" };
                    }
                    return { upstream: "http://127.0.0.1:8080", set_headers: { "x-gateway": "proxysss" } };
                },
            };
            "#,
        );
        let runtime =
            ScriptRuntime::spawn(&base_config(&dir, "gateway.ts"), &BTreeMap::new()).unwrap();
        let route = runtime.route_http(http("/healthz", None)).await.unwrap();
        assert_eq!(route.upstream, "proxysss://healthz");

        let route = runtime.route_http(http("/anything", None)).await.unwrap();
        assert_eq!(route.upstream, "http://127.0.0.1:8080");
        assert_eq!(
            route.set_headers.get("x-gateway").map(String::as_str),
            Some("proxysss")
        );
    }

    #[tokio::test]
    async fn plugin_overrides_fallback_and_failures_are_isolated() {
        let dir = temp_dir();
        write(
            &dir,
            "gateway.ts",
            r#"export default { name: "gateway", access() { return { upstream: "http://127.0.0.1:8080" }; } };"#,
        );
        let plugin_path = write(
            &dir,
            "affinity.ts",
            r#"
            export default {
                name: "affinity",
                priority: 100,
                access(message: { ctx: { path?: string; player_id?: string } }) {
                    if ((message.ctx.path ?? "").startsWith("/sdk/login")) {
                        return { upstream: "http://127.0.0.1:9000", affinity_key: message.ctx.player_id };
                    }
                },
            };
            "#,
        );
        let broken_path = write(
            &dir,
            "broken.ts",
            r#"export default { name: "broken", priority: 200, access() { throw new Error("boom"); } };"#,
        );

        let runtime =
            ScriptRuntime::spawn(&base_config(&dir, "gateway.ts"), &BTreeMap::new()).unwrap();
        runtime
            .load_plugin(ScriptPluginSpec {
                name: "affinity".into(),
                module_path: plugin_path,
                priority: 0,
                enabled: true,
                config: JsonValue::Null,
            })
            .await
            .unwrap();
        runtime
            .load_plugin(ScriptPluginSpec {
                name: "broken".into(),
                module_path: broken_path,
                priority: 0,
                enabled: true,
                config: JsonValue::Null,
            })
            .await
            .unwrap();

        let route = runtime
            .route_http(http("/sdk/login", Some("player-7")))
            .await
            .unwrap();
        assert_eq!(route.upstream, "http://127.0.0.1:9000");
        assert_eq!(route.affinity_key.as_deref(), Some("player-7"));

        let plugins = runtime.list_plugins().await.unwrap();
        assert!(plugins.iter().any(|plugin| plugin.name == "affinity"));
    }

    #[tokio::test]
    async fn infinite_loop_plugin_is_interrupted() {
        let dir = temp_dir();
        write(
            &dir,
            "gateway.ts",
            r#"export default { name: "gateway", access() { return { upstream: "http://127.0.0.1:8080" }; } };"#,
        );
        let loop_path = write(
            &dir,
            "loop.ts",
            r#"export default { name: "loop", priority: 50, access() { while (true) {} } };"#,
        );

        let mut config = base_config(&dir, "gateway.ts");
        config.timeout_ms = 200;
        let runtime = ScriptRuntime::spawn(&config, &BTreeMap::new()).unwrap();
        runtime
            .load_plugin(ScriptPluginSpec {
                name: "loop".into(),
                module_path: loop_path,
                priority: 0,
                enabled: true,
                config: JsonValue::Null,
            })
            .await
            .unwrap();

        let route = runtime.route_http(http("/", None)).await.unwrap();
        assert_eq!(route.upstream, "http://127.0.0.1:8080");
    }
}
