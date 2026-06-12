# proxysss

## Installation

**Linux and macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

**Windows PowerShell:**
```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

**Upgrade to a specific version:**
```bash
proxysss update --version v1.2.11
```

proxysss is a high-performance load balancer and reverse proxy server built to replace nginx as a general-purpose edge gateway. It handles HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP/KCP-style datagrams, MQTT/IoT stream gateways, FTP, WebDAV, AI API reverse proxying, and static delivery in one Rust binary while keeping the operational model straightforward.

Current version: v1.2.11

## Why proxysss

- One runtime config file: keep gateway settings in a single YAML file, usually `proxysss.yaml`.
- Explicit config path support: use `-config`, `--config`, or `-c` to point at a different YAML path.
- YAML-only gateway config: JSON config files are intentionally unsupported.
- Domain-first reverse proxying: `services.domain_routes` is the primary grouping unit for multi-domain HTTP services.
- Agent-native control plane: password or bearer-token admin API on `127.0.0.1:7777` with secure defaults (`enable_write_ops: false` until explicitly enabled).
- Cluster automation API: upsert/delete routes, provision managed ACME or acme.sh wildcard TLS, and persist atomically to the single YAML file.
- Hot reload: the main YAML config, the main script, and auto-loaded plugins participate in reload fingerprinting.
- Optional scripting: TypeScript plugins are for custom business logic, not for ordinary gateway setup.

## Supported gateway surface

- HTTP/1.1, HTTPS, HTTP/2, HTTP/3, and WebSocket
- Low-latency TCP stream proxying with `TCP_NODELAY` and upstream connect timeout controls
- UDP/KCP-style stream proxying with per-listener association TTL and caps for large game, voice, realtime, and device fleets
- MQTT and IoT gateway patterns: MQTT TCP, MQTT TLS passthrough, MQTT over WebSocket, CoAP-style UDP, and stream rate policies
- FTP nginx module directive-level parity with control-channel proxying, passive and active data-channel rewriting, allow/deny policy, command/transfer hooks, and per-user policies
- WebDAV and static file serving
- First-class AI reverse proxy routes for New API, sub2api, and OpenAI-compatible upstreams through `services.ai_proxy`
- Managed ACME with HTTP-01, TLS-ALPN-01, and built-in DNS-01 wildcard certificates; legacy `acme_dns_external` + acme.sh remains only for non-built-in DNS providers
- Shared cache zones with stale-while-revalidate, compression, access control, fixed-window/token-bucket/leaky-bucket HTTP and stream rate limiting, retries, and active health checks
- Runtime watchdog with critical background task restart, heartbeat metrics, and nonblocking access/error logging
- Prometheus metrics on `/metrics`, weighted load balancing, round-robin, least-connections, source-hash, and rendezvous affinity
- gRPC-over-HTTP/2, WebSocket, sticky sessions, passive quarantine (circuit breaker), and upstream failover retries

## proxysss vs nginx

proxysss is designed as a same-level nginx replacement for general gateway work: protocol termination, routing, reverse proxying, static delivery, stream forwarding, reload, observability, and operator automation. The comparison below uses nginx open source core modules and common nginx deployment patterns as the baseline. NGINX Plus and third-party module stacks can add behavior that is not present in a minimal nginx build, while proxysss keeps these gateway surfaces in one Rust binary and one YAML configuration model.

### Performance comparison

Performance depends on kernel, network card, TLS settings, payload shape, cache behavior, backend latency, worker count, and whether scripts/plugins sit on the request path. The table below is the checked repository quick gate, not a universal benchmark claim.

| Benchmark item | proxysss v1.2.11 | nginx 1.31.0 | Result |
| --- | ---: | ---: | --- |
| Workload | static HTTP payload | static HTTP payload | same local Windows host |
| Command | `.\scripts\benchmark-gateways.ps1 -Quick` | `.\scripts\benchmark-gateways.ps1 -Quick` | run on 2026-06-12 |
| Concurrency / duration | 128 / 10s | 128 / 10s | same gate settings |
| Requests per second | 11,559.70 | 6,349.90 | proxysss 1.82x nginx in this run |
| Throughput | 2.01 MiB/s | 1.10 MiB/s | static payload gate |
| p50 latency | 10.894 ms | 19.971 ms | lower is better |
| p95 latency | 13.418 ms | 24.488 ms | lower is better |
| p99 latency | 14.251 ms | 26.355 ms | lower is better |
| Errors | 0 | 0 | benchmark gate passed |

For production decisions, benchmark the exact traffic you plan to migrate:

| Traffic shape | What to compare | proxysss verification path |
| --- | --- | --- |
| Static HTTP | RPS, p95/p99, sendfile/file cache behavior, access log cost | `scripts/benchmark-gateways.ps1`, `proxysss bench http` |
| Reverse proxy APIs | upstream latency, keepalive reuse, retry behavior, header rewrite cost | `proxysss bench http` plus app-level synthetic checks |
| HTTPS / HTTP/2 / HTTP/3 | handshake cost, ALPN, certificate reload, QUIC behavior | staging TLS listener and browser/curl probes |
| AI streaming / SSE | first token latency, stream flush behavior, backpressure | `services.ai_proxy` route with real New API/sub2api/OpenAI-compatible upstreams |
| WebSocket | upgrade latency, idle timeout, long connection stability | route through `ws://` or `wss://` upstreams |
| TCP streams | connection churn, upstream selection, `TCP_NODELAY`, SNI routing | `proxysss bench tcp`, `tcp.listeners`, `tcp.stream_routes` |
| UDP / KCP-style traffic | association TTL, packet loss behavior, max association caps | `proxysss bench udp`, `udp.listeners` |
| MQTT / IoT | long-lived device connections, TLS passthrough, MQTT-over-WebSocket | `examples/iot-mqtt.example.yaml` |
| FTP / WebDAV | control/data channel behavior, upload/download throughput, policy hooks | migration tests against target clients |
| Cache/compression | hit ratio, stale behavior, CPU cost for gzip/brotli/zstd | `services.response_policy`, route cache settings |

