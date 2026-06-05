# proxysss

proxysss 是一个 nginx 同级的通用 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、WebSocket、TCP、UDP、FTP、WebDAV、静态文件和反向代理入口职责。

产品目标：作为 nginx 同级通用网关完整替代 nginx 的常见入口职责，同时提供更适合人类和 agent 接管的配置、查询和输出体验。默认 HTTP 端口与 nginx 一致为 80，首页是更友好的 `Welcome to proxysss`。proxysss 不是“业务网关优先”的产品；脚本/插件扩展层类似 nginx + Lua，用来承载可选的业务逻辑。

当前版本：v0.2.6

## 核心能力

- 多协议统一入口：HTTP/1.1、HTTP/2（TCP）+ HTTP/3（UDP）+ TCP + UDP
- 声明式反向代理：`services.reverse_proxy.routes` 支持 host/path 匹配、upstream 池、strip_prefix
- 限流：`services.rate_limit.http` 支持按 IP、Host 或 Header 的请求速率限制
- TS/JS 可编程路由：显式开启 script/plugins 后才进入 TypeScript 运行时，默认 HTTP/HTTPS 走 YAML 内建能力
- 插件机制：作为可选扩展层，支持自动加载、动态 load/unload/list
- 负载均衡：rendezvous、round_robin、least_connections、source_hash
- 被动健康检查：失败阈值隔离 + 隔离期后恢复
- 重试策略：可配置最大重试次数
- 亲和路由：基于 playerId/uid/pid 稳定选路
- 管理端：内置 admin API（可关闭）
- 配置热重载：配置文件变更后自动校验并重载
- TLS 模式：self_signed / manual / acme_external，以及 proxysss YAML 风格 `http.tls.auto_https`
- 显式子配置：通过 `include.enabled` + `include.files` 声明子配置，不自动扫目录
- 扩展服务：内建 `services.static_sites` 静态文件服务、`services.ftp` TCP 透传、内建 `services.webdav` 运行时
- 热重载：配置、显式 include、主扩展脚本、自动加载插件脚本均参与热重载 fingerprint
- 日志：访问日志（`logs/access.log`）、错误日志（`logs/error.log`）、`debug/info/warn/error` 级别控制，默认 `info`；`debug` 用于项目内部诊断
- 性能目标：以 nginx 级吞吐/延迟为基线，热路径设计优先考虑低分配、背压、锁竞争和可水平扩展

## 运行环境

- Windows / Linux / macOS
- x86_64(amd64) / arm64
- 脚本运行时：由 proxysss 发布包内置并随安装一起解包，用户不需要额外安装任何解释器或运行时

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
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- v0.2.6
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.2.6
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

- 下载对应平台 bundle（内含 proxysss 二进制和内置 TypeScript 运行时）
- 安装到 PATH 可访问位置
- 把内置 TypeScript 运行时解包到 proxysss 运行目录
- 执行 init/check-config
- 安装并启用服务（开机自启动）

## 升级与降级

### CLI 一键升级/切版

```bash
proxysss update --version latest
proxysss update --version v0.2.6
proxysss switch-version v0.1.4 --allow-downgrade
```

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.2.6
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.0
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.2.6 -DryRun
```

### Linux / macOS

当前 install.sh 使用版本参数重装目标版本（可用于升级/降级）：

```bash
bash ./scripts/install.sh v0.2.6
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

## proxysss 自动 SSL 配置

目标效果：像 Caddy 那样省心地自动拿证书和续期，但配置风格必须是 proxysss 自己的 YAML，不搬其他产品的配置格式。公网服务器只需要写域名、邮箱和 upstream，proxysss 会把 `http.tls.auto_https` 展开到 ACME 签发/续期流程。

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
  reverse_proxy:
    routes:
      - name: app
        hosts: [example.com, www.example.com]
        path_prefix: /
        upstream: http://127.0.0.1:9000
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

`http.tls.auto_https` 是 proxysss 风格的高层入口；底层会映射到现有 ACME 外部客户端签发/续期流程。`self_signed` 只适合开发或内网。

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
- examples/gateway.ts
- examples/plugins/player-affinity.ts

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
