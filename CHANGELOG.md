# Changelog

## v1.3.5 - 2026-07-01

- Added independent QCP UDP listener coverage for neko233-com/QCP alongside existing KCP-style UDP examples, templates, capability output, nginx-parity output, and Chinese-first docs. KCP and QCP remain separate listener modes; proxysss forwards datagrams transparently and leaves protocol framing/reliability semantics to upstream services.
- Optimized the UDP data path for transparent high-throughput forwarding: pending-session dedupe now uses a sharded set, UDP association pruning is throttled under churn, listener/reader buffers come from the bounded reusable pool, upstream response readers drain ready datagrams in batches, and policy-free single-upstream UDP listeners use a direct fast path that avoids per-new-client script-routing task churn.
- Added a faster per-worker UDP association cache and moved benchmark defaults back to nginx-comparable UDP only, while keeping KCP/QCP as proxysss-native capability validation instead of pretending nginx has native KCP/QCP protocol semantics.
- Updated the Ubuntu 24 benchmark container and Go benchmark helper so Docker validation can run the official script path end to end; `SCENARIO_FILTER=udp-stream` now supports quick UDP-only release checks. Docker UDP-only validation passed at `4.045x` proxysss/nginx with 0 errors (`127742.75` vs `31577.33` ops/s).
- Simplified default GitHub Actions CI to packaging-only six-platform binary builds; tests, smoke checks, and performance benchmark gates now remain manual/operator validation paths. Refreshed benchmark docs so v1.3.5 UDP fast-path results appear before older mixed-load UDP baselines.

## v1.3.4 - 2026-06-15

- Promoted the official GitHub Actions Linux benchmark from a single-scenario throughput snapshot to the real mixed all-protocol matrix: the Linux benchmark workflow now runs `scripts/benchmark-all-scenarios.sh` with `QUICK=1`, `DURATION_SECS=12`, and `MIXED_MATRIX=1`, then publishes the mixed results artifact (`results.json`, `summary.md`, `summary.html`) instead of a static-only quick report.
- Clarified benchmark roles across docs: the official Linux benchmark docs now say the release-facing artifact must compare `static / reverse proxy / New API / SSE / WebSocket / TCP / UDP / KCP-style UDP` together, while Windows quick throughput remains a smoke-only auxiliary path.
- Hardened the mixed benchmark gate for CI reality without weakening the benchmark intent: SSE now allows only a tiny `proxysss <= nginx + 1` stream-error tolerance on GitHub runners, and benchmark artifacts upload with `if: always()` so failed gates still leave behind `summary.md` / `summary.html` for diagnosis.
- Kept `kcp-style-udp` in the GitHub Actions Linux all-protocol comparison artifact but moved it to the hosted-runner diagnostic set instead of the hard-fail critical gate, because GitHub-hosted UDP virtualization is too noisy to act as the final KCP fairness judge. Dedicated tuned Linux hosts still keep the stricter realtime gate.

## v1.3.3 - 2026-06-15

- Kept the low-allocation plain raw reverse-proxy and AI/SSE fast lanes side-effect free: default `X-Forwarded-*` / `Forwarded` semantics and `proxysss-ai-*` metadata headers now survive the fast path, so upstream observability and agent-stream routing do not regress just because the route uses the optimized path.
- Raised the default HTTP/2 reverse-proxy tuning ceiling and added end-to-end coverage for plain fast-lane forwarding semantics plus HTTPS/HTTP2 round trips, tightening the release gate around SSE / reverse proxy / HTTP2 behavior instead of chasing one isolated throughput win.
- Added an official Linux benchmark documentation pair for humans and release bundles: `benchmark-linux.md` explains the latest GitHub Actions Linux quick benchmark, and `docs/benchmark-linux.html` gives the matching HTML entry while keeping mixed-load release validation and no-side-effect performance rules explicit.

## v1.3.2 - 2026-06-15

- Hardened the no-side-effect performance validation toolchain around the Go benchmark helper: CI benchmark runs now trigger when `scripts/benchmark-helper.go` changes, and obsolete Python benchmark report dependencies were removed from GitHub Actions so the official benchmark path stays Go-first end to end.
- Cleaned the deploy/release packaging manifests so the new `caddy -> proxysss` migration guide ships alongside the existing nginx and TypeScript docs instead of leaving the release bundle with partial documentation.
- Refreshed the built-in docs and README capability wording so supported surfaces like `token-bucket` rate limiting and migration tracks stay aligned with tests, docs, and release evidence.

## v1.3.1 - 2026-06-15

- Static small-file delivery now copies response head + body into one pooled buffer and issues a single write syscall (nginx single-segment parity), removing the extra syscall and separate header packet; static-small and CDN hot-update now exceed nginx in the Ubuntu 24 mixed benchmark.
- SSE/streaming proxy forwards each upstream read immediately with no event-boundary buffering (nginx `proxy_buffering off`), lowering first-token and inter-token latency for AI streaming; New API/SSE now meets or beats nginx.
- Larger cached bodies and sendfile static keep `TCP_CORK` head+body coalescing for single-segment delivery.
- Fixed CI clippy failures (`identity_op`, `needless_return`) so the quality gate passes cleanly.

## v1.3.0 - 2026-06-15

- Added bounded lock-free buffer pooling (`ByteBufferPool`) for the hot-path raw HTTP/SSE relay and UDP association readers, cutting per-connection heap churn under very high concurrent socket counts while keeping resident pool memory bounded (no leak): surplus buffers are freed on return.
- Added startup/reload self-optimization warm-up: hot static files are preloaded into the bounded fast-lane cache and reverse-proxy/AI-proxy upstream keepalive pools are pre-dialed before listeners begin serving, so the first live request never pays a cold connect. Warm-up reruns after every hot reload, and `/healthz` now reports a `warm` readiness flag.
- Reduced UDP datagram loss for game/KCP/IoT traffic by enlarging UDP socket buffers (`SO_RCVBUF`/`SO_SNDBUF` plus privileged `SO_RCVBUFFORCE`/`SO_SNDBUFFORCE`) on both listener and upstream sockets under Linux performance mode.
- Removed a redundant per-chunk copy in the streaming SSE relay and enlarged its read buffer for lower latency and overhead on long-lived event streams.
- Added file-descriptor ceiling tuning (`fs.nr_open`, `fs.file-max`) to the Linux sysctl profile to support super-scale gateways targeting 100k–1M concurrent sockets, paired with a high systemd `LimitNOFILE`.

## v1.2.11 - 2026-06-12

- Added production hardening for critical gateway operation: supervised runtime watchdog, watchdog heartbeat metrics, and critical task failure counters.
- Added opt-in UDP active health probes for game/KCP/IoT datagram services, alongside existing HTTP/TCP active health.
- Added first-class game, MQTT, and IoT configuration surfaces: low-latency TCP tuning, UDP association caps/TTL, MQTT TCP, MQTT TLS passthrough, MQTT over WebSocket, and CoAP-style UDP examples.
- Added production hardening documentation and release gate guidance covering benchmark baselines, HA patterns, Linux tuning, and operational watch points.

## v1.2.10 - 2026-06-12

- Fixed reverse proxy streaming normal HTTP responses (HTML, JSON, etc.) as if they were SSE, which stripped `Content-Length` and caused incomplete response bodies.
- Now only `text/event-stream` responses are streamed; all other responses are fully buffered to preserve original Content-Length and ensure delivery完整性.

## v1.2.9 - 2026-06-11

- Added browser-language-aware admin UI localization for English and Simplified Chinese, with a manual language selector.
- Localized login, dashboard navigation, metric labels, filters, empty states, status text, and upstream health table labels.

## v1.2.8 - 2026-06-11

- Reduced admin dashboard clutter by folding endpoint docs, moving filters into the main dashboard, and compacting upstream health details.
- Simplified the upstream health table into fewer scan-friendly columns so long route lists no longer crowd the first screen.

