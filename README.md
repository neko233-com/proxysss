# proxysss

proxysss 是一个 nginx 同级的通用 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、WebSocket、TCP、UDP、FTP、WebDAV、静态文件和反向代理入口职责。

产品目标：作为 nginx 同级通用网关完整替代 nginx 的常见入口职责，同时提供更适合人类和 agent 接管的配置、查询和输出体验。默认 HTTP 端口与 nginx 一致为 80，首页是更友好的 `Welcome to proxysss`。proxysss 不是“业务网关优先”的产品；脚本/插件扩展层类似 nginx + Lua，用来承载可选的业务逻辑。

当前版本：v0.2.4

## 核心能力

- 多协议统一入口：HTTP/1.1、HTTP/2（TCP）+ HTTP/3（UDP）+ TCP + UDP
- 声明式反向代理：`services.reverse_proxy.routes` 支持 host/path 匹配、upstream 池、strip_prefix
- 限流：`services.rate_limit.http` 支持按 IP、Host 或 Header 的请求速率限制
- TS/JS 可编程路由：默认通过 Deno 运行 gateway.ts
- 插件机制：支持插件自动加载、动态 load/unload/list
- 负载均衡：rendezvous、round_robin、least_connections、source_hash
- 被动健康检查：失败阈值隔离 + 隔离期后恢复
- 重试策略：可配置最大重试次数
- 亲和路由：基于 playerId/uid/pid 稳定选路
- 管理端：内置 admin API（可关闭）
- 配置热重载：配置文件变更后自动校验并重载
- TLS 模式：self_signed / manual / acme_external
- 显式子配置：通过 `include.enabled` + `include.files` 声明子配置，不自动扫目录
- 扩展服务：内建 `services.static_sites` 静态文件服务、`services.ftp` TCP 透传、内建 `services.webdav` 运行时
- 热重载：配置、显式 include、主扩展脚本、自动加载插件脚本均参与热重载 fingerprint
- 日志：访问日志（`logs/access.log`）、错误日志（`logs/error.log`）、`debug/info/warn/error` 级别控制，默认 `info`；`debug` 用于项目内部诊断
- 性能目标：以 nginx 级吞吐/延迟为基线，热路径设计优先考虑低分配、背压、锁竞争和可水平扩展

## 运行环境

- Windows / Linux / macOS
- x86_64(amd64) / arm64
- 脚本运行时：Deno（安装脚本会自动安装）

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
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- v0.2.4
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.2.4
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

- 下载对应平台二进制
- 安装到 PATH 可访问位置
- 自动安装 Deno（若缺失）
- 执行 init/check-config
- 安装并启用服务（开机自启动）

## 升级与降级

### CLI 一键升级/切版

