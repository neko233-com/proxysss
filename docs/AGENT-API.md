# proxysss agent API

Agents configure proxysss through **password or bearer-token authenticated** HTTP calls to the admin API (`127.0.0.1:7777` by default). Every mutation validates input, writes atomically to the single YAML file, and reloads in process.

## Authentication

**Basic auth** (username + password from `admin` config):

```bash
curl -u ops:change-me http://127.0.0.1:7777/v1/stats
```

**Bearer token** (recommended for automation):

```bash
curl -H "Authorization: Bearer long-random-cluster-token" \
  http://127.0.0.1:7777/v1/stats
```

Enable writes in config before using mutation endpoints:

```yaml
admin:
  enable_write_ops: true
  bearer_token: long-random-cluster-token
```

## Reverse proxy routes

### Upsert domain route

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "api",
    "domains": ["api.example.com"],
    "path_prefix": "/",
    "upstream": "http://10.0.0.12:8080",
    "upstreams": ["http://10.0.0.13:8080"],
    "upstream_weights": {
      "http://10.0.0.12:8080": 1,
      "http://10.0.0.13:8080": 3
    },
    "strip_prefix": false
  }'
```

### Delete domain route

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"name": "api"}'
```

### Upsert path-based reverse proxy route

```bash
curl -X POST http://127.0.0.1:7777/v1/reverse-proxy-routes/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "internal-api",
    "hosts": ["internal.local"],
    "path_prefix": "/v1",
    "upstream": "http://127.0.0.1:9000",
    "strip_prefix": true
  }'
```

### Delete path-based route

```bash
curl -X POST http://127.0.0.1:7777/v1/reverse-proxy-routes/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"name": "internal-api"}'
```

### Full JSON fields (agents)

The admin console forms cover common fields only. Agents should POST the full `DomainRouteConfig` / `ReverseProxyRouteConfig` JSON accepted by the Rust config schema, including:

**Domain route** (`POST /v1/domain-routes/upsert`): `name`, `domains`, `path_prefix`, `upstream`, `upstreams`, `upstream_weights`, `strip_prefix`, `set_headers`, `strip_headers`, `compression`, `cache`, `rate_limit`, `active_health`, `ssl` (`mode`: `inherit|disabled|auto|manual`, `cert_path`, `key_path`, `email`).

**Reverse proxy route** (`POST /v1/reverse-proxy-routes/upsert`): `name`, `hosts`, `path_prefix`, `upstream`, `upstreams`, `upstream_weights`, `strip_prefix`, `set_headers`, `strip_headers`, `compression`, `cache`, `rate_limit`.

Inspect defaults with `proxysss config explain` and mirror shapes from generated `proxysss.yaml` templates.

## TLS / ACME

Agent workflow:

1. **Bootstrap** (first certificate): use loopback admin `http://127.0.0.1:7777/v1/tls/*` while `admin.loopback_only: true`.
2. **Automation** (after cert material exists): enable `admin.https.enabled` and call the same `/v1/*` endpoints over **HTTPS** on the main gateway listener.

```yaml
admin:
  enable_write_ops: true
  bearer_token: long-random-cluster-token
  https:
    enabled: true
    path_prefix: /_proxysss/admin
    hosts: ["ops.example.com"]   # optional; empty = any TLS host
```

```bash
# Inspect TLS + HTTPS admin base path
curl https://ops.example.com/_proxysss/admin/v1/tls/summary \
  -H "Authorization: Bearer long-random-cluster-token"

# After TLS is ready: upsert routes / ACME / FileCloud over HTTPS
curl -X POST https://ops.example.com/_proxysss/admin/v1/domain-routes/upsert \
  -H "Authorization: Bearer long-random-cluster-token" \
  -H "Content-Type: application/json" \
  -d '{"name":"api","domains":["api.example.com"],"upstream":"http://127.0.0.1:8080","path_prefix":"/"}'
```

Plain HTTP to `admin.https.path_prefix` is rejected. HTTPS write operations require existing certificate/key material (`GET /v1/tls/summary` → `https_api.tls_ready: true`).

### Managed HTTPS（只给域名；HTTP-01 / TLS-ALPN-01）

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/auto-https/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["wss.example.com"]
  }'
```

`email` 与 `production` 都可省略：API 默认使用 Let's Encrypt 正式环境的 HTTP-01；不填邮箱不会阻塞签发或续期，但不会收到到期/安全通知。域名 A/AAAA 必须指向该网关，80/443 必须能从公网访问。需要 DNS-01 泛域名时仍使用下一节的接口。

### Wildcard certificate via built-in DNS-01 (no external ACME client)

Built-in managed ACME supports wildcard certificates through embedded DNS-01 providers. Configure from the admin console (`http://127.0.0.1:7777/` → **TLS / ACME**) or via API:

```bash
curl http://127.0.0.1:7777/v1/tls/summary -u ops:change-me
curl http://127.0.0.1:7777/v1/tls/dns-providers -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/tls/wildcard-dns/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["example.com", "*.example.com"],
    "email": "admin@example.com",
    "dns_provider": "cloudflare",
    "credentials": {
      "api_token": "your-cloudflare-api-token"
    }
  }'
```

Built-in providers: `cloudflare`, `aliyun_cn`, `aliyun_intl`, `tencent`, `volcengine`, `aws`, `azure`, `google`, `manual`.

- `manual` needs **no API key** — proxysss prints the TXT record and polls public DNS until propagation completes.
- Legacy `acme_dns_external` + `acme.sh` remains only for providers not implemented natively.

Credentials are persisted into YAML and redacted from `proxysss config show`.