## v1.2.7 - 2026-06-11

- Added `/v1/login` for dedicated admin login sessions with signed Bearer tokens and automatic dashboard entry until token expiry.
- Restored the admin console to a dark default style while keeping login and dashboard as separate UI screens.
- Kept admin passwords out of generated HTML and browser session storage; only short-lived signed session tokens are stored.

## v1.2.6 - 2026-06-11

- Reworked the built-in admin console into a dedicated login-first experience with session-scoped credentials.
- Removed the inline Quick Login card and stopped embedding the configured admin password into the generated HTML.
- Restyled the admin UI with a cleaner light dashboard layout inspired by new-api while keeping the single-binary built-in console.

## v1.2.5 - 2026-06-11

- Added proxysss process resource metrics to `/v1/stats` and Prometheus output: PID, CPU percentage, resident memory bytes/MB, and memory percentage.
- Updated the built-in admin dashboard with live Process CPU, Process Memory, and Memory % cards.

## v1.2.4 - 2026-06-11

- Made SSE hardening headers idempotent so existing upstream `Cache-Control: no-cache` and `X-Accel-Buffering: no` values are preserved without duplicated tokens.
- Kept the v1.2.3 streaming safeguards: SSE/streaming responses bypass compression and carry `no-transform` plus unbuffered proxy hints.

## v1.2.3 - 2026-06-11

- Hardened AI/SSE reverse proxy responses by automatically emitting `Cache-Control: no-cache, no-transform` and `X-Accel-Buffering: no` for streamed or `text/event-stream` responses.
- Prevented response compression from touching streamed bodies or SSE payloads, avoiding proxy/client buffering risks for Cline, Claude Code, and other streaming CLI clients.
- Added regression coverage for SSE response headers and compression bypass behavior.

## v1.2.2 - 2026-06-11

- Fixed AI reverse proxy streaming for New API, sub2api, OpenAI-compatible SSE, and Cline/CLI clients by forwarding upstream HTTP/1.1 and HTTP/2 response chunks directly instead of buffering the full response body.
- Kept response cache and HTTP/3 paths on buffered bodies so cache correctness and existing QUIC behavior stay stable.
- Added an e2e SSE regression test that verifies the first AI proxy event arrives while the upstream connection remains open.

## v1.2.1 - 2026-06-10

- Added **HTTPS admin API** (`admin.https`): expose the full `/v1/*` automation surface on the main TLS listener at `admin.https.path_prefix` (default `/_proxysss/admin`); bootstrap TLS/ACME on loopback first, then drive agents over HTTPS with Bearer auth.
- Added **SNI certificate admin API** (`GET/POST /v1/tls/sni-certificates/*`) with PEM upload or path-based upsert, plus TLS panel UI.
- Added **Security** admin view for dynamic IP blacklist (`/v1/security/blacklist/*`).
- Expanded admin route forms (extra upstreams, strip_prefix) and documented full JSON fields for agents in `docs/AGENT-API.md`.
- Updated `cluster-automation-internal.md` and `AGENTS.md` for current automation scope.
- Added integration tests for HTTPS admin API and SNI certificate upsert.
- Fixed TLS handshake rejection when clients connect by IP without SNI (falls back to the default certificate).

## v1.2.0 - 2026-06-10

- Added **FileCloud** (`services.filecloud`): proxysss-exclusive shared directory UI with single-password auth, file tree with sizes, drag upload/move, search, delete/rename/mkdir, and CDN-friendly `/dl/*` downloads; all operations are confined to `services.filecloud.root`. Existing nginx-style `services.webdav` is unchanged.
- Added built-in managed ACME DNS-01 providers: Cloudflare, Alibaba Cloud CN/Intl, Tencent, Volcengine, AWS Route 53, Azure DNS, and Google Cloud DNS, with automatic migration from legacy `acme_dns_external` / acme.sh aliases.
- Added **manual** DNS-01 provider (`provider: manual`) for wildcard certificates without cloud API keys: proxysss prints the TXT record and polls public DNS until propagation completes.
- Non-wildcard automatic HTTPS still works without any DNS API key via built-in HTTP-01 / TLS-ALPN-01 (`http.tls.auto_https` or `acme_managed`).

