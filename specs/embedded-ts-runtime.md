# Spec: Embedded TypeScript Runtime (proxysss + TS = nginx + Lua)

Status: Accepted (autonomous landing)
Owners: proxysss core
Supersedes: external `deno` sidecar script runtime

## 1. Problem & Goal

proxysss previously executed gateway/plugin TypeScript by spawning an external
`deno` process and talking JSON over stdin/stdout. That approach has three
problems:

1. It requires a bundled or system `deno` binary (a sidecar), violating the
   product goal of a single self-contained `proxysss` binary.
2. Cross-process debugging makes input/output hard to control deterministically.
3. A misbehaving script process is harder to isolate, bound, and observe.

**Goal:** Embed a TypeScript-capable execution engine *inside* the proxysss
binary so that:

- There is **no external `deno`, `node`, or `tsc`** dependency. One binary.
- Plugin/script input and output are **fully controlled in Rust** (no IPC).
- A buggy plugin (throw, infinite loop, runaway memory) **never** affects normal
  proxy traffic and is reported to the error log on every failing invocation.
- We reach **nginx + Lua** ergonomics: `proxysss` core (Rust) + TypeScript
  extension hooks, where the hot proxy path stays native and scripts are an
  optional routing/observability layer.

## 2. Engine Decision

| Option | Engine | Embed | Perf | Binary/build | Safety controls | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| **A (chosen)** | QuickJS via `rquickjs` (statically linked C) + `swc_ts_fast_strip` for TS→JS | True single binary, no downloads | Good; ample for routing hooks | Small, fast build, pre-generated Windows MSVC bindings | Interrupt handler (hard timeout), memory limit, per-call isolation, `catch_unwind` | **Land** |
| B | V8 via `deno_core` | Single binary but prebuilt V8 download | Best raw JS | Large binary, slow first build, V8 blob | Isolate terminate, heap limits | Rejected: heavyweight, conflicts with lean-binary goal |
| C | `boa_engine` (pure Rust) | Single binary, no C/V8 | Lowest | Portable | Step budget | Rejected: perf/maturity |

`deno_runtime` specifically is rejected: it drags in the whole Deno ops and
permissions ecosystem, which proxysss does not need.

**Rationale for A:** QuickJS links statically (no V8 download, no sidecar),
exposes a hard interrupt handler and per-runtime memory cap, and each plugin
call is isolated. We trade some raw JS throughput vs V8, but plugin hooks are an
*optional* layer — the proxy fast path remains native Rust — so this is the
correct tradeoff for a gateway that prizes control, safety, and a single binary.

TypeScript support uses `swc_ts_fast_strip` (the same fast type-stripper behind
Node.js `--experimental-strip-types` and Deno). QuickJS already runs modern
ECMAScript, so only type *syntax* is removed; no down-leveling.

## 3. Architecture

```
HTTP/TCP/UDP request
        │
        ▼
 Native Rust routing (static sites, builtin routes, YAML reverse proxy, FTP)
        │  (matched? → serve natively, scripts never involved)
        ▼  (no native match AND script enabled)
 ScriptRuntime  ── async channel ──▶  ScriptWorker (dedicated OS thread)
   (public API)                         owns rquickjs Runtime + Context
                                        owns transpiled plugin modules
                                        runs pipeline + hooks IN RUST
```

### 3.1 Threading model

QuickJS runtimes are not `Sync`. A single dedicated OS thread (`ScriptWorker`)
owns the `rquickjs::Runtime` and `Context` and all loaded plugin state. The async
`ScriptRuntime` handle (held by the gateway) sends `Job`s over an `mpsc` channel;
each `Job` carries the request and a `oneshot` responder. This mirrors the prior
process-pending-map design, but the "process" is now an in-process worker — no
IPC, no serialization across a pipe.

### 3.2 Safety & isolation (core requirement)

- **Hard timeout:** an interrupt handler checks a shared deadline
  (`Arc<AtomicU64>` nanos). Before each hook call the worker arms the deadline
  (`script.timeout_ms`); a runaway loop is interrupted and surfaced as an error.
- **Memory cap:** `runtime.set_memory_limit(bytes)` and
  `runtime.set_max_stack_size(bytes)` bound a plugin's resource use.
- **Per-hook error isolation:** every plugin hook is invoked independently; a
  throw/timeout/JS exception is caught, **error-logged** (`tracing::error!` with
  plugin name + hook + reason), the offending hook is skipped, and the pipeline
  continues with remaining plugins.
