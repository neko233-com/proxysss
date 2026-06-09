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

## TLS / ACME

### Managed HTTPS (HTTP-01 / TLS-ALPN-01)

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/auto-https/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["example.com", "www.example.com"],
    "email": "admin@example.com",
    "production": true
  }'
```

### Wildcard certificate via acme.sh DNS-01

Wildcard certificates are **not** issued by built-in managed ACME. Use this non-default path only when DNS-01 is required (`*.example.com`).

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/wildcard-dns/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["example.com", "*.example.com"],
    "email": "admin@example.com",
    "dns_provider": "dns_cf",
    "credentials": {
      "CF_Token": "your-cloudflare-api-token"
    }
  }'
```

`dns_provider` is the `acme.sh --dns` provider name. Credentials are persisted into YAML and redacted from `proxysss config show`.

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
curl -X POST http://127.0.0.1:7777/v1/tcp-listeners/upsert \
  -u ops:change-me \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game",
    "bind": "0.0.0.0:7000",
    "upstream": "127.0.0.1:9000",
    "upstreams": ["127.0.0.1:9001"]
  }'
```

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
