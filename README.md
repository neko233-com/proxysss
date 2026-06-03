# proxysss

proxysss 是面向超大型游戏服务器集群 / 超大型聊天服务器集群的 Rust 顶级网关。

目标能力：

- L7 + L4 统一入口：HTTP/1.1、HTTP/2、HTTP/3、TCP、UDP
- 动态逻辑脚本：TypeScript/JavaScript（默认 Deno）
- 大规模连接场景：面向游戏登录/注册、长连接会话、实时 UDP 流
- 亲和路由：基于 playerId 的稳定上游绑定
- 热重载：配置热更新（通过校验后生效）
- 自动证书：自签/手工证书/外部 ACME 自动化
- 免配置自动 SSL：开关式启用，默认可直接运行
- 管理后台：内置 Admin Web，可关闭以提升压测纯性能
- 结构化日志：JSON 访问日志 + 慢请求观测 + 安全头脱敏

默认网关端口为 23380（HTTP/1.1 + HTTP/2 走 TCP，HTTP/3 走 UDP）。

## 1. 快速开始

### 1.1 初始化

运行：

proxysss init

初始化后会生成：

- proxysss.yaml
- gateway.ts
- plugins/player-affinity.ts
- certs/proxysss-cert.pem
- certs/proxysss-key.pem

### 1.2 配置检查

运行：

proxysss check-config --config ./proxysss.yaml

如果校验通过，会输出 passed；如果失败，会输出详细错误项。

### 1.3 启动网关

运行：

proxysss run --config ./proxysss.yaml

### 1.4 热重载

默认开启：

runtime.hot_reload.enabled: true
runtime.hot_reload.interval_ms: 1500

修改配置文件后，网关会自动检测并重载。重载前会执行完整校验。

注意：监听地址、监听器集合、TLS 模式、Admin 监听开关等基础拓扑变更需要重启。

## 2. 一键安装（参考 unicli 风格）

macOS / Linux：

curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash

Windows PowerShell：

irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex

Windows PowerShell（带参数，支持更新/升级/降级）：

& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action upgrade -Version v0.1.1
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action downgrade -Version v0.1.0

安装脚本会：

- 下载平台二进制
- 配置 PATH
- 自动安装 Deno（缺失时）
- 执行 proxysss init
- 执行 proxysss service install（默认开机自启动）

install.ps1 支持参数：

- `-Action install|update|upgrade|downgrade`
- `-Version latest|vX.Y.Z`
- `-AllowDowngrade`
- `-NoServiceRestart`
- `-SkipInit`
- `-DryRun`

## 3. 现代化配置（YAML 推荐）

推荐 YAML：可读性强、适合大型集群运维评审。

也支持 JSON（扩展名 .json 时按 JSON 优先解析，失败会回退 YAML）。

默认配置可输出：

proxysss print-default-config --format yaml
proxysss print-default-config --format json

### 3.1 核心配置示例

```yaml
config_version: 1
log_filter: info,proxysss=info

logging:
  format: json
  filter: info,proxysss=info
  access_log: true
  access_sample_rate: 1.0
  slow_request_ms: 300
  redact_headers:
    - authorization
    - cookie
    - set-cookie

http:
  plain_bind: ""
  tls_bind: 0.0.0.0:23380
  h3_bind: 0.0.0.0:23380
  request_timeout_ms: 15000
  allow_insecure_upstreams: false
  tls:
    mode: self_signed
    cert_path: certs/proxysss-cert.pem
    key_path: certs/proxysss-key.pem
    generate_self_signed_if_missing: true
    server_name: gateway.local
    acme:
      client: acme.sh
      email: ops@example.com
      domains:
        - gw.example.com
      cache_dir: certs/acme-cache
      challenge: tls_alpn_01
      directory_production: true
      renew_interval_hours: 12
      extra_args: []

tcp:
  listeners:
    - name: game-login
      bind: 0.0.0.0:26379

udp:
  listeners:
    - name: game-realtime
      bind: 0.0.0.0:2053

script:
  command: deno
  args:
    - run
    - -A
    - gateway.ts
  cwd: .
  timeout_ms: 500

plugins:
  enabled: true
  auto_load_dir: plugins
  extensions: [ts, js, mjs, cjs]
  allow_admin_manage: true

affinity:
  enabled: true
  sticky_ttl_secs: 3600
  fallback_to_remote_addr: true
  http:
    query_keys: [playerId, pid, uid]
    header_keys: [x-player-id, x-uid]
    cookie_keys: [playerId, pid]
  stream:
    probe_prefixes: ["playerId=", "pid=", "uid="]
    probe_delimiters: ["|", ";", ",", "\n", "\r", " "]
    peek_bytes: 256
    peek_timeout_ms: 5

admin:
  enabled: true
  bind: 127.0.0.1:23381
  username: root
  password: root
  expose_config: true
  enable_write_ops: true

load_balance:
  algorithm: rendezvous
  retries:
    enabled: true
    max_retries: 2
  passive_health:
    enabled: true
    fail_threshold: 3
    quarantine_secs: 15

runtime:
  hot_reload:
    enabled: true
    interval_ms: 1500
```

## 4. 自动 SSL 证书（类似 Caddy 的自动化思路）

TLS 支持三种模式：

- self_signed：自动自签（开发/内网）
- manual：使用现有 cert/key
- acme_external：调用外部 ACME 客户端自动签发/续期（默认 acme.sh）

开关式启用示例：

- 开发环境（免配置直接起）：tls.mode: self_signed
- 生产环境（自动证书）：tls.mode: acme_external
- 手工证书：tls.mode: manual

acme_external 的流程：

