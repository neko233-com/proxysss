# proxysss

[English](README.md) · **简体中文** · [使用文档](https://neko233-com.github.io/proxysss/) · [下载安装包](https://github.com/neko233-com/proxysss/releases/latest)

一个 Rust 二进制、一份 YAML，完成网站、API 与实时连接的网关工作。proxysss 面向 nginx 同级通用网关场景；需要业务逻辑时，再使用内置 TypeScript 脚本和插件扩展。

## 安装与后台运行

Windows PowerShell：

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
```

Linux / macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

安装后检查配置与启动状态：

```bash
proxysss --version
proxysss check-config
proxysss service status
```

首次手动安装时，运行 `proxysss init` 创建配置，再运行 `proxysss service install` 注册后台启动。Windows 使用隐藏启动器和用户登录自启动；Linux 使用 systemd，macOS 使用 LaunchAgent。Linux 监听 80/443 时需要相应端口权限。

| 入口 | 默认用途 |
| --- | --- |
| `http://127.0.0.1/` | 简洁的欢迎页与文档入口，端口 80 |
| `http://127.0.0.1:7777/` | 管理后台，仅监听本机 |
| `http://127.0.0.1:7777/docs.html` | 后台内置使用文档 |
| `http://127.0.0.1/docs.html` | 公共监听器上的内置文档 |

后台默认账号为 `root` / `root`。在生产使用前修改登录凭据；配置展示和写入默认关闭，阅读状态无需开启它们。后台保留登录与会话验证，普通浏览不会修改网关配置。

## 配置你的第一个网站

默认配置文件为 `proxysss.yaml`。可以用 `-c`、`--config` 或 `-config` 指定其他位置。保留已有配置，将下列内容合入相应配置块：

```yaml
services:
  reverse_proxy:
    routes:
      - name: my-app
        hosts: [app.example.com]
        path_prefix: /
        upstream: http://127.0.0.1:3000
```

将域名指向这台服务器，确认上游服务正常，再检查配置、查看重载边界并重启后台进程：

```bash
proxysss check-config
proxysss config reload-plan
proxysss restart
```

发布静态网站时，使用静态站点配置：

```yaml
services:
  static_sites:
    - name: my-static-site
      path_prefix: /
      root: ./public
      index_files: [index.html]
      autoindex: false
```

静态服务支持 HTML、图片、字体、音视频、大文件流式传输、HEAD、Range 断点续传与缓存验证。目录索引需显式开启；默认隐藏点文件，并限制文件访问在站点根目录内。[静态与 CDN 源站指南](https://neko233-com.github.io/proxysss/cdn-origin.html) 提供完整参数。

## 主要能力

| 场景 | 支持内容 |
| --- | --- |
| 网站与 API | HTTP/1.1、HTTPS、HTTP/2、HTTP/3、gRPC-over-HTTP/2、WebSocket、反向代理、压缩、代理缓存 |
| 流量控制 | IP/CIDR 访问控制、固定窗口 / token-bucket / leaky-bucket 限流、健康检查、加权上游、重试与故障隔离 |
| 文件与 CDN | 静态站点、受保护的 CDN 回源、签名 URL、WebDAV、FTP 与 FileCloud |
| 实时协议 | TCP/UDP、游戏长连接、MQTT TCP/TLS/WebSocket、CoAP 风格 UDP、KCP/QCP 透明转发 |
| AI 与服务发现 | New API / sub2api / OpenAI 兼容接口转发，Consul / etcd / Nacos 注册中心联动 |
| 证书与扩展 | 内建 ACME、DNS-01 泛域名证书、配置热重载、进程内 TypeScript、可选插件 |

MQTT 由上游 broker 提供业务功能；proxysss 负责边缘转发。内置 TypeScript 使用 QuickJS 与进程内类型移除，不需要 Node、Deno 或外部编译器。

普通域名可通过 `http.tls.auto_https.domains` 启用自动证书；泛域名使用 `http.tls.acme.challenge: dns01` 并配置 DNS 服务商。[证书与配置指南](https://neko233-com.github.io/proxysss/configuration.html)

## 日常运维

```bash
proxysss config explain          # 配置说明
proxysss config routes           # 路由拓扑
proxysss config security         # 安全开关、默认值和建议
proxysss config performance      # 本机系统与 socket 适配
proxysss config watched-scripts  # 参与热重载的脚本
proxysss config nginx-parity --format yaml
proxysss service status
```

日志默认为 `logs/access.log` 与 `logs/error.log`，默认级别 `info`。安全策略与性能参数应依据部署场景调整：[安全与系统适配指南](https://neko233-com.github.io/proxysss/security-performance.html)。性能测试在本地 Linux 执行；GitHub Actions 仅负责发布材料校验与六平台打包。功能发布不自动代表取得严格领先 nginx 的性能证据。

## 继续阅读

- [安装、配置与常见场景](https://neko233-com.github.io/proxysss/)
- [配置指南](https://neko233-com.github.io/proxysss/configuration.html) · [架构](https://neko233-com.github.io/proxysss/architecture.html)
- [TypeScript 与插件](https://neko233-com.github.io/proxysss/ts-how-to-use.html)
- [从 nginx 迁移](https://neko233-com.github.io/proxysss/nginx-to-proxysss.html) · [从 Caddy 迁移](https://neko233-com.github.io/proxysss/caddy-to-proxysss.html)

本地验证、缓存与安装包使用项目内固定的 `.tmp`、`.cache`、`target`、`dist` 目录。Windows 可运行 `test.cmd`，Docker 场景验证使用 `scripts/verify-docker-scenarios.ps1`；验证容器固定为 `proxysss-verify`，结束后自动删除。
