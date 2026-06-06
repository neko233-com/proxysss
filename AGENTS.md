# AGENTS

## Product Direction

proxysss exists to **fully replace nginx** as a same-level general-purpose gateway. It is **not** a business gateway. Business-aware behavior belongs in optional scripts/plugins, similar to using Lua modules with nginx.

Keep these invariants aligned across code, docs, examples, tests, and generated config:

- Default public HTTP port is `80` (nginx parity).
- Default public `/` route is a polished `Welcome to proxysss` page.
- Default admin console/API port is `7777`.
- Default HTTPS/HTTP2/HTTP3 port is `443`.
- Default FTP control port is `21` when `services.ftp.enabled=true`.
- Configuration must be more human-friendly than nginx while still covering nginx-level gateway duties.
- Sub-config support must be explicit through `include.enabled` and `include.files`; do not silently scan include directories.
- CLI output must stay easy for agents to inspect quickly through commands such as `proxysss config explain`, `proxysss config includes`, `proxysss config capabilities`, `proxysss config routes`, `proxysss config reload-plan`, and `proxysss config nginx-parity`.
- FTP, WebDAV, HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, static/reverse-proxy style behavior, logging, reload, and service operation are nginx-parity requirements, not optional marketing text.
- Config, explicit include files, the main extension script, and auto-loaded plugin scripts must participate in hot reload.
- Logging must expose access logs (`logs/access.log`), error logs (`logs/error.log`), and level control for `debug`, `info`, `warn`, and `error`; default to `info`, with `debug` reserved for internal diagnostics.
- Official demo plugins ship with `proxysss init`: `structured-log` (log hook demo), `traffic-stats` (traffic/error counters), and `player-affinity` (affinity routing demo).
- Automated tests should protect nginx-parity defaults and capability declarations whenever related code changes.
- Legacy compatibility is not a product constraint unless the user explicitly asks for it. Prefer clean, high-performance, maintainable designs over preserving old internal shapes.
- Performance must be treated as a core product requirement: aim for nginx-class throughput/latency and leave room to exceed nginx where proxysss can use Rust, async IO, and script isolation effectively.
- Architecture should favor extensibility without putting hot-path traffic behind unnecessary dynamic dispatch, allocation, serialization, or script calls.
- TypeScript scripting is a required product surface, and it is implemented as a single `proxysss` binary with an embedded TypeScript-capable engine (QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip`) executing inside the process with hot reload. There is no external `deno`/`node`/`tsc` dependency, no bundled `deno.exe`, and no sidecar runtime directory — those are removed legacy states. See `specs/embedded-ts-runtime.md`.

## What proxysss Is

| Layer | Responsibility |
| --- | --- |
| Core gateway (Rust) | nginx-equivalent protocol termination, routing, static files, WebDAV, stream proxy, TLS, rate limits, logging, reload |
| Extension scripts (TS/JS via embedded proxysss runtime target) | Optional business routing, plugins, affinity, custom upstream selection — like nginx + Lua |
| Admin API (`127.0.0.1:7777`) | Health, stats, config inspect, plugin load/unload, manual reload |

Do **not** describe proxysss as "more business gateway than nginx". Describe it as a **general gateway with script/plugin extension hooks**.

## Agent Skills

| Skill | Path | Use when |
| --- | --- | --- |
| Install / update proxysss | `skills/proxysss-install/SKILL.md` | Bootstrap gateway, verify ports 80 and 7777 |
| Monitor GitHub Actions | `skills/gh-cli/SKILL.md` | Inspect CI/release runs, logs, reruns, release assets |
| Edit workflow YAML | `.github/skills/github-actions/SKILL.md` | Fix or extend `.github/workflows/*` locally |

### Mandatory agent rules

- **GitHub Actions / release work must use `skills/gh-cli/SKILL.md`.** Do not guess workflow status from memory or stale logs. Always confirm with `gh run list`, `gh run view`, `gh run watch`, and `gh release view` before reporting success or failure.
- **Workflow JavaScript actions must target Node.js 24 LTS or newer.** Use `actions/upload-artifact@v6` and `actions/download-artifact@v6` (or later). Do not add `actions/*-artifact@v4` or other Node 20 actions without upgrading.
- **Release tags require a matching changelog section on the same commit.** Before pushing `vX.Y.Z`, ensure `CHANGELOG.md` contains `## vX.Y.Z` and `Cargo.toml` `version` matches. CI validates this; `release.yml` publish fails if the section is missing.

One-click bootstrap for autonomous agents:

- `skills/proxysss-install/SKILL.md`
- `skills/gh-cli/SKILL.md`

After install, hand off these inspect commands:

```bash
proxysss config explain
proxysss config capabilities
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
```

## Hot Reload Boundaries

**Hot-reloadable (fingerprinted):**

- Merged configuration values except listener identity
- Explicit `include.files`
- Main script (`script.args`)
- Auto-loaded plugin scripts (`plugins.auto_load_dir`)
- `services.reverse_proxy.routes`, `static_sites`, `webdav`, FTP upstream

**Restart required:**

- `http.plain_bind`, `http.tls_bind`, `http.h3_bind`
- `admin.enabled`, `admin.bind`
- TCP/UDP listener name/bind sets
- `services.ftp.enabled`, `services.ftp.bind`
- `http.tls.mode`
- `logging.format`, `logging.filter`, `logging.level`
- `logging.access_log_path`, `logging.error_log_path`

## Development Rules

- Prefer adding real runtime capability over documenting intent.
- When a capability is incomplete, represent that honestly in code/docs and keep moving it toward full nginx parity.
- Any new user-facing command should be useful for both humans and autonomous agents.
- Keep install paths and startup instructions scriptable so an agent can bootstrap proxysss without manual discovery.
- Prefer CLI inspection surfaces for agent workflows.
- For hot-path code, measure or reason about throughput, allocation pressure, backpressure, and lock contention before adding abstractions.
- Do not keep legacy code merely because it already exists; if a simpler high-performance design better serves nginx replacement, migrate decisively and cover it with tests.

## Known nginx Parity Gaps (track honestly)

- Response compression (gzip/brotli)
- Proxy cache zones
- Native FTP passive/active channel awareness (currently TCP passthrough)
- Multi-cert SNI certificate selection

These are tracked in `proxysss config nginx-parity` and should move toward `supported` with tests, not disappear from the matrix.