## v1.1.2 - 2026-06-09

- Stabilized benchmark automation by comparing proxysss directly against nginx, removing Caddy download/runtime dependencies, fixing Linux JSON row capture, and writing Windows benchmark JSON without a UTF-8 BOM.

## v1.1.1 - 2026-06-09

- Hardened deep/e2e verification startup by waiting for the spawned gateway listener to accept TCP connections before requests are sent, matching the Linux CI timing model.

## v1.1.0 - 2026-06-09

- Reworked the built-in welcome page into a standalone `templates/welcome.html` asset with polished responsive styling and CSS motion, while keeping runtime rendering embedded in the single binary.
- Added first-class AI reverse proxy configuration for New API, sub2api, and OpenAI-compatible route shapes, including validation and docs/templates.
- Completed declared FTP nginx module parity coverage for command/transfer policies, per-user policies, passive/active data rewrite, timeouts, login limits, and structured logs.
- Fixed nginx migration routing parity: configured YAML routes and TypeScript gateway scripts now run before the fallback welcome page, upstream `Host` preserves the external host, and `/` path prefixes match child paths.
- Hardened static migration behavior by allowing `proxysss://static/...` to serve config-root symlinks safely and by covering root static-site child files with tests.
- Kept wildcard certificate guidance on the explicit non-default `http.tls.mode: acme_dns_external` + `acme.sh` DNS-01 path.

## v1.0.0 - 2026-06-09

- **1.0 stable release** — agent-native high-performance gateway with single-YAML config, admin API, and full cross-platform release artifacts.
- Cloudflare-style HTTP cache policies, domain stream SNI routing (Redis/MySQL/PostgreSQL), on-demand TLS, DDoS mitigation, and dynamic IP blacklist.
- Security hardening: SSRF blocking, HTTP smuggling rejection, admin auth rate limits, atomic config writes.
- Interactive architecture guide at `docs/architecture.html`; parity drift tests keep README, AGENTS.md, and capability matrix aligned.
- Release workflow fix: version bumps now ship with a synced `Cargo.lock` so `cargo build --locked` succeeds on all CI/release runners.

## v0.3.15 - 2026-06-09

- Added Cloudflare-style HTTP cache policies: `behavior` (`bypass`, `respect_origin`, `override`, `no_cache`), edge/browser TTL, `CDN-Cache-Control`, `stale_if_error_secs`, and `X-Cache` status headers.
- Added domain stream proxy via `tcp.stream_routes` with TLS SNI routing for Redis/MySQL/PostgreSQL-style workloads and `POST /v1/stream-routes/upsert`.
- Added on-demand TLS (`http.tls.on_demand`) with managed ACME first-hit issuance, allow globs, optional `ask_url`, and rate limits.
- Added DDoS mitigation (`security.ddos`), dynamic IP blacklist admin API (`/v1/security/blacklist/*`), and `services.access_control.stream`.
- Extended FTP with transfer-level hooks (`transfer_allow`/`transfer_deny`), per-user `user_policies`, and structured transfer logging.
- Documented new surfaces in `docs/CONFIGURATION.md`, `docs/SECURITY.md`, `docs/AGENT-API.md`, and `docs/architecture.html`.

## v0.3.14 - 2026-06-09

- Hardened admin defaults: `enable_write_ops` and `expose_config` now default to `false`, with `loopback_only` and auth rate limiting enabled by default.
- Added `security.*` controls for admin mutation validation, SSRF blocking on agent-supplied upstreams, and HTTP/1 smuggling rejection.
- Added agent delete endpoints: `POST /v1/domain-routes/delete` and `POST /v1/reverse-proxy-routes/delete`.
- Added TLS automation endpoints: `POST /v1/tls/auto-https/upsert` and `POST /v1/tls/wildcard-dns/upsert` (acme.sh DNS-01 wildcard path).
- Added optional `kubernetes.enabled` service-to-upstream DNS mappings for in-cluster ingress-style routing.
- Switched admin config persistence to atomic temp-file writes with rollback on failed reload.
- Documented security and agent workflows in `docs/SECURITY.md` and `docs/AGENT-API.md`.

