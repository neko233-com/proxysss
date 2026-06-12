# AGENTS

## Product Direction

proxysss exists to **fully replace nginx** as a same-level general-purpose gateway. It is **not** a business gateway. Business-aware behavior belongs in optional scripts/plugins, similar to using Lua modules with nginx.

Keep these invariants aligned across code, docs, examples, tests, and generated config:

- Default public HTTP port is `80` (nginx parity).
- Default public `/` route is a polished `Welcome to proxysss` page.
- Default admin console/API port is `7777` on loopback with `enable_write_ops=false` and `expose_config=false` until explicitly enabled for automation.
- Default HTTPS/HTTP2/HTTP3 port is `443`.
- Default FTP control port is `21` when `services.ftp.enabled=true`.
- Configuration must be more human-friendly than nginx while still covering nginx-level gateway duties.
- Runtime configuration should live in a single YAML file by default, normally `proxysss.yaml`; custom locations are selected with `-config`, `--config`, or `-c`.
- Admin API automation may update `services.domain_routes`, `services.reverse_proxy.routes`, `tcp.listeners`, `udp.listeners`, `tcp.stream_routes`, `http.tls.*`, `services.filecloud`, and dynamic blacklist entries over token-authenticated HTTP; loopback admin bootstraps TLS/ACME, then `admin.https` exposes the same `/v1/*` surface on the main TLS listener.
- Local token inspection and rotation should go through `proxysss token show` / `proxysss token set`; general config display paths should redact secrets instead of exposing them.
- Wildcard ACME certificates use built-in managed DNS-01: `http.tls.mode: acme_managed`, `http.tls.acme.challenge: dns01`, `http.tls.acme.dns.provider`, and redacted `http.tls.acme.dns.credentials`. One cloud vendor = one provider strategy (`aliyun_cn` vs `aliyun_intl` are separate). Legacy `acme_dns_external` + `acme.sh` remains for non-built-in providers only.
- CLI output must stay easy for agents to inspect quickly through commands such as `proxysss config explain`, `proxysss config capabilities`, `proxysss config routes`, `proxysss config reload-plan`, and `proxysss config nginx-parity`.
- FTP, WebDAV, HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, static/reverse-proxy style behavior, logging, reload, and service operation are nginx-parity requirements, not optional marketing text.
- New API, sub2api, and OpenAI-compatible AI reverse proxy routes are first-class `services.ai_proxy` gateway behavior and must stay supported in code, docs, examples, tests, and generated config surfaces.
- Large-scale AI tool, game, voice, MQTT/IoT device, and KCP-style traffic must stay supported through direct `tcp.listeners`/`udp.listeners`: TCP exposes `nodelay` and `connect_timeout_ms`; UDP exposes `session_ttl_secs` and `max_associations` to bound realtime churn.
- MQTT/IoT support means transparent MQTT TCP, MQTT TLS passthrough/SNI, MQTT over WebSocket, CoAP-style UDP, stream rate/access policy, active health, and templates/examples. Do not claim proxysss is an MQTT broker; it is the edge gateway in front of brokers/device services.
- Production stability is a first-class surface: `load_balance.active_health` must cover HTTP/TCP and opt-in UDP probes; `runtime.watchdog` must keep critical background loops observable and restartable; `/metrics` must expose watchdog and task-failure counters.
- The main YAML config, the main extension script, and auto-loaded plugin scripts must participate in hot reload.
- Logging must expose access logs (`logs/access.log`), error logs (`logs/error.log`), and level control for `debug`, `info`, `warn`, and `error`; default to `info`, with `debug` reserved for internal diagnostics.
- Official demo plugins ship with `proxysss init`: `structured-log` (log hook demo), `traffic-stats` (traffic/error counters), and `player-affinity` (affinity routing demo).
- Automated tests should protect nginx-parity defaults and capability declarations whenever related code changes.
- `AGENTS.md`、内建 `docs.html` / `/docs` 页面、`docs/architecture.html`、`ts-how-to-use.md`、`nginx-to-proxysss.md`、`proxysss-script.d.ts` 必须随能力和脚本 API 一起维护，不允许文档长期落后于实现。
- **Agents MUST update `docs/architecture.html`** whenever proxysss architecture or request/data-path behavior changes (new listeners, policy chains, reload boundaries, extension hooks, etc.).
- Legacy compatibility is not a product constraint unless the user explicitly asks for it. Prefer clean, high-performance, maintainable designs over preserving old internal shapes.
- Performance must be treated as a core product requirement: aim for nginx-class throughput/latency and leave room to exceed nginx where proxysss can use Rust, async IO, and script isolation effectively.
- Architecture should favor extensibility without putting hot-path traffic behind unnecessary dynamic dispatch, allocation, serialization, or script calls.
- TypeScript scripting is a required product surface, and it is implemented as a single `proxysss` binary with an embedded TypeScript-capable engine (QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip`) executing inside the process with hot reload. There is no external `deno`/`node`/`tsc` dependency, no bundled `deno.exe`, and no sidecar runtime directory — those are removed legacy states. See `specs/embedded-ts-runtime.md`.

## What proxysss Is

| Layer | Responsibility |
| --- | --- |
| Core gateway (Rust) | nginx-equivalent protocol termination, routing, static files, WebDAV, stream proxy, TLS, rate limits, logging, reload |
| Extension scripts (TS/JS via embedded proxysss runtime target) | Optional business routing, plugins, affinity, custom upstream selection — like nginx + Lua |
| Admin API (`127.0.0.1:7777`) | Health, stats, config inspect, plugin load/unload, manual reload, token-authenticated route/listener upsert |

Do **not** describe proxysss as "more business gateway than nginx". Describe it as a **general gateway with script/plugin extension hooks**.

## Agent Skills

| Skill | Path | Use when |
| --- | --- | --- |
| Install / update proxysss | `skills/proxysss-install/SKILL.md` | Bootstrap gateway, verify ports 80 and 7777 |
| Monitor GitHub Actions | `skills/gh-cli/SKILL.md` | Inspect CI/release runs, logs, reruns, release assets |
| ACME DNS-01 cloud providers | `skills/acme-dns-providers/SKILL.md` | Add/fix built-in DNS-01 strategies under `src/acme/dns/` |
| Edit workflow YAML | `.github/skills/github-actions/SKILL.md` | Fix or extend `.github/workflows/*` locally |

## Local Agent Runtime Files

- `.ssh/` is a local-only operator workspace for test server connection records, migration notes, temporary SSH keys, known hosts, and deployment artifacts. It must stay ignored by git and must not be committed.

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
- The main `proxysss.yaml` file
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
- 任何涉及脚本 API、内建文档、nginx 对照配置、模板、错误页、运维入口的变更，都必须同步更新 `AGENTS.md`、内建 `docs.html`、`docs/architecture.html`、`ts-how-to-use.md`、`nginx-to-proxysss.md`、`proxysss-script.d.ts`、README/模板/测试中的对应内容。
- For hot-path code, measure or reason about throughput, allocation pressure, backpressure, and lock contention before adding abstractions.
- Do not keep legacy code merely because it already exists; if a simpler high-performance design better serves nginx replacement, migrate decisively and cover it with tests.

## Known nginx Parity Gaps (track honestly)

- FTP: nginx ftp module directive-level parity is supported through control-channel proxying, passive/active data rewriting, `allow`/`deny`, `command_allow`/`command_deny`, `transfer_allow`/`transfer_deny`, per-user `user_policies`, timeouts, passive port ranges, public passive address rewriting, rate/login knobs, and structured control/transfer lifecycle logs.
- Compression: supported via `services.response_policy` and route overrides (zstd/brotli/gzip).
- Proxy cache: Cloudflare-style behaviors (`bypass`, `respect_origin`, `override`, `no_cache`), edge/browser TTL, `CDN-Cache-Control`, `stale_while_revalidate_secs`, `stale_if_error_secs`, PURGE, `vary_headers`, and `key_prefix`.
- Domain stream proxy: `tcp.stream_routes` for Redis/MySQL/PostgreSQL/MongoDB-style TLS SNI routing with optional per-route access control.
- On-demand TLS: `http.tls.on_demand` with managed ACME first-hit issuance, `allow` glob patterns, optional `ask_url`, and rate limits.
- DDoS mitigation: `security.ddos` sliding-window bans, dynamic blacklist admin API (`/v1/security/blacklist/*`), and `services.access_control.stream`.
- Rate limiting: supported with fixed-window, token-bucket, or leaky-bucket HTTP policies, stream shared zones, and HTTP connection caps.
- Wildcard DNS-01 certificates are built into managed ACME via `http.tls.mode: acme_managed` + `http.tls.acme.challenge: dns01` and strategy-factory DNS providers (`cloudflare`, `aliyun_cn`, `aliyun_intl`, `tencent`, `volcengine`, `aws`, `azure`, `google`). Legacy `acme_dns_external` + `acme.sh` remains only for providers not yet implemented natively.

These are tracked in `proxysss config nginx-parity` and should move toward `supported` with tests, not disappear from the matrix.