- **Panic safety:** the worker loop wraps dispatch in `catch_unwind` so a Rust
  panic in glue code cannot kill the worker thread or the process.
- **No native match dependency:** native/YAML routing executes *before* scripts,
  so script failures cannot break normal proxy traffic. When the script is the
  fallback router and fails, the request returns a clean gateway error and the
  `script_fail_total` counter increments.

### 3.3 Pipeline (orchestrated in Rust)

The previous JS harness owned pipeline ordering. It now lives in Rust for full
control of inputs/outputs:

- HTTP: `access` hooks (priority desc) → `balancer` hooks → fallback → normalize
  → `log` hooks.
- TCP/UDP: `preread` hooks → fallback → normalize → `log` hooks.

`RouteDecision` merge/normalize semantics are reimplemented in Rust:
- merge: scalar fields from later hook override; `set_headers` shallow-merged;
  `strip_headers` union (deduped); `upstreams` from later hook if present.
- normalize: `upstream` is required (trimmed, non-empty); `upstreams` trimmed and
  the primary `upstream` is prepended if missing; header maps default to empty.

## 4. Host ↔ Script Contract (unchanged for authors)

A plugin/script is a TypeScript (or JS) ES module with a `default` export:

```ts
export default {
  name?: string,
  priority?: number,   // higher runs first
  enabled?: boolean,
  init_worker?(ctx: { spec }): void | Promise<void>,
  onDispose?(): void | Promise<void>,
  access?(message, current?): RouteDecision | void,
  balancer?(message, current?): RouteDecision | void,
  preread?(message, current?): RouteDecision | void,   // TCP/UDP
  log?(ctx: { message, route }): void,
};
```

`message = { id, kind, listener?, ctx }`. Hooks are synchronous (a returned
Promise is settled by draining the QuickJS job queue, bounded by the call
deadline). The **main gateway script** is loaded as the lowest-priority plugin,
so it provides default/fallback routing that user plugins can override — exactly
the nginx default-server + Lua override model.

The old stdin/stdout harness in `templates/gateway.ts` is removed; the template
becomes a normal default-export plugin providing house routing.

## 5. Configuration

`script` and `plugins` config keep their author-facing fields. Removed:
external-runtime fields (`script.command`, deno-style `script.args`, the managed
deno path helper) and the deno download in `install`.

Retained: `script.enabled`, `script.entry`, `script.cwd`, `script.env`,
`script.timeout_ms`, `plugins.enabled`, `plugins.auto_load_dir`,
`plugins.extensions`, `plugins.allow_admin_manage`. New optional safety knobs:
`script.memory_limit_mb` (default 64), `script.max_stack_size_kb` (default 512).

Hot reload boundaries are unchanged: merged config (except listener identity),
explicit include files, the main script, and auto-loaded plugin scripts are all
hot-reloadable. The embedded worker is rebuilt on reload like the prior runtime.

## 6. Public Rust API (preserved)

`ScriptRuntime` keeps its surface so `gateway.rs` call sites are unchanged:

```rust
ScriptRuntime::spawn(&ScriptConfig, &BTreeMap<String,String>) -> Result<Self>
async route_http(HttpContext) -> Result<RouteDecision>
async route_tcp(StreamContext) -> Result<RouteDecision>
async route_udp(StreamContext) -> Result<RouteDecision>
async list_plugins() -> Result<Vec<ScriptPluginInfo>>
async load_plugin(ScriptPluginSpec) -> Result<Value>
async unload_plugin(&str) -> Result<Value>
```

## 7. Verification

- **Unit:** TS transpile (strip types, enums, pass-through JS, syntax errors);
  plugin load/transpile/eval; route dispatch + merge/normalize; error isolation
  (throwing plugin → error logged, pipeline continues); timeout interrupt;
  memory limit; unload/dispose.
- **Integration:** load the shipped demo plugins, run HTTP/TCP/UDP dispatch, and
  assert affinity/log/stats behavior.
- **Parity guards:** existing tests for nginx-parity defaults and capability
  matrix continue to pass; `proxysss config watched-scripts` still lists the main
  script + plugin files.
- **Packaging:** `cargo build --release` produces a single binary with **no**
  external runtime; `scripts/verify-release.ps1` asserts the binary boots,
  serves the default welcome page on `:80`, exposes admin on `:7777`, and runs a
  script-routed request without any `deno` present on `PATH`.

## 8. Out of Scope / Non-goals

- Full Deno std library / permissions model.
- npm/ES module graph resolution across files (each plugin is self-contained).
- Down-leveling to old ECMAScript targets.