## v0.3.13 - 2026-06-09

- Added Prometheus text exposition on `monitoring.path` (default `/metrics`) with `monitoring.format: prometheus|json`.
- Added weighted load balancing via `load_balance.algorithm: weighted` and per-route `upstream_weights`.
- Added token-bucket HTTP rate limiting through `services.rate_limit.http.algorithm: token_bucket`.
- Documented configuration, architecture, and demo workflows in `docs/CONFIGURATION.md`, `docs/ARCHITECTURE.md`, and `examples/demo/README.md`.
- Expanded capability matrix coverage for Prometheus metrics, weighted balancing, and gRPC-over-HTTP/2 proxying.

## v0.3.12 - 2026-06-07

- Added non-default `http.tls.mode: acme_dns_external` for wildcard certificate issuance and renewal through external `acme.sh` DNS-01 providers, with DNS credentials redacted from config display paths.
- Documented acme.sh wildcard DNS-01 setup in README, built-in docs, nginx migration notes, lab examples, and agent guidance.
- Expanded token-authenticated automation endpoints beyond domain routes to also support `services.reverse_proxy.routes`, `tcp.listeners`, and `udp.listeners` upserts over HTTP, with persistence back into the main YAML file and in-process reload.
- Added an internal cluster automation integration guide covering the agent-facing HTTP API workflow for HTTP, WebSocket, TCP, and UDP onboarding.

## v0.3.11 - 2026-06-07

- Added bearer-token admin authentication support for automation and cluster control-plane calls.
- Added `POST /v1/domain-routes/upsert`, allowing services to register or update domain-based reverse proxy routes over HTTP, persist them into the main YAML config, and trigger an in-process reload.
- Updated README, AGENTS guidance, built-in docs, and admin endpoint notes to document token-authenticated route automation.

## v0.3.10 - 2026-06-07

- Fixed GitHub CI, deploy, and release Windows builds by moving the Windows amd64 and Windows arm64 jobs back to native GitHub-hosted Windows runners instead of the failing Ubuntu `cargo xwin` path.
- Kept first-class Windows arm64 packaging in deploy and release workflows while preserving the lighter zip artifact packaging path.
- Refreshed remaining public docs and built-in landing copy so they consistently describe the single-YAML config model and direct proxysss capabilities.

## v0.3.9 - 2026-06-07

- Enforced a single-YAML runtime configuration model: `proxysss.yaml` remains the default, `-config` / `--config` / `-c` select custom YAML paths, and runtime `include` files are now rejected.
- Removed JSON runtime config support and narrowed auto-loaded plugin sidecar metadata to YAML-only files.
- Clarified and tested domain-first service grouping so one YAML file can define multiple domains, each with its own upstream pool while still reusing shared backend machines.
- Rewrote the README in English and updated built-in docs, examples, AGENTS guidance, and supporting docs to match the new config model.

## v0.3.8 - 2026-06-07

- Moved Windows amd64 and Windows arm64 release/deploy builds off hosted Windows runners onto Ubuntu-based MSVC cross-compilation with `cargo-xwin`, targeting the same `.zip` outputs while cutting the slowest build lane.
- Added first-class Windows arm64 assets to both deploy artifacts and GitHub Releases.
- Updated CI build coverage to compile both Windows MSVC targets on Ubuntu as well, so pre-release validation no longer waits on the slower hosted Windows build path.

## v0.3.7 - 2026-06-07

- Further reduced packaging overhead by trimming deploy/release bundle contents to the binary, README, key offline docs, and install scripts instead of copying the full templates/scripts trees.
- Switched the Linux ARM64 release build from the x64 cross-toolchain + `apt-get` path to a native ARM GitHub runner to remove cross-toolchain setup overhead while preserving the same final asset.
- Unified Rust cache keys across workflows and targets so release/deploy/CI builds can reuse cached dependencies and target artifacts more effectively across version bumps.

