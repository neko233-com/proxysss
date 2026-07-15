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
- Domain-only automatic WSS uses built-in managed ACME with `http.tls.auto_https.domains`; its production default is zero-external-tool TLS-ALPN-01 (DNS plus public 443), while explicit `challenge: http01` remains supported for existing port-80 deployments.
- Wildcard ACME certificates use built-in managed DNS-01: `http.tls.mode: acme_managed`, `http.tls.acme.challenge: dns01`, `http.tls.acme.dns.provider`, and redacted `http.tls.acme.dns.credentials`. One cloud vendor = one provider strategy (`aliyun_cn` vs `aliyun_intl` are separate). Legacy `acme_dns_external` + `acme.sh` remains for non-built-in providers only.
- CLI output must stay easy for agents to inspect quickly through commands such as `proxysss config explain`, `proxysss config capabilities`, `proxysss config routes`, `proxysss config reload-plan`, and `proxysss config nginx-parity`.
- FTP, WebDAV, HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, static/reverse-proxy style behavior, logging, reload, and service operation are nginx-parity requirements, not optional marketing text.
- Static asset serving must cover HTML, CSS, JS, images, fonts, audio/video, CDN-origin assets, large-file streaming, and byte `Range` resumable downloads.
- API gateway surfaces must cover HTTP/1.1, HTTP/2, gRPC-style requests, WebSocket reverse proxying, edge rate limiting, active/passive health, retry, weighted/canary upstreams, and circuit-breaker-style passive quarantine without making proxysss a business gateway.
- WAF, anti-CC, hotlink, and crawler defenses must stay split correctly: generic edge primitives live in core config (`security`, `services.access_control`, `services.rate_limit`), while business-specific Referer/User-Agent/bot-score decisions belong in scripts/plugins.
- Consul, etcd, and Nacos registry linkage uses `services.service_discovery` plus admin/automation-updated upstream pools; ordinary HTTP/TCP/UDP hot paths must select from in-memory pools and not do per-request registry network calls.
- Kubernetes ingress-style service mapping, CDN origin use, and IPv6 bind/access-control rules are supported gateway scenarios and must stay represented in examples, docs, CLI capability output, and Docker scenario validation.
- New API, sub2api, and OpenAI-compatible AI reverse proxy routes are first-class `services.ai_proxy` gateway behavior and must stay supported in code, docs, examples, tests, and generated config surfaces.
- Large-scale AI tool, game, voice, MQTT/IoT device, KCP-style traffic, and independent QCP UDP traffic must stay supported through direct `tcp.listeners`/`udp.listeners`: TCP exposes `nodelay` and `connect_timeout_ms`; UDP exposes `session_ttl_secs` and `max_associations` to bound realtime churn.
- MQTT/IoT support means transparent MQTT TCP, MQTT TLS passthrough/SNI, MQTT over WebSocket, CoAP-style UDP, stream rate/access policy, active health, and templates/examples. Do not claim proxysss is an MQTT broker; it is the edge gateway in front of brokers/device services.
- Production stability is a first-class surface: `load_balance.active_health` must cover HTTP/TCP and opt-in UDP probes; `runtime.watchdog` must keep critical background loops observable and restartable; `/metrics` must expose watchdog and task-failure counters.
- Runtime performance adaptation is default-on through `runtime.performance.enabled=true`: startup/restart must detect the host OS/distro version, log enabled and skipped tuning, use Ubuntu 24.x extreme Linux socket tuning when supported, and downgrade older/unknown systems with an explicit log reason instead of silently pretending it is active. Persistent `proxysss tune linux --apply` host changes must be SSH-safe by default: no firewall/routing/sshd mutation, unsupported sysctl filtering, backup, and rollback on failed apply.
- The main YAML config, the main extension script, and auto-loaded plugin scripts must participate in hot reload.
- Logging must expose access logs (`logs/access.log`), error logs (`logs/error.log`), and level control for `debug`, `info`, `warn`, and `error`; default to `info`, with `debug` reserved for internal diagnostics.
- Official demo plugins ship with `proxysss init`: `structured-log` (log hook demo), `traffic-stats` (traffic/error counters), and `player-affinity` (affinity routing demo).
- Automated tests should protect nginx-parity defaults and capability declarations whenever related code changes.
- `AGENTS.md`、内建 `docs.html` / `/docs` 页面、`docs/index.html`、`docs/configuration.html`、`docs/architecture.html`、`docs/ts-how-to-use.html`、`docs/nginx-to-proxysss.html`、`docs/caddy-to-proxysss.html`、`docs/CONFIGURATION.md`、`ts-how-to-use.md`、`nginx-to-proxysss.md`、`caddy-to-proxysss.md`、`proxysss-script.d.ts` 必须随能力和脚本 API 一起维护，不允许文档长期落后于实现。
- 官方文档统一 `中文 first`，只在必要术语、协议名、配置面名、命令名处保留 `English`。不要把官方文档写成英文营销页，也不要写成只给熟手看的参数堆。
- `docs/*.html` 是给人看的官方文档入口，公共站点和内建 docs 导航必须只链接 HTML，不要把用户从 HTML 页面跳到 `.md`；`.md` 文档主要给 agent / 机器 / 仓库检索使用，`README.md` 和性能对比文档除外。
- 官方文档中的重要主题应同时维护 `.md` 与 `docs/*.html` 两个入口，尤其是 `docs/index.html`、`docs/configuration.html`、`docs/architecture.html`、`docs/ts-how-to-use.html`、`docs/nginx-to-proxysss.html`、`docs/caddy-to-proxysss.html` 及其对应 Markdown 文档，并保持内容口径一致。
- **Agents MUST update `docs/architecture.html`** whenever proxysss architecture or request/data-path behavior changes (new listeners, policy chains, reload boundaries, extension hooks, etc.).
- Legacy compatibility is not a product constraint unless the user explicitly asks for it. Prefer clean, high-performance, maintainable designs over preserving old internal shapes.
- Performance must be treated as a core product requirement: production performance claims and release gates are Linux-only because proxysss targets real Linux gateway deployments, not Windows/macOS desktop benchmarks. Aim to match or exceed nginx on Ubuntu/Linux after `proxysss tune linux`, where proxysss can use Rust, async IO, Linux `SO_REUSEPORT` HTTP/TCP accept fanout, direct no-policy TCP fast paths, and script isolation effectively.
- Linux data-plane concurrency must adapt automatically to detected CPU cores. Do not hard-code small worker caps for HTTP accept loops, TCP stream accept loops, UDP datagram workers, or stream runtimes; higher-core hosts should get proportionally more accept/runtime parallelism without operator config.
- Hot-path proxy data structures must avoid global locks. Use per-connection state, per-listener/per-worker sharding, atomics, DashMap sharding, or lock-free queues for data-plane pooling. Mutex/RwLock synchronization is acceptable for control plane, configuration reload, certificate management, and one-time static-cache fill coordination, but not as a shared bottleneck in ordinary HTTP/TCP/UDP forwarding.
- Linux performance validation must prioritize mixed multi-proxy load over isolated single-scenario wins: CDN/static delivery, HTTP reverse proxy, generic SSE/streaming HTTP, game/TCP streams, generic TCP streams, and nginx-comparable UDP streams must run together per gateway. New API provider routes and KCP/QCP special UDP encapsulations are not part of the default nginx head-to-head benchmark matrix; `EXTENDED_REALTIME=1` may add transparent QCP forwarding to the same mixed wave, but must not claim QCP frame termination. Single-scenario runs are diagnostic only and must not be used to justify release success by cherry-picking.
- The local/native Ubuntu 24 x86_64 Docker strict matrix starts static-large at two equal flows per detected container CPU for both gateways, then scales every workload together at 1x/2x/4x; one bulk flow per `SO_REUSEPORT` shard is a sparse hash-placement diagnostic, not the saturated release gate. An arm64 local Docker daemon may use `linux/amd64` emulation, but artifacts must record `execution_mode=emulated-amd64` and must not be described as native physical-x86 evidence. GitHub Actions must never run performance benchmarks; it is packaging-only.
- Official benchmark fixtures, protocol mocks, result parsers, and gate/report helpers must use Go or native compiled tooling. Do not depend on Python for ad hoc SSE fixtures or official benchmark/gate paths.
- The default Linux release gate prioritizes game/realtime long-connection production traffic and uses a fair ratio floor instead of blindly requiring `>1.0`: WebSocket long connections, game-style TCP, generic TCP, and nginx-comparable UDP default to `CRITICAL_RATIO=0.97`, allowing a 1-3% gap because proxysss carries many gateway features that nginx commonly needs extra modules or policy config to match. Static-small, CDN hot-update, HTTP reverse proxy, and generic SSE must still run concurrently and stay above the default soft floor (`MIN_RATIO=0.50`) with low errors; HTTPS small static and static-large are diagnostic by default unless explicitly promoted for a TLS/static or bulk-transfer release. Use explicit `CRITICAL_RATIO=1.0` or higher only for strict head-to-head performance experiments. A strict `>1.0` claim needs the CPU-role preflight, balanced repetitions, raw artifacts, and an independent gateway/backend/client-host replay that verifies distinct machine IDs and enforces the declared CPU/nofile cgroup for both candidates. Memory must report cgroup current/peak and per-connection cost, with optional `MemoryMax` for a stated envelope; it must not be rejected merely for missing an arbitrary fixed RAM threshold. Docker-only data must never be presented as cross-host evidence.
- Performance optimization must be side-effect free by default: do not accept a local win that makes sibling scenarios slower, less stable, or more memory-hungry without explicit user approval. Treat cross-scenario regressions in static, reverse-proxy, generic SSE, WebSocket, TCP, and nginx-comparable UDP mixed validation as a failed optimization, not an acceptable trade. KCP/QCP remain capability surfaces; their transparent forwarding may be opt-in extended evidence, but is not part of the default performance benchmark matrix.
- A bounded, evidence-backed memory-for-performance trade is explicitly allowed: proxysss may use reasonably more resident memory when it materially improves throughput or p50/p95/p99 and remains safe in the declared production envelope, but its RSS/cgroup current, peak, and per-connection cost must each stay at or below **2x nginx** in the same fair run. Every such change must report both gateways' current and peak (including per-connection cost), preserve bounded pools/backpressure, and pass the full mixed-scenario regression matrix without OOM or runaway growth. The 4-core/20k-idle-WSS and 4096-active-WSS case is a reproducible reference workload, not an arbitrary fixed RAM admission cap; set `MemoryMax`/Docker memory only when an operator declares a real production budget. Higher 100k-class capacity remains an opt-in diagnostic, not the default production requirement. Saving memory is not a win when it measurably reduces gateway performance.
- Static delivery must preserve small-file benchmark efficiency while streaming large files to avoid per-request memory spikes and preserve backpressure on Linux. `runtime.performance.traffic_profile` is the native knob for this tradeoff: default `small` favors cached small-file/HTTP2/SSE/TCP/UDP feedback, `bulk` favors sendfile/zero-copy large transfers, and `balanced` enables both preload styles.
- Static sites should participate in config-load optimization: after loading config, eligible hot index/top-level static files are preloaded into the bounded cache or have sendfile descriptors prepared according to `runtime.performance.traffic_profile`.
- Config load and hot reload must trigger a fast self-optimization warm-up before listeners serve traffic: preload hot static files into the bounded fast-lane cache and pre-dial reverse-proxy/AI-proxy upstream keepalive pools so the first live request never pays a cold connect. Warm-up reruns after every reload, and `/healthz` exposes a `warm` readiness flag. Benchmarks should start only after warm-up.
- Hot-path relay and datagram loops must reuse buffers from bounded, lock-free pools (`ByteBufferPool`) instead of allocating per request/datagram, to cap allocation churn at 100k-1M concurrent sockets while keeping resident pool memory bounded and leak-free (surplus buffers are freed on return). Pair high socket counts with `fs.nr_open`/`fs.file-max` sysctl ceilings and a high systemd `LimitNOFILE`.
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
- **Release tags require a matching changelog section and strict Linux evidence on the same commit.** Before pushing `vX.Y.Z`, ensure `CHANGELOG.md` contains `## vX.Y.Z`, `Cargo.toml` `version` matches, and `performance-evidence/vX.Y.Z.json` passes `go run scripts/verify-production-evidence.go --manifest performance-evidence/vX.Y.Z.json --tag vX.Y.Z --commit <tag-commit>`. The default CI is packaging-only; `release.yml` validates the changelog and evidence manifest during publish.

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