```bash
proxysss update --version latest
proxysss update --version v0.2.4
proxysss switch-version v0.1.4 --allow-downgrade
```

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.2.4
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.0
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.2.4 -DryRun
```

### Linux / macOS

当前 install.sh 使用版本参数重装目标版本（可用于升级/降级）：

```bash
bash ./scripts/install.sh v0.2.4
bash ./scripts/install.sh v0.1.0
```

## 主流 Proxy 对比

下表覆盖当前最常见、最容易和 proxysss 放在一起评估的几类代理/网关。proxysss 的目标不是做业务网关，而是用更人性化、agent 友好的配置和 CLI 覆盖 nginx 同级入口职责；业务逻辑放到脚本/插件层，类似 nginx + Lua。

| 产品 | 层级定位 | HTTP/HTTPS | HTTP/3 | TCP/UDP/L4 | 自动 HTTPS | 配置与热重载 | 更适合的场景 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| proxysss | nginx 同级通用网关 + TS/JS 扩展 hooks | HTTP/1.1、HTTPS、HTTP/2、WebSocket | 是 | TCP/UDP、FTP passthrough、WebDAV、静态/反代 | self_signed / manual / acme_external；目标是继续靠近 Caddy 式自动证书体验 | YAML/JSON，显式 include，`config explain/routes/reload-plan/nginx-parity`，配置/脚本/插件热重载 | agent 接管、nginx 替代、通用入口、可脚本扩展 |
| Nginx | 通用 Web/server gateway | 极成熟 | 部分支持，依赖版本/构建/配置 | stream 模块支持 TCP/UDP | 通常配 Certbot/acme.sh 等外部工具 | 指令体系强但学习成本高，reload 成熟 | 传统 Web 入口、静态资源、反向代理 |
| LVS | Linux 内核级 L4 负载均衡 | 不处理 HTTP 语义 | 否 | 极强，四层转发/DR/NAT/TUN | 不处理 TLS 证书 | 依赖内核/IPVS/keepalived 等运维体系 | 超高性能四层 VIP、数据中心入口 |
| HAProxy | 高性能 L4/L7 负载均衡 | 极强 | 有限/前沿 | 极强 | 可终止 TLS，但证书自动化通常接外部工具 | 配置强大，reload/Runtime API 成熟 | 高性能负载均衡、复杂 L4/L7 调度 |
| Caddy | 通用 Web server / reverse proxy | 强 | 是 | 核心偏 HTTP，L4 需插件/扩展 | 默认自动签发、续期证书，并自动 HTTP->HTTPS | Caddyfile 极简，reload 简单 | 快速 HTTPS、个人/中小型服务、证书省心 |
| Envoy | 云原生代理/数据面 | 强 | 是 | 强 | 通常由控制面/证书系统管理 | xDS 强，配置复杂 | Service Mesh、多集群治理、复杂过滤器 |
| Traefik | 云原生入口控制器 | 强 | 是 | HTTP 强，TCP/UDP 可用 | ACME 自动证书集成成熟 | 动态配置，擅长 Docker/K8s 自动发现 | 容器平台入口、Kubernetes Ingress |

快速选型建议：

- 如果你要一个默认端口、通用入口职责和 nginx 对齐，同时希望配置、热重载和 CLI 更适合 agent 接管，优先选 proxysss。
- 如果你只需要极致四层 VIP 转发，LVS 仍然是经典选择。
- 如果你需要成熟高性能 L4/L7 负载均衡规则，HAProxy 仍然是强项。
- 如果你最快想把一个域名转到后端并自动拿到 SSL，Caddy 是极省心的参考实现。
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

## 超快自动 SSL 配置

Caddy 的 Automatic HTTPS 是 proxysss 证书体验要追赶的标杆：当配置里出现公开域名、DNS A/AAAA 已指向服务器、80/443 对外开放、Caddy 有权限绑定端口并且数据目录可持久化时，Caddy 会自动向 Let's Encrypt/ZeroSSL 等 ACME CA 申请证书、自动续期，并自动把 HTTP 跳转到 HTTPS。官方说明见 [Automatic HTTPS](https://caddyserver.com/docs/automatic-https) 和 [Reverse proxy quick-start](https://caddyserver.com/docs/quick-starts/reverse-proxy)。

最快的 Caddy 反代写法：

```bash
# 后端服务监听 127.0.0.1:9000
caddy reverse-proxy --from example.com --to 127.0.0.1:9000
```

等价 Caddyfile：

```caddyfile
example.com {
  reverse_proxy 127.0.0.1:9000
}
```

服务器上线检查清单：

- 域名 `example.com` 的 A/AAAA 记录已经指向服务器公网 IP。
- 云厂商安全组、防火墙、路由器都放行 `80/tcp` 和 `443/tcp`。
- Caddy 能绑定 80/443；Linux 可用 `sudo` 运行，或给二进制加能力：

```bash
sudo setcap cap_net_bind_service=+ep "$(which caddy)"
```

- Caddy 的 data 目录必须持久化，容器部署时不要把证书缓存放到临时文件系统。
- 测试时不要频繁打正式 ACME CA；需要反复试配置时用 staging，避免触发证书签发限流。

proxysss 当前对应做法：

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  h3_bind: 0.0.0.0:443
  tls:
    mode: acme_external
    server_name: example.com
    acme:
      client: acme.sh
      email: admin@example.com
      domains: [example.com]
      challenge: tls_alpn01
      directory_production: true
```

推荐上线顺序：

```bash
proxysss init
proxysss cert-bootstrap
proxysss check-config --config ./proxysss.yaml
proxysss config reload-plan --config ./proxysss.yaml
proxysss run --config ./proxysss.yaml
```

`acme_external` 适合生产自动证书路线；`self_signed` 只适合开发或内网。后续 proxysss 会继续把 ACME 自动化做得更接近 Caddy 的“写域名就自动 HTTPS”体验。

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