- 启动时若 cert/key 不存在，自动执行 ACME 签发
- 运行期按 renew_interval_hours 自动续期
- cert/key 文件落到配置路径（cert_path/key_path）

建议：

- 公网生产流量使用 acme_external + directory_production: true
- 确保 acme 客户端已安装且域名已解析到网关入口

## 5. playerId 亲和路由（HTTP/TCP/UDP）

### 5.1 HTTP

网关会按配置从以下来源提取 playerId：

- Query（playerId/pid/uid）
- Header（x-player-id/x-uid）
- Cookie（playerId/pid）

### 5.2 TCP/UDP

网关会在首包中按前缀提取，例如：

- playerId=12345
- pid=12345
- uid=12345

基于 playerId 做稳定路由：

- 上游列表由脚本返回（route.upstreams）
- 通过 rendezvous hashing 选择目标
- sticky_ttl_secs 控制粘性保持时间

这适用于：

- 游戏登录/注册落同一分区
- 聊天会话固定到同一会话节点
- 减少跨节点状态同步成本

## 6. 动态脚本（TS/JS）与插件化机制

默认脚本见 examples/gateway.ts。

默认网关脚本内置插件生命周期与 OpenResty 风格阶段：

- access（HTTP 请求进入时）
- balancer（HTTP 上游选择前）
- preread（TCP/UDP 首包预读）
- log（请求完成后观测）

插件目录默认为 plugins/，启动时会自动加载 .ts/.js/.mjs/.cjs。

可通过 Admin API 或 CLI 动态装载/卸载，无需重启网关：

- proxysss plugin list
- proxysss plugin load --name player-affinity --module-path ./plugins/player-affinity.ts
- proxysss plugin unload --name player-affinity

脚本入参会包含：

- kind（http/tcp/udp）
- listener
- ctx.request_id
- ctx.player_id
- HTTP: host/path/method/headers
- TCP/UDP: payload_len/first_packet_preview

脚本返回：

- upstream（单上游）
- upstreams（多上游，用于亲和）
- affinity_key（可覆盖网关提取的 playerId）
- rewrite_path/set_headers/strip_headers

## 7. 管理后台（可关闭）

默认开启，监听 admin.bind。

- GET /healthz
- GET /v1/stats（需鉴权）
- GET /v1/upstreams（需鉴权，查看上游健康与连接占用）
- GET /v1/config（需鉴权，可配置关闭）
- POST /v1/reload（需鉴权，可配置关闭）
- GET /v1/plugins（需鉴权）
- POST /v1/plugins/load（需鉴权，可配置关闭写操作）
- POST /v1/plugins/unload（需鉴权，可配置关闭写操作）

鉴权：HTTP Basic。

默认账号：

- username: root
- password: root

生产必须修改默认账号和密码，默认值会触发告警提示。

## 7.1 LVS / HAProxy 能力覆盖

当前已经落地并可用于生产调优的关键能力：

- 四层 + 七层统一转发（TCP/UDP/HTTP1.1/2/3）
- 多算法负载策略（rendezvous/source_hash/round_robin/least_connections）
- 被动健康检查（失败阈值自动摘除 + 隔离后自动恢复）
- 失败重试（可配置重试次数）
- 连接亲和（playerId 稳定路由）
- 管理面观测（stats/upstreams/plugin 管理）

说明：LVS 的内核态转发模型与本项目用户态实现路线不同，本项目提供的是在用户态网关内对等可配置能力（特别是调度、重试、健康摘除、观测与脚本扩展），用于覆盖超大规模业务接入场景。

压测建议关闭后台：

admin.enabled: false

## 8. 高性能压测

内置：

- proxysss bench http
- proxysss bench tcp
- proxysss bench udp

建议做 nginx 对标时保持变量一致：

- 相同机器与内核参数
- 相同 TLS 模式
- 相同上游服务与响应体
- 关闭 admin 与高频日志采样（access_sample_rate 可调）

示例：

proxysss demo http-echo --listen 127.0.0.1:8081
proxysss bench http --url https://127.0.0.1:23380/sdk/login?playerId=123 --concurrency 1024 --duration-secs 60 --insecure

proxysss demo tcp-echo --listen 127.0.0.1:7001
proxysss bench tcp --addr 127.0.0.1:26379 --connections 2000 --duration-secs 60 --payload-bytes 1024

proxysss demo udp-echo --listen 127.0.0.1:8101
proxysss bench udp --addr 127.0.0.1:2053 --connections 2000 --duration-secs 60 --payload-bytes 512

## 9. 日志体系（对比传统 nginx 日志痛点）

proxysss 默认提供结构化 JSON 日志：

- request_id 贯穿
- 上游目标、延迟、状态码、远端地址
- 慢请求标记
- 敏感头脱敏列表
- 采样控制（降低高并发日志开销）

这样可以显著改善传统文本 access log 在超大规模场景中的检索、关联、脱敏和成本问题。

## 10. 重要说明

当前实现已具备超大型网关骨架能力，但如果你要在生产达到并持续超过 nginx，需要继续做：

- io_uring / 内核参数 / NUMA 绑核调优
- 零拷贝与流式 body 透传优化
- 更细颗粒的背压、限流、熔断、健康检查
- 指标体系（Prometheus/OpenTelemetry）与压测回归流水线
- 更完善的 H3 专项压测与握手复用优化

## 11. CI/CD 与 Windows 一键命令

仓库内已包含：

- GitHub Actions：.github/workflows/ci.yml、.github/workflows/deploy.yml、.github/workflows/release.yml
- Windows 命令：run.cmd、test.cmd、build.cmd、deploy.cmd

Windows 快捷用法：

- run.cmd proxysss.yaml
- test.cmd
- build.cmd
- deploy.cmd proxysss.yaml
