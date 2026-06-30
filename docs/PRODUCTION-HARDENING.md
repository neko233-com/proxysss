# proxysss production hardening

This checklist is for high-importance gateway deployments where correctness, latency, and recovery matter more than convenience.

## Release gate

Run these before promoting a build:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release
```

Performance promotion is Linux-only because proxysss production deployments are Linux gateways. Windows/macOS benchmark scripts are development diagnostics and must not be used as nginx-replacement release evidence.

Run the production nginx comparison matrix after Linux host tuning:

```bash
proxysss tune linux --apply
scripts/benchmark-all-scenarios.sh
```

That default gate is mixed-load (`MIXED_MATRIX=1`, `FAST_GATE=0`, `CRITICAL_RATIO=0.97`, `AGGREGATE_RATIO=0.97`). nginx runs the full concurrent wave first, then proxysss runs the same full concurrent wave. The wave enables the realistic nginx-comparable multi-service gateway shape at once: static small files, static large files, CDN hot-update static files, HTTPS static files, HTTP reverse proxying, New API/SSE streaming, WebSocket long connections, game-style long TCP connections, TCP stream proxying, and UDP stream proxying. KCP-style UDP and QCP UDP stay supported as independent proxysss listener modes, but they are not default nginx head-to-head scenarios because nginx has no native KCP/QCP semantics. Concurrency is derived from detected CPU cores and weighted by traffic shape; operators should not hand-tune worker counts or per-scenario defaults for release evidence. Official benchmark fixtures and parsers in this flow are Go-based via `scripts/benchmark-helper.go`; do not substitute Python protocol mocks or Python gate helpers in official runs.

The default critical gate is game/realtime oriented but fairness-adjusted: aggregate mixed load plus WebSocket long connections, game TCP, generic TCP, and nginx-comparable UDP default to a `0.97` proxysss/nginx ratio floor, not a blind `>1.0`. A 1-3% gap is acceptable because proxysss includes built-in gateway behavior that nginx commonly needs extra modules, directives, or policy paths to approximate; all functional checks and benchmark error budgets still must pass. Static-small, CDN hot-update, reverse proxy, and New API/SSE are part of the same wave and must stay above the soft floor (`MIN_RATIO=0.50`) with low errors. HTTPS small static and static-large remain in the same concurrent wave and report, but are diagnostic by default because TLS/static and large-file sendfile tuning can trade off against small-file/long-connection feedback. For a TLS/static or bulk-transfer release, add `https-static-small` or `static-large` to `CRITICAL_SCENARIOS` and set `runtime.performance.traffic_profile` to the profile under test. For strict head-to-head speed experiments, explicitly set `CRITICAL_RATIO=1.0` or higher and `AGGREGATE_RATIO=1.0` or higher.

Performance optimization is only acceptable when it is side-effect free at the gateway level. A local improvement that slows down, destabilizes, or increases allocation pressure on sibling paths like reverse proxy, SSE, WebSocket, TCP, UDP, or static delivery is a failed optimization unless the tradeoff is explicitly approved and documented. Treat mixed-load regression checks as release criteria, not as optional follow-up.

Use `QUICK=1` only when you need a shorter mixed smoke gate:

```bash
QUICK=1 scripts/benchmark-all-scenarios.sh
```

Single-scenario benchmark runs are root-cause diagnostics only. Do not promote a performance build because one isolated module beats nginx while the combined CDN/static/reverse-proxy/SSE/WebSocket/TCP/UDP gateway load does not. Results are written to `.benchmark/runs/all-scenarios/results.json`, `summary.md`, and `summary.html`; the default gate fails on unexpected errors, any default critical stream scenario below `CRITICAL_RATIO=0.97`, non-diagnostic scenarios below `MIN_RATIO`, or aggregate mixed-load ratio below `AGGREGATE_RATIO=0.97`.

## Protocol smoke baselines

Use the built-in benchmark client against staging listeners:

```powershell
.\target\release\proxysss.exe bench http --url http://127.0.0.1:8080/healthz --concurrency 512 --duration-secs 60
.\target\release\proxysss.exe bench tcp --addr 127.0.0.1:7000 --connections 1024 --duration-secs 60 --payload-bytes 1024
.\target\release\proxysss.exe bench udp --addr 127.0.0.1:7001 --connections 4096 --duration-secs 60 --payload-bytes 512
```

For AI gateways, include a long-lived SSE endpoint in staging and verify that token streams arrive incrementally rather than after buffering. For MQTT/IoT gateways, include MQTT TCP `1883`, MQTT TLS passthrough `8883`, MQTT over WebSocket, and CoAP/UDP staging checks against your real broker/device-service behavior.

## Production config baseline

```yaml
load_balance:
  algorithm: rendezvous
  retries:
    enabled: true
    max_retries: 2
  passive_health:
    enabled: true
    fail_threshold: 3
    quarantine_secs: 15
  active_health:
    enabled: true
    http_enabled: true
    tcp_enabled: true
    udp_enabled: false
    interval_secs: 10
    timeout_ms: 2000
    path: /healthz
    expected_statuses: [200, 204]
    failure_threshold: 2
    success_threshold: 2
    jitter_percent: 20
    udp_payload: proxysss-health
    udp_expect_response: true

