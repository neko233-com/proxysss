# proxysss Configuration Guide

proxysss 的配置目标不是“把 nginx 指令重新拼一遍”，而是让你先看懂，再改对，再上线。

这份文档按两条路线写：

- `新手路线`：先复制能跑的 YAML，再理解每一段为什么这样写。
- `高手路线`：直接定位 `domain_routes`、`reverse_proxy.routes`、`ai_proxy`、`tcp.listeners`、`udp.listeners`、TLS、热重载和性能边界。

所有例子都基于产品默认值：

- 公网 HTTP 默认端口 `80`
- 公网 HTTPS / HTTP2 / HTTP3 默认端口 `443`
- 管理面默认 `127.0.0.1:7777`
- 默认配置文件 `proxysss.yaml`

## 1. 新手先从这里开始

### 1.1 第一个可工作的反向代理

适用场景：你只想先把一个站点从 `127.0.0.1:9000` 代理出来。

```yaml
http:
  plain_bind: 0.0.0.0:80

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

这段配置做了什么：

- `http.plain_bind` 让 proxysss 在 80 端口接收 HTTP 请求。
- `match.hosts` 表示只有访问 `app.example.com` 才命中这条规则。
- `path_prefix: "/"` 表示根路径及其所有子路径都转发给后端。
- `upstreams.url` 是你的源站地址，支持 HTTP 或 HTTPS。

常见错误：

- 源站只监听了错误的地址，结果 proxysss 连不上。
- 域名没有解析到这台机器，浏览器永远打不到网关。
- `match.hosts` 写的是 `example.com`，但你实际访问的是 `www.example.com`。

### 1.2 只填域名就给 WebSocket 加 WSS

适用场景：你已经能跑 HTTP，现在要让浏览器通过 `wss://app.example.com` 连接同一条 WebSocket 路由。

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  tls:
    auto_https:
      domains: [app.example.com]

services:
  domain_routes:
    - name: game-wss
      domains: [app.example.com]
      path_prefix: /ws
      upstream: ws://127.0.0.1:9000
```

这段配置做了什么：

- 只要 `auto_https.domains` 非空，proxysss 就自动切到内建 `acme_managed`，默认在 Let's Encrypt 正式环境使用 TLS-ALPN-01：A/AAAA 指向网关且公网 443 可达即可，不需要额外证书工具、DNS API 或邮箱。显式 `challenge: http01` 仍完整兼容（需要公网 80），`tls_alpn01` 与 DNS-01 入口也保持不变。
- 不用额外跑 `certbot`、`acme.sh` 或云厂商 CLI，也不必填写邮箱。邮箱是可选项：填写 `http.tls.auto_https.email` 才会收到到期和安全通知。
- WebSocket upgrade 路由照常声明；签证完成后同一条 `/ws` 自动同时支持 `ws://` 与 `wss://`。

上线前请确认：

- 80 和 443 都已放行。
- `app.example.com` 已解析到当前网关。
- 这台机器没有别的程序先占用了 80 / 443。
- 80 端口不能被 CDN、负载均衡器或另一台网关截断 `/.well-known/acme-challenge/`；需要泛域名、多入口或无法开放 80 时，改用内建 DNS-01。

### 1.3 一个站点放静态文件，另一个站点做代理

适用场景：官网走静态目录，API 继续走源站服务。

```yaml
http:
  plain_bind: 0.0.0.0:80

services:
  static_sites:
    - name: homepage
      hosts: ["www.example.com"]
      root_dir: "./www"
      index_files: ["index.html"]

  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["api.example.com"]
          path_prefix: "/"
        upstreams:
          - url: "http://127.0.0.1:8080"
```

为什么这比“把所有东西都塞进一个服务里”更清晰：

- 静态文件和 API 分别声明，排障时不会混淆。
- 静态站点能参与 proxysss 的热文件缓存和 warm-up。
- 你可以独立给 API 增加限流、健康检查、缓存和 SSE 优化。

## 2. 路由面怎么选

### 2.1 `services.reverse_proxy.routes` 适合什么

适合：普通网站、API、后台系统、下载站、SSE、WebSocket、HTTP/2 gRPC 透传。

你会最常用到这些字段：

- `match.hosts`
- `match.path_prefix`
- `upstreams`
- `load_balance`
- `response_policy`
- `rate_limit`

如果你只是在做“HTTP 请求 -> HTTP 后端”的代理，大多数时候就选这个面。

### 2.2 `services.domain_routes` 适合什么

适合：你想按域名统一声明站点入口，再把不同路径分流到不同服务。

```yaml
services:
  domain_routes:
    - domain: "example.com"
      routes:
        - path_prefix: "/"
          service: static:homepage
        - path_prefix: "/api"
          service: reverse_proxy:main-api

  static_sites:
    - name: homepage
      hosts: ["example.com"]
      root_dir: "./www"

  reverse_proxy:
    routes:
      - name: main-api
        match:
          hosts: ["example.com"]
          path_prefix: "/api"
        upstreams:
          - url: "http://127.0.0.1:8080"
```

理解方式：

- `domain_routes` 更像站点总路由表。
- `reverse_proxy.routes`、`static_sites`、`webdav` 等是真正的能力面。
- 想做多服务统一编排时，用 `domain_routes` 会更直观。

### 2.3 AI / SSE / OpenAI-compatible 代理

适用场景：New API、OpenAI-compatible API、SSE 流式输出、长连接推理响应。

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443

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
          - url: "https://api.deepseek.com"
        load_balance:
          strategy: latency_first
          active_health:
            interval_secs: 5
            timeout_ms: 1500
            path: "/v1/models"
        transport:
          flush_interval_ms: 0
          tcp_nodelay: true
          keepalive_secs: 75
```

为什么这份配置适合 SSE：

- `traffic_profile: small` 更偏向小包、频繁 flush、HTTP2/SSE/TCP/UDP 的交互式流量。
- `flush_interval_ms: 0` 让事件尽快往下游刷，减少 token 被代理层攒包。
- `tcp_nodelay: true` 降低小块流式响应等待合并的概率。
- `active_health` 让长连接代理在上游异常时更快切走。

建议：

- 做性能优化时不要只盯单路 SSE，要按项目要求跑 mixed-load 压测。
- 一切优化都要保持无副作用，不能让 SSE 快了却把静态、TCP、UDP 或 reload 打差。

### 2.4 静态站点

适合：HTML、CSS、JS、图片、字体、音频、视频、安装包、CDN 回源静态对象。

`services.static_sites` 会处理 `GET` / `HEAD`、index 文件、可选目录列表、小文件热缓存和大文件流式发送。下载类请求支持 `Range: bytes=...`，会返回 `206 Partial Content`、`Content-Range`、`Accept-Ranges: bytes`，无效区间返回 `416`，所以断点续传和播放器拖动进度条不需要额外业务服务。

```yaml
services:
  static_sites:
    - name: public-assets
      path_prefix: /assets
      root: ./public
      index_files: [index.html, index.htm]
      autoindex: false
```

### 2.5 注册中心联动

Consul、etcd、Nacos 这类注册中心属于控制面信息，不应该让普通 HTTP/TCP/UDP 热路径每次请求都去查网络。proxysss 用 `services.service_discovery` 声明 registry 和 mapping，自动化或管理面可以把发现结果刷新进同一份 `proxysss.yaml` 的 upstream pools，再热重载生效。

```yaml
services:
  service_discovery:
    enabled: true
    interval_secs: 15
    registries:
      - name: consul-main
        provider: consul
        endpoint: http://consul.service.consul:8500
      - name: etcd-main
        provider: etcd
        endpoint: http://etcd.default.svc.cluster.local:2379
      - name: nacos-main
        provider: nacos
        endpoint: http://nacos.default.svc.cluster.local:8848
    mappings:
      - name: api-from-consul
        registry: consul-main
        service: spring-api
        target: reverse_proxy_route
        target_name: api
        scheme: http
```

这能覆盖 Spring Cloud Gateway、Kong、APISIX 前面的统一入口，也能把 TCP/UDP listener 的后端池交给注册中心自动化维护。业务判定仍然放在插件或上游服务里，proxysss 保持通用网关身份。

```yaml
runtime:
  performance:
    enabled: true
    traffic_profile: balanced

services:
  static_sites:
    - name: docs
      hosts: ["docs.example.com"]
      root_dir: "./site"
      index_files: ["index.html"]
      not_found_page: "404.html"
      cache:
        strategy: override
        edge_ttl_secs: 300
```

重点理解：

- `traffic_profile: balanced` 适合既有首页小文件，也有较大下载资源的站点。
- 静态资源会参与配置加载后的 warm-up。
- 如果你的发布流量明显偏大文件，可以改成 `bulk`；如果主要是首页、小图标、小 JS，就优先 `small`。

### 2.5 WebDAV

```yaml
services:
  webdav:
    - name: assets-dav
      hosts: ["dav.example.com"]
      path_prefix: "/"
      root_dir: "./dav-data"
      read_only: false
      auth:
        basic:
          - username: "editor"
            password: "change-me"
```

适合：挂载设计资源、共享构建产物、轻量团队文件协作。

要点：

- WebDAV 是文件协作入口，不等于对象存储。
- 对公网开放时，至少要加认证和访问控制。
- 如果你只想分发文件，不要默认启用写权限。

### 2.6 TCP 直通

```yaml
tcp:
  listeners:
    - name: postgres
      bind: 0.0.0.0:5432
      nodelay: true
      connect_timeout_ms: 2000
      routes:
        - name: postgres-main
          upstreams:
            - addr: "10.0.0.12:5432"
```

这类配置适合：

- PostgreSQL / MySQL / Redis / MongoDB 这类纯 TCP 服务
- 游戏长连接网关
- 你不想让业务协议被 HTTP 层感知的场景

### 2.7 UDP、KCP-style、QCP、CoAP-style

```yaml
udp:
  listeners:
    - name: kcp-gateway
      bind: 0.0.0.0:4000
      session_ttl_secs: 120
      max_associations: 200000
      routes:
        - name: kcp-upstream
          upstreams:
            - addr: "10.0.0.20:4000"
    - name: qcp-gateway
      bind: 0.0.0.0:4001
      protocol: qcp
      session_ttl_secs: 120
      max_associations: 200000
      upstreams:
        - 10.0.0.21:4001
```

理解方式：

- `session_ttl_secs` 控制 UDP 会话保活窗口。
- `max_associations` 控制实时设备或玩家会话上限，避免无界膨胀。
- KCP 和 QCP 是两套独立 UDP listener：KCP 用 `protocol: kcp`，neko233-com/QCP 用 `protocol: qcp`。
- QCP 走透明 UDP 转发；QCP 帧、可靠性和业务语义仍由上游服务处理。
- 这条面向的是转发和边缘治理，不是协议终端本身。

### 2.8 MQTT / IoT

```yaml
services:
  reverse_proxy:
    routes:
      - name: mqtt-ws
        match:
          hosts: ["iot.example.com"]
          path_prefix: "/mqtt"
        upstreams:
          - url: "http://127.0.0.1:8083"

tcp:
  listeners:
    - name: mqtt-tcp
      bind: 0.0.0.0:1883
      nodelay: true
      routes:
        - name: mqtt-broker
          upstreams:
            - addr: "10.0.0.30:1883"

udp:
  listeners:
    - name: coap
      bind: 0.0.0.0:5683
      session_ttl_secs: 30
      max_associations: 50000
      routes:
        - name: coap-core
          upstreams:
            - addr: "10.0.0.31:5683"
```

这一组配置表示：

- MQTT over WebSocket 走 HTTP 面。
- MQTT 原生 TCP 走 `tcp.listeners`。
- CoAP-style UDP 走 `udp.listeners`。
- proxysss 是边缘网关，不是 broker。

## 3. 高级用户最常用的组合

### 3.1 反向代理 + 权重 + 健康检查 + 缓存 + 限流

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
            interval_secs: 5
            timeout_ms: 1200
            path: "/healthz"
        cache:
          strategy: respect_origin
          stale_while_revalidate_secs: 30
          stale_if_error_secs: 120
        rate_limit:
          requests_per_second: 200
          burst: 400
```

逐段解释：

- `weight` 控制流量倾斜，不是绝对配额。
- `active_health` 会主动剔除坏节点，不是被动等请求失败。
- `respect_origin` 适合源站已经认真设置缓存头的 API 或静态服务。
- `stale_*` 让缓存层在后端抖动时更稳。
- `rate_limit` 用于削峰，不是代替鉴权。

### 3.2 Wildcard TLS + DNS-01

```yaml
http:
  tls_bind: 0.0.0.0:443
  tls:
    mode: acme_managed
    acme:
      enabled: true
      email: "ops@example.com"
      challenge: dns01
      dns:
        provider: cloudflare
        credentials:
          api_token: "REDACTED"
```

适用场景：

- `*.example.com` 这类泛域名证书
- 不方便走 HTTP-01 的多节点或内网入口

注意：

- `credentials` 要走配置文件密文或安全分发，不要把真实 token 放进示例仓库。
- 云厂商 provider 是内建策略工厂的一部分，不要混用旧的外部 `acme.sh` 方式，除非目标厂商还没有内建支持。

### 3.3 管理面 + 自动化

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  enable_write_ops: true
  expose_config: false
  token: "REDACTED"
```

建议理解为：

- `bind: 127.0.0.1:7777` 是默认安全姿势。
- `enable_write_ops` 只有在你明确要做自动化改配时才开启。
- 配置查看尽量走 `proxysss config *` CLI，避免把敏感信息直接铺给外部系统。

### 3.4 游戏长连接 + UDP 同时接入

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      nodelay: true
      connect_timeout_ms: 1500
      routes:
        - name: zone-a
          upstreams:
            - addr: "10.0.1.10:7000"

udp:
  listeners:
    - name: game-udp
      bind: 0.0.0.0:7001
      session_ttl_secs: 45
      max_associations: 150000
      routes:
        - name: zone-a-udp
          upstreams:
            - addr: "10.0.1.10:7001"
```

为什么这里要关注性能边界：

- TCP 长连接和 UDP 实时流量会拉高并发 socket 数。
- 这类场景要结合 `proxysss tune linux --apply`、系统 `LimitNOFILE`、`fs.nr_open` / `fs.file-max` 一起看。
- release gate 不是只看单项压测，而是看 mixed-load 是否仍然稳。

## 4. 热重载、预热和运维节奏

### 4.1 热重载会帮你做什么

proxysss 在配置加载和热重载后会做几件关键事：

- 重新计算路由和能力面。
- 预热热文件缓存或 sendfile 描述符。
- 预拨反向代理 / AI proxy 上游 keepalive 池。
- 重新读取 `services.service_discovery` 映射和注册中心目标池声明。
- 更新 `/healthz` 的 `warm` 状态。

这意味着：

- 压测应该在 warm-up 完成后再开始。
- 第一波线上流量不应该为“冷连接”额外付出代价。

### 4.2 哪些改动能热重载，哪些要重启

能热重载：

- 大部分合并后的配置值
- `proxysss.yaml`
- 主脚本与自动加载插件脚本
- `services.reverse_proxy.routes`
- `static_sites`
- `webdav`
- FTP upstream

需要重启：

- `http.plain_bind`
- `http.tls_bind`
- `http.h3_bind`
- `admin.bind`
- TCP / UDP listener bind 集合
- `http.tls.mode`
- 日志路径和级别核心设置

## 5. 性能建议要这样理解

### 5.1 先调 Linux，再谈结论

Linux 生产验证流程建议固定为：

```bash
proxysss tune linux --apply
proxysss config explain
proxysss config capabilities
proxysss config reload-plan
scripts/benchmark-ubuntu24-amd64-docker.sh
```

