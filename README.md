# proxysss

proxysss 是一个可编程 Rust 网关，统一支持 HTTP/1.1、HTTP/2、HTTP/3、TCP、UDP，适合游戏网关、聊天网关和通用高并发接入层。

当前版本：v0.1.1

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
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash -s -- v0.1.1
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1 | iex
```

参数化执行（推荐）：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action update -Version v0.1.1
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

### Windows

升级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action upgrade -Version v0.1.1
```

降级到指定版本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action downgrade -Version v0.1.0
```

演练模式（不落盘，不修改服务）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install.ps1 -Action update -Version v0.1.1 -DryRun
```

### Linux / macOS

当前 install.sh 使用版本参数重装目标版本（可用于升级/降级）：

```bash
bash ./scripts/install.sh v0.1.1
bash ./scripts/install.sh v0.1.0
```

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