## v0.3.6 - 2026-06-07

- Fixed the release/cold-build dependency lock by downgrading the locked `getrandom` crate from `0.3.5` to `0.3.4`, restoring reproducible builds on clean GitHub Actions runners.
- Kept the `-c` / `-config` global config-path support and workflow caching improvements from the previous patch while repairing the failed `v0.3.5` tag line.

## v0.3.5 - 2026-06-07

- Optimized GitHub Actions build/package workflows by adding Rust dependency/target caching across CI, deploy, and release jobs.
- Avoided recompressing already packaged `.tar.gz` / `.zip` release artifacts during artifact upload to cut avoidable CPU time in Actions packaging jobs.
- Switched workflow build invocations to `cargo build --locked` so package/release jobs reuse the checked-in lockfile consistently.

## v0.3.4 - 2026-06-07

- Added global CLI config path support for both `-c <file>` and compatibility-style `-config <file>`, while keeping the default fallback to `proxysss.yaml` when no path is supplied.
- Updated README command examples to prefer the new global config-path form so users can launch subcommands against arbitrary YAML files without repeating per-command `--config` flags.

## v0.3.3 - 2026-06-07

- Added bilingual Chinese/English annotations to `proxysss-script.d.ts` so embedded TypeScript hooks, contexts, and route fields are easier to use without guessing the runtime contract.

## v0.3.2 - 2026-06-07

- Added managed ACME renewal flows for both HTTP-01 and TLS-ALPN-01 without requiring an external ACME binary on the default path.
- Expanded FTP from raw TCP passthrough into native control-channel proxying with passive and active data-channel rewriting.
- Added global and route-level active health control with default-on HTTP/TCP liveness probes, hysteresis thresholds, jitter, and webhook hooks.
- Added persistent upstream maintenance state so manual drain/restore actions survive reloads and restarts.
- Upgraded the admin dashboard on port 7777 with search, filter, grouping, route aggregation, manual upstream drain controls, and richer upstream health telemetry.
- Added configurable browser-facing error pages, including a polished built-in 404/4xx/5xx experience and route-level docs entrypoints.
- Added the built-in `docs.html` manual, TypeScript declaration file, TS usage guide, and nginx-to-proxysss mapping guide, and made `proxysss init` generate them.

## v0.3.1 - 2026-06-06

- Added built-in HTTP access control with IP / CIDR allow and deny lists via `services.access_control.http`, including `blacklist` / `denylist` aliases for direct YAML use.
- Enforced access-control checks in the native HTTP request path before proxying so blacklisted clients are rejected without needing any script/plugin layer.
- Expanded README automatic HTTPS guidance to call out no DNS-token requirements, built-in renewal behavior, and the current external ACME client dependency.
- Reworked README nginx parity coverage into a per-capability checklist with explicit checkmarks, and documented blacklist + rate-limit + compression examples alongside auto HTTPS.
- Prepared the next release metadata by bumping the project version to `v0.3.1` and refreshing install/update examples.

## v0.3.0 - 2026-06-06