runtime:
  performance:
    enabled: true
    profile: edge
    traffic_profile: small
    adaptive_system: true
    socket_extreme: true
    log_on_start: true
  watchdog:
    enabled: true
    restart_critical_tasks: true
    restart_backoff_secs: 2
    heartbeat_interval_secs: 30
  maintenance_state:
    enabled: true
    path: ./runtime/maintenance-state.json

logging:
  level: info
  access_log: true
  access_sample_rate: 1.0
  slow_request_ms: 1000
  access_log_path: logs/access.log
  error_log_path: logs/error.log
```

Enable `udp_enabled` only when the UDP, KCP, or QCP backend has a known probe response. If a service can accept a probe but not respond, set `udp_expect_response: false` and monitor passive failures plus real traffic metrics.

## Linux host tuning

Use the built-in assistant first:

```bash
proxysss tune tcp
proxysss tune linux
```

Normal gateway startup never writes host sysctl files. Use `proxysss tune linux --apply` only during a controlled maintenance window. The apply path is guarded by default: it does not mutate sshd, firewall, routes, or `rp_filter`; it filters unsupported sysctl keys, skips unavailable congestion controls, writes a backup of the previous proxysss sysctl profile, and restores that profile if `sysctl --system` fails. Reserve `--unsafe-apply` for disposable lab hosts.

For large TCP, UDP, KCP, and QCP fleets, verify these outside proxysss as part of host provisioning:

- file descriptor limit and service `LimitNOFILE`
- `net.core.somaxconn`, `net.ipv4.tcp_max_syn_backlog`
- UDP receive/send buffers
- ephemeral port range for high outbound fanout
- conntrack limits when firewall/NAT is in the path
- NIC queue, RSS, and CPU pinning on very high packet rates
- MQTT broker connection/session limits and WebSocket idle timeout alignment

For nginx-parity reverse proxy benchmarks or routes that do not need forwarding metadata, set `forward_headers: false` on `services.reverse_proxy.routes`, `services.domain_routes`, or `services.ai_proxy.routes`. The default remains `true` so upstream applications still receive `X-Forwarded-*` and `Forwarded` unless operators opt out.

## HA pattern

proxysss is a single-node gateway binary. Production high availability should run at least two nodes behind one of:

- cloud load balancer with health checks
- Keepalived/VRRP floating IP
- DNS failover with short TTL for lower-criticality domains

Keep runtime config in one YAML per node, use the admin API only with token auth and loopback/private access, and replicate releases through your normal deployment system rather than hand-editing live nodes.

## Operational watch points

Alert on:

- `proxysss_critical_task_failures_total` increasing
- `proxysss_watchdog_heartbeat_total` missing increments
- upstream active health changing to failed
- nonzero benchmark or synthetic-check errors
- process RSS growth under steady traffic
- `proxysss_blocked_requests_total` or `proxysss_ddos_bans_total` spikes
- TCP active sessions approaching host file descriptor limits

Treat config changes to listener binds, TLS mode, log sinks, and FTP bind as restart changes; route, script, plugin, and most service policy changes remain hot-reloadable.
