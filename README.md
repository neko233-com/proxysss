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
proxysss update --version v1.2.2
```

proxysss is a high-performance load balancer and reverse proxy server built to replace nginx as a general-purpose edge gateway. It handles HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, FTP, WebDAV, and static delivery in one Rust binary while keeping the operational model straightforward.

Current version: v1.2.2

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
- TCP and UDP stream proxying
- FTP nginx module directive-level parity with control-channel proxying, passive and active data-channel rewriting, allow/deny policy, command/transfer hooks, and per-user policies
- WebDAV and static file serving
- First-class AI reverse proxy routes for New API, sub2api, and OpenAI-compatible upstreams through `services.ai_proxy`
- Managed ACME with HTTP-01 and TLS-ALPN-01, plus explicit acme.sh DNS-01 for wildcard certificates
- Shared cache zones with stale-while-revalidate, compression, access control, fixed-window/token-bucket/leaky-bucket HTTP and stream rate limiting, retries, and active health checks
- Prometheus metrics on `/metrics`, weighted load balancing, round-robin, least-connections, source-hash, and rendezvous affinity
- gRPC-over-HTTP/2, WebSocket, sticky sessions, passive quarantine (circuit breaker), and upstream failover retries

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

In that example:

- `example.com` has one backend machine.
- `neko233.store` reuses that same machine and adds one more backend.
- each domain route is its own service group with its own routing, health, cache, compression, and TLS policy.

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
- [docs/SECURITY.md](docs/SECURITY.md) — security defaults and hardening
- [docs/AGENT-API.md](docs/AGENT-API.md) — password/token agent automation API
- [examples/demo/README.md](examples/demo/README.md) — demo commands
- `ts-how-to-use.md`
- `nginx-to-proxysss.md`
- `proxysss-script.d.ts`
- `http://localhost/docs.html`
- `http://localhost/docs`