The hot path is intentionally simple: Rust async I/O, transparent stream forwarding for opaque protocols, route policy in native structures, optional TypeScript plugins outside ordinary gateway setup, nonblocking logs, and runtime watchdog metrics. Add plugin hooks only where business behavior is needed.

### Protocol and gateway surface comparison

| Surface | nginx open source baseline | proxysss | Migration note |
| --- | --- | --- | --- |
| HTTP/1.1 reverse proxy | Supported through `http`, `server`, `location`, `proxy_pass` | Supported through `services.domain_routes` and `services.reverse_proxy.routes` | proxysss groups routes by domain/service in YAML |
| HTTPS / TLS / SNI | Supported through SSL directives and certificate files | Supported with manual certs, self-signed bootstrap, managed ACME, SNI, and on-demand TLS | proxysss can own certificate automation |
| HTTP/2 | Supported on TLS listeners | Supported on `http.tls_bind` | enabled as part of the TLS listener path |
| HTTP/3 / QUIC | Supported in modern nginx builds when enabled and built with required TLS support | Supported through `http.h3_bind` | keep UDP 443 open |
| WebSocket | Supported with HTTP Upgrade and proxy headers | Supported through HTTP routes and `ws://` / `wss://` upstreams | proxysss hides most upgrade boilerplate |
| gRPC | Supported through nginx gRPC module | Supported as HTTP/2 reverse proxy traffic | use TLS/HTTP2 route checks in staging |
| Static files | Supported | Supported | proxysss also serves the default `Welcome to proxysss` page |
| WebDAV | Supported by nginx WebDAV module | Supported as built-in gateway behavior | file operations stay under configured roots |
| FTP | nginx FTP module style deployments depend on module availability | Built-in directive-level parity: control proxying, passive/active data rewriting, allow/deny, command/transfer policies, per-user policies, lifecycle logs | configure `services.ftp` |
| TCP stream proxy | Supported by stream module | Supported by `tcp.listeners` and `tcp.stream_routes` | SNI stream routing covers Redis/MySQL/PostgreSQL/MongoDB-style TLS routing |
| UDP proxy | Supported by stream proxy module | Supported by `udp.listeners` with association TTL and max association caps | useful for KCP, game, voice, CoAP-style traffic |
| MQTT TCP | Generic stream proxy | Generic stream proxy with `protocol: mqtt` observability hints and IoT template | MQTT broker semantics stay upstream |
| MQTT TLS passthrough | Generic stream/SNI routing | `tcp.stream_routes` with `tls_mode: passthrough` | edge stays transparent |
| MQTT over WebSocket | HTTP WebSocket proxy configuration | HTTP route to `ws://` / `wss://` upstream | works with browser/device MQTT clients |
| CoAP-style UDP | Generic UDP stream proxy | Generic UDP listener with IoT template | protocol parsing remains upstream |
| AI API reverse proxy | Generic reverse proxy, usually plus Lua/njs/app config for provider behavior | First-class `services.ai_proxy` for New API, sub2api, and OpenAI-compatible upstreams | path rewrite and provider metadata are native config |
| Cache | `proxy_cache` and related directives | Shared cache zones, stale-while-revalidate, stale-if-error, CDN cache-control behavior, PURGE, vary headers | configure per route or policy |
| Compression | gzip module and optional brotli/zstd depending on build/modules | `services.response_policy` and route overrides for zstd/brotli/gzip | native policy surface |
| Rate limiting | HTTP limit modules and stream limits | fixed-window, token-bucket, leaky-bucket HTTP policies, stream shared zones, connection caps | one YAML policy model |
| DDoS / blacklist | commonly assembled from modules, maps, WAF, firewall, or Plus features | `security.ddos`, dynamic blacklist admin API, stream access control | token-authenticated runtime updates |
| Admin automation | typically external scripts, config generation, signals/reload | Built-in loopback admin API on `127.0.0.1:7777`, token auth, write ops disabled by default | route/listener updates can persist to YAML |
| Metrics | stub status or external exporters depending on setup | Prometheus `/metrics` and admin stats | enabled by config |
| Runtime watchdog | master/worker process supervision model | runtime watchdog for critical tasks, heartbeat metrics, restart backoff | complements systemd/service supervision |
| Scripting/plugins | C modules, dynamic modules, njs, or third-party Lua/OpenResty | embedded TypeScript/JavaScript runtime in the same binary | scripts are optional extension hooks, not required for normal routes |

