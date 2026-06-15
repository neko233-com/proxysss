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
proxysss update --version v1.3.1
```

proxysss is a high-performance load balancer and reverse proxy server built to replace nginx as a general-purpose edge gateway. It handles HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP/KCP-style datagrams, MQTT/IoT stream gateways, FTP, WebDAV, AI API reverse proxying, and static delivery in one Rust binary while keeping the operational model straightforward.

Current version: v1.3.1

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

Performance depends on kernel, network card, TLS settings, payload shape, cache behavior, backend latency, worker count, and whether scripts/plugins sit on the request path. proxysss performance claims and release gates are Linux-only because production gateway deployments run on Linux; Windows/macOS measurements are development diagnostics, not nginx replacement evidence.

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

On Ubuntu/Linux, run `proxysss tune linux --apply` first, then run `scripts/benchmark-all-scenarios.sh` on the host or in the Ubuntu 24 benchmark container. The default benchmark is a mixed multi-proxy load (`MIXED_MATRIX=1`, `FAST_GATE=0`, `CRITICAL_RATIO=0.97`, `AGGREGATE_RATIO=0.97`): nginx runs first, then proxysss runs the same concurrent wave. The wave includes static small files, static large files, CDN hot-update static files, HTTPS static files, HTTP reverse proxy, New API/SSE streaming, WebSocket long connections, game-style long TCP connections, TCP stream, UDP stream, and KCP-style UDP. Scenario concurrency is derived from detected CPU cores instead of hard-coded user input and weighted for the default small/latency traffic profile. Release success uses a fair ratio floor rather than a blind `>1.0`: the default critical game/realtime scenarios (WebSocket long connections, game TCP, generic TCP, UDP, and KCP-style UDP) and aggregate mixed load may be within 1-3% of nginx because proxysss ships many gateway policy surfaces that nginx commonly needs extra modules or config to match. Static-small, CDN hot-update, reverse proxy, and New API/SSE still run in the same wave and must remain above the soft floor (`MIN_RATIO=0.50`) with low errors; HTTPS static and static-large are reported but diagnostic by default unless a TLS/static or bulk-transfer release promotes them. Strict `>1.0` head-to-head gates can still be requested with explicit environment overrides.

The hot path is intentionally simple but not one-size-fits-all: Rust async I/O, explicit TCP sockets with large backlog and `TCP_NODELAY`, default-on `runtime.performance` adaptive Linux tuning, Ubuntu 24.x extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`) with logged downgrade on older or unknown distros, Linux plain-HTTP and TCP stream accept fanout through `SO_REUSEPORT`, HTTP/1 writev plus HTTP/2 adaptive-window server settings, tuned HTTP upstream keepalive pools, Linux builds using jemalloc for lower allocation overhead, hot static files served from a bounded cache that can hold mmap-backed `Bytes`, eligible plain-HTTP static files served through a traffic-profile-aware fast lane, raw plain-HTTP reverse/SSE/WebSocket lanes for simple no-policy routes, and transparent TCP forwarding with an isolated stream runtime plus independent relay profiles. `runtime.performance.traffic_profile` defaults to `small`, favoring cached small files, HTTP/2/SSE feedback latency, TCP/WebSocket long connections, and UDP/KCP-style realtime traffic; `bulk` moves static fast lanes toward sendfile/zero-copy large transfers; `balanced` enables both preload styles. Data-plane worker counts are not exposed as user knobs: Linux accept loops and runtime budgets derive from detected CPU cores so higher-core hosts automatically scale up and users cannot accidentally cap production throughput with a bad config value. The stream runtime receives an automatic CPU budget instead of taking every core, leaving HTTP/static/SSE capacity available during mixed gateway load. Shared hot-path pools avoid global mutex bottlenecks; raw HTTP upstream keepalive uses a lock-free bounded queue, while control-plane synchronization remains isolated to config reload, certificates, and one-time static-cache fill coordination. Config load preloads eligible static index/top-level hot files or sendfile descriptors according to `traffic_profile`. When performance mode is enabled, TCP listeners run on a dedicated `proxysss-stream` Tokio runtime with per-bind reuseport accept workers so long-lived game/MQTT/tool connections do not compete with HTTP/static/SSE workers. Single-upstream direct TCP routes with scripts, affinity, active health, and passive health disabled bypass the generic upstream planner entirely. TCP latency streams use 16KB relay buffers and parallel one-way pumps; explicit bulk/file stream protocols can use a Linux `splice` profile when zero-copy beats user-space copy. Raw reverse avoids `Uri` reparsing, byte-filters hop headers, skips redundant `Content-Length: 0`, and can coalesce response head plus small fixed bodies into one write; raw SSE uses connection-close byte passthrough after the response head to minimize first-token latency; raw WebSocket tunnels no-policy `ws://` routes before the general Hyper upgrade path. Route policy stays in native structures, optional TypeScript plugins stay outside ordinary gateway setup, logs are nonblocking, and watchdog metrics cover background loops. File watching is opt-in; the default reload model is manual `/v1/reload` or admin mutations that persist YAML and reload in process. Normal startup never writes host sysctl files; `proxysss tune linux --apply` uses guarded sysctl apply with unsupported-key filtering, SSH-safe scope, backup, and rollback unless `--unsafe-apply` is explicitly used. When active/passive upstream health is disabled, the single-upstream path skips runtime health bookkeeping. Add plugin hooks only where business behavior is needed.

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
      # Optional nginx-parity/high-throughput mode when you do not need
      # X-Forwarded-* / Forwarded headers added by the gateway.
      forward_headers: false
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

Use `services.ai_proxy` for New API, sub2api, and OpenAI-compatible upstreams when the gateway should own path rewrites and optional provider metadata without custom business code.

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
        emit_metadata_headers: false
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
  performance:
    enabled: true
    profile: latency
    traffic_profile: small
    adaptive_system: true
    socket_extreme: true
    log_on_start: true
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

- Public GitHub Pages docs hub: [https://neko233-com.github.io/proxysss/](https://neko233-com.github.io/proxysss/)
- Public architecture lab: [https://neko233-com.github.io/proxysss/architecture.html](https://neko233-com.github.io/proxysss/architecture.html)
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

GitHub Pages serves the repository `docs/` directory as the site root, so the public URL path is `/proxysss/...`, not `/proxysss/docs/...`.
