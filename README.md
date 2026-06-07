# proxysss

proxysss is a high-performance load balancer and reverse proxy server built to replace nginx as a general-purpose edge gateway. It handles HTTP, HTTPS, HTTP/2, HTTP/3, WebSocket, TCP, UDP, FTP, WebDAV, and static delivery in one Rust binary while keeping the operational model straightforward.

Current version: v0.3.10

## Why proxysss

- One runtime config file: keep gateway settings in a single YAML file, usually `proxysss.yaml`.
- Explicit config path support: use `-config`, `--config`, or `-c` to point at a different YAML path.
- YAML-only gateway config: JSON config files are intentionally unsupported.
- Domain-first reverse proxying: `services.domain_routes` is the primary grouping unit for multi-domain HTTP services.
- Built-in control plane: admin API and dashboard on `127.0.0.1:7777` by default.
- Hot reload: the main YAML config, the main script, and auto-loaded plugins participate in reload fingerprinting.
- Optional scripting: TypeScript plugins are for custom business logic, not for ordinary gateway setup.

## Supported gateway surface

- HTTP/1.1, HTTPS, HTTP/2, HTTP/3, and WebSocket
- TCP and UDP stream proxying
- FTP control-channel proxying with passive and active data-channel rewriting
- WebDAV and static file serving
- Managed ACME with HTTP-01 and TLS-ALPN-01
- Shared cache zones, compression, access control, rate limiting, retries, and active health checks

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

In that example:

- `example.com` has one backend machine.
- `neko233.store` reuses that same machine and adds one more backend.
- each domain route is its own service group with its own routing, health, cache, compression, and TLS policy.

## Automatic HTTPS

Automatic certificate issuance and renewal are built in.

- Challenge types: HTTP-01 and TLS-ALPN-01
- No external ACME binary is required for the managed path
- Domain-level `ssl.type: auto` and global `http.tls.auto_https` both expand into the managed ACME flow

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

Check the embedded TypeScript runtime:

```bash
proxysss script run-file ./examples/gateway.ts
proxysss script eval "console.log('proxysss ts runtime ok')"
```

## Plugin sidecar metadata

If you use auto-loaded plugins, sidecar metadata is YAML-only as well.

- `plugins/<name>.plugin.yaml`
- `plugins/<name>.plugin.yml`

## Installation

Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

Upgrade to a specific version:

```bash
proxysss update --version v0.3.10
```

## Operational defaults

- Admin bind: `127.0.0.1:7777`
- Default admin credentials: `root / root`
- Access log: `logs/access.log`
- Error log: `logs/error.log`

Change the default admin credentials before production use.

## Related docs

- `ts-how-to-use.md`
- `nginx-to-proxysss.md`
- `proxysss-script.d.ts`
- `http://localhost/docs.html`
- `http://localhost/docs`