Default GitHub Actions CI is pure packaging: six release bundles are built and uploaded. GitHub Actions must not contain, schedule, or manually dispatch tests, smoke benchmarks, or performance benchmarks. For scenario-surface Docker validation, run `scripts/verify-docker-scenarios.sh` on Linux/macOS or `scripts/verify-docker-scenarios.ps1` on Windows; it checks `examples/all-scenarios.example.yaml`, static Range downloads, service discovery config, capability output, and nginx-parity declarations in an Ubuntu 24 container. For local Ubuntu/x86 performance validation, run `scripts/benchmark-ubuntu24-amd64-docker.sh`; on an arm64 Docker host it requires Zig + cargo-zigbuild + the Rust x86_64-unknown-linux-gnu target, cross-builds at native host speed, and executes the resulting ELF in Ubuntu 24 amd64 before measurement; it hard-checks Ubuntu 24.04 x86_64, records native versus emulated amd64, and drives gateway/backend/client containers on disjoint cpusets so faster closed-loop traffic cannot steal CPU from the gateway under test. Each wave uses one client container with 11 independent benchmark processes; each scale keeps one backend and both gateways warm while pausing the inactive gateway, cutting container startup out of the 2-second samples. The strict 1x/2x/4x mixed and equal-offered-load matrices include static-small, static-large, CDN hot-update static, HTTPS static, HTTP reverse proxy, generic SSE, WebSocket long connections, game long-connection TCP, TCP stream, UDP stream, and transparent QCP; every row requires zero errors and strict superiority. The one-minute default uses a 2-second window and one sample per gateway/phase at every scale. Equal-load defaults to 25% of the slower gateway saturation rate, leaving latency headroom while all 11 isolated generators run; delayed client timer slots are caught up under bounded per-worker concurrency instead of being silently dropped. Saturation clients retain the full client cpuset; fixed-rate equal-load uses one Tokio I/O worker per small-message client and two for static-large, so 11 generators do not each multiply by the full cpuset; both candidates must still complete at least 98% of target. nginx remains mainline 1.31.2 built with `-O3 -fno-plt`, the same gateway cpuset, and equivalent protocol configuration. The older shared-role diagnostic is `scripts/benchmark-all-scenarios.sh` with `MIXED_MATRIX=1`; its default fair release floor remains `CRITICAL_RATIO=0.97` for WebSocket, game TCP, generic TCP, and nginx-comparable UDP plus `AGGREGATE_RATIO=0.97`. New API provider routes and KCP special encapsulation are excluded from the default head-to-head matrix; QCP evidence is transparent UDP edge forwarding and must not claim frame termination. Single-scenario runs are diagnostic only and must not justify release success by cherry-picking. v1.3.5 UDP-only Docker validation for `udp-stream` passed at `4.045x` (`127742.75` vs `31577.33` ops/s, 0 errors).
Benchmark helpers in that flow are Go-based (`scripts/benchmark-helper.go`); do not swap in Python fixtures or parsers for official runs.