默认 GitHub Actions CI 只负责全平台二进制打包，禁止自动或手动 workflow 运行测试、smoke 或性能压测。上面的 benchmark 必须直连 Ubuntu 24.04 x86_64 Docker 专机手动运行；它在容器内构建当前 checkout，并默认把 HTTP/HTTPS/static/SSE/WebSocket/TCP/UDP/透明 QCP 一起按 1x/2x/4x 放大，逐档要求零错误和严格 `>1.0`。role-isolated 默认 4+4+8 CPU 包络会先拒绝 cpuset 重叠，物理网络 WSS 结论还要从独立 client 主机运行 `scripts/benchmark-cross-host-wss.sh`：它会检查三台机器的 machine-id 不同，并用远端 systemd cgroup 强制网关 `4 CPU / 300k nofile`，保存同 SHA、`nginx -V`、cgroup memory current/peak、每连接成本、主机指纹和原始样本。内存上限仅在声明生产预算时通过 `GATEWAY_MEMORY_MAX` 设置，默认不以固定物理 RAM 拒绝结果。v1.3.5 UDP fast path 的当前专项结果是 `udp-stream 4.045x`：`proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`，两边 0 错误。

如果要先在 Docker 里验证“全场景配置面没有退化”，运行：

```bash
scripts/verify-docker-scenarios.sh
```

Windows PowerShell：

```powershell
.\scripts\verify-docker-scenarios.ps1
```

它会在 Ubuntu 24 容器里检查 `examples/all-scenarios.example.yaml`、Range 下载、注册中心配置、能力矩阵和 nginx-parity 输出；这不是默认 GitHub CI，CI 仍然保持 packaging-only。

### 5.2 只赢一条链路，不算赢

对 proxysss 来说，合理的性能优化必须满足这些条件：

- 不引入明显副作用。
- 不把某一类流量优化成“特供冠军”。
- 至少能解释它对 HTTP、SSE、WebSocket、TCP、UDP、静态文件、reload 的影响。
- 最终以 mixed-load 压测为准，而不是 cherry-pick 单场景截图。

### 5.3 `traffic_profile` 怎么选

```yaml
runtime:
  performance:
    enabled: true
    traffic_profile: small
```

- `small`：优先小文件、SSE、HTTP2、频繁 flush、交互式流量。
- `balanced`：默认推荐，大多数“网站 + API + 一点实时流量”场景都能接受。
- `bulk`：明显偏大文件传输、下载分发、零拷贝吞吐优先。

## 6. 一组好用的排障命令

```bash
proxysss config explain
proxysss config capabilities
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
proxysss token show
```

这些命令分别解决的问题：

- `config explain`：当前配置到底生效成了什么。
- `config capabilities`：这个实例现在有哪些能力面真的开着。
- `watched-scripts`：哪些脚本会参与热重载。
- `routes`：最终路由视图。
- `reload-plan`：哪些改动能热重载，哪些需要重启。
- `nginx-parity`：当前 nginx 对照矩阵。
- `token show`：本地查看管理 token，而不是去翻 YAML 原文。

## 7. 常见误区

### 7.1 把所有问题都塞进脚本

脚本适合做业务定制，不适合替代网关本体的基础能力。HTTP/TLS/路由/限流/缓存/流量治理优先用原生配置面。

### 7.2 拿单条 SSE benchmark 当发布结论

项目要求已经明确：后续所有性能优化都要压测，而且要无副作用。generic SSE、静态、HTTP reverse proxy、TCP、UDP 都应该一起看；KCP-style 和 QCP 作为 proxysss 独立 UDP listener 能力保留，但不进入当前性能 benchmark 矩阵，也不默认拿 nginx 做协议语义对照。

### 7.3 管理面一上来就开公网写操作

默认的 loopback + `enable_write_ops=false` 是刻意设计的安全姿势。自动化需要时再显式开启。

## 8. 继续往下看什么

如果你刚跑起来：

- 先读 `README.md`
- 再看 `docs/ARCHITECTURE.md`
- 然后按本文档的场景段落复制配置

如果你已经在做正式网关：

- 去看 `docs/ARCHITECTURE.md`
- 去看 `nginx-to-proxysss.md`
- 做 Linux 调优和 mixed-load benchmark
