# AGENTS

## 工作入口与文档分层

proxysss 是可扩展的通用网关；核心协议和普通转发保持原生 Rust，业务逻辑通过可选脚本/插件扩展。开始修改前阅读 [当前架构](docs/ARCHITECTURE.md)；维护说明文档遵循 [docs/AGENTS.md](docs/AGENTS.md)。

| 工作 | 首先阅读 |
| --- | --- |
| HTTP、CDN、下载 | [架构请求路径](docs/ARCHITECTURE.md)、[CDN 下载](docs/CDN-ORIGIN.md) |
| TCP/UDP/WebSocket 与 Linux 调度 | [架构](docs/ARCHITECTURE.md)、[性能决策](docs/decisions/linux-performance.md) |
| 配置、重载、脚本 | [配置](docs/CONFIGURATION.md)、[脚本](ts-how-to-use.md)、`proxysss-script.d.ts` |
| 验证、打包、发布 | [验证流程](docs/VALIDATION.md)、[发布证据](performance-evidence/README.md)、[gh-cli](skills/gh-cli/SKILL.md) |

根文件放常驻规则；架构页描述当前实现；主题页维护详细语义；决策页保存取舍。代码与相关文档同批更新，不能在现状说明中堆叠失效参数或以历史实验作为新提交证据。该组织参考 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)，不引入其运行时或插件框架。

## Product Direction

proxysss exists to **fully replace nginx** as a same-level general-purpose gateway. It is **not** a business gateway. Business-aware behavior belongs in optional scripts/plugins, similar to using Lua modules with nginx.

Keep these invariants aligned across code, docs, examples, tests, and generated config:

