# proxysss configuration guide

proxysss uses a **single YAML file** as the runtime source of truth. The default path is `proxysss.yaml`; override it with `-config`, `--config`, or `-c`.

## File layout

```yaml
config_version: 1
http:          # listeners, TLS, error pages
admin:         # control plane on 127.0.0.1:7777
monitoring:    # /metrics exposition
load_balance:  # algorithms, retries, health checks
affinity:      # sticky sessions
services:      # HTTP routes, static, webdav, ftp, policies
tcp:           # stream listeners
udp:           # datagram listeners
script:        # embedded TypeScript entry
plugins:       # auto-loaded plugin directory
logging:       # access/error logs and levels
runtime:       # hot reload, watchdog, and maintenance state
```

## AI reverse proxy

`services.ai_proxy` is the native surface for New API, sub2api, and OpenAI-compatible upstreams. It runs before generic domain/reverse-proxy routes, supports host/path matching, path rewrite, provider headers, and header strip/set behavior.

```yaml
services:
  ai_proxy:
    enabled: true
    header_prefix: proxysss-
    routes:
      - name: new-api
        provider: new-api
        match_host: ai.example.com
        path_prefix: /v1
        upstream: http://127.0.0.1:3000
        rewrite_base_path: /v1
      - name: sub2api
        provider: sub2api
        match_host: sub2api.example.com
        path_prefix: /
        upstream: http://127.0.0.1:3001
        rewrite_base_path: /v1
        strip_headers: [x-debug-token]
```

## HTTP reverse proxy

### Domain-first routes (recommended)

```yaml
services:
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      upstreams:
        - http://127.0.0.1:8081
      upstream_weights:
        http://127.0.0.1:8080: 1
        http://127.0.0.1:8081: 3
      strip_prefix: true
      compression:
        enabled: true
      rate_limit:
        enabled: true
        algorithm: token_bucket   # fixed_window, token_bucket, or leaky_bucket
        requests: 120
        window_ms: 60000
        burst: 30
      cache:
        enabled: true
        ttl_secs: 30
        stale_while_revalidate_secs: 15
        vary_headers: [Accept-Encoding]
        key_prefix: api
      active_health:
        path: /healthz
```

### Path/host routes

```yaml
services:
  reverse_proxy:
    routes:
      - name: legacy-api
        hosts: [internal.local]
        path_prefix: /v1
        upstream: http://127.0.0.1:9000
```

## Load balancing

Set the global algorithm under `load_balance.algorithm`:

| Value | Behavior |
| --- | --- |
| `rendezvous` | Consistent hash with optional sticky affinity (default) |
| `round_robin` | Rotate primaries per scope |
| `least_connections` | Prefer upstreams with fewer active connections |
| `source_hash` | Hash client affinity key (IP, cookie, header, query) |
| `weighted` | Weighted rotation using `upstream_weights` |

Passive circuit breaking quarantines failing upstreams:

```yaml
load_balance:
  algorithm: weighted
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
    path: /healthz
    interval_secs: 10
    timeout_ms: 2000
    udp_payload: proxysss-health
    udp_expect_response: true
```

UDP probes are opt-in because many KCP/game protocols do not echo generic health payloads. When `udp_expect_response=false`, proxysss records a successful UDP send as the health signal.

## Runtime watchdog

Critical background loops are supervised by `runtime.watchdog`. If a supervised task exits unexpectedly, proxysss increments `proxysss_critical_task_failures_total`; when `restart_critical_tasks=true`, it restarts the task after `restart_backoff_secs`.

```yaml
runtime:
  watchdog:
    enabled: true
    restart_critical_tasks: true
    restart_backoff_secs: 2
    heartbeat_interval_secs: 30
```

Heartbeat ticks are exposed as `proxysss_watchdog_heartbeat_total` on `/metrics`. For strict process-manager deployments, set `restart_critical_tasks=false` so a critical task failure terminates the process and lets systemd/supervisor restart the whole gateway.

## TLS, HTTP/2, HTTP/3, WebSocket, gRPC

- Terminate TLS on `http.tls_bind` (default `0.0.0.0:443`).
- HTTP/2 and HTTP/3 share the TLS listener configuration.
- WebSocket upgrades are proxied when routes use `ws://` or `wss://` upstreams.
- gRPC (`application/grpc`, `application/grpc+proto`) works over HTTP/2 reverse proxying without extra directives.

