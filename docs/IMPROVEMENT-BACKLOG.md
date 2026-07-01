# proxysss improvement backlog

proxysss already covers the core nginx-replacement surface. The next improvements should focus on proving stability under realistic production pressure, reducing operator uncertainty, and keeping hot-path latency predictable.

## Stability and reliability

- Add chaos fixtures for upstream resets, slow response bodies, broken DNS, TLS renewal failure, UDP packet loss, log backpressure, and reload races.
- Add long-running soak tests for HTTP, WebSocket, TCP, UDP, KCP-style UDP, QCP UDP, MQTT, FTP, WebDAV, and generic streaming routes.
- Add service-manager examples for systemd watchdog, Windows service recovery, restart backoff, file descriptor limits, and graceful drain before deploy.
- Add HA deployment patterns for active-active edge nodes, shared certificate storage, config distribution, and health-check based failover.

## Performance

- Keep the performance benchmark gate focused on nginx-comparable generic traffic: TLS, HTTP/2, HTTP/3, reverse proxy JSON, generic SSE streaming, WebSocket, TCP, UDP, cache, and compression. KCP/QCP stay in protocol capability and soak validation, not the current nginx head-to-head performance matrix.
- Keep a Linux benchmark profile with kernel settings documented: socket backlog, `somaxconn`, `ulimit -n`, ephemeral port range, TCP reuse, UDP receive/send buffers, and QUIC UDP buffers.
- Add allocation and lock-contention profiles for hot paths: route matching, rate-limit zones, cache lookup, access logs, and upstream health state.
- Add per-feature overhead reports so operators know what cache, compression, script hooks, debug logging, and admin metrics cost.

## Protocol verification

- Keep small upstream simulators for MQTT, CoAP-style UDP, FTP active/passive modes, WebDAV clients, gRPC streaming, and SSE token streaming.
- Add migration fixtures that replay common nginx configs into proxysss YAML and verify route behavior, status codes, headers, and connection lifecycle.
- Expand HTTP/3 validation with browser and curl probes, QUIC connection migration checks, and UDP 443 firewall diagnostics.
- Add FTP client matrix coverage for passive, active, TLS passthrough, command policy, transfer policy, per-user policy, timeout, and rate cases.

## Security

- Add reusable hardening profiles for public internet edge, private cluster edge, AI API edge, game edge, and IoT edge.
- Add scripted checks for secret redaction, admin write-op disablement, token rotation, dynamic blacklist behavior, and SSRF guard coverage.
- Add examples for zero-trust admin exposure through `admin.https`, mTLS fronting, and private network-only management.
- Add default alert recommendations for DDoS bans, auth failures, upstream quarantine, ACME errors, watchdog restarts, and config reload failures.

## Observability and operations

- Ship Grafana dashboard JSON and Prometheus alert rules for core gateway SLOs.
- Add `proxysss doctor` to inspect ports, certificates, YAML, filesystem permissions, DNS, kernel limits, and reachable upstreams.
- Add `proxysss config diff-plan` for before/after reload and restart impact.
- Add a production runbook with rollback, drain, certificate renewal, admin token rotation, and emergency blacklist flows.

## Plugin and scripting safety

- Document script hook cost budgets and recommended hook placement.
- Add examples for plugin timeout, memory budgeting, structured errors, and safe upstream selection.
- Keep business logic optional: ordinary gateway behavior should remain native YAML and Rust hot-path code.
