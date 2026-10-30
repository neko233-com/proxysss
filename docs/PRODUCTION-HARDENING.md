# proxysss production hardening

This checklist is for high-importance gateway deployments where correctness, latency, and recovery matter more than convenience.

## 发布与验收

发布以 Ubuntu 24.04、Debian 12 的功能检查、实际目标环境验收、Windows 功能测试与六平台打包为准；不以性能对比或压测作为发布前提。具体命令与结果边界见 [验证流程](VALIDATION.md)，配置字段见 [配置指南](CONFIGURATION.md)。

性能对比仅在需要时作为可选诊断执行；历史结果不能作为当前版本的成绩或容量证明。GitHub Actions 负责打包，不能代替本地功能和生产验收。

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