## Access control and rate limits

```yaml
services:
  access_control:
    http:
      enabled: true
      deny: [203.0.113.0/24]
      allow: [198.51.100.0/24]
  rate_limit:
    http:
      enabled: true
      algorithm: fixed_window   # fixed_window, token_bucket, or leaky_bucket
      zone: public
      requests: 100
      window_ms: 60000
      burst: 20
      max_connections: 200
    stream:
      enabled: true
      zone: edge-tcp
      algorithm: leaky_bucket
      connections: 60
      window_ms: 60000
      burst: 10
```

## Caching and compression

Global defaults live in `services.response_policy`; routes can override `compression` and `cache`. Use `services.cache_zones` for shared zones and optional disk backing.

Cache modes (Cloudflare-style):

| `behavior` | Effect |
| --- | --- |
| `respect_origin` | Default. Honor origin `Cache-Control` for storage; use `ttl_secs` when origin has no max-age. |
| `override` | Force edge TTL from `ttl_secs`; ignore origin max-age for storage. |
| `bypass` | Skip cache lookup and storage. |
| `no_cache` | Always fetch upstream; emit `no-cache` when configured. |

```yaml
services:
  response_policy:
    cache:
      enabled: true
      behavior: override
      ttl_secs: 3600          # edge TTL
      browser_ttl_secs: 300   # 0 = pass through origin Cache-Control
      stale_while_revalidate_secs: 60
      stale_if_error_secs: 86400
      emit_cdn_cache_control: true
      vary_headers: [Accept-Encoding]
```

Send `PURGE` to invalidate entries when `allow_purge: true`. Responses include `X-Cache: HIT|MISS|STALE` when caching is active.

## Domain stream proxy (Redis, MySQL, etc.)

Route TCP/TLS workloads by SNI hostname (nginx `ssl_preread` / HAProxy style):

```yaml
tcp:
  stream_routes:
    - name: redis-prod
      domains: [redis.example.com]
      listen: 6379
      upstream: redis.internal:6379
      protocol: redis
    - name: mysql-prod
      domains: [db.example.com]
      listen: 3306
      upstream: mysql.internal:3306
      protocol: mysql
      tls_mode: passthrough
```

`listen` accepts `0.0.0.0:6379` or shorthand `6379`. `protocol` is an observability hint. Per-route `access_control` supports allow/deny IP lists.

## Direct TCP/UDP listeners for games, AI tools, and KCP-style traffic

Direct listeners are for large realtime fleets that do not need HTTP semantics: game TCP, KCP-style UDP, voice, MQTT/IoT device protocols, AI tool bridges, and other long-lived binary connections.

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      protocol: game_tcp
      nodelay: true
      connect_timeout_ms: 3000
      upstream: 127.0.0.1:9000
      upstreams:
        - 127.0.0.1:9000
        - 127.0.0.1:9001

udp:
  listeners:
    - name: game-kcp
      bind: 0.0.0.0:7001
      protocol: kcp
      session_ttl_secs: 180
      max_associations: 262144
      upstreams:
        - 127.0.0.1:9100
        - 127.0.0.1:9101
```

- `tcp.listeners[].nodelay` defaults to `true` so latency-sensitive streams do not wait on Nagle batching.
- `tcp.listeners[].connect_timeout_ms` defaults to `3000`; tune lower for hot failover, higher for cold backends.
- `udp.listeners[].session_ttl_secs` defaults to `180`; keep it above the client heartbeat interval for KCP or mobile reconnect traffic.
- `udp.listeners[].max_associations` defaults to `262144`; set `0` only for controlled labs where unbounded UDP association growth is acceptable.
- `protocol` is an observability hint only. The hot path stays transparent and does not parse KCP/game payloads.

## MQTT and IoT patterns

Use proxysss as the edge and keep MQTT/CoAP protocol semantics in the upstream broker or device service.

```yaml
tcp:
  listeners:
    - name: mqtt
      bind: 0.0.0.0:1883
      protocol: mqtt
      nodelay: true
      connect_timeout_ms: 3000
      upstreams:
        - 127.0.0.1:18831
        - 127.0.0.1:18832
  stream_routes:
    - name: mqtt-tls
      domains: [mqtt.example.com]
      listen: 0.0.0.0:8883
      upstream: 127.0.0.1:88831
      protocol: mqtt
      tls_mode: passthrough