### Configuration syntax comparison

nginx uses directive blocks and often spreads production configuration across `nginx.conf`, `conf.d/*.conf`, snippets, maps, includes, and module-specific files. That model is powerful and mature, but it is harder for automation agents to inspect because the final behavior depends on include order, directive inheritance, and module availability.

proxysss uses one YAML document by default, normally `proxysss.yaml`. The same file can describe HTTP routes, TLS, ACME, TCP, UDP, FTP, WebDAV, AI proxy routes, MQTT/IoT listeners, cache, compression, rate limits, admin API behavior, logs, and plugin loading. Use `proxysss config explain`, `proxysss config routes`, `proxysss config reload-plan`, `proxysss config capabilities`, and `proxysss config nginx-parity --format yaml` to inspect the effective shape.

Basic reverse proxy:

```nginx
server {
    listen 80;
    server_name example.com www.example.com;

    location / {
        proxy_pass http://127.0.0.1:9000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```yaml
services:
  domain_routes:
    - name: example
      domains: [example.com, www.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
```

MQTT TCP plus MQTT-over-WebSocket:

```nginx
stream {
    upstream mqtt_brokers {
        server 127.0.0.1:18831;
        server 127.0.0.1:18832;
    }

    server {
        listen 1883;
        proxy_pass mqtt_brokers;
        tcp_nodelay on;
    }
}

http {
    server {
        listen 80;
        server_name mqtt-ws.example.com;

        location /mqtt {
            proxy_pass http://127.0.0.1:8083;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
        }
    }
}
```

```yaml
tcp:
  listeners:
    - name: mqtt
      bind: 0.0.0.0:1883
      protocol: mqtt
      nodelay: true
      upstreams:
        - 127.0.0.1:18831
        - 127.0.0.1:18832

services:
  reverse_proxy:
    routes:
      - name: mqtt-websocket
        hosts: [mqtt-ws.example.com]
        path_prefix: /mqtt
        upstream: ws://127.0.0.1:8083
```

### Plugin and extension comparison

| Area | nginx | proxysss |
| --- | --- | --- |
| Built-in extension model | C modules compiled into or loaded by nginx | Rust core plus embedded TypeScript/JavaScript extension hooks |
| Common scripting path | njs or Lua through OpenResty/third-party module stacks | QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip` |
| Runtime dependency | depends on selected nginx package and modules | single `proxysss` binary, no Node/Deno/tsc sidecar |
| Business logic placement | often Lua/njs modules, sidecars, or upstream apps | optional scripts/plugins, similar in role to nginx + Lua |
| Hot reload boundary | nginx reloads config; script behavior depends on module/runtime | YAML, main script, and auto-loaded plugin scripts are fingerprinted for reload |
| Plugin metadata | module/package specific | YAML sidecars: `plugins/<name>.plugin.yaml` or `.yml` |
| Agent inspectability | parse final nginx config and module inventory | dedicated CLI: `capabilities`, `routes`, `reload-plan`, `watched-scripts`, `nginx-parity` |
| Hot-path policy | module/directive driven | native Rust policies first; plugin calls only where configured |

The product rule is deliberate: proxysss is not a business gateway. It is a general gateway with extension hooks. Put ordinary gateway duties in YAML and reserve scripts/plugins for custom business routing, affinity, headers, metadata, or upstream selection.

### Operations comparison

| Operational task | nginx common approach | proxysss approach |
| --- | --- | --- |
| Install | package manager, source build, container, or vendor package | install script or release archive for one binary |
| Main config | `nginx.conf` plus includes | one `proxysss.yaml` by default |
| Validate config | `nginx -t` | `proxysss -config ./proxysss.yaml check-config` |
| Reload | signal/service reload | hot reload for config/script/plugin boundaries, restart where listener identity changes |
| Discover routes | inspect merged config manually or generated output | `proxysss config routes` |
| Discover parity/capabilities | module inventory plus docs | `proxysss config capabilities` and `proxysss config nginx-parity --format yaml` |
| Admin API | usually external control plane | built-in loopback admin API, write ops opt-in |
| Token handling | external auth/control tooling | `proxysss token show` / `proxysss token set`, config display redacts secrets |
| Wildcard ACME | usually external certbot/acme.sh or platform automation | built-in DNS-01 providers for managed ACME; `acme_dns_external` only for non-built-in providers |
| Logs | access/error logs | `logs/access.log`, `logs/error.log`, level control |
| Metrics | status modules/exporters | Prometheus `/metrics` and admin stats |
| Production hardening | OS/service/nginx module practices | benchmark gates, watchdog, active health, maintenance state, admin-safe defaults |

### When nginx is still the better fit

- You already depend on a specific nginx or OpenResty third-party module with no proxysss equivalent.
- You need exact directive compatibility rather than a cleaner YAML migration.
- Your fleet is already deeply standardized around nginx package builds, module signing, and mature internal runbooks.
- You need a niche mail/module behavior outside proxysss current gateway scope.

### When proxysss is the better fit

- You want nginx-class gateway duties in one binary with one YAML file.
- You need first-class New API, sub2api, or OpenAI-compatible AI reverse proxy routes.
- You want MQTT/IoT, game TCP/UDP/KCP-style, WebDAV, FTP, HTTP, HTTPS, HTTP/2, HTTP/3, and WebSocket surfaces managed in one config.
- You want TypeScript plugins without deploying Node, Deno, tsc, or a sidecar runtime.
- You want an agent-friendly admin API and CLI inspection surface for automation.
- You want managed ACME DNS-01 wildcard certificates built into the gateway.

### Comparison sources and verification commands

nginx comparison baseline:

- [nginx HTTP core module](https://nginx.org/en/docs/http/ngx_http_core_module.html)
- [nginx stream core module](https://nginx.org/en/docs/stream/ngx_stream_core_module.html)
- [nginx stream proxy module](https://nginx.org/en/docs/stream/ngx_stream_proxy_module.html)
- [nginx WebSocket proxying](https://nginx.org/en/docs/http/websocket.html)
- [nginx gRPC module](https://nginx.org/en/docs/http/ngx_http_grpc_module.html)
- [nginx development guide](https://nginx.org/en/docs/dev/development_guide.html)
- [NGINX TCP/UDP load balancing admin guide](https://docs.nginx.com/nginx/admin-guide/load-balancer/tcp-udp-load-balancer/)

Before a migration, verify the exact local build:

```bash
proxysss -config ./proxysss.yaml check-config
proxysss -config ./proxysss.yaml config capabilities
proxysss -config ./proxysss.yaml config routes
proxysss -config ./proxysss.yaml config reload-plan
proxysss -config ./proxysss.yaml config nginx-parity --format yaml
proxysss config create-template iot ./iot.yaml
proxysss -config ./iot.yaml check-config
.\scripts\benchmark-gateways.ps1 -Quick
```

## Configuration model

proxysss treats the runtime config as a single YAML document.

- Default config name: `proxysss.yaml`
- Custom config path: `proxysss -config ./edge.yaml`, `proxysss --config ./edge.yaml`, or `proxysss -c ./edge.yaml`
- `include` is unsupported for runtime config
- JSON config files are unsupported

That keeps onboarding and operations simple: one file, one source of truth, and one hot-reload target.

## Quick start

Initialize a working directory:

```bash
proxysss init
```

This generates:

- `proxysss.yaml`
- `gateway.ts`
- `proxysss-script.d.ts`
- `ts-how-to-use.md`
- `nginx-to-proxysss.md`
- `plugins/player-affinity.ts`
- `plugins/traffic-stats.ts`
- `plugins/structured-log.ts`
- `plugins/geo-headers.ts`
- `plugins/ai-api-compat.ts`
- `certs/proxysss-cert.pem`
- `certs/proxysss-key.pem`

Validate the default config:

```bash
proxysss -config ./proxysss.yaml check-config
```

For production release gates and HA hardening, see [docs/PRODUCTION-HARDENING.md](docs/PRODUCTION-HARDENING.md).

Run the gateway:

```bash
proxysss -config ./proxysss.yaml
```

Or use a custom YAML file:

```bash
proxysss -c ./my-edge.yaml
```

Default ports:

- `80` for public HTTP
- `443` for HTTPS, HTTP/2, and HTTP/3
- `7777` for the admin console and API

## Example: multiple domains in one YAML file

This is the recommended model when one machine hosts multiple services.

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  h3_bind: 0.0.0.0:443
  tls:
    auto_https:
      enabled: true
      email: admin@example.com

services:
  access_control:
    http:
      enabled: true
      blacklist: [203.0.113.10, 198.51.100.0/24]

  rate_limit:
    http:
      enabled: true
      requests: 120
      window_ms: 60000
      burst: 30

  domain_routes:
    - name: example-site
      domains: [example.com, www.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
      compression:
        enabled: true

    - name: neko233-store
      domains: [neko233.store]
      path_prefix: /
      upstream: http://127.0.0.1:9000
      upstreams:
        - http://127.0.0.1:9001
      cache:
        enabled: true
        ttl_secs: 30
      active_health:
        path: /healthz
        failure_threshold: 2
        success_threshold: 2
```

In that example:

- `example.com` has one backend machine.
- `neko233.store` reuses that same machine and adds one more backend.
- each domain route is its own service group with its own routing, health, cache, compression, and TLS policy.

## Example: AI API reverse proxy

Use `services.ai_proxy` for New API, sub2api, and OpenAI-compatible upstreams when the gateway should own path rewrites and provider metadata without custom business code.

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
```

## Example: game TCP and KCP-style UDP edge

Use direct stream listeners for latency-sensitive game servers, AI tool bridges, device gateways, voice, or KCP-style UDP protocols. `protocol` is an observability hint; TCP disables Nagle by default, and UDP listeners prune idle associations so churn cannot grow without bound.

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      protocol: game_tcp
      nodelay: true
      connect_timeout_ms: 3000
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

load_balance:
  active_health:
    enabled: true
    http_enabled: true
    tcp_enabled: true
    udp_enabled: true
    udp_payload: proxysss-health
    udp_expect_response: true

runtime:
  watchdog:
    enabled: true
    restart_critical_tasks: true
    restart_backoff_secs: 2
    heartbeat_interval_secs: 30
```

## Example: MQTT and IoT edge

MQTT brokers usually stay protocol-aware upstreams; proxysss keeps the edge transparent and handles listener binding, TLS passthrough, WebSocket upgrades, upstream pools, rate limits, health, and observability.

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

## Automatic HTTPS

Automatic certificate issuance and renewal are built in.

- Challenge types: HTTP-01, TLS-ALPN-01, and built-in DNS-01
- No external ACME binary is required for the managed path
- Domain-level `ssl.type: auto` and global `http.tls.auto_https` both expand into the managed ACME flow
- Wildcard certificates use built-in managed DNS-01 via `http.tls.mode: acme_managed` + `http.tls.acme.challenge: dns01`

Minimal public setup:

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  h3_bind: 0.0.0.0:443
  tls:
    auto_https:
      enabled: true
      domains: [example.com, www.example.com]
      email: admin@example.com
      production: true
      challenge: tls_alpn01

services:
  domain_routes:
    - name: app
      domains: [example.com, www.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
```

Wildcard setup with built-in DNS-01:

```yaml
http:
  tls:
    mode: acme_managed
    cert_path: certs/proxysss-cert.pem
    key_path: certs/proxysss-key.pem
    generate_self_signed_if_missing: false
    server_name: example.com
    acme:
      email: admin@example.com
      challenge: dns01
      domains: [example.com, "*.example.com"]
      directory_production: true
      renew_interval_hours: 12
      dns:
        provider: cloudflare
        credentials:
          api_token: your-cloudflare-api-token
```

Built-in DNS providers (one vendor = one strategy): `cloudflare`, `aliyun_cn`, `aliyun_intl`, `tencent`, `volcengine`, `aws`, `azure`, `google`. Without cloud credentials, use `http.tls.auto_https` or `acme_managed` with `http01` / `tls_alpn01` — no external ACME binary required.

## Commands you will actually use

Inspect config and runtime shape:

```bash
proxysss -config ./proxysss.yaml config explain
proxysss -config ./proxysss.yaml config routes
proxysss -config ./proxysss.yaml config reload-plan
proxysss -config ./proxysss.yaml config nginx-parity --format yaml
proxysss -config ./proxysss.yaml config capabilities
```

Start and manage the service:

```bash
proxysss -config ./proxysss.yaml start
proxysss -config ./proxysss.yaml restart
proxysss -config ./proxysss.yaml status
proxysss -config ./proxysss.yaml stop
```

Inspect or rotate the local automation token:

```bash
proxysss token show
proxysss token set
proxysss token set my-custom-cluster-token
```

The dedicated `token` command is the supported local query path. Normal config display surfaces redact both the admin password and bearer token.

Check the embedded TypeScript runtime:

```bash
proxysss script run-file ./examples/gateway.ts
proxysss script eval "console.log('proxysss ts runtime ok')"
```

## Cluster automation

For cluster startup automation, configure a bearer token on the admin API and let services register themselves over HTTP.

Example admin config:

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  bearer_token: change-this-cluster-token
  enable_write_ops: true
```

Example domain-route registration call:

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/upsert \
  -H "Authorization: Bearer change-this-cluster-token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "node-17-api",
    "domains": ["api.example.com"],
    "path_prefix": "/",
    "upstream": "http://10.0.0.17:8080",
    "upstreams": ["http://10.0.0.18:8080"],
    "strip_prefix": false
  }'
```

That API call:

- authenticates with a bearer token
- upserts the route by name inside the main `proxysss.yaml`
- persists the updated YAML to disk
- reloads the gateway in process so the route becomes live immediately

This is the intended path when a node or service instance should self-register into the cluster edge layer.

Other automation endpoints follow the same pattern:

- `POST /v1/reverse-proxy-routes/upsert`
- `POST /v1/tcp-listeners/upsert`
- `POST /v1/udp-listeners/upsert`

WebSocket and WSS flows continue to live on the HTTP route layer, so they are covered by `domain-routes` or `reverse-proxy-routes` automation.

## Plugin sidecar metadata

If you use auto-loaded plugins, sidecar metadata is YAML-only as well.

- `plugins/<name>.plugin.yaml`
- `plugins/<name>.plugin.yml`

## Operational defaults

- Admin bind: `127.0.0.1:7777`
- Default admin credentials: `root / root`
- Access log: `logs/access.log`
- Error log: `logs/error.log`

Change the default admin credentials before production use.

## Monitoring

Prometheus-compatible counters are exposed on the public HTTP listener:

```yaml
monitoring:
  enabled: true
  path: /metrics
  format: prometheus   # set to json for the previous JSON payload
```

Scrape `http://<host>/metrics` or inspect JSON stats from the admin API at `GET /v1/stats`.

## Related docs

- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — configuration tutorial
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — runtime architecture
- [docs/architecture.html](docs/architecture.html) — animated protocol and architecture lab for first-year readers
- [docs/IMPROVEMENT-BACKLOG.md](docs/IMPROVEMENT-BACKLOG.md) — next stability, performance, security, and protocol work
- [docs/SECURITY.md](docs/SECURITY.md) — security defaults and hardening
- [docs/AGENT-API.md](docs/AGENT-API.md) — password/token agent automation API
- [examples/demo/README.md](examples/demo/README.md) — demo commands
- `ts-how-to-use.md`
- `nginx-to-proxysss.md`
- `proxysss-script.d.ts`
- `http://localhost/docs.html`
- `http://localhost/docs`
