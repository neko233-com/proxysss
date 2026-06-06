# proxysss

proxysss 是一个 nginx 同级的通用 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、WebSocket、TCP、UDP、FTP、WebDAV、静态文件和反向代理入口职责。

产品目标：作为 nginx 同级通用网关完整替代 nginx 的常见入口职责，同时提供更适合人类和 agent 接管的配置、查询和输出体验。默认 HTTP 端口与 nginx 一致为 80，首页是更友好的 `Welcome to proxysss`。proxysss 不是“业务网关优先”的产品；脚本/插件扩展层类似 nginx + Lua，用来承载可选的业务逻辑。

当前版本：v0.3.1

## 核心能力

- 多协议统一入口：HTTP/1.1、HTTP/2（TCP）+ HTTP/3（UDP）+ TCP + UDP
- 声明式反向代理：`services.domain_routes` 以域名为核心组织 HTTP 反代，支持 per-domain SSL/压缩/缓存；`services.reverse_proxy.routes` 继续兼容 host/path 匹配
- IP 黑名单/白名单：`services.access_control.http` 支持按 IP / CIDR 内建 allow/deny
- 限流：`services.rate_limit.http` 支持按 IP、Host 或 Header 的请求速率限制
- TS/JS 可编程路由：显式开启 script/plugins 后才进入 TypeScript 运行时，默认 HTTP/HTTPS 走 YAML 内建能力
- 插件机制：作为可选扩展层，支持自动加载、动态 load/unload/list、插件 sidecar YAML/JSON 配置，默认模板插件全部默认关闭
- 负载均衡：rendezvous、round_robin、least_connections、source_hash
- 被动健康检查：失败阈值隔离 + 隔离期后恢复
- 重试策略：可配置最大重试次数
- 亲和路由：基于 playerId/uid/pid 稳定选路
- 转发语义：自动补齐 `x-real-ip`、`x-forwarded-for`、`x-forwarded-host`、`x-forwarded-proto`、`forwarded`
- AI API 入口：支持 OpenAI 兼容 / New API / 聚合转发这类 HTTP API 作为普通反向代理入口，并提供默认关闭的 AI API 兼容插件模板
- 管理端：内置 admin API（可关闭）
- 配置热重载：配置文件变更后自动校验并重载
- TLS 模式：self_signed / manual / acme_external，以及 proxysss YAML 风格 `http.tls.auto_https`、多证书 SNI、域名级 `ssl.type`
- 显式子配置：通过 `include.enabled` + `include.files` 声明子配置，不自动扫目录
- 扩展服务：内建 `services.static_sites` 静态文件服务、`services.ftp` TCP 透传、内建 `services.webdav` 运行时
- 热重载：配置、显式 include、主扩展脚本、自动加载插件脚本均参与热重载 fingerprint
- 日志：访问日志（`logs/access.log`）、错误日志（`logs/error.log`）、`debug/info/warn/error` 级别控制，默认 `info`；`debug` 用于项目内部诊断
- 性能目标：以 nginx 级吞吐/延迟为基线，热路径设计优先考虑低分配、背压、锁竞争和可水平扩展

## 运行环境

- Windows / Linux / macOS
- x86_64(amd64) / arm64
- 脚本运行时：内嵌在 proxysss 二进制内，用户不需要额外安装任何解释器或运行时

## 快速开始

### 1. 初始化

```bash
proxysss init
```

会生成默认文件：

- proxysss.yaml
- gateway.ts
- plugins/player-affinity.ts
- plugins/traffic-stats.ts
- plugins/structured-log.ts
- plugins/geo-headers.ts
- plugins/ai-api-compat.ts
- certs/proxysss-cert.pem
- certs/proxysss-key.pem

