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
      domains: [wss.example.com]

services:
  domain_routes:
    - name: game-wss
      domains: [wss.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
```

How to think about it:

- `auto_https.domains` 非空就自动启用内建 managed ACME。生产默认使用 TLS-ALPN-01，因此只需域名 A/AAAA 指向网关并开放 443，即可得到 `wss://`；不需要 `certbot`、`acme.sh`、DNS API 或账号邮箱。原有显式 `challenge: http01` 仍完整兼容（需开放 80）。
- The domain's public A/AAAA record must reach this host and ports 80 and 443 must be reachable. `email` is optional; adding it enables certificate-expiry/security notices.
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
bash scripts/benchmark-ubuntu24-amd64-docker.sh
STRICT_SUPERIORITY=1 PROXY_BIN=target/release/proxysss \
  scripts/benchmark-all-scenarios-isolated.sh
PROXY_BIN=target/release/proxysss \
  scripts/benchmark-production-scale-matrix.sh
```

`benchmark-ubuntu24-amd64-docker.sh` accepts either a native amd64 Docker daemon or a local arm64 daemon with `linux/amd64` emulation, but hard-checks the benchmark container as Ubuntu 24.04 x86_64 and derives concurrency from its CPU count. It builds the current checkout there, records `native-amd64` versus `emulated-amd64`, then runs strict 1x/2x/4x mixed waves. Every scale expands HTTP/HTTPS/static/SSE/WebSocket/TCP/UDP together, enables transparent QCP forwarding, takes three interleaved medians, requires zero errors, and retains raw evidence under `.benchmark/direct-ubuntu24-amd64/`. Emulated local results compare both gateways under the same cost but must not be presented as physical-x86 evidence. The role-isolated gate then pins a 4c gateway, backend, and clients to disjoint CPU sets/cgroups (the default allocation needs a 16-core Linux host), compares against nginx 1.31.2 mainline, and keeps three measurements separate: mixed aggregate saturation, per-scenario isolated saturation, and equal-offered-load p50/p95/p99. Memory is observed and reported by default; set a Docker or systemd memory limit only for a real declared production envelope. Mixed/equal-load phases default to four order-balanced samples; isolated saturation uses three. Metrics use medians while retaining the maximum observed error count. Fixed-load rows must complete at least 98% of the declared target; reports include both ratios and percentage improvements. The scale driver repeats the complete gate at 1x/2x/4x workload.

For physical-network WSS evidence, run the additional strict replay from an independent Linux client host. It stages one hashed proxysss binary to separate gateway and backend hosts, then uses a remote systemd cgroup to enforce `AllowedCPUs=0-3` and `LimitNOFILE=300000` for both nginx and proxysss. It records cgroup current/peak memory plus per-connection cost, host/`nginx -V` fingerprints and raw samples, and refuses equality for throughput or p50/p95/p99. `GATEWAY_MEMORY_MAX` is optional (default `infinity`); set it to `8G` only when that is the declared production envelope. Set the addresses reachable between roles; do not set `BUILD_NATIVE=1` unless all three hosts have compatible CPUs.

```bash
GATEWAY_HOST=gw-ssh BACKEND_HOST=be-ssh \
GATEWAY_ADDR=10.0.0.10 BACKEND_ADDR=10.0.0.20 \
  bash scripts/benchmark-cross-host-wss.sh
```

For the release-evidence 1x/2x/4x cross-host replay, keep the realistic 20k idle hold and scale only active WSS load by default:

```bash
GATEWAY_HOST=gw-ssh BACKEND_HOST=be-ssh \
GATEWAY_ADDR=10.0.0.10 BACKEND_ADDR=10.0.0.20 \
  bash scripts/benchmark-cross-host-scale-matrix.sh
```

When a stale Docker benchmark network occupies the default subnet, set an unused `/16` such as `BENCH_SUBNET=172.31.0.0/16`; the WSS scripts derive every role address from it. A restricted Linux controller may use a trusted, same-architecture Go-native helper with `PREBUILT_BENCH_HELPER=/opt/benchmark-helper`; build it from this repository with `GOOS=linux GOARCH=amd64 go build -o /opt/benchmark-helper scripts/benchmark-helper.go`.

Default GitHub Actions CI is packaging-only: it builds and uploads the six release bundles. Tests, smoke benchmarks, and performance gates must not run in GitHub Actions, including manual workflows; performance evidence is collected by running local Ubuntu 24 x86_64 Docker containers or dedicated Linux hosts directly. A release tag additionally requires `performance-evidence/vX.Y.Z.json`: strict 1x/2x/4x role-isolated plus cross-host evidence, raw-artifact hashes, role fingerprints, and memory observations. The v2 manifest records direct per-scenario ops/s, p50/p95/p99, errors, WSS capacity metrics, and proxysss/nginx current/peak/per-connection memory; the release validator rejects a missing scenario, non-zero error, equality/regression in any metric, proxysss memory above 2x nginx, or a synthetic 100k capacity claim. The release workflow validates and publishes that manifest with the assets.

Current UDP fast-path evidence for v1.3.5:

- Docker Ubuntu 24 UDP-only official script path: `4.045x`
- `proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`
- errors: `0 / 0`

What that benchmark means:

- it is Linux-only release evidence
- it is a UDP-only diagnostic from the official Go helper path, not proof that the full mixed matrix passed
- the release gate separately compares nginx-comparable static, reverse proxy, generic SSE, WebSocket, game TCP, generic TCP, and UDP together
- New API provider routes and KCP/QCP special UDP encapsulations stay supported as product capabilities, but they are excluded from the current performance benchmark matrix
- it uses a fair default ratio floor instead of pretending every feature-rich gateway must win every micro-benchmark outright

For WebSocket capacity, distinguish active-message `ops/s` from concurrent connections. The native hold mode opens and keeps sockets without converting every connection into an echo-rate workload:

```bash
proxysss bench websocket --url ws://gateway.example.com/gateway/ws \
  --connections 20000 --hold-connections --connect-workers 128 \
  --connect-timeout-ms 10000 --connect-retries 4 --duration-secs 30
```

The production gate defaults to 20k idle WSS connections and up to 4096 active message connections on a 4c gateway. It records resource use and only applies a Docker/systemd memory limit when the operator declares one. If you intentionally raise the capacity target above one source address's ephemeral-port range, use multiple client source IPs and multiple backend `IP:port` tuples (or a proxy source-IP pool); that four-tuple limit is a TCP constraint rather than a gateway implementation detail.

For a production-style WSS gateway comparison, run `bash scripts/benchmark-websocket-production-gate.sh` on a 16-core-or-larger Linux Docker host with enough RAM for four backends and multiple client containers. It compares nginx and proxysss with the same active WSS workload (ops/s, p50/p95/p99) and a 20k idle-connection hold test, with repeated interleaved runs and median gates. The Docker roles are cgroup/network-namespace isolated; resource snapshots are mandatory, while Docker memory limits are opt-in declared budgets. Use `benchmark-cross-host-wss.sh` before making a physical-network latency claim.

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
