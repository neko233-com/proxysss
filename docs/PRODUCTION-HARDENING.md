# proxysss production hardening

This checklist is for high-importance gateway deployments where correctness, latency, and recovery matter more than convenience.

## Release gate

Run these before promoting a build:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release
.\scripts\benchmark-gateways.ps1 -Quick
```

For a full HTTP parity baseline against nginx:

```powershell
.\scripts\benchmark-gateways.ps1 -Concurrency 1024 -DurationSecs 60
.\scripts\benchmark-gate-check.ps1 -ResultsFile .benchmark\runs\latest\results.json
```

The gate compares proxysss against nginx on the same static payload and fails when errors are nonzero or the configured ratio drops below `scripts/benchmark-baseline.json`.

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

Enable `udp_enabled` only when the UDP/KCP backend has a known probe response. If a service can accept a probe but not respond, set `udp_expect_response: false` and monitor passive failures plus real traffic metrics.

## Linux host tuning

Use the built-in assistant first:

```bash
proxysss tune tcp
```

For large TCP/UDP/KCP fleets, verify these outside proxysss as part of host provisioning:

- file descriptor limit and service `LimitNOFILE`
- `net.core.somaxconn`, `net.ipv4.tcp_max_syn_backlog`
- UDP receive/send buffers
- ephemeral port range for high outbound fanout
- conntrack limits when firewall/NAT is in the path
- NIC queue, RSS, and CPU pinning on very high packet rates
- MQTT broker connection/session limits and WebSocket idle timeout alignment

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