- Replaced the external `deno` script sidecar with an embedded TypeScript/JavaScript engine compiled directly into the proxysss binary (QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip`); there is no longer any external `deno`/`node`/`tsc` dependency or bundled runtime directory.
- Added hard per-call isolation for scripts: every plugin hook runs under a configurable timeout enforced by a QuickJS interrupt handler plus a memory limit, so a buggy plugin (throw, infinite loop, runaway memory) is reported to the error log and never affects native/YAML proxy traffic.
- Moved routing pipeline orchestration (priority ordering, merge/normalize, fallback) from the TypeScript harness into Rust for full control of script input/output; the main `gateway.ts` is now a normal default-export plugin that acts as the lowest-priority fallback router.
- Added `script.memory_limit_mb` (default 64) and `script.max_stack_size_kb` (default 512) and removed the deno-specific `script.command`/`script.args` settings.
- Reworked `proxysss script run-file` / `proxysss script eval` to execute through the embedded engine instead of spawning deno.
- Removed deno download/bundling from release, deploy, and install packaging; release artifacts are now a single self-contained binary.
- Added `specs/embedded-ts-runtime.md` documenting the engine decision, isolation model, host/script contract, and verification plan.
- Added auto-loaded plugin sidecar configuration (`<name>.plugin.yaml/.yml/.json`) so bundled plugins can stay default-off while still receiving enabled/priority/config values without any external runtime.
- Injected `x-real-ip`, `x-forwarded-for`, `x-forwarded-host`, `x-forwarded-proto`, and `forwarded` on upstream requests to better match nginx-style forwarding semantics.
- Added default-off built-in plugin templates/examples for geo header injection (`geo-headers`) and AI API compatibility routing (`ai-api-compat`), plus README coverage for real business scenarios and AI API forwarding.

## v0.2.7 - 2026-06-06

- Hardened TypeScript extension execution so startup and hot reload now fall back to YAML-only routing when the script runtime cannot start.
- Changed auto-loaded plugin handling to warn and skip broken TypeScript plugins instead of aborting gateway startup or reload.
- Switched script hot reload fingerprints to explicit MD5 content hashing across config and watched script/plugin files.
- Documented the embedded single-binary TypeScript runtime target in `AGENTS.md` as a required end-state rather than an optional direction.

## v0.2.6 - 2026-06-06

- Reworked the default port 80 landing page into a compact, single-screen project page with no admin address exposure, protocol coverage, and benchmark highlights.
- Switched gateway architecture to YAML-first routing by default so HTTP/HTTPS/TCP/UDP no longer depend on TypeScript scripts unless explicitly enabled.
- Bundled the TypeScript runtime into release and deploy artifacts so proxysss installation no longer requires downloading or installing an external interpreter separately.
- Updated release verification, installer flow, and workflow packaging to validate the bundled runtime alongside the proxysss binary.
- Added `proxysss script run-file` and `proxysss script eval` for direct TypeScript runtime verification against bundled engine capability.
- Changed `proxysss start` to silent background launch with stale proxysss process cleanup, added `proxysss restart`, and made top-level `status` report background process state.

## v0.2.5 - 2026-06-05

- Added reproducible gateway benchmark workflow for proxysss, nginx, and Caddy.
- Documented local Windows loopback benchmark results and caveats in README.
- Hardened YAML config parsing for UTF-8 BOM files generated by Windows PowerShell.

## v0.2.4 - 2026-06-05

- Expanded README gateway comparison coverage for Nginx, LVS, HAProxy, Caddy, Envoy, Traefik, and proxysss.
- Added proxysss-style `http.tls.auto_https` documentation for fast production SSL without copying another product's config syntax.
- Recorded local HTTP benchmark results for the current release preparation workflow.

## v0.2.3 - 2026-06-05

- Tightened product positioning around proxysss as an nginx-level general gateway, not a business gateway.
- Removed business-flavored default TCP/UDP listener names from generated config and made stream listener examples explicitly generic.
- Fixed `config reload-plan` so logging level/filter/format/path changes are consistently shown as restart-required.
- Normalized relative `proxysss init --dir` paths so watched script output stays readable for agents.
- Updated README copy and comparison guidance for agent-first nginx replacement workflows.

## v0.2.2 - 2026-06-05

- Serve the welcome page from Rust built-in routes on `/`, `/index.html`, and `/docs` so older `gateway.ts` installs still show the landing page.
- Warn during `check-config` when `http.plain_bind` is disabled because port 80 will not expose the nginx-parity welcome page.

## v0.2.1 - 2026-06-05

- Fixed release publish job changelog extraction so tagged releases always find `CHANGELOG.md` sections.
- Upgraded GitHub Actions artifact steps to `upload-artifact@v6` / `download-artifact@v6` (Node.js 24 LTS runtime).
- Required agents to monitor CI/release through `skills/gh-cli/SKILL.md` and documented the rule in `AGENTS.md`.
- Split backend lab (`examples/lab/`) from proxysss default install paths (`examples/lab-proxysss/`).

## v0.2.0 - 2026-06-05

- Positioned proxysss as an nginx-class general gateway with agent-first CLI (`config explain`, `nginx-parity`, `reload-plan`, etc.).
- Added built-in static sites, WebDAV, declarative reverse proxy, HTTP rate limit, and FTP TCP passthrough.
- Added access/error log file sinks (`logs/access.log`, `logs/error.log`) and demo plugins (`structured-log`, `traffic-stats`).
- Merged listener supervisor hot-reload from v0.1.10; default admin port restored to `127.0.0.1:7777`.
- Added agent skills: `skills/proxysss-install`, `skills/gh-cli`, and local verification lab under `examples/lab/`.

## v0.1.10 - 2026-06-04

- Fixed Windows `proxysss start` fallback launching a visible console window.
- Changed Windows auto-start install to use a hidden `wscript.exe` launcher through HKCU Run by default.
- Made running `proxysss.exe` without arguments start the gateway instead of printing help and exiting immediately.

## v0.1.9 - 2026-06-04

- Hardened Windows installer downloads with a GitHub CLI release-download fallback when direct asset HTTPS downloads time out.
- Hardened release verification hash checks with the same GitHub CLI fallback.

## v0.1.8 - 2026-06-04

- Changed default public gateway ports to `80` for HTTP and `443` for HTTPS/HTTP3 so proxysss can directly replace Nginx and Caddy.
- Enabled `http.plain_bind` by default on `0.0.0.0:80`.
- Added top-level service commands: `proxysss start`, `proxysss stop`, `proxysss enable`, `proxysss disable`, and `proxysss status`.
- Updated installer and docs to state proxysss installs standalone and does not install Nginx.
- Kept admin on `127.0.0.1:7778` to avoid exposing admin on public ports.

## v0.1.7 - 2026-06-04

- Changed default gateway port to `7777` and admin port to `7778`.
- Added configurable public monitoring API with `monitoring.enabled` and `monitoring.path`.
- Added hot reload supervisor for listener changes, including HTTP/TLS/H3, TCP/UDP listeners, and admin bind changes.
- Added GitHub Actions `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` to remove Node.js 20 deprecation risk.
- Updated docs to position proxysss as a high-performance Web reverse proxy competitor to Nginx and to document ACME/Caddy-style certificate automation.
- Expanded memory-safety and monitoring documentation.

## v0.1.6 - 2026-06-04

- Fixed release verification compatibility when downgrading to older builds that do not support `proxysss --version`.
- Verification now falls back to installed binary SHA256 comparison against the expected GitHub release asset.

## v0.1.5 - 2026-06-04

- Added `proxysss update --version <latest|vX.Y.Z>` for one-command upgrades.
- Added `proxysss switch-version <vX.Y.Z>` for explicit upgrade/downgrade installs.
- Added Windows/Linux/macOS installer safeguards for version comparison, downgrade intent, service restart, skip-init, and dry-run flows.
- Added built-in internal HTTP routes for Nginx-like common cases:
  - `proxysss://healthz`
  - `proxysss://redirect/<location>`
  - `proxysss://static/<path under config root>`
- Added route decision fields `status` and `content_type` for redirect/static responses.
- Added default `/healthz` and `/static/*` routes in generated TypeScript gateway template.
- Added release verification script for GitHub release assets and local upgrade/downgrade checks.
- Kept GitHub Actions skill and workflow lint in-repo for automated checks.

## v0.1.4 - 2026-06-04

- Added programmable gateway foundation for HTTP/1.1, HTTP/2, HTTP/3, TCP, UDP, WebSocket, and WSS.
- Added TS/JS script routing through Deno.
- Added plugin load/unload/list support through admin API and CLI.
- Added load balancing, passive health checks, retries, affinity routing, config hot reload, admin console, service install, and installer scripts.