## Hot Reload Boundaries

**Hot-reloadable (fingerprinted):**

- Merged configuration values except listener identity
- The main `proxysss.yaml` file
- Main script (`script.args`)
- Auto-loaded plugin scripts (`plugins.auto_load_dir`)
- `services.reverse_proxy.routes`, `services.service_discovery`, `static_sites`, `webdav`, FTP upstream

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
- 任何涉及脚本 API、内建文档、nginx 对照配置、caddy 对照配置、模板、错误页、运维入口的变更，都必须同步更新 `AGENTS.md`、内建 `docs.html`、`docs/index.html`、`docs/configuration.html`、`docs/architecture.html`、`docs/ts-how-to-use.html`、`docs/nginx-to-proxysss.html`、`docs/caddy-to-proxysss.html`、`docs/CONFIGURATION.md`、`ts-how-to-use.md`、`nginx-to-proxysss.md`、`caddy-to-proxysss.md`、`proxysss-script.d.ts`、README/模板/测试中的对应内容。
- For hot-path code, measure or reason about throughput, allocation pressure, backpressure, and lock contention before adding abstractions.
- Every performance optimization must ship with benchmark evidence for the target path and a mixed-load regression check over adjacent gateway paths. If the change speeds up one path but degrades another without explicit approval, keep iterating instead of landing the regression.
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
