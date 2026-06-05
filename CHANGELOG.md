# Changelog

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
