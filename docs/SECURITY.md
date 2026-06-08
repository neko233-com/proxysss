# proxysss security guide

proxysss is designed as an **agent-native edge gateway** with secure defaults and explicit production hardening paths.

## Secure defaults

| Setting | Default | Purpose |
| --- | --- | --- |
| `admin.bind` | `127.0.0.1:7777` | Admin API is loopback-only |
| `admin.loopback_only` | `true` | Reject non-loopback admin clients when bind is local |
| `admin.enable_write_ops` | `false` | Mutations require explicit enablement |
| `admin.expose_config` | `false` | Full config export disabled by default |
| `admin.auth_rate_limit.enabled` | `true` | Brute-force protection on admin auth |
| `security.validate_admin_mutations` | `true` | Validate route/listener payloads from the admin API |
| `security.block_ssrf_targets` | `true` | Block metadata/private IPs in admin mutation upstreams |
| `security.reject_ambiguous_http1` | `true` | Reject ambiguous `Content-Length` + `Transfer-Encoding` |

Enable automation explicitly when you trust the admin network:

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  username: ops
  password: change-me
  bearer_token: long-random-cluster-token
  enable_write_ops: true
  expose_config: false
  loopback_only: true
  auth_rate_limit:
    enabled: true
    max_failures: 8
    window_secs: 300
    lockout_secs: 900
```

## Threat mitigations

### Path traversal

Static and WebDAV handlers reject `..` segments and unsafe encodings.

### SSRF via admin API

When agents register upstreams, `security.block_ssrf_targets` rejects:

- `127.0.0.0/8`, RFC1918, link-local, and `169.254.169.254`
- Hostnames in `security.blocked_upstream_hosts`

YAML-configured internal upstreams remain allowed; SSRF policy applies to **admin mutation payloads**.

### HTTP request smuggling

Ambiguous HTTP/1 requests with both `Content-Length` and `Transfer-Encoding` are rejected when `security.reject_ambiguous_http1=true`.

### Admin brute force

Failed basic/bearer auth attempts per client IP are counted. After `max_failures` inside `window_secs`, the client receives `429` until `lockout_secs` elapses.

### TLS

- Use `http.tls.mode: acme_managed` for public sites (HTTP-01 / TLS-ALPN-01).
- Use `http.tls.mode: acme_dns_external` with `acme.sh` for wildcard certificates.
- Avoid `self_signed` on the public internet.

### Atomic configuration writes

Admin mutations write via a temp file + rename so partial YAML is not left on disk if the process crashes mid-write. Failed reloads restore the previous file.

## Production checklist

1. Change `admin.username` / `admin.password` and set a unique `admin.bearer_token`.
2. Keep `admin.loopback_only: true` or bind admin to a private interface only.
3. Set `admin.enable_write_ops: true` only on nodes that run cluster automation.
4. Leave `admin.expose_config: false` unless config export is required.
5. Review `security.blocked_upstream_hosts` for your cloud metadata endpoints.
6. Scrape `/metrics` from an internal network; do not expose admin on `0.0.0.0` without a firewall.

## Reporting issues

Open security-related reports in the GitHub repository with reproduction steps and affected version.
