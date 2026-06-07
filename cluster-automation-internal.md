# Cluster Automation Internal Guide

This document is for operators and agents that need to take over cluster routing without manually editing `proxysss.yaml` on every node.

## Scope

These endpoints are intended for internal automation only.

- `POST /v1/domain-routes/upsert`
- `POST /v1/reverse-proxy-routes/upsert`
- `POST /v1/tcp-listeners/upsert`
- `POST /v1/udp-listeners/upsert`

All of them:

- authenticate with `Authorization: Bearer <token>`
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

## Reverse proxy route upsert

Use this for host/path matcher-driven HTTP routing when the domain-first model is not enough.

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

## TCP listener upsert

```bash
curl -X POST http://127.0.0.1:7777/v1/tcp-listeners/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game-tcp",
    "bind": "0.0.0.0:7000",
    "upstream": "10.0.0.17:7000",
    "upstreams": ["10.0.0.18:7000"]
  }'
```

## UDP listener upsert

```bash
curl -X POST http://127.0.0.1:7777/v1/udp-listeners/upsert \
  -H "Authorization: Bearer replace-with-a-cluster-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "game-udp",
    "bind": "0.0.0.0:7001",
    "upstreams": ["10.0.0.17:7001", "10.0.0.18:7001"]
  }'
```

## Automation model

Recommended pattern for agents:

1. Discover node/service health outside the gateway.
2. Upsert the desired route or listener by name.
3. Re-run the same upsert when backends or pools change.
4. Keep `name` stable so updates replace instead of append.
5. Treat `proxysss.yaml` as the persisted source of truth after each successful call.

## Protocol mapping

- HTTP/HTTPS/HTTP2/HTTP3/WebSocket/WSS: `domain-routes` or `reverse-proxy-routes`
- TCP: `tcp-listeners`
- UDP: `udp-listeners`
- FTP: still managed through the YAML service block
- Static sites and WebDAV: still managed through the YAML service blocks

## Current gap notes

- Wildcard certificate automation through DNS provider APIs is not implemented in this document's API surface yet.
- Route deletion and listener deletion endpoints are not implemented yet.
- Bulk transaction/compare-and-swap semantics are not implemented yet.