### On-demand TLS (first-hit issuance)

Requires `http.tls.mode: acme_managed`. Configure allow globs from the admin console or API:

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/on-demand/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "allow": ["*.example.com", "edge.example.com"]
  }'
```

Trigger immediate managed certificate issuance (uses current auto-HTTPS / ACME domain list):

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/issue-now \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{}'
```

### SNI manual certificates (`http.tls.certificates`)

List, upsert PEM inline, or reference existing cert/key files on disk:

```bash
curl http://127.0.0.1:7777/v1/tls/sni-certificates -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/tls/sni-certificates/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["api.example.com", "*.internal.example.com"],
    "cert_pem": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n",
    "key_pem": "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n"
  }'

curl -X POST http://127.0.0.1:7777/v1/tls/sni-certificates/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"domain": "api.example.com"}'
```

Path-based upsert (files must already exist):

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/sni-certificates/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["legacy.example.com"],
    "cert_path": "./certs/legacy.crt",
    "key_path": "./certs/legacy.key"
  }'
```

### Domain routes (admin UI + API)

```bash
curl http://127.0.0.1:7777/v1/domain-routes -u ops:change-me
```

Use the **Domain Routes** tab in the admin console or `POST /v1/domain-routes/upsert` (requires `admin.enable_write_ops=true`).

### List path-based reverse proxy routes

```bash
curl http://127.0.0.1:7777/v1/reverse-proxy-routes -u ops:change-me
```

## FileCloud (proxysss-exclusive shared folder)

Not nginx WebDAV — password-protected UI with CRUD, drag upload, search, and CDN-friendly `/dl/*` downloads confined to `services.filecloud.root`.

```bash
curl http://127.0.0.1:7777/v1/filecloud/summary -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/filecloud/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "path_prefix": "/filecloud",
    "root": "./filecloud-data",
    "password": "change-me",
    "title": "Shared Files",
    "allow_upload": true,
    "allow_delete": true,
    "allow_mkdir": true,
    "allow_move": true,
    "max_upload_bytes": 536870912,
    "cdn_cache_secs": 86400
  }'
```

Omit `password` or send an empty string to keep the existing password. Password is redacted from `GET /v1/config`.

## Domain stream routes (Redis, MySQL, etc.)

```bash
curl -X POST http://127.0.0.1:7777/v1/stream-routes/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "redis-prod",
    "domains": ["redis.example.com"],
    "listen": "6379",
    "upstream": "redis.internal:6379",
    "protocol": "redis"
  }'
```

Routes are persisted under `tcp.stream_routes` in the main YAML file and hot-reloaded.

```bash
curl http://127.0.0.1:7777/v1/stream-routes -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/stream-routes/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"name": "redis-prod"}'
```

## Dynamic IP blacklist

```bash
curl http://127.0.0.1:7777/v1/security/blacklist -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/security/blacklist/add \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"ip":"203.0.113.5","ban_secs":3600}'

curl -X POST http://127.0.0.1:7777/v1/security/blacklist/remove \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"ip":"203.0.113.5"}'
```

## Stream listeners

```bash
curl http://127.0.0.1:7777/v1/tcp-listeners -u ops:change-me
curl http://127.0.0.1:7777/v1/udp-listeners -u ops:change-me

curl -X POST http://127.0.0.1:7777/v1/tcp-listeners/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game",
    "bind": "0.0.0.0:7000",
    "protocol": "game_tcp",
    "nodelay": true,
    "connect_timeout_ms": 3000,
    "upstream": "127.0.0.1:9000",
    "upstreams": ["127.0.0.1:9001"]
  }'

curl -X POST http://127.0.0.1:7777/v1/udp-listeners/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "voice-kcp",
    "bind": "0.0.0.0:7001",
    "protocol": "kcp",
    "session_ttl_secs": 180,
    "max_associations": 262144,
    "upstream": "127.0.0.1:9001"
  }'

curl -X POST http://127.0.0.1:7777/v1/tcp-listeners/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"name": "game"}'

curl -X POST http://127.0.0.1:7777/v1/udp-listeners/delete \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{"name": "voice-kcp"}'
```

TCP listener upserts accept `protocol`, `nodelay`, and `connect_timeout_ms`. UDP listener upserts accept `protocol`, `session_ttl_secs`, and `max_associations` so automation can provision game/KCP/voice fleets without custom scripts.

## Production health surfaces

Use `/metrics` plus `/v1/upstreams` to verify release health after automation:

```bash
curl http://127.0.0.1:7777/metrics -u ops:change-me
curl http://127.0.0.1:7777/v1/upstreams -u ops:change-me
```

Watch `proxysss_critical_task_failures_total`, `proxysss_watchdog_heartbeat_total`, active health status, and manual drain state before promoting a node.

## Kubernetes ingress-style mappings (config-gated)

Enable in `proxysss.yaml`:

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

On load/reload, proxysss expands each mapping into a `services.domain_routes` entry targeting `http://{service}.{namespace}.svc.{cluster_domain}:{port}`.

## Observability

| Endpoint | Method | Auth | Description |
| --- | --- | --- | --- |
| `/v1/stats` | GET | yes | JSON counters |
| `/v1/upstreams` | GET | yes | Health + drain state |
| `/v1/config` | GET | yes | Redacted config (`expose_config` required) |
| `/v1/reload` | POST | yes | Reload from disk |
| `/metrics` | GET | no | Prometheus text on public HTTP listener |

## CLI helpers for agents

```bash
proxysss token show
proxysss token set my-cluster-token
proxysss config routes
proxysss config reload-plan
proxysss config capabilities
```
