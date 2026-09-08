# nginx 迁移到 proxysss

这份文档只回答一个实际问题：

`nginx` 里常见的配置，在 `proxysss` 里应该怎么表达？

它不是功能总表，而是迁移手册。写法也按两条路线组织：

- `新手路线`：先迁一条能跑的链路
- `高手路线`：快速确认该把配置落在哪个能力面

文档规范说明：

- 这份文档是 `中文 first`
- 术语保留 `English`
- 它有对应 HTML 子页：`docs/nginx-to-proxysss.html`

## 1. 先建立迁移心智

### 1.1 nginx 的 `server` / `location`，在 proxysss 里怎么想

| nginx 思路 | proxysss 更常用的落点 | 什么时候用 |
| --- | --- | --- |
| `server_name + location /` | `services.reverse_proxy.routes` | 普通整站反代 |
| `location /api/` + `proxy_pass` | `match.path_prefix` + `strip_prefix` | API 前缀代理 |
| 同一域名下多个路径组装一个站点 | `services.domain_routes` | 站点级编排 |
| `upstream` 权重和健康管理 | `upstreams` + `load_balance` | 集群反代 |
| `stream {}` | `tcp.listeners` / `udp.listeners` | 非 HTTP 协议 |

一个很实用的判断方法：

- 你只是做 `HTTP -> HTTP`：优先看 `reverse_proxy.routes`
- 你想按“整站结构”组织配置：优先看 `domain_routes`
- 你在做数据库、游戏、MQTT、UDP：直接看 `tcp.listeners` / `udp.listeners`
- 没有用户路由命中 `/` 时，proxysss 才返回只含 GitHub 和 GitHub Docs 的内建欢迎页；迁移业务首页时应显式声明 route/static site

## 2. 新手先迁最常见的 6 类 nginx 配置

### 2.1 反代某个 API 前缀

nginx：

```nginx
server {
    listen 80;
    server_name api.example.com;

    location /api/ {
        proxy_pass http://127.0.0.1:8080/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

proxysss：

```yaml
http:
  plain_bind: 0.0.0.0:80

services:
  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["api.example.com"]
          path_prefix: "/api"
        strip_prefix: true
        upstreams:
          - url: "http://127.0.0.1:8080"
```

怎么理解：

- `strip_prefix: true` 对应 nginx `location /api/` + `proxy_pass .../`
- 常见 `Host` / `X-Real-IP` / `X-Forwarded-*` 会自动补齐
- 这类场景优先用 `reverse_proxy.routes`，不要先上脚本

### 2.2 反代整个域名到一台后端

nginx：

```nginx
server {
    listen 80;
    server_name app.example.com;

    location / {
        proxy_pass http://127.0.0.1:9000;
    }
}
```

proxysss：

```yaml
services:
  reverse_proxy:
    routes:
      - name: app
        match:
          hosts: ["app.example.com"]
          path_prefix: "/"
        upstreams:
          - url: "http://127.0.0.1:9000"
