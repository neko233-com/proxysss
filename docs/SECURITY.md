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

### IP allow/deny and stream access control

HTTP clients are filtered by `services.access_control.http` (aliases: `whitelist`/`blacklist`). TCP/stream listeners honor `services.access_control.stream` globally and per `tcp.stream_routes[].access_control`.

### DDoS mitigation

```yaml
security:
  ddos:
    enabled: true
    max_connections: 50
    window_secs: 10
    ban_secs: 300
    burst: 20
  dynamic_blacklist:
    enabled: true
    path: runtime/dynamic-blacklist.json
```

When a client exceeds `max_connections + burst` inside `window_secs`, proxysss temporarily bans the IP for `ban_secs`. Metrics: `proxysss_blocked_requests_total`, `proxysss_ddos_bans_total`.

Agent API (requires `admin.enable_write_ops`):

- `GET /v1/security/blacklist`
- `POST /v1/security/blacklist/add` — body `{"ip":"203.0.113.5","ban_secs":3600}`
- `POST /v1/security/blacklist/remove` — body `{"ip":"203.0.113.5"}`

### MAC 黑名单的支持边界

`security.mac_deny` 从未接入实际数据路径，现统一拒绝非空配置，避免造成防护已生效的错觉。使用 `services.access_control` 的 IP/CIDR 策略；需要二层 MAC 过滤时在主机防火墙或网络设备上配置。

### TLS

- Use `http.tls.mode: acme_managed` for public sites (HTTP-01 / TLS-ALPN-01).
- Managed ACME defaults to ECDSA P-256; use `http.tls.acme.key_algorithm: rsa2048` only when legacy-client compatibility requires it.
- Use `http.tls.on_demand` for first-hit managed ACME with `allow` glob patterns and optional `ask_url` gate.
- Use built-in `http.tls.mode: acme_managed` + `challenge: dns01` for wildcard certificates (`manual` needs no API key). Legacy `acme_dns_external` + `acme.sh` is optional only for unsupported DNS vendors.
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

## CDN 回源、安全下载与幂等验证

静态源站令牌、签名、真实对端 ACL 在缓存之前校验。签名资源默认 no-store；默认隐藏点文件并拒绝越过根目录的真实路径。CDN 缓存命中时的访客授权和流量费用限制由边缘执行。

完整说明：[CDN 回源与安全下载](CDN-ORIGIN.md)；面向人的入口：[HTML 文档](cdn-origin.html)。使用 `static-sign --site cdn --path /assets/file.bin --ttl-secs 120` 签发短期 URL，密钥从 YAML 读取。

`test.cmd` 连续验证两轮；临时数据和最新报告放在 `.tmp/`，依赖放在 `.cache/`，编译缓存放在 `target/`，本地包固定为 `dist/proxysss-local.zip`，重复运行覆盖并清理 staging。


## 安全开关与跨系统性能适配

`proxysss config security` 输出安全开关、默认值和建议；CDN 的 enabled/origin_token_enabled/allowed_peers_enabled 与 FTP 各类策略支持独立停用并保留参数。`config performance` 探测 Windows IOCP、macOS kqueue、Linux epoll 与实际 socket 能力。Windows/macOS 保持现有调度并按系统适配 socket，Linux 保留独立数据运行时并继续发行版/CPU 自适应。runtime.performance 只在启动时应用，变更需重启。Docker 验证固定命名 `proxysss-verify`，前后清理同名项目容器，覆盖项目内报告。完整说明见 [安全与性能指南](SECURITY-PERFORMANCE.md)。
