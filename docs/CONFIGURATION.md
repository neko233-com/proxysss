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
runtime:       # hot reload and maintenance state
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
      cache:
        enabled: true
        ttl_secs: 30
      rate_limit:
        enabled: true
        algorithm: token_bucket
        requests: 120
        window_ms: 60000
        burst: 30
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
    path: /healthz
    interval_secs: 10
```

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
      algorithm: fixed_window   # or token_bucket
      zone: public
      requests: 100
      window_ms: 60000
      burst: 20
      max_connections: 200
```

## Caching and compression

Global defaults live in `services.response_policy`; routes can override `compression` and `cache`. Use `services.cache_zones` for shared zones and optional disk backing. Send `PURGE` to invalidate entries when `allow_purge: true`.

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
  auth_rate_limit:
    enabled: true
    max_failures: 8
    window_secs: 300
    lockout_secs: 900
```

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
