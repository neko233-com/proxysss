# proxysss

proxysss 是一个可编程 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、TCP、UDP，适合游戏网关、聊天网关和通用高并发接入层。

当前版本：v0.1.5

## 核心能力

- 多协议统一入口：HTTP/1.1、HTTP/2（TCP）+ HTTP/3（UDP）+ TCP + UDP
- TS/JS 可编程路由：默认通过 Deno 运行 gateway.ts
- 插件机制：支持插件自动加载、动态 load/unload/list
- 负载均衡：rendezvous、round_robin、least_connections、source_hash
- 被动健康检查：失败阈值隔离 + 隔离期后恢复
- 重试策略：可配置最大重试次数
- 亲和路由：基于 playerId/uid/pid 稳定选路
- 管理端：内置 admin API（可关闭）
- 配置热重载：配置文件变更后自动校验并重载
- TLS 模式：self_signed / manual / acme_external
- Nginx 常用内建能力：反代、负载均衡、TLS、HTTP/2、HTTP/3、WebSocket、TCP/UDP、热重载、静态文件、redirect、healthz

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

默认网关端口：23380

- TCP: HTTP/1.1 + HTTP/2
- UDP: HTTP/3

## 一键安装

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

安装指定版本：

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- --version v0.1.5
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.1.5
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action downgrade -Version v0.1.4
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
- 更新时优先停止服务、原子替换二进制、再启动服务；Windows 不能对运行中 exe 做真正原地替换，所以默认走快速冷重启

## 升级与降级

### CLI 一键升级/切版

```bash
proxysss update --version latest
proxysss update --version v0.1.5
proxysss switch-version v0.1.4 --allow-downgrade
```

配置变更可热重载。二进制更新按平台限制处理：能热更新则热更新；不能无感替换时走“停服务 -> 替换 -> 启服务”的快速冷启动。

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.1.5
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.4
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.1.5 -DryRun
```

### Linux / macOS

install.sh 支持显式 action/version：

```bash
bash ./scripts/install.sh --action upgrade --version v0.1.5
bash ./scripts/install.sh --action downgrade --version v0.1.4
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

## Nginx 功能覆盖策略

proxysss 当前不是 Nginx 逐指令复刻。它按现有架构覆盖入口网关高频能力，并把细粒度规则放进 TS/JS 脚本和插件：

- 已内建：HTTP/TLS/HTTP2/HTTP3、WebSocket、TCP/UDP stream、反向代理、负载均衡、重试、被动健康检查、热重载、静态文件、redirect、healthz、access log、admin API、ACME 外部证书。
- 用脚本/插件实现：按 host/path/header/cookie/query 路由、rewrite、header 改写、鉴权、限流、灰度、会话亲和、业务级 upstream 选择。
- 不承诺逐条兼容：Nginx rewrite DSL、module ABI、mail proxy、SSI、autoindex、复杂 cache 等；这些可按需求逐项插件化。

## 配置与安全

- 推荐配置格式：YAML（同样支持 JSON）
- 默认管理端地址：127.0.0.1:23381
- 默认管理端账号：root / root
- 生产环境必须修改默认管理账号密码

输出默认配置：

```bash
proxysss print-default-config --format yaml
proxysss print-default-config --format json
```

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

## 内建内部路由

脚本可返回：

```ts
{ upstream: "proxysss://healthz" }
{ upstream: "proxysss://redirect/https://example.com", status: 301 }
{ upstream: "proxysss://static/public/index.html", content_type: "text/html; charset=utf-8" }
```

静态文件路径被限制在配置目录内，防止 `..` 越界。

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
proxysss bench http --url https://127.0.0.1:23380/ --concurrency 512 --duration-secs 30 --insecure
proxysss bench tcp --addr 127.0.0.1:26379 --connections 512 --duration-secs 30 --payload-bytes 512
proxysss bench udp --addr 127.0.0.1:2053 --connections 512 --duration-secs 30 --payload-bytes 256
```

## CI/CD 与发布

仓库工作流：

- .github/workflows/ci.yml
- .github/workflows/deploy.yml
- .github/workflows/release.yml

release.yml 会在推送 v* tag 时构建并发布多平台二进制。

本地发布后验证：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-release.ps1 -Version v0.1.5 -PreviousVersion v0.1.4
```

版本变更记录见 [CHANGELOG.md](CHANGELOG.md)。

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
