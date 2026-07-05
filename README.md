# proxysss

proxysss is a same-level replacement for nginx as a general-purpose edge gateway. It keeps HTTP, HTTPS, HTTP/2, HTTP/3, gRPC-over-HTTP/2, WebSocket, TCP, UDP, MQTT/IoT edge patterns, FTP, WebDAV, AI reverse proxy routes, TLS automation, logs, and reload behavior in one Rust binary and one YAML file.

It also keeps practical gateway features like static asset serving with Range downloads, cache/proxy cache, compression, Consul/etcd/Nacos discovery configuration, Kubernetes ingress-style mappings, CDN origin routes, IPv6 CIDR access policy, and rate limiting algorithms including fixed-window, token-bucket, and leaky-bucket in the same configuration surface.

If you only remember one thing, remember this: `proxysss` is for ordinary gateway work first, and optional TypeScript plugins second.

## Read This README In Two Ways

### Beginner path

Use this path if you want to:

- put one website or API behind a gateway
- understand which YAML block to touch
- get a copy-paste example and a plain-language explanation

### Expert path

Use this path if you want to:

- choose between `domain_routes`, `reverse_proxy`, `ai_proxy`, `tcp.listeners`, and `udp.listeners`
- benchmark against nginx
- expose the admin API safely
- tune Linux for production without hurting other traffic shapes

## Beginner Path

### 1. Install and initialize

Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

Create a starter workspace:

```bash
proxysss init
```

This gives you:

- `proxysss.yaml`
- `gateway.ts`
- `proxysss-script.d.ts`
- `ts-how-to-use.md`
- `nginx-to-proxysss.md`
- example plugins and self-signed certs

### 2. Your first working reverse proxy

If you have an app listening on `127.0.0.1:9000`, this is the easiest production-shaped starting point:

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  h3_bind: 0.0.0.0:443

