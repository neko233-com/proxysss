# proxysss 安全指南

proxysss 是可扩展的通用网关。核心负责协议边界、访问策略、限流、认证和安全下载；业务身份、Referer/User-Agent 与 bot-score 判断由应用或可选脚本承担。面向人的入口是 [安全与系统适配 HTML](security-performance.html)，详细字段以 [配置指南](CONFIGURATION.md) 为准。

## 安全默认值

| 配置 | 默认值 | 作用 |
| --- | --- | --- |
| `admin.bind` | `127.0.0.1:7777` | 管理接口默认在 loopback |
| `admin.loopback_only` | `true` | 本地绑定拒绝非 loopback 客户端 |
| `admin.enable_write_ops` | `false` | 自动化写入须显式开启 |
| `admin.expose_config` | `false` | 默认不导出完整配置 |
| `admin.auth_rate_limit.enabled` | `true` | 限制管理登录失败尝试 |
| `admin.monitor.enabled` | `false` | 移动端/桌面监控独立启用 |
| `admin.monitor.password` | 空 | 启用时使用至少 12 字符的独立密码 |
| `admin.monitor.path_prefix` | `/_proxysss/monitor` | 仅通过 HTTPS 提供只读监控 |
| `security.validate_admin_mutations` | `true` | 校验管理路由/listener 更新 |
| `security.block_ssrf_targets` | `true` | 限制管理更新指定的上游目标 |
| `security.reject_ambiguous_http1` | `true` | 拒绝歧义 HTTP/1 消息边界 |

可信自动化可显式开启写入；示例中的密码和令牌必须替换：

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

## 管理、监控和上游目标

管理 Basic/bearer 登录失败按客户端 IP 计数，在 `window_secs` 内达到 `max_failures` 后返回 429，直至锁定到期。并发失败不会清除或延长已经生效的锁定；过期失败窗口由独立控制任务清理。

`admin.monitor` 使用独立密码和短期 `monitor:read` 会话，登录后仅允许 `GET /v1/health`、`summary`、`stats`、`ssl`。这些路径位于配置的 monitor 前缀下，由主 HTTPS listener 提供；管理员 bearer token 不能代替监控会话。SSL 摘要不包含证书/密钥路径和 DNS 凭据，写入返回 `monitor_read_only`。移动端凭据应放在系统安全存储中，避免进入日志、URL 或版本库。

管理路由更新会检查上游主机/IP：IPv4 loopback、RFC1918、link-local、元数据地址，IPv6 loopback/未指定地址，以及配置的 blocked host/CIDR 会被对应策略拒绝。该检查面向管理更新输入，不查询 DNS，也不能当作 DNS 重绑定防护；可信 YAML/脚本仍可指向内网上游。完整目标语法和边界见配置指南。

## HTTP、文件和 IP 策略

HTTP/1 消息边界校验拒绝冲突长度、TE/CL 共存和不支持的编码。普通代理、SSE、WebSocket 非升级响应都要保持正文边界；截断必须失败，不能把下一条响应混入当前正文。

静态、WebDAV 和 FileCloud 检查目录边界及真实路径，CDN 令牌、签名和对端 ACL 在缓存/Range 处理之前执行。可信运维应以原子替换方式发布文件，避免不可信本地进程并发修改目录链接；路径校验不是目录描述符沙箱。下载语义、取消和资源上限见 [CDN 回源与安全下载](CDN-ORIGIN.md) / [HTML](cdn-origin.html)。

`services.access_control.http` 控制 HTTP，`services.access_control.stream` 和 stream route 策略控制 TCP/UDP。新 TCP 连接和 UDP association 共用连接准入，既有 UDP 数据报不重复扣连接配额。IPv4/IPv6 CIDR 和 mapped IPv4 的匹配语义由真实准入回归保护。

`security.mac_deny` 未接入数据路径，非空配置会被拒绝。网关使用 IP/CIDR；二层 MAC 过滤应由主机或网络设备承担。

## DDoS 与动态黑名单

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

窗口内超过 `max_connections + burst` 时临时封禁 IP；指标包括 `proxysss_blocked_requests_total` 和 `proxysss_ddos_bans_total`。独立控制任务每秒回收失效窗口/封禁，不依赖热重载或健康探测开关；活动状态保留。TTL 按单调时间计算，超大秒数不会导致未来 Instant 溢出。

令牌鉴权的管理接口：

- `GET /v1/security/blacklist`：当前有效 IP 列表。
- `POST /v1/security/blacklist/add`：`{"ip":"203.0.113.5","ban_secs":3600}`。
- `POST /v1/security/blacklist/remove`：`{"ip":"203.0.113.5"}`。

两个 POST 还要求 `admin.enable_write_ops=true`。开启持久化时先完整同步同目录临时文件并原子替换，再更新内存/DDoS；成功返回 `persisted=true`，写失败返回 500 并保留原内存状态。关闭持久化仍可更新内存，返回 `persisted=false`。单 writer 排队不阻塞普通连接检查；已经交给 worker 的请求在客户端断开后继续完成，重试前可查询列表。

版本 1 快照保存原始 UTC 秒/纳秒和 TTL，重启扣除停机时间，已到期封禁不复活，重复保存不延长期限。缺失文件可从空列表启动；启用持久化时损坏、不可读、未知版本和重复条目都会报错。旧纯 IP 数组缺少期限，须明确时间/TTL 后迁移。每次管理更新保存采集到的活动列表；此后新增的自动封禁在下一次管理快照前只保存在内存。热更新开关/路径影响后续保存，外部文件在启动时读取。格式见 [配置指南](CONFIGURATION.md) / [HTML](configuration.html#blacklist-persistence)。

## TLS 与自动化

公网通常使用 `http.tls.mode: acme_managed`，默认 TLS-ALPN-01 和 ECDSA P-256；HTTP-01 可显式选择，旧客户端需要时可选择 `rsa2048`。通配符使用内建 DNS-01 provider，未内建厂商才使用外部 acme.sh 模式。按需签发由 `http.tls.on_demand` 的 allow/ask_url 控制；self_signed 适合开发或内部环境。

路由/listener 管理更新采用临时文件和 rename 写 YAML，重载失败会尝试恢复原文件。各管理操作的并发、失败和生效边界以相应 API 文档为准，不能把文件替换等同于所有网关状态的事务。

## 运维与验证

替换默认管理凭据，按需启用写入/配置导出；远程管理使用受控网络和 HTTPS。监控使用独立密码及 hosts 范围，指标采集放在受控网络。安全问题报告应给出受影响版本、最小复现与脱敏日志，通过仓库报告渠道提交。

`proxysss config security` 检查安全开关，`config performance` 检查系统/socket 能力，不执行主机修改。`runtime.performance` 在启动时应用，改变需重启。先完成 Ubuntu 24 Docker 全功能和完整混合性能验证，再做本机 Windows 快速 `cargo test --locked`；所有门槛通过后才发布。工件和缓存按 [验证流程](VALIDATION.md) 保留，单项性能结果不能冒充完整场景通过。