默认生成的配置会优先使用 YAML 内建能力处理 HTTP/HTTPS、反向代理、静态文件和 WebDAV。gateway.ts 与 plugins/*.ts 只是可选扩展层，只有在把 script.enabled 或 plugins.enabled 打开后才会参与请求处理。

### 2. 校验配置

```bash
proxysss check-config --config ./proxysss.yaml
```

### 3. 启动

```bash
proxysss run --config ./proxysss.yaml
```

后台静默启动 / 重启：

```bash
proxysss start --config ./proxysss.yaml
proxysss restart --config ./proxysss.yaml
proxysss status --config ./proxysss.yaml
proxysss stop --config ./proxysss.yaml
```

默认端口：

- 80: HTTP welcome page
- 7777: admin console/API
- 443: HTTPS/HTTP2 与 HTTP/3

## 一键安装

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

安装指定版本：

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- v0.3.1
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.3.1
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action downgrade -Version v0.1.0
```

install.ps1 支持参数：

- -Action install|update|upgrade|downgrade
- -Version latest|vX.Y.Z
- -AllowDowngrade
- -NoServiceRestart
- -SkipInit
- -DryRun

安装脚本会完成：

- 下载对应平台 bundle（内含 proxysss 单文件二进制）
- 安装到 PATH 可访问位置
- 执行 init/check-config
- 安装并启用服务（开机自启动）

## 升级与降级

### CLI 一键升级/切版

```bash
proxysss update --version latest
proxysss update --version v0.3.1
proxysss switch-version v0.1.4 --allow-downgrade
```

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.3.1
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.0
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.3.1 -DryRun
```

### Linux / macOS

当前 install.sh 使用版本参数重装目标版本（可用于升级/降级）：

```bash
bash ./scripts/install.sh v0.3.1
bash ./scripts/install.sh v0.1.0
```

## 主流 Proxy 对比

下表覆盖当前最常见、最容易和 proxysss 放在一起评估的几类代理/网关。proxysss 的目标不是做业务网关，而是用更人性化、agent 友好的配置和 CLI 覆盖 nginx 同级入口职责；业务逻辑放到脚本/插件层，类似 nginx + Lua。

| 产品 | 层级定位 | HTTP/HTTPS | HTTP/3 | TCP/UDP/L4 | 自动 HTTPS | 配置与热重载 | 更适合的场景 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| proxysss | nginx 同级通用网关 + TS/JS 扩展 hooks | HTTP/1.1、HTTPS、HTTP/2、WebSocket | 是 | TCP/UDP、FTP passthrough、WebDAV、静态/反代 | `http.tls.auto_https`，proxysss YAML 风格自动证书入口 | YAML/JSON，显式 include，`config explain/routes/reload-plan/nginx-parity`，配置/脚本/插件热重载 | agent 接管、nginx 替代、通用入口、可脚本扩展 |
| Nginx | 通用 Web/server gateway | 极成熟 | 部分支持，依赖版本/构建/配置 | stream 模块支持 TCP/UDP | 通常配 Certbot/acme.sh 等外部工具 | 指令体系强但学习成本高，reload 成熟 | 传统 Web 入口、静态资源、反向代理 |
| LVS | Linux 内核级 L4 负载均衡 | 不处理 HTTP 语义 | 否 | 极强，四层转发/DR/NAT/TUN | 不处理 TLS 证书 | 依赖内核/IPVS/keepalived 等运维体系 | 超高性能四层 VIP、数据中心入口 |
| HAProxy | 高性能 L4/L7 负载均衡 | 极强 | 有限/前沿 | 极强 | 可终止 TLS，但证书自动化通常接外部工具 | 配置强大，reload/Runtime API 成熟 | 高性能负载均衡、复杂 L4/L7 调度 |
| Caddy | 通用 Web server / reverse proxy | 强 | 是 | 核心偏 HTTP，L4 需插件/扩展 | 默认自动签发、续期证书，并自动 HTTP->HTTPS | 声明式配置极简，reload 简单 | 快速 HTTPS、个人/中小型服务、证书省心 |
| Envoy | 云原生代理/数据面 | 强 | 是 | 强 | 通常由控制面/证书系统管理 | xDS 强，配置复杂 | Service Mesh、多集群治理、复杂过滤器 |
| Traefik | 云原生入口控制器 | 强 | 是 | HTTP 强，TCP/UDP 可用 | ACME 自动证书集成成熟 | 动态配置，擅长 Docker/K8s 自动发现 | 容器平台入口、Kubernetes Ingress |

快速选型建议：

- 如果你要一个默认端口、通用入口职责和 nginx 对齐，同时希望配置、热重载和 CLI 更适合 agent 接管，优先选 proxysss。
- 如果你只需要极致四层 VIP 转发，LVS 仍然是经典选择。
- 如果你需要成熟高性能 L4/L7 负载均衡规则，HAProxy 仍然是强项。
- 如果你最快想把一个域名转到后端并自动拿到 SSL，proxysss 使用 `http.tls.auto_https`，不是其他产品的配置格式。
- 如果你需要把会话、设备 ID 或租户等业务逻辑接入路由层，把它放在 proxysss 脚本/插件里，而不是核心网关里。
- 如果你已经在 Kubernetes 或 Service Mesh 体系里，Traefik 或 Envoy 更贴合现有控制面。

## 配置与安全

- 推荐配置格式：YAML（同样支持 JSON）
- 默认管理端地址：127.0.0.1:7777
- 默认管理端账号：root / root
- 生产环境必须修改默认管理账号密码

输出默认配置：

```bash
proxysss print-default-config --format yaml
proxysss print-default-config --format json
```

快速查阅当前配置：

```bash
proxysss config show --format yaml
proxysss config includes
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
proxysss config explain
proxysss config capabilities
```

直接验证内置 TypeScript 运行时：

```bash
proxysss script run-file ./examples/gateway.ts
proxysss script eval "console.log('proxysss ts runtime ok')"
```

## proxysss 自动 SSL 配置

目标效果：像 Caddy 那样省心地自动拿证书和续期，但配置风格必须是 proxysss 自己的 YAML，不搬其他产品的配置格式。公网服务器只需要写域名、邮箱和 upstream，proxysss 会把 `services.domain_routes[*].ssl.type=auto` / `is_auto_ssl=true` 与 `http.tls.auto_https` 汇总展开到 ACME 签发/续期流程。

- 自动 HTTPS 当前走 HTTP-01 / TLS-ALPN-01，不需要 CF token、DNS API token 这类额外凭证。
- 自动续期由网关后台循环触发，默认按 `renew_interval_hours` 周期执行。
- 当前签发/续期仍调用外部 ACME 客户端（默认 `acme.sh`），所以“零外部依赖”这一项还没做完。

最小生产配置：

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
      client: acme.sh
      challenge: tls_alpn01

services:
  access_control:
    http:
      enabled: true
      blacklist: [203.0.113.10, 198.51.100.0/24]
  domain_routes:
    - name: app
      domains: [example.com, www.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
      ssl:
        type: auto
        email: admin@example.com
      compression:
        enabled: true
      cache:
        enabled: true
        ttl_secs: 30
  rate_limit:
    http:
      enabled: true
      requests: 120
      window_ms: 60000
      burst: 30
```

推荐上线顺序：

```bash
proxysss init
proxysss cert-bootstrap
proxysss check-config --config ./proxysss.yaml
proxysss config reload-plan --config ./proxysss.yaml
proxysss run --config ./proxysss.yaml
```

服务器上线检查清单：

- 域名 `example.com` / `www.example.com` 的 A/AAAA 记录已经指向服务器公网 IP。
- 云厂商安全组、防火墙、路由器都放行 `80/tcp` 和 `443/tcp`。
- proxysss 能绑定 80/443；Linux 可用 root/service 运行，或给二进制加能力：

```bash
sudo setcap cap_net_bind_service=+ep "$(which proxysss)"
```

- `auto_https.production: false` 用于测试，避免频繁打正式 ACME CA 触发限流；确认无误后再改成 `true`。
- 证书和 ACME 缓存目录必须持久化，容器部署时不要放到临时文件系统。

`http.tls.auto_https` 是 proxysss 风格的高层入口；`services.domain_routes[*].ssl.type` / `is_auto_ssl` 则是更贴近“域名即配置核心”的入口。当前底层仍映射到现有 ACME 外部客户端签发/续期流程；`self_signed` 只适合开发或内网。`services.access_control.http` 可直接补上公网 IP 黑名单 / 白名单，`services.rate_limit.http` 可直接补上内建请求限流。

手动泛域名 / 多证书 SNI 示例：

```yaml
http:
  tls:
    mode: manual
    cert_path: ./certs/default-fullchain.pem
    key_path: ./certs/default-key.pem

services:
  domain_routes:
    - name: panel
      domains: [panel.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9001
      ssl:
        type: manual
        cert_path: ./certs/panel-fullchain.pem
        key_path: ./certs/panel-key.pem
    - name: wildcard-app
      domains: ["*.example.com"]
      path_prefix: /
      upstream: http://127.0.0.1:9002
      ssl:
        type: manual
        cert_path: ./certs/wildcard-fullchain.pem
        key_path: ./certs/wildcard-key.pem
```

显式子配置示例：

```yaml
include:
  enabled: true
  required: true
  files:
    - ./conf.d/http.yaml
    - ./conf.d/services.yaml
```

FTP/WebDAV 示例：

```yaml
services:
  reverse_proxy:
    routes:
      - name: api
        hosts: [api.example.com]
        path_prefix: /api
        upstream: http://127.0.0.1:8080
        upstreams:
          - http://127.0.0.1:8080
          - http://127.0.0.1:8081
        strip_prefix: true
        set_headers:
          x-gateway: proxysss
  rate_limit:
    http:
      enabled: true
      key: remote_addr
      requests: 120
      window_ms: 60000
      burst: 20
      status: 429
  static_sites:
    - name: public
      path_prefix: /assets
      root: ./public
      index_files: [index.html, index.htm]
      autoindex: false
  ftp:
    enabled: true
    bind: 0.0.0.0:21
    upstream: 127.0.0.1:2121
  webdav:
    enabled: true
    path_prefix: /dav
    root: ./webdav
    allow_write: true
```

WebDAV 内建支持 `OPTIONS`、`PROPFIND`、`GET`、`HEAD`、`PUT`、`DELETE`、`MKCOL`、`COPY`、`MOVE`。启用后请求会在脚本路由之前由 proxysss 热路径处理，避免把通用文件流量压到业务脚本上。

官方默认扩展脚本：

- `plugins/structured-log.ts`：演示 log hook、结构化访问事件输出。
- `plugins/traffic-stats.ts`：演示 log hook、访问事件、错误计数、流量统计。
- `plugins/player-affinity.ts`：演示亲和路由。
- `plugins/geo-headers.ts`：默认关闭，基于真实 socket 客户端 IP 注入 `proxysss-*` 地理 header；支持 sidecar 规则做离线国家/城市映射。
- `plugins/ai-api-compat.ts`：默认关闭，演示 OpenAI/New API/聚合 AI API 的 host/path 匹配、rewrite_path、审计 header 注入。

给 agent 的 skill 位于：

- skills/proxysss-install/SKILL.md — 一键安装与接管
- skills/gh-cli/SKILL.md — GitHub Actions 监控（`gh run list/view/watch`）

本地功能验证：

- `examples/lab/` → 复制到 `D:\Server\proxysss_dir`（**仅后端服务**，不含 proxysss）
- `examples/lab-proxysss/` → 显式 include，挂到 `%APPDATA%\proxysss\` 默认配置

## 管理接口

- GET /healthz
- GET /v1/stats
- GET /v1/upstreams
- GET /v1/config
- POST /v1/reload
- GET /v1/plugins
- POST /v1/plugins/load
- POST /v1/plugins/unload

说明：除 healthz 外，其余接口默认需要 Basic Auth。

## 插件与脚本

CLI 插件命令：

```bash
proxysss plugin list
proxysss plugin load --name player-affinity --module-path ./plugins/player-affinity.ts
proxysss plugin unload --name player-affinity
```

默认模板：

- templates/gateway.ts
- templates/plugins/player-affinity.ts
- templates/plugins/traffic-stats.ts
- templates/plugins/structured-log.ts
- templates/plugins/geo-headers.ts
- templates/plugins/ai-api-compat.ts
- examples/gateway.ts
- examples/plugins/player-affinity.ts
- examples/plugins/geo-headers.ts
- examples/plugins/ai-api-compat.ts

自动加载插件可选 sidecar 配置：

- `plugins/<name>.plugin.yaml`
- `plugins/<name>.plugin.yml`
- `plugins/<name>.plugin.json`

sidecar 字段：

- `enabled`: 是否启用该插件
- `priority`: 插件优先级
- `config`: 传给插件 `init_worker({ spec })` 的配置对象

`geo-headers` 示例：

```yaml
enabled: true
priority: 180
config:
  header_prefix: proxysss-
  include_city: true
  rules:
    - cidr: 203.0.113.0/24
      country: Exampleland
      country_code: EX
      city: Example City
      source: bundled_rule
      confidence: high
```

`ai-api-compat` 示例：

```yaml
enabled: true
priority: 220
config:
  header_prefix: proxysss-
  rules:
    - name: openai-chat
      provider: openai-compatible
      match_host: ai.local
      path_prefix: /v1/chat/completions
      rewrite_base_path: /v1/chat/completions
      upstream: https://api.openai.com
      add_headers:
        proxysss-ai-profile: openai-chat
```

## Nginx 功能对照清单

下面这张表按常见 nginx 入口职责逐项打勾，方便直接看哪些已经落地、哪些还在补齐：

| Nginx 常见入口能力 | 是否做完 | 当前状态 | proxysss 对应能力 / 差距 |
| --- | --- | --- | --- |
| 默认 HTTP 80 端口接管 | ✅ | 已完成 | 默认 `http.plain_bind=0.0.0.0:80` |
| 默认 HTTPS / HTTP3 443 端口接管 | ✅ | 已完成 | 默认 `http.tls_bind/http.h3_bind=0.0.0.0:443` |
| 声明式反向代理 | ✅ | 已完成 | `services.reverse_proxy.routes` |
| 静态文件服务 | ✅ | 已完成 | `services.static_sites` |
| WebDAV | ✅ | 已完成 | 内建常见 WebDAV 方法 |
| TCP / UDP 四层转发 | ✅ | 已完成 | `tcp.listeners` / `udp.listeners` |
| FTP 入口 | ⬜ | 部分完成 | 当前是 `services.ftp` TCP passthrough，未做原生 FTP 控制/被动通道感知 |
| TLS 证书加载 / 多证书 SNI | ✅ | 已完成 | `http.tls.certificates` + 域名级 manual SSL |
| 自动 HTTPS（无需 CF token / DNS token） | ✅ | 已完成 | `http.tls.auto_https` + HTTP-01/TLS-ALPN-01，写域名和邮箱即可 |
| 自动 HTTPS 完全内置零外部依赖 | ⬜ | 未完成 | 当前仍默认调用外部 `acme.sh` |
| 访问日志 / 错误日志 | ✅ | 已完成 | `logs/access.log` / `logs/error.log` |
| 热重载 | ✅ | 已完成 | 配置、include、脚本、插件都进 fingerprint |
| 压缩 | ⬜ | 部分完成 | 已支持 gzip+brotli；zstd 待补齐 |
| IP 黑名单 / 白名单 | ✅ | 已完成 | `services.access_control.http.allow/deny` 支持 IP / CIDR |
| 缓存 / proxy cache | ⬜ | 部分完成 | 已支持内存 GET 缓存，未做共享区/磁盘层 |
| 限流 | ⬜ | 部分完成 | 已支持请求速率限制，未做连接数限制/共享区风格策略 |
| 转发头语义 | ✅ | 已完成 | 自动补齐 `x-real-ip`、`x-forwarded-*`、`forwarded` |
| AI API / 通用 HTTP 透传 | ✅ | 已完成 | 核心 HTTP 反代 + 可选兼容插件 |
| 插件 sidecar 配置 | ✅ | 已完成 | `<name>.plugin.yaml/.yml/.json` |

## 真实业务场景对标补充

下面这组场景是 proxysss 当前版本重点对齐 nginx/通用入口职责时的真实业务视角清单：

| 场景 | 当前状态 | 说明 |
| --- | --- | --- |
| 普通反向代理 / 多 upstream | 已支持 | YAML `services.reverse_proxy.routes` + 负载均衡 + 重试 + 被动健康检查 |
| 静态站点 / 下载站 | 已支持 | 内建 `services.static_sites` |
| WebSocket | 已支持 | HTTP/1.1 upgrade + 上游透传 |
| WebDAV | 已支持 | 内建常见方法集 |
| TCP / UDP / FTP 透传 | 已支持 / FTP 部分支持 | FTP 目前仍是 TCP passthrough |
| HTTP/2 / HTTP/3 入口 | 已支持 | HTTP/2 over TLS/TCP，HTTP/3 over QUIC |
| 自动 HTTPS + 自动续期 | 已支持 | 无需 CF token / DNS token，当前默认外部 ACME 客户端负责签发/续期 |
| 真实客户端 IP 透传 | 已增强 | 自动补齐 `x-real-ip`、`x-forwarded-*`、`forwarded` |
| IP 黑名单 / 白名单 | 已支持 | `services.access_control.http` 支持单 IP 和 CIDR |
| 热重载 | 已支持 | 配置、主脚本、自动加载插件脚本与 sidecar 一起进入 fingerprint |
| AI API / New API 转发 | 已支持基础入口 + 可选插件 | 核心先保证 HTTP 代理；插件负责路径兼容、审计 header、地理 header |
| 压缩 | 部分支持 | `services.domain_routes[*].compression` 已支持 gzip+brotli，zstd 仍待补齐 |
| 缓存 | 部分支持 | `services.domain_routes[*].cache` 已支持内存 GET 缓存，磁盘缓存/共享缓存区仍待补齐 |
| 多证书 / SNI 细粒度选择 | 已增强 | `http.tls.certificates` 与域名级 manual SSL 已支持按 SNI 选证书 |

## AI API / New API 转发说明

- proxysss 把 OpenAI 兼容接口、Anthropic 风格接口、Gemini/OpenRouter/New API 这类入口统一视为 HTTP 反向代理场景。
- 默认建议先用 YAML 内建反向代理保证鉴权头、状态码、请求体、响应体原样透传。
- 需要兼容层时，再开启 `plugins/ai-api-compat.ts` 这类默认关闭插件，只做 host/path 选择、`rewrite_path`、审计 header 注入，不把通用入口逻辑硬编码进核心。
- 需要地理信息时，再叠加 `plugins/geo-headers.ts`，把 `proxysss-client-ip`、`proxysss-geo-country`、`proxysss-geo-country-code`、`proxysss-geo-city`、`proxysss-geo-source`、`proxysss-geo-confidence` 注入给后续 upstream。

## 服务命令

```bash
proxysss service install
proxysss service uninstall
proxysss service start
proxysss service stop
proxysss service status
```

## 压测与示例

示例回环服务：

```bash
proxysss demo http-echo --listen 127.0.0.1:8081
proxysss demo tcp-echo --listen 127.0.0.1:7001
proxysss demo udp-echo --listen 127.0.0.1:8101
```

压测命令：

```bash
proxysss bench http --url https://127.0.0.1/ --concurrency 512 --duration-secs 30 --insecure
proxysss bench tcp --addr 127.0.0.1:26379 --connections 512 --duration-secs 30 --payload-bytes 512
proxysss bench udp --addr 127.0.0.1:2053 --connections 512 --duration-secs 30 --payload-bytes 256
```

横向压测：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\benchmark-gateways.ps1
```

本地 Windows loopback 横向结果（2026-06-05，`proxysss bench http` 作为统一客户端，同一静态 HTML 文件，顺序执行，concurrency=512，duration=30s）：

| 网关 | 版本/运行方式 | URL | 成功请求 | 错误 | ops/sec | p50 | p95 | p99 |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| proxysss | v0.2.5 dev build，内置 `services.static_sites` | `127.0.0.1:18083/bench/index.html` | 398,174 | 0 | 13,272.47 | 38.384 ms | 42.178 ms | 44.934 ms |
| Caddy | v2.11.3 Windows amd64，`file-server` | `127.0.0.1:18082/index.html` | 358,264 | 0 | 11,942.13 | 42.160 ms | 50.181 ms | 54.104 ms |
| nginx | 1.31.0 nginx/Windows，static alias | `127.0.0.1:18081/bench/index.html` | 14,496 | 1,748,575 | 483.20 | 43.551 ms | 70.809 ms | 28,958.072 ms |

说明：

- 这是 Windows 本机 loopback 的开发机数据，不代表 Linux 生产最终上限。
- nginx 官方 Windows 版本主要适合开发/测试；生产性能对比应在 Linux 同机、同内核参数、同 worker 数、同连接限制下复测。
- 当前数据只覆盖静态 HTTP GET；TLS、HTTP/2、HTTP/3、反向代理、WebDAV、TCP/UDP 需要单独矩阵。

## CI/CD 与发布

仓库工作流：

- .github/workflows/ci.yml
- .github/workflows/deploy.yml
- .github/workflows/release.yml

release.yml 会在推送 v* tag 时构建并发布多平台二进制。

## Windows 快捷脚本

- run.cmd
- test.cmd
- build.cmd
- deploy.cmd

## 开发构建

```bash
cargo build --release
cargo test --workspace --all-targets
```

## License

MIT