```

这代表：

- `/` 和所有子路径都交给后端
- 后端自己决定 `/dashboard`、`/assets/app.js`、`/api/*` 等路径如何处理

### 2.3 同一域名下前后端分离

例如：

- `/` 走前端 SSR 或静态服务
- `/api` 走后端 API

nginx：

```nginx
location /api/ {
    proxy_pass http://127.0.0.1:8080/;
}

location / {
    proxy_pass http://127.0.0.1:3000;
}
```

proxysss：

```yaml
services:
  reverse_proxy:
    routes:
      - name: backend-api
        match:
          hosts: ["app.example.com"]
          path_prefix: "/api"
        strip_prefix: true
        upstreams:
          - url: "http://127.0.0.1:8080"

      - name: frontend-app
        match:
          hosts: ["app.example.com"]
          path_prefix: "/"
        upstreams:
          - url: "http://127.0.0.1:3000"
```

为什么这样迁比较直观：

- `/api/*` 会优先命中更长前缀
- 你不用在一个 `server` 块里堆很多 `location`
- 路由优先级从规则本身就能读出来

### 2.4 静态站点

nginx：

```nginx
server {
    listen 80;
    server_name www.example.com;

    root /srv/www;
    index index.html;
}
```

proxysss：

```yaml
services:
  static_sites:
    - name: homepage
      hosts: ["www.example.com"]
      root_dir: "/srv/www"
      index_files: ["index.html"]
```

适合：

- 官网
- 文档站
- 小型下载站

与 nginx 迁移时要注意：

- proxysss 会把静态文件纳入 warm-up 和 `traffic_profile`
- 如果你同时还有 API，不需要再拆第二套配置格式

### 2.5 自定义 `header`

nginx：

```nginx
location /api/ {
    proxy_pass http://127.0.0.1:8080/;
    proxy_set_header X-Tenant-Id tenant-a;
    proxy_set_header X-Env production;
}
```

proxysss：

```yaml
services:
  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["api.example.com"]
          path_prefix: "/api"
        strip_prefix: true
        upstreams:
          - url: "http://127.0.0.1:8080"
        set_headers:
          x-tenant-id: tenant-a
          x-env: production
```

如果要删 `header`：

```yaml
        strip_headers:
          - x-legacy-header
          - x-debug-token
```

### 2.6 自动 HTTPS

nginx 常见做法通常是外部申请证书或自己管理 `certbot`。

proxysss 推荐：

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  tls:
    auto_https:
      domains: [wss.example.com]
```

只给域名就会启用免费的内建 managed ACME，默认正式环境 TLS-ALPN-01 与 ECDSA P-256；无需 `certbot`、`acme.sh`、DNS API 或邮箱。域名必须解析到本机且公网 443 可达。`email` 是可选通知地址；极老旧客户端可设 `http.tls.acme.key_algorithm: rsa2048`，显式 `challenge: http01`（需 80）、`tls_alpn01` 和 DNS-01 配置仍完全支持。

如果你做的是泛域名：

```yaml
http:
  tls:
    mode: acme_managed
    acme:
      enabled: true
      email: "ops@example.com"
      challenge: dns01
      key_algorithm: ecdsa_p256
      dns:
        provider: cloudflare
        credentials:
          api_token: "REDACTED"
```

## 3. 高手迁移：不只是普通 HTTP

### 3.1 用 `domain_routes` 组织整站

当你的 nginx 已经变成“一个域名下混静态、API、下载、管理后台”的结构，可以切成站点级编排：

```yaml
services:
  domain_routes:
    - domain: "example.com"
      routes:
        - path_prefix: "/"
          service: static:homepage
        - path_prefix: "/api"
          service: reverse_proxy:api

  static_sites:
    - name: homepage
      hosts: ["example.com"]
      root_dir: "./www"

  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["example.com"]
          path_prefix: "/api"
        upstreams:
          - url: "http://127.0.0.1:8080"
```

### 3.2 upstream 集群迁移

```yaml
services:
  reverse_proxy:
    routes:
      - name: api-cluster
        match:
          hosts: ["api.example.com"]
          path_prefix: "/"
        upstreams:
          - url: "http://10.0.0.11:8080"
            weight: 3
          - url: "http://10.0.0.12:8080"
            weight: 2
        load_balance:
          strategy: weighted_round_robin
          active_health:
            path: "/healthz"
            interval_secs: 5
            timeout_ms: 1200
```

这对应 nginx 中的 `upstream` + 健康管理诉求，但 proxysss 把它放在单条路由旁边，更适合人读。

### 3.3 `stream` 迁到 TCP / UDP

TCP：

```yaml
tcp:
  listeners:
    - name: postgres
      bind: 0.0.0.0:5432
      nodelay: true
      connect_timeout_ms: 2000
      routes:
        - name: main
          upstreams:
            - addr: "10.0.0.12:5432"
```

UDP：

```yaml
udp:
  listeners:
    - name: realtime
      bind: 0.0.0.0:7001
      session_ttl_secs: 45
      max_associations: 150000
      routes:
        - name: zone-a
          upstreams:
            - addr: "10.0.1.10:7001"
```

### 3.4 SSE / AI 代理迁移

如果你在 nginx 上已经做了 `OpenAI-compatible` / `New API` / `SSE` 代理，建议切到 `services.ai_proxy`：

```yaml
runtime:
  performance:
    enabled: true
    traffic_profile: small

services:
  ai_proxy:
    routes:
      - name: llm-edge
        match:
          hosts: ["ai.example.com"]
          path_prefix: "/v1"
        upstreams:
          - url: "https://api.openai.com"
        transport:
          flush_interval_ms: 0
          tcp_nodelay: true
```

原因很直接：

- 流式输出对 flush、`tcp_nodelay`、keepalive 更敏感
- 后续性能优化必须看 mixed-load，而不是单独挑一条 SSE 压测图；涉及公网/NIC 延迟时再用 `scripts/benchmark-cross-host-wss.sh` 在独立 client/gateway/backend 主机保存原始证据

## 4. 迁移时最容易踩的坑

### 4.1 把 `domain_routes` 和 `reverse_proxy.routes` 当成互斥关系

它们不是二选一：

- 简单代理用 `reverse_proxy.routes`
- 站点级编排再用 `domain_routes`

### 4.2 只迁 YAML 语法，不迁验证流程

真正的迁移不是“配置能启动就算结束”，而是：

```bash
proxysss tune linux --apply
proxysss config explain
proxysss config routes
proxysss config nginx-parity --format yaml
scripts/benchmark-ubuntu24-amd64-docker.sh
```

### 4.3 只挑最强单场景 benchmark

proxysss 的迁移证明应该看 Linux mixed-load：CDN/static、reverse proxy、generic SSE、WebSocket、TCP、UDP 与透明 QCP 同时跑，而不是 cherry-pick 一条最漂亮的图。New API provider 和 KCP 专用封装不放进 nginx 公平对标矩阵。

## 5. 一句迁移建议

如果你今天就要从 `nginx` 迁到 `proxysss`：

1. 先把一条 `reverse_proxy.routes` 跑通
2. 再拆静态站点、API、AI proxy、TCP/UDP
3. 最后补健康检查、缓存、TLS 和 benchmark

这样最稳，也最容易发现有没有副作用回归。

## CDN 回源、安全下载与幂等验证

`autoindex on/off` 对应 `static_sites[].autoindex`，`index` 对应 `index_files`，`root` 对应 `root`；目录索引仅提供 HTML。源站令牌和 HMAC 签名用 `security` 配置，不兼容 nginx secure_link 的签名格式。

完整说明：[CDN 回源与安全下载](docs/CDN-ORIGIN.md)；面向人的入口：[HTML 文档](docs/cdn-origin.html)。使用 `static-sign --site cdn --path /assets/file.bin --ttl-secs 120` 签发短期 URL，密钥从 YAML 读取。

`test.cmd` 连续验证两轮；临时数据和最新报告放在 `.tmp/`，依赖放在 `.cache/`，编译缓存放在 `target/`，本地包固定为 `dist/proxysss-local.zip`，重复运行覆盖并清理 staging。


## 安全开关与跨系统性能适配

`proxysss config security` 输出安全开关、默认值和建议；CDN 的 enabled/origin_token_enabled/allowed_peers_enabled 与 FTP 各类策略支持独立停用并保留参数。`config performance` 探测 Windows IOCP、macOS kqueue、Linux epoll 与实际 socket 能力。Windows/macOS 保持现有调度并按系统适配 socket，Linux 保留独立数据运行时并继续发行版/CPU 自适应。runtime.performance 只在启动时应用，变更需重启。Docker 验证固定命名 `proxysss-verify`，前后清理同名项目容器，覆盖项目内报告。完整说明见 [安全与性能指南](docs/SECURITY-PERFORMANCE.md)。


## 管理界面与内置文档

管理后台默认位于 `http://127.0.0.1:7777/`，使用中文导航、状态概览、表格与表单；原始 JSON 放在可折叠的诊断区。登录、会话、配置脱敏和默认只读限制仍由 Rust 管理 API 执行。管理监听器也提供 `/docs` 与 `/docs.html`，与公共监听器使用同一个内置文档模板。模板和 CSS/JavaScript 编译进单个二进制，不依赖外部前端运行时。公开 HTML 文档与内置文档共用排版与复制按钮。

`README.md` 为英文入口，`README-CN.md` 为对应中文入口。Windows 后台安装使用隐藏启动器与登录自启动；本机管理界面仅监听 loopback，公共 80 首页保持简洁的欢迎文字和 GitHub / GitHub Docs 两个链接。
