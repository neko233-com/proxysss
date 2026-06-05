# proxysss

proxysss 是一个可编程 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、TCP、UDP，适合游戏网关、聊天网关和通用高并发接入层。

产品目标：作为 nginx 同级通用网关完整替代 nginx 的常见入口职责，同时提供更适合人类和 agent 接管的配置、查询和输出体验。默认 HTTP 端口与 nginx 一致为 80，首页是更友好的 `Welcome to proxysss`。proxysss 不是“业务网关优先”的产品；脚本/插件扩展层类似 nginx + Lua，用来承载可选的业务逻辑。

当前版本：v0.2.0

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
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- v0.2.0
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.2.0
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
proxysss update --version v0.2.0
proxysss switch-version v0.1.4 --allow-downgrade
```

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.2.0
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.0
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.2.0 -DryRun
```

### Linux / macOS

当前 install.sh 使用版本参数重装目标版本（可用于升级/降级）：

```bash
bash ./scripts/install.sh v0.2.0
bash ./scripts/install.sh v0.1.0
```

## 主流 Proxy 对比

下表覆盖当前最常见、最容易和 proxysss 放在一起评估的几类代理/网关。它不是“所有代理软件”的穷举，但已经覆盖大多数通用接入层选型场景。

| 产品 | 可编程路由 | HTTP/3 | 原生 TCP/UDP | 服务发现/控制面 | 配置复杂度 | 更适合的场景 |
| --- | --- | --- | --- | --- | --- | --- |
| proxysss | TS/JS 脚本 + 插件，偏业务路由 | 是 | 是 | 内置轻量管理面，偏单体网关 | 中 | 游戏网关、聊天网关、按 playerId/uid 做亲和与脚本路由 |
| Nginx | 有限，主要靠指令与模块 | 部分支持，依赖构建与配置 | TCP/UDP 依赖 stream 模块 | 弱，通常接外部服务注册 | 中 | 稳定 Web 入口、静态资源、传统反向代理 |
| Caddy | 中，偏声明式与插件 | 是 | 以 HTTP 为主，L4 需插件或扩展 | 弱到中 | 低 | 快速 HTTPS 接入、简单网站和 API 代理 |
| HAProxy | 中，规则强但业务脚本弱 | 有限且偏前沿 | 是，L4/L7 很强 | 中 | 中到高 | 高性能四层/七层负载均衡、传统流量调度 |
| Envoy | 强，过滤器体系完整 | 是 | 是 | 强，适合配合 xDS/mesh | 高 | Service Mesh、云原生边车、复杂多集群治理 |
| Traefik | 中，偏声明式与自动发现 | 是 | HTTP 强，TCP/UDP 可用 | 强，擅长 Docker/K8s 自动发现 | 低到中 | 容器平台入口、Kubernetes Ingress、自动发现路由 |

快速选型建议：

- 如果你要按玩家、会话、设备 ID 做稳定选路，并且希望把业务路由逻辑直接写在脚本里，优先选 proxysss。
- 如果你主要做通用 Web 反代，且希望生态成熟、运维习惯稳定，Nginx 或 Caddy 更省心。
- 如果你已经在 Kubernetes 或 Service Mesh 体系里，Traefik 或 Envoy 更贴合现有控制面。
- 如果核心诉求是极强的 L4/L7 性能与成熟负载均衡规则，HAProxy 仍然是强项。

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

给 agent 的一键安装 skill 位于：

- skills/proxysss-install/SKILL.md

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