udp:
  listeners:
    - name: coap
      bind: 0.0.0.0:5683
      protocol: coap
      session_ttl_secs: 120
      max_associations: 262144
      upstreams:
        - 127.0.0.1:56831

services:
  reverse_proxy:
    routes:
      - name: mqtt-websocket
        hosts: [mqtt-ws.example.com]
        path_prefix: /mqtt
        upstream: ws://127.0.0.1:8083
```

- MQTT TCP on `1883` uses transparent TCP proxying with upstream pools.
- MQTT TLS on `8883` can be routed by SNI with `tcp.stream_routes` and `tls_mode: passthrough`.
- MQTT over WebSocket uses the normal HTTP/WebSocket reverse proxy path.
- CoAP and proprietary UDP device traffic use `udp.listeners` with TTL/cap controls.
- Use `services.rate_limit.stream`, `services.access_control.stream`, active health, and watchdog metrics for production IoT fleets.

## TLS, on-demand certificates, and wildcards

- Default automatic HTTPS uses built-in managed ACME with HTTP-01 and TLS-ALPN-01 (`http.tls.auto_https` or `http.tls.mode: acme_managed`).
- On-demand TLS (Caddy-style first-hit issuance):

```yaml
http:
  tls:
    mode: acme_managed
    on_demand:
      enabled: true
      allow: ['*.customers.example.com']
      max_active_certs: 100
      max_issues_per_hour: 30
      ask_url: http://127.0.0.1:8080/allow-tls?domain={domain}
```

- Wildcard certificates use built-in managed DNS-01: `http.tls.mode: acme_managed` + `http.tls.acme.challenge: dns01` + `http.tls.acme.dns.provider` (`cloudflare`, `aliyun_cn`, `aliyun_intl`, `tencent`, `volcengine`, `aws`, `azure`, `google`, or `manual` without API keys). Configure from the admin console at `http://127.0.0.1:7777/` or YAML/API. Legacy `acme_dns_external` + `acme.sh` remains for non-built-in providers only.

## Monitoring

```yaml
monitoring:
  enabled: true
  path: /metrics
  format: prometheus   # or json
```

Prometheus scrapers should hit `http://<public-host>/metrics`. JSON format remains available for legacy dashboards.

## Security

```yaml
security:
  validate_admin_mutations: true
  block_ssrf_targets: true
  reject_ambiguous_http1: true
  blocked_upstream_hosts: [metadata.google.internal, 169.254.169.254]
```

See [SECURITY.md](./SECURITY.md) for the full hardening guide.

## Admin console

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  username: ops
  password: change-me
  bearer_token: cluster-automation-token
  enable_write_ops: true   # default false — enable for agent automation
  expose_config: false
  loopback_only: true
  https:
    enabled: false         # set true after initial TLS bootstrap
    path_prefix: /_proxysss/admin
    hosts: []              # optional Host allowlist for HTTPS admin API
  auth_rate_limit:
    enabled: true
    max_failures: 8
    window_secs: 300
    lockout_secs: 900
```

Bootstrap ACME/TLS on loopback (`http://127.0.0.1:7777/v1/tls/*`). After certificate material exists, enable `admin.https` and drive the same `/v1/*` endpoints over HTTPS at `path_prefix` (Bearer token recommended). Plain HTTP to the public admin path is rejected; HTTPS writes require existing cert/key files.

Agent automation examples: [AGENT-API.md](./AGENT-API.md).

## Kubernetes ingress-style mode

```yaml
kubernetes:
  enabled: true
  namespace: prod
  cluster_domain: cluster.local
  mappings:
    - name: api
      service: api-svc
      port: 8080
      domains: [api.example.com]
      path_prefix: /
```

When enabled, mappings are expanded into `services.domain_routes` on each load/reload.

Open `http://127.0.0.1:7777/` for the dashboard. Automation APIs live under `/v1/*`.

## Hot reload vs restart

Inspect boundaries anytime:

```bash
proxysss config reload-plan
proxysss config routes
```

Hot reload covers routes, scripts, plugins, and most `services.*` values. Listener binds, TLS mode, and logging sink paths require a process restart.

## Learning templates

Generate focused starter files:

```bash
proxysss config create-template full ./starter.yaml
proxysss config create-template http ./http-only.yaml
proxysss config create-template tcp ./tcp-only.yaml
```
