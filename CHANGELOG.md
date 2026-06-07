# Changelog

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
