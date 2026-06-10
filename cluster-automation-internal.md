# Cluster Automation Internal Guide

This document is for operators and agents that need to take over cluster routing without manually editing `proxysss.yaml` on every node.

## Scope

### Loopback bootstrap (first TLS / ACME)

Use `http://127.0.0.1:7777/v1/*` while `admin.loopback_only: true`:

- TLS / ACME: `/v1/tls/auto-https/upsert`, `/v1/tls/wildcard-dns/upsert`, `/v1/tls/on-demand/upsert`, `/v1/tls/issue-now`
- SNI certs: `/v1/tls/sni-certificates/*`

### HTTPS automation (after cert material exists)

Enable `admin.https.enabled` and call the same endpoints on the main gateway TLS listener:

`https://<host>/_proxysss/admin/v1/*`

Plain HTTP to the public admin path is rejected. HTTPS writes require existing cert/key material.

### Core route/listener endpoints

- `POST /v1/domain-routes/upsert` · `POST /v1/domain-routes/delete`
- `POST /v1/reverse-proxy-routes/upsert` · `POST /v1/reverse-proxy-routes/delete`
- `POST /v1/tcp-listeners/upsert` · `POST /v1/tcp-listeners/delete`
- `POST /v1/udp-listeners/upsert` · `POST /v1/udp-listeners/delete`
- `POST /v1/stream-routes/upsert` · `POST /v1/stream-routes/delete`
- `GET/POST /v1/security/blacklist/*`
- `POST /v1/filecloud/upsert` · `GET /v1/filecloud/summary`

All mutation endpoints:

- authenticate with `Authorization: Bearer <token>` (or Basic auth)
- write back into the main `proxysss.yaml`
- trigger in-process reload on success
- restore the original YAML if reload fails

Local token operations:

- `proxysss token show` prints the effective local token for the selected config
- `proxysss token set` resets the token to the default local value
- `proxysss token set <value>` rotates the token in the YAML file
- normal config display surfaces redact the token instead of printing it

## Admin token setup

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  bearer_token: replace-with-a-cluster-secret
  enable_write_ops: true
  loopback_only: true
  https:
    enabled: true          # after initial TLS bootstrap
    path_prefix: /_proxysss/admin
    hosts: []              # optional Host allowlist
```

## Domain route upsert

Use this for domain-first HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, and WSS traffic.

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "api-edge",
    "domains": ["api.example.com"],
    "path_prefix": "/",
    "upstream": "http://10.0.0.17:8080",
    "upstreams": ["http://10.0.0.18:8080"],
    "strip_prefix": false
  }'
```

Delete:

```bash
curl -X POST http://127.0.0.1:7777/v1/domain-routes/delete \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{"name": "api-edge"}'
```

## Reverse proxy route upsert

```bash
curl -X POST http://127.0.0.1:7777/v1/reverse-proxy-routes/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "admin-api",
    "hosts": ["edge.example.com"],
    "path_prefix": "/admin",
    "upstream": "http://10.0.0.19:9000",
    "strip_prefix": true
  }'
```

## Wildcard TLS (built-in DNS-01)

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/wildcard-dns/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["example.com", "*.example.com"],
    "email": "admin@example.com",
    "dns_provider": "cloudflare",
    "credentials": {"api_token": "..."}
  }'
```

Use `dns_provider: manual` when no cloud API key is available.

## SNI manual certificate upsert

```bash
curl -X POST http://127.0.0.1:7777/v1/tls/sni-certificates/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "domains": ["legacy.example.com"],
    "cert_pem": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n",
    "key_pem": "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n"
  }'
```

## TCP / UDP listener upsert

```bash
curl -X POST http://127.0.0.1:7777/v1/tcp-listeners/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game-tcp",
    "bind": "0.0.0.0:7000",
    "upstream": "10.0.0.17:7000"
  }'
```

```bash
curl -X POST http://127.0.0.1:7777/v1/udp-listeners/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game-udp",
    "bind": "0.0.0.0:7001",
    "upstream": "10.0.0.17:7001"
  }'
```

## Automation model

Recommended pattern for agents:

1. Bootstrap TLS/ACME on loopback admin.
2. Enable `admin.https` and switch automation traffic to HTTPS.
3. Discover node/service health outside the gateway.
4. Upsert the desired route, listener, or certificate by stable name/domain.
5. Treat `proxysss.yaml` as the persisted source of truth after each successful call.

## Protocol mapping

- HTTP/HTTPS/HTTP2/HTTP3/WebSocket/WSS: `domain-routes` or `reverse-proxy-routes`
- TCP: `tcp-listeners` · UDP: `udp-listeners` · TLS SNI stream: `stream-routes`
- FTP / static sites / WebDAV / FileCloud: YAML or dedicated admin endpoints (`/v1/filecloud/*`)

## Remaining gaps

- Bulk transaction / compare-and-swap semantics for multi-resource updates
- Admin HTTP API for static sites, WebDAV, FTP, and AI proxy route blocks (YAML-only today)
- Bearer token rotation via HTTP (`proxysss token set` CLI only)

See [docs/AGENT-API.md](docs/AGENT-API.md) for the full endpoint catalog.