- Default public HTTP port is `80` (nginx parity).
- Default public `/` route is a minimal `Welcome to proxysss` page with only GitHub and GitHub Docs links.
- Default admin console/API port is `7777` on loopback with `enable_write_ops=false` and `expose_config=false` until explicitly enabled for automation.
- Default HTTPS/HTTP2/HTTP3 port is `443`.
- Default FTP control port is `21` when `services.ftp.enabled=true`.
- Configuration must be more human-friendly than nginx while still covering nginx-level gateway duties.
- Runtime configuration should live in a single YAML file by default, normally `proxysss.yaml`; custom locations are selected with `-config`, `--config`, or `-c`.
- Admin API automation may update `services.domain_routes`, `services.reverse_proxy.routes`, `tcp.listeners`, `udp.listeners`, `tcp.stream_routes`, `http.tls.*`, `services.filecloud`, and dynamic blacklist entries over token-authenticated HTTP; loopback admin bootstraps TLS/ACME, then `admin.https` exposes the same `/v1/*` surface on the main TLS listener.
- Mobile and desktop monitoring must use the separate `admin.monitor` HTTPS-only surface with its own password and `monitor:read` session; it may expose only health, summary, stats, and SSL status, never admin bearer tokens or write APIs.
- Local token inspection and rotation should go through `proxysss token show` / `proxysss token set`; general config display paths should redact secrets instead of exposing them.
- Domain-only automatic WSS uses free built-in managed ACME with `http.tls.auto_https.domains`; its production default is zero-external-tool TLS-ALPN-01 (DNS plus public 443) and ECDSA P-256 certificate keys. `http.tls.acme.key_algorithm: rsa2048` is the explicit legacy-client compatibility option, while explicit `challenge: http01` remains supported for existing port-80 deployments.
- Wildcard ACME certificates use built-in managed DNS-01: `http.tls.mode: acme_managed`, `http.tls.acme.challenge: dns01`, `http.tls.acme.dns.provider`, and redacted `http.tls.acme.dns.credentials`. One cloud vendor = one provider strategy (`aliyun_cn` vs `aliyun_intl` are separate). Legacy `acme_dns_external` + `acme.sh` remains for non-built-in providers only.
- CLI output must stay easy for agents to inspect quickly through commands such as `proxysss config explain`, `proxysss config capabilities`, `proxysss config routes`, `proxysss config reload-plan`, and `proxysss config nginx-parity`.
- FTP, WebDAV, HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, static/reverse-proxy style behavior, logging, reload, and service operation are nginx-parity requirements, not optional marketing text.
- Static asset serving must cover HTML, CSS, JS, images, fonts, audio/video, CDN-origin assets, large-file streaming, and byte `Range` resumable downloads.
- 可变静态源文件不得作为跨请求的裸 mmap 正文交给 TLS/压缩；使用有界不可变快照。文件截断、删除、局部下载及 H2 连接缓存失效必须有回归覆盖，具体规则见 [CDN 下载](docs/CDN-ORIGIN.md)。
- API gateway surfaces must cover HTTP/1.1, HTTP/2, gRPC-style requests, WebSocket reverse proxying, edge rate limiting, active/passive health, retry, weighted/canary upstreams, and circuit-breaker-style passive quarantine without making proxysss a business gateway.
- SSE 快路径必须保持 HTTP 消息边界与正文编码：转换到 TLS/HTTP2 响应时解码上游 chunked，按 Content-Length/终止块结束，截断报错；不能把 chunk 标记/trailer 当作事件数据，不能移除未解压正文的 Content-Encoding。
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
- Linux 调度、sendfile、H2 与实时 relay 的当前职责见 [架构](docs/ARCHITECTURE.md)；修改前阅读 [性能决策记录](docs/decisions/linux-performance.md)。历史已拒绝实验不得恢复为默认值；任何替代方案须重新证明完整混合负载无回归。
- Hot-path proxy data structures must avoid global locks. Use per-connection state, per-listener/per-worker sharding, atomics, DashMap sharding, or lock-free queues for data-plane pooling. Mutex/RwLock synchronization is acceptable for control plane, configuration reload, certificate management, and one-time static-cache fill coordination, but not as a shared bottleneck in ordinary HTTP/TCP/UDP forwarding.
- Linux performance validation must prioritize mixed multi-proxy load over isolated single-scenario wins: CDN/static delivery, HTTP reverse proxy, generic SSE/streaming HTTP, game/TCP streams, generic TCP streams, and nginx-comparable UDP streams must run together per gateway. New API provider routes and KCP/QCP special UDP encapsulations are not part of the default nginx head-to-head benchmark matrix; `EXTENDED_REALTIME=1` may add transparent QCP forwarding to the same mixed wave, but must not claim QCP frame termination. Single-scenario runs are diagnostic only and must not be used to justify release success by cherry-picking.
- The local/native Ubuntu 24 x86_64 Docker strict matrix starts static-large at two equal flows per detected container CPU for both gateways, then scales every workload together at 1x/2x/4x; one bulk flow per `SO_REUSEPORT` shard is a sparse hash-placement diagnostic, not the saturated release gate. An arm64 local Docker daemon may use `linux/amd64` emulation, but artifacts must record `execution_mode=emulated-amd64` and must not be described as native physical-x86 evidence. GitHub Actions must never run performance benchmarks; it is packaging-only.
- One-second equal-offered-load plans must remain mathematically executable: HTTP/SSE workers use centered uniform phase offsets and a rounded aggregate schedule; synchronized UDP/QCP workers count only whole per-connection ticks that leave a 1 ms completion guard, TCP leaves 2 ms for both relay legs, and WebSocket leaves 5 ms for framing and both user-space protocol transitions. The generators may still complete a later fast tick, but the mandatory 98% target cannot assume an unrealistically short final round trip. A worker waiting beyond the measurement deadline exits immediately instead of extending process grace.
- Persistent Docker matrices pause the inactive gateway. After each unpause, both nginx and proxysss must receive the same default 1000 ms settle window outside active measurement before the future start time is scheduled; otherwise cold CPU caches and overdue timer/I/O bookkeeping can create candidate-wide false p99 spikes.
- The default 20-second feedback matrix spends 6 seconds on one full saturation pass and 12 seconds on two equal-load repetitions with reversed candidate order, for 18 seconds active measurement. Latency reports use the two-sample median; do not revert to a single p99 sample when the budget can cover balanced repetitions.
- The strict Docker matrix measures steady gateway data-path latency: before the shared absolute start, both candidates preconnect plain HTTP/1 pools at the declared scenario concurrency and HTTPS/H2 preconnects its multiplexed session, execute and drain two real GET/SSE resource requests, and complete one WebSocket/TCP/UDP/QCP echo on every declared connection. This finishes static/reverse cache work, stale-while-revalidate, stream relay activation, and UDP association creation before measurement. Client connection/resource/session warm-up and the default 2000 ms future-start lead are excluded from active measurement; cold-connect, cold-cache, and cold-association capacity are separate diagnostics and must not contaminate one-second steady p99.
- WSS 容量证据必须在完整 idle hold 后逐连接 echo 验证 retained=requested、hold failures=0，并在连接仍存活时采 cgroup current/peak；拒绝仅 opened 或退出后内存样本。活跃 WSS warm-up 完成后才发布共同开始时间，客户端/后端显式设置与 CPU 配额匹配的 TOKIO_WORKER_THREADS。
- Official benchmark fixtures, protocol mocks, result parsers, and gate/report helpers must use Go or native compiled tooling. Do not depend on Python for ad hoc SSE fixtures or official benchmark/gate paths.
- The default Linux release gate prioritizes game/realtime long-connection production traffic and uses a fair ratio floor instead of blindly requiring `>1.0`: WebSocket long connections, game-style TCP, generic TCP, and nginx-comparable UDP default to `CRITICAL_RATIO=0.97`, allowing a 1-3% gap because proxysss carries many gateway features that nginx commonly needs extra modules or policy config to match. Static-small, CDN hot-update, HTTP reverse proxy, and generic SSE must still run concurrently and stay above the default soft floor (`MIN_RATIO=0.50`) with low errors; HTTPS small static and static-large are diagnostic by default unless explicitly promoted for a TLS/static or bulk-transfer release. Use explicit `CRITICAL_RATIO=1.0` or higher only for strict head-to-head performance experiments. A strict `>1.0` claim needs the CPU-role preflight, balanced repetitions, raw artifacts, and an independent gateway/backend/client-host replay that verifies distinct machine IDs and enforces the declared CPU/nofile cgroup for both candidates. Memory must report cgroup current/peak and per-connection cost, with optional `MemoryMax` for a stated envelope; it must not be rejected merely for missing an arbitrary fixed RAM threshold. Docker-only data must never be presented as cross-host evidence.
- Performance optimization must be side-effect free by default: do not accept a local win that makes sibling scenarios slower, less stable, or more memory-hungry without explicit user approval. Treat cross-scenario regressions in static, reverse-proxy, generic SSE, WebSocket, TCP, and nginx-comparable UDP mixed validation as a failed optimization, not an acceptable trade. KCP/QCP remain capability surfaces; their transparent forwarding may be opt-in extended evidence, but is not part of the default performance benchmark matrix.
- A bounded, evidence-backed memory-for-performance trade is explicitly allowed: proxysss may use reasonably more resident memory when it materially improves throughput or p50/p95/p99 and remains safe in the declared production envelope, but its RSS/cgroup current, peak, and per-connection cost must each stay at or below **2x nginx** in the same fair run. Every such change must report both gateways' current and peak (including per-connection cost), preserve bounded pools/backpressure, and pass the full mixed-scenario regression matrix without OOM or runaway growth. The 4-core/20k-idle-WSS and 4096-active-WSS case is a reproducible reference workload, not an arbitrary fixed RAM admission cap; set `MemoryMax`/Docker memory only when an operator declares a real production budget. Higher 100k-class capacity remains an opt-in diagnostic, not the default production requirement. Saving memory is not a win when it measurably reduces gateway performance.
- Linux 静态 sendfile 不做逐响应 SO_SNDBUF 调优：设置后即使恢复数字，发送缓冲自动调优的用户锁仍保留，wmem_max 还可能让请求增大变成缩小。保留 EAGAIN 背压、内核自动调优和有界文件/正文池。
- Static delivery must preserve small-file benchmark efficiency while streaming large files to avoid per-request memory spikes and preserve backpressure on Linux. `runtime.performance.traffic_profile` is the native knob for this tradeoff: default `small` favors cached small-file/HTTP2/SSE/TCP/UDP feedback, `bulk` favors sendfile/zero-copy large transfers, and `balanced` enables both preload styles.
- Static sites should participate in config-load optimization: after loading config, eligible hot index/top-level static files are preloaded into the bounded cache or have sendfile descriptors prepared according to `runtime.performance.traffic_profile`.
- Config load and hot reload must trigger a fast self-optimization warm-up before listeners serve traffic: preload hot static files into the bounded fast-lane cache, prebuild reusable HTTP/1 keep-alive bytes for eligible small objects, and pre-dial a small bounded pair of reverse-proxy/AI-proxy upstream keepalive sockets per deduplicated pool. Do not bulk pre-dial sockets on the startup runtime and then use them as long-lived data-shard lanes; Tokio socket registration remains tied to the creating reactor and cross-runtime ownership regresses latency. Numeric upstream IPs must be parsed once into `SocketAddr` instead of repeating string address resolution on every cold connect. Warm-up reruns after every reload, and `/healthz` exposes a `warm` readiness flag. Benchmarks should start only after warm-up.
- Hot-path relay and datagram loops must reuse buffers from bounded, lock-free pools (`ByteBufferPool`) instead of allocating per request/datagram, to cap allocation churn at 100k-1M concurrent sockets while keeping resident pool memory bounded and leak-free (surplus buffers are freed on return). Pair high socket counts with `fs.nr_open`/`fs.file-max` sysctl ceilings and a high systemd `LimitNOFILE`.
- Architecture should favor extensibility without putting hot-path traffic behind unnecessary dynamic dispatch, allocation, serialization, or script calls.
- TypeScript scripting is a required product surface, and it is implemented as a single `proxysss` binary with an embedded TypeScript-capable engine (QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip`) executing inside the process with hot reload. There is no external `deno`/`node`/`tsc` dependency, no bundled `deno.exe`, and no sidecar runtime directory — those are removed legacy states. See `specs/embedded-ts-runtime.md`.

## What proxysss Is

| Layer | Responsibility |
| --- | --- |
| Core gateway (Rust) | nginx-equivalent protocol termination, routing, static files, WebDAV, stream proxy, TLS, rate limits, logging, reload |
| Extension scripts (TS/JS via embedded proxysss runtime target) | Optional business routing, plugins, affinity, custom upstream selection — like nginx + Lua |
| Admin API (`127.0.0.1:7777`) | Health, stats, config inspect, plugin load/unload, manual reload, token-authenticated route/listener upsert; separate `admin.monitor` exposes password-authenticated read-only mobile data |

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
- Webpage visual/interaction validation must use the installed in-app Browser plugin only. Do not use Computer Use, Chrome control, standalone Playwright, or screenshot substitutes unless the user explicitly changes this rule.

### Mandatory agent rules

- **GitHub Actions / release work must use `skills/gh-cli/SKILL.md`.** Do not guess workflow status from memory or stale logs. Always confirm with `gh run list`, `gh run view`, `gh run watch`, and `gh release view` before reporting success or failure.
- **Workflow JavaScript actions must target Node.js 24 LTS or newer.** Use `actions/upload-artifact@v6` and `actions/download-artifact@v6` (or later). Do not add `actions/*-artifact@v4` or other Node 20 actions without upgrading.
- **Release tags require a matching changelog section and strict Linux evidence on the same commit.** Before pushing `vX.Y.Z`, ensure `CHANGELOG.md` contains `## vX.Y.Z`, `Cargo.toml` `version` matches, and `performance-evidence/vX.Y.Z.json` passes `go run scripts/verify-production-evidence.go --manifest performance-evidence/vX.Y.Z.json --tag vX.Y.Z --commit <tag-commit>`. Functional tests and benchmarks run locally; GitHub Actions only validates release metadata/evidence and packages six platforms. All release bundles include the binary, changelog, Chinese HTML documentation, configuration examples/templates, script declarations, and install scripts; `scripts/package-release.ps1` cleans fixed project-local staging and atomically replaces each archive.
- **One-time operator exception, 2026-09-08:** the user explicitly authorized publishing `v1.3.8` as a functional release after local functional checks and six-platform packaging passed, waiving strict Linux performance evidence for this tag only. Do not describe this release as a strict nginx performance win. Other tags retain the evidence gate.

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

验证顺序：Ubuntu 24 LTS Docker 功能 → 角色隔离的全场景 Linux 混合性能 → Windows 功能 → push/六平台打包/发布。具体命令、CPU 资源、预热/测量时间、等负载算法、内存和三机证据要求集中维护在 [验证指南](docs/VALIDATION.md)。单场景、Windows 或 Docker-only 结果不能冒充严格跨主机发布证据；保存原始工件时显式设置 `KEEP_BENCH_ARTIFACTS=1`，禁止清理整个共享 benchmark/cache。

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

## CDN 源站与本地产物约束

- 静态站点 `security.origin_token`、`allowed_peers`、`signed_url` 必须在缓存、HEAD、Range 和条件响应之前校验；源站只信任 TCP 对端，不信任访客提交的转发 IP。配置展示必须脱敏 origin_token 和 signed_url.secret。
- `autoindex` 默认 false，`hide_dotfiles` 默认 true；预加载、index 和目录浏览都必须遵守真实路径边界，索引扫描有界，URL 编码和 HTML 转义分开处理。签名站点不能开启目录索引。
- 签名响应与静态错误响应使用 no-store；不要声称源站能约束已命中的 CDN 缓存。Referer 与客户端防逆向属于业务/边缘层。
- 热重载/管理更新部署禁用连接免策略静态快路径，配置修改后重新预热；不可变无策略 H2 保留连接 OnceLock 与 ArcSwap payload。
- 使用 `scripts/verify-local.ps1` 连续两轮幂等测试，`.tmp/tests` 完成或失败后清空；`.tmp/verification/latest` 覆盖报告，`.cache`/`target` 复用依赖/编译缓存，`dist/proxysss-local.zip` 为固定打包结果。清理必须验证项目绝对路径与链接边界，不得删除整棵共享历史 benchmark/cache。
- 同步维护 `docs/CDN-ORIGIN.md` 与 `docs/cdn-origin.html`。


## 安全开关与跨系统性能适配

`proxysss config security` 输出安全开关、默认值和建议；CDN 的 enabled/origin_token_enabled/allowed_peers_enabled 与 FTP 各类策略支持独立停用并保留参数。`config performance` 探测 Windows IOCP、macOS kqueue、Linux epoll 与实际 socket 能力。Windows/macOS 保持现有调度并按系统适配 socket，Linux 保留独立数据运行时并继续发行版/CPU 自适应。runtime.performance 只在启动时应用，变更需重启。Docker 验证固定命名 `proxysss-verify`，前后清理同名项目容器，覆盖项目内报告。完整说明见 [安全与性能指南](docs/SECURITY-PERFORMANCE.md)。


## 内置界面维护

- 管理后台模板为 `templates/admin.html` 与 `templates/admin.css`，内置文档模板为 `templates/docs.html`，共享公开文档资源 `docs/site.css` / `docs/site.js`；全部嵌入二进制。公开文档只链接 HTML。
- 管理监听器 `/docs` 和 `/docs.html` 在管理传输访问检查后提供静态使用文档，不暴露实际运行配置或凭据。后台中文默认，原始 JSON 诊断默认折叠，保持登录、会话与只读安全边界。
- `README.md` 使用英文，`README-CN.md` 使用中文，两者的配置示例和运维说明同步维护并随安装包分发。
- 网页视觉和交互验收仅使用已安装的应用内 Browser 插件。