services:
  domain_routes:
    - name: app
      domains: [example.com, www.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
```

What each part means:

- `plain_bind` exposes public HTTP on port `80`.
- `tls_bind` exposes HTTPS and HTTP/2 on port `443`.
- `h3_bind` exposes HTTP/3 on port `443/udp`.
- `domain_routes` is the recommended HTTP routing model when you care about hostnames.
- `domains` is the host list this route should answer for.
- `upstream` is the backend app that will receive the request.

Check the file before you run it:

```bash
proxysss -config ./proxysss.yaml check-config
```

Start the gateway:

```bash
proxysss -config ./proxysss.yaml
```

### 3. Add automatic HTTPS

If you want managed TLS without writing certificate files yourself:

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

How to think about it:

- `auto_https` is the easiest managed path for normal public domains.
- `challenge: tls_alpn01` avoids needing a writable web root.
- The route still lives in `domain_routes`; TLS automation does not change how you declare backends.

### 4. Serve a static site

If you just want to serve files from disk:

```yaml
http:
  plain_bind: 0.0.0.0:80

services:
  static_sites:
    - name: marketing
      path_prefix: /
      root: ./public
      index_files: [index.html]
      autoindex: false
```

What matters here:

- `path_prefix` is the URL path the static site owns.
- `root` is the local directory to serve from.
- `index_files` controls which file becomes `/`.
- `autoindex: false` is the normal public-site default.

### 5. Proxy AI streaming / SSE correctly

Use `services.ai_proxy` when you want the gateway to understand New API, sub2api, or OpenAI-compatible upstreams instead of treating them as generic HTTP:

```yaml
services:
  ai_proxy:
    enabled: true
    routes:
      - name: new-api
        provider: new-api
        match_host: ai.example.com
        path_prefix: /v1
        upstream: http://127.0.0.1:3000
        rewrite_base_path: /v1
        emit_metadata_headers: false
        forward_headers: false
```

Why this is different from a normal reverse proxy:

- `provider` selects the built-in AI gateway behavior.
- `rewrite_base_path` lets the public URL and upstream URL differ cleanly.
- `emit_metadata_headers: false` is useful for nginx-parity and SSE-sensitive paths.
- `forward_headers: false` is useful when the upstream does not need `X-Forwarded-*` and you want the leanest path.

## Expert Path

### Pick the right routing surface

| If you need to route... | Use this | Why |
| --- | --- | --- |
| one or more hostnames to HTTP backends | `services.domain_routes` | best default for websites and APIs |
| path/host-based HTTP without domain-first grouping | `services.reverse_proxy.routes` | lower-level HTTP route model |
| New API / sub2api / OpenAI-compatible traffic | `services.ai_proxy.routes` | native AI path rewrite and streaming-friendly behavior |
| static files | `services.static_sites` | built-in file serving and welcome page fallback |
| large downloads / CDN origin assets | `services.static_sites` + `services.response_policy.cache` | byte Range downloads, hot small-file cache, streaming large files |
| WebDAV | `services.webdav` | built-in authoring file surface |
| raw TCP | `tcp.listeners` | long-lived binary streams, games, tools, MQTT |
| raw UDP, KCP-style UDP, or QCP UDP traffic | `udp.listeners` | realtime datagram traffic with TTL and cap control |
| TLS SNI stream routing | `tcp.stream_routes` | Redis/MySQL/PostgreSQL/MongoDB-style passthrough |
| Consul / etcd / Nacos linkage | `services.service_discovery` | registry metadata that automation can map into HTTP/TCP/UDP upstream pools |

### Reverse proxy with cache, rate limit, and health

```yaml
load_balance:
  algorithm: weighted
  retries:
    enabled: true
    max_retries: 2
  active_health:
    enabled: true
    http_enabled: true
    tcp_enabled: true
    udp_enabled: false
    path: /healthz
    interval_secs: 10
    timeout_ms: 2000

services:
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /v1
      upstream: http://127.0.0.1:8080
      upstreams:
        - http://127.0.0.1:8080
        - http://127.0.0.1:8081
      upstream_weights:
        "http://127.0.0.1:8080": 1
        "http://127.0.0.1:8081": 3
      strip_prefix: true
      cache:
        enabled: true
        ttl_secs: 30
        stale_while_revalidate_secs: 15
      rate_limit:
        enabled: true
        algorithm: token_bucket
        requests: 120
        window_ms: 60000
        burst: 30
```

What this does in practice:

- gives you one hot route with a weighted upstream pool
- retries failed upstream attempts
- actively probes `/healthz`
- caches short-lived API responses
- rate-limits the edge before bad traffic reaches the app

For the broad production matrix, start from `examples/all-scenarios.example.yaml`. It covers static HTML/CSS/JS/image/font/audio/video assets, Range downloads, HTTP/1.1 and HTTP/2/gRPC reverse proxying, WebSocket, API gateway policy chains, ACME, WAF/anti-CC primitives, hotlink/crawler plugin hooks, TCP/UDP stream load balancing, Consul/etcd/Nacos discovery mappings, Kubernetes ingress-style service mapping, CDN origin routes, and IPv6 access rules.

### MQTT / IoT edge

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

How to read this example:

- plain MQTT is just TCP at the edge
- MQTT TLS passthrough is a stream route keyed by SNI
- MQTT over WebSocket stays on the HTTP/WebSocket surface
- CoAP-style device traffic stays on `udp.listeners`

### Game TCP, KCP-style UDP, and QCP UDP

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
    - name: game-qcp
      bind: 0.0.0.0:7002
      protocol: qcp
      session_ttl_secs: 180
      max_associations: 262144
      upstreams:
        - 127.0.0.1:9200
        - 127.0.0.1:9201
```

Important knobs:

- `nodelay: true` keeps latency-sensitive TCP from waiting on Nagle batching.
- `connect_timeout_ms` controls how long a new upstream dial may block.
- `session_ttl_secs` should be comfortably above your client heartbeat interval.
- `max_associations` protects the box from unbounded UDP churn.
- KCP and QCP are configured as independent UDP listeners. Use `protocol: kcp` for KCP-style traffic and `protocol: qcp` for neko233-com/QCP.
- `protocol: qcp` is transparent UDP forwarding; QCP framing and reliability stay in your upstream service.

### Built-in wildcard TLS with DNS-01

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

Use this when:

- you need a wildcard certificate
- you want DNS-01 inside the gateway instead of external `acme.sh`
- your provider is one of the built-in strategies: `cloudflare`, `aliyun_cn`, `aliyun_intl`, `tencent`, `volcengine`, `aws`, `azure`, `google`

## Commands You Will Actually Use

Inspect the effective config:

```bash
proxysss -config ./proxysss.yaml check-config
proxysss -config ./proxysss.yaml config explain
proxysss -config ./proxysss.yaml config routes
proxysss -config ./proxysss.yaml config reload-plan
proxysss -config ./proxysss.yaml config capabilities
proxysss -config ./proxysss.yaml config nginx-parity --format yaml
```

Validate the full scenario sample in Docker when you need Linux-grounded evidence:

```bash
scripts/verify-docker-scenarios.sh
```

On Windows PowerShell:

```powershell
.\scripts\verify-docker-scenarios.ps1
```

Manage the local automation token:

```bash
proxysss token show
proxysss token set
proxysss token set my-custom-cluster-token
```

Run the embedded TypeScript runtime:

```bash
proxysss script run-file ./examples/gateway.ts
proxysss script eval "console.log('proxysss ts runtime ok')"
```

## Admin API Example

The admin plane is loopback-first and write-disabled by default. Turn on write operations only when you intentionally want automation to mutate `proxysss.yaml`.

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  bearer_token: change-this-cluster-token
  enable_write_ops: true
  expose_config: false
  loopback_only: true
```

Example route upsert:

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/upsert \
  -H "Authorization: Bearer change-this-cluster-token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "node-17-api",
    "domains": ["api.example.com"],
    "path_prefix": "/",
    "upstream": "http://10.0.0.17:8080"
  }'
```

What happens after this call:

- the token is checked
- the named route is inserted or updated in the main YAML file
- the updated file is written back to disk
- hot-reloadable parts are reloaded in process

## Performance And Production Rules

Performance work in `proxysss` follows two rules:

- benchmark the path you changed
- prove you did not make sibling paths worse

That means a faster SSE path is not accepted if it makes static delivery, reverse proxy, WebSocket, TCP, or nginx-comparable UDP slower or less stable without explicit approval. KCP/QCP stay supported as listener capabilities, but they are not part of the current performance benchmark matrix.

Production validation flow:

```bash
proxysss tune linux --apply
scripts/benchmark-all-scenarios.sh
```

Default GitHub Actions CI is packaging-only: it builds and uploads the six release bundles, and no longer runs tests, smoke benchmarks, or performance gates automatically. Performance evidence is collected manually on Linux hosts or benchmark containers.

Current UDP fast-path evidence for v1.3.5:

- Docker Ubuntu 24 UDP-only official script path: `4.045x`
- `proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`
- errors: `0 / 0`

What that benchmark means:

- it is Linux-only release evidence
- it runs a mixed matrix, not a cherry-picked single test
- it compares nginx-comparable static, reverse proxy, generic SSE, WebSocket, game TCP, generic TCP, and UDP together
- New API provider routes and KCP/QCP special UDP encapsulations stay supported as product capabilities, but they are excluded from the current performance benchmark matrix
- it uses a fair default ratio floor instead of pretending every feature-rich gateway must win every micro-benchmark outright

## Docs Map

- [docs/configuration.html](docs/configuration.html) — human-facing configuration guide
- [docs/PRODUCTION-HARDENING.md](docs/PRODUCTION-HARDENING.md) — Linux tuning, benchmark gates, and production guardrails
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — text architecture walkthrough
- [docs/architecture.html](docs/architecture.html) — visual architecture lab
- [docs/AGENT-API.md](docs/AGENT-API.md) — admin API automation examples
- [docs/SECURITY.md](docs/SECURITY.md) — security defaults and hardening
- [docs/nginx-to-proxysss.html](docs/nginx-to-proxysss.html) — human-facing nginx migration guide
- [docs/caddy-to-proxysss.html](docs/caddy-to-proxysss.html) — human-facing Caddy migration guide
- [docs/ts-how-to-use.html](docs/ts-how-to-use.html) — human-facing embedded TypeScript guide
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) — machine-facing configuration cookbook
- [nginx-to-proxysss.md](nginx-to-proxysss.md) — machine-facing nginx migration notes
- [caddy-to-proxysss.md](caddy-to-proxysss.md) — machine-facing Caddy migration notes
- [ts-how-to-use.md](ts-how-to-use.md) — machine-facing embedded TypeScript runtime guide
- [proxysss-script.d.ts](proxysss-script.d.ts) — scripting types

Public docs site:

- [https://neko233-com.github.io/proxysss/](https://neko233-com.github.io/proxysss/)
- [https://neko233-com.github.io/proxysss/configuration.html](https://neko233-com.github.io/proxysss/configuration.html)
- [https://neko233-com.github.io/proxysss/architecture.html](https://neko233-com.github.io/proxysss/architecture.html)
- [https://neko233-com.github.io/proxysss/nginx-to-proxysss.html](https://neko233-com.github.io/proxysss/nginx-to-proxysss.html)
- [https://neko233-com.github.io/proxysss/caddy-to-proxysss.html](https://neko233-com.github.io/proxysss/caddy-to-proxysss.html)
- [https://neko233-com.github.io/proxysss/ts-how-to-use.html](https://neko233-com.github.io/proxysss/ts-how-to-use.html)

Runtime-built docs inside a live gateway:

- `http://localhost/docs`
- `http://localhost/docs.html`
