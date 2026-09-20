# proxysss 配置指南

proxysss 的配置目标不是“把 nginx 指令重新拼一遍”，而是让你先看懂，再改对，再上线。

这份文档按两条路线写：

- `新手路线`：先复制能跑的 YAML，再理解每一段为什么这样写。
- `高手路线`：直接定位 `domain_routes`、`reverse_proxy.routes`、`ai_proxy`、`tcp.listeners`、`udp.listeners`、TLS、热重载和性能边界。

以下片段按需合并到同一个 `proxysss.yaml`，合并同名映射而不是重复声明 `services`、`http` 等顶层键。Markdown 与 HTML 的 18 个示例使用真实配置类型校验并逐字段回读，避免无效字段静默失效；占位域名、地址、目录和凭据仍需按部署环境替换。

所有例子都基于产品默认值：

- 公网 HTTP 默认端口 `80`
- 公网 HTTPS / HTTP2 / HTTP3 默认端口 `443`
- 管理面默认 `127.0.0.1:7777`
- 默认配置文件 `proxysss.yaml`
- 未配置用户根路由时，`/` 返回只含 GitHub 与 GitHub Docs 链接的 `Welcome to proxysss` 页面
- 默认 443 TLS 会生成缺省自签证书，普通客户端 ALPN 优先协商 HTTP/2；生产环境应改为 managed ACME 或显式证书

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
        hosts: [app.example.com]
        path_prefix: /
        upstream: http://127.0.0.1:9000
```

这段配置做了什么：

- `http.plain_bind` 让 proxysss 在 80 端口接收 HTTP 请求。
- `hosts` 表示只有访问 `app.example.com` 才命中这条规则。
- `path_prefix: "/"` 表示根路径及其所有子路径都转发给后端。
- `upstream` 是你的源站地址，支持 HTTP 或 HTTPS；多后端使用字符串数组 `upstreams`。

常见错误：

- 源站只监听了错误的地址，结果 proxysss 连不上。
- 域名没有解析到这台机器，浏览器永远打不到网关。
- `hosts` 写的是 `example.com`，但你实际访问的是 `www.example.com`。

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

- 只要 `auto_https.domains` 非空，proxysss 就自动切到免费的内建 `acme_managed`，默认在 Let's Encrypt 正式环境使用 TLS-ALPN-01 与 ECDSA P-256 证书密钥：A/AAAA 指向网关且公网 443 可达即可，不需要额外证书工具、DNS API 或邮箱。ECDSA P-256 握手开销更小，是推荐稳定默认；只有兼容极老旧客户端时才设置 `http.tls.acme.key_algorithm: rsa2048`。显式 `challenge: http01` 仍完整兼容（需要公网 80），`tls_alpn01` 与 DNS-01 入口也保持不变。
- 不用额外跑 `certbot`、`acme.sh` 或云厂商 CLI，也不必填写邮箱。邮箱是可选项：填写 `http.tls.auto_https.email` 才会收到到期和安全通知。
- WebSocket upgrade 路由照常声明；签证完成后同一条 `/ws` 自动同时支持 `ws://` 与 `wss://`。

上线前请确认：

- 公网 443 已放行，TLS-ALPN-01 能直接到达网关。
- `app.example.com` 已解析到当前网关。
- 这台机器没有别的程序先占用了 80 / 443。
- 只有显式使用 HTTP-01 时才要求公网 80 的 `/.well-known/acme-challenge/` 到达网关；泛域名或无法让验证流量直达网关时使用内建 DNS-01。

### 1.3 静态资源和 API 按路径分开

适用场景：`/assets` 走静态目录，`/api` 继续走源站服务。当前 `static_sites` 按路径匹配，不支持 `hosts`；不要用无效字段假装实现了静态虚拟主机隔离。

```yaml
http:
  plain_bind: 0.0.0.0:80

services:
  static_sites:
    - name: homepage
      path_prefix: /assets
      root: ./www
      index_files: ["index.html"]

  reverse_proxy:
    routes:
      - name: api
        hosts: [api.example.com]
        path_prefix: /api
        upstream: http://127.0.0.1:8080
        strip_prefix: true
```

为什么这比“把所有东西都塞进一个服务里”更清晰：

- 静态文件和 API 分别声明，排障时不会混淆。
- 静态站点能参与 proxysss 的热文件缓存和 warm-up。
- 你可以独立给 API 增加限流、健康检查、缓存和 SSE 优化。

## 2. 路由面怎么选

### 2.1 `services.reverse_proxy.routes` 适合什么

适合：普通网站、API、后台系统、下载站、SSE、WebSocket、HTTP/2 gRPC 透传。

你会最常用到这些字段：

- `hosts`、`path_prefix`
- `upstream`、`upstreams`、`upstream_weights`
- `active_health`（路由覆盖），根 `load_balance`（算法、重试和健康默认值）
- `compression`、`cache`
- `rate_limit`

如果你只是在做“HTTP 请求 -> HTTP 后端”的代理，大多数时候就选这个面。

### 2.2 `services.domain_routes` 适合什么

适合：按域名与路径声明 HTTP/WebSocket 上游，并可设置路由证书策略；每条路由直接指定上游，不引用静态或 WebDAV 服务名。

```yaml
services:
  domain_routes:
    - name: main-api
      domains: [example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      strip_prefix: true
    - name: game-ws
      domains: [example.com]
      path_prefix: /ws
      upstream: ws://127.0.0.1:9000
```

理解方式：

- `domains` 是这条代理路由匹配的域名列表。
- `/api` 去掉前缀后转到 HTTP 后端，`/ws` 保留路径转到 WebSocket 后端。
- 静态文件和 WebDAV 分别使用自己的配置面，不支持这里的嵌套 `routes` 或 `service: static:...` 写法。

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
    enabled: true
    routes:
      - name: llm-edge
        provider: new-api
        match_host: ai.example.com
        path_prefix: /v1
        upstream: http://127.0.0.1:3000
        rewrite_base_path: /v1
        forward_headers: true
        emit_metadata_headers: true
```

为什么这份配置适合 SSE：

- `traffic_profile: small` 更偏向小包、频繁 flush、HTTP2/SSE/TCP/UDP 的交互式流量。
- 内建 SSE 转发按下游需求读取，不等待完整事件再发送。TCP_NODELAY 由运行时启用，AI route 没有 `transport.flush_interval_ms` 等字段。
- `match_host` 和 `path_prefix` 匹配入口，`upstream` 是单个 New API/sub2api/兼容 API 服务；不同供应商的账号、密钥和模型选择由该服务管理。通用多上游权重与健康检查使用 `services.reverse_proxy.routes` 和根 `load_balance` 配置。
- 原始 SSE 转 TLS/HTTP2 时解码 chunked，按终止块或 Content-Length 结束，不等 keep-alive 连接断开。分块行上限 16 KiB、trailer 总上限 64 KiB；trailer 不作为事件数据转发。提前 EOF 报错，已有 gzip 等 Content-Encoding 保留。

建议：

- 做性能优化时不要只盯单路 SSE，要按项目要求跑 mixed-load 压测。
- 一切优化都要保持无副作用，不能让 SSE 快了却把静态、TCP、UDP 或 reload 打差。

### 2.4 静态站点

适合：HTML、CSS、JS、图片、字体、音频、视频、安装包、CDN 回源静态对象。

`services.static_sites` 会处理 `GET` / `HEAD`、index 文件、可选目录列表、小文件热缓存和大文件流式发送。下载支持单段及多段 `Range: bytes=...`，成功返回 `206 Partial Content`；合法但全部越界的区间返回 `416`，语法无效的 Range 忽略并返回完整表示。完整条件请求与分段规则见 [CDN 下载](CDN-ORIGIN.md)。

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

该片段声明发现映射；部署时还需创建名为 `api` 的反代目标。也可把 TCP/UDP listener 的后端池交给注册中心自动化维护。业务判定仍然放在插件或上游服务里，proxysss 保持通用网关身份。

```yaml
runtime:
  performance:
    enabled: true
    traffic_profile: balanced

services:
  static_sites:
    - name: docs
      path_prefix: /docs-site
      root: ./site
      index_files: ["index.html"]
      cache_control: "public, max-age=300"
```

重点理解：

- `traffic_profile: balanced` 适合既有首页小文件，也有较大下载资源的站点。
- 静态资源会参与配置加载后的 warm-up。
- 如果你的发布流量明显偏大文件，可以改成 `bulk`；如果主要是首页、小图标、小 JS，就优先 `small`。

### 2.6 WebDAV

```yaml
services:
  webdav:
    enabled: true
    path_prefix: /dav
    root: ./dav-data
    allow_write: false
```

适合：挂载设计资源、共享构建产物、轻量团队文件协作。

要点：

- WebDAV 是文件协作入口，不等于对象存储。
- 这个配置面没有 `auth.basic`。写操作需在可信网络、外部认证入口或明确的扩展认证之后开启；需要内建密码与文件管理时查看 `services.filecloud`。
- 如果你只想分发文件，不要默认启用写权限。

### 2.7 TCP 直通

```yaml
tcp:
  listeners:
    - name: postgres
      bind: 0.0.0.0:5432
      nodelay: true
      connect_timeout_ms: 2000
      upstream: 10.0.0.12:5432
```

这类配置适合：

- PostgreSQL / MySQL / Redis / MongoDB 这类纯 TCP 服务
- 游戏长连接网关
- 你不想让业务协议被 HTTP 层感知的场景

### 2.8 UDP、KCP-style、QCP、CoAP-style

```yaml
udp:
  listeners:
    - name: kcp-gateway
      bind: 0.0.0.0:4000
      session_ttl_secs: 120
      max_associations: 200000
      protocol: kcp
      upstream: 10.0.0.20:4000
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

### 2.9 MQTT / IoT

```yaml
services:
  reverse_proxy:
    routes:
      - name: mqtt-ws
        hosts: [iot.example.com]
        path_prefix: /mqtt
        upstream: http://127.0.0.1:8083

tcp:
  listeners:
    - name: mqtt-tcp
      bind: 0.0.0.0:1883
      nodelay: true
      upstream: 10.0.0.30:1883

udp:
  listeners:
    - name: coap
      bind: 0.0.0.0:5683
      session_ttl_secs: 30
      max_associations: 50000
      upstream: 10.0.0.31:5683
```

这一组配置表示：

- MQTT over WebSocket 走 HTTP 面。
- MQTT 原生 TCP 走 `tcp.listeners`。
- CoAP-style UDP 走 `udp.listeners`。
- proxysss 是边缘网关，不是 broker。

## 3. 高级用户最常用的组合

### 3.1 反向代理 + 权重 + 健康检查 + 缓存 + 限流

```yaml
load_balance:
  algorithm: weighted
  active_health:
    enabled: true
    interval_secs: 5
    timeout_ms: 1200
    path: /healthz

services:
  reverse_proxy:
    routes:
      - name: api-cluster
        hosts: [api.example.com]
        path_prefix: /
        upstream: http://10.0.0.11:8080
        upstreams: [http://10.0.0.11:8080, http://10.0.0.12:8080]
        upstream_weights:
          "http://10.0.0.11:8080": 3
          "http://10.0.0.12:8080": 2
        cache:
          enabled: true
          behavior: respect_origin
          ttl_secs: 30
          stale_while_revalidate_secs: 30
          stale_if_error_secs: 120
        rate_limit:
          enabled: true
          requests: 200
          window_ms: 1000
          burst: 400
```

逐段解释：

- `upstream_weights` 配合根 `algorithm: weighted` 控制流量倾斜，不是绝对配额。
- `active_health` 会主动剔除坏节点，不是被动等请求失败。
- `respect_origin` 尊重源站是否允许缓存；边缘存储时长使用 `ttl_secs`，并非自动采用源站 max-age。仅为可共享响应启用缓存。
- `stale_*` 让缓存层在后端抖动时更稳。
- `rate_limit` 用于削峰，不是代替鉴权。

### 3.2 Wildcard TLS + DNS-01

```yaml
http:
  tls_bind: 0.0.0.0:443
  tls:
    mode: acme_managed
    acme:
      domains: [example.com, "*.example.com"]
      email: "ops@example.com"
      challenge: dns01
      key_algorithm: ecdsa_p256 # 推荐默认；兼容极老旧客户端时改 rsa2048
      dns:
        provider: cloudflare
        credentials:
          api_token: "REDACTED"
```

适用场景：

- `*.example.com` 这类泛域名证书
- 不方便走 HTTP-01 的多节点或内网入口

注意：

- `credentials` 是敏感配置；保护文件权限并安全分发，不要把真实 token 放进仓库。示例中的占位符不能用于真实签证，普通配置展示应脱敏。
- 云厂商 provider 是内建策略工厂的一部分，不要混用旧的外部 `acme.sh` 方式，除非目标厂商还没有内建支持。

### 3.3 管理面 + 自动化

```yaml
admin:
  enabled: true
  bind: 127.0.0.1:7777
  enable_write_ops: true
  expose_config: false
  bearer_token: "REPLACE-WITH-A-LONG-RANDOM-TOKEN"
```

建议理解为：

- `bind: 127.0.0.1:7777` 是默认安全姿势。
- `enable_write_ops` 只有在你明确要做自动化改配时才开启。
- 用 `proxysss token set` 设置真实随机 token，用 `proxysss token show` 本地检查；其他配置展示使用脱敏输出。不要沿用示例占位符。

### 3.4 手机/桌面只读监控面

给 iPhone、iPad、macOS 和后续 Android 客户端使用的不是管理员密码，而是独立的 `admin.monitor.password`。它只在主 HTTPS listener 下提供查询接口：

```yaml
admin:
  monitor:
    enabled: true
    path_prefix: /_proxysss/monitor
    password: "至少 12 位的独立监控密码"
    session_ttl_secs: 43200
    hosts: [monitor.example.com]
```

客户端先调用：

```bash
curl -X POST https://monitor.example.com/_proxysss/monitor/v1/login \
  -H 'Content-Type: application/json' \
  -d '{"password":"至少 12 位的独立监控密码"}'
```

然后使用返回的 `monitor:read` Bearer token 查询：

```text
GET /_proxysss/monitor/v1/health
GET /_proxysss/monitor/v1/summary
GET /_proxysss/monitor/v1/stats
GET /_proxysss/monitor/v1/ssl
```

只读监控面拒绝所有非 GET 查询方法，不能使用管理员 bearer token，也不会返回配置文件路径、证书私钥路径或 DNS 凭据。生产环境必须使用真实 HTTPS 证书；不要把这个密码放进 Git、日志、剪贴板或 iCloud 普通数据。

### 3.5 游戏长连接 + UDP 同时接入

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      nodelay: true
      connect_timeout_ms: 1500
      upstream: 10.0.1.10:7000

udp:
  listeners:
    - name: game-udp
      bind: 0.0.0.0:7001
      session_ttl_secs: 45
      max_associations: 150000
      upstream: 10.0.1.10:7001
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
- `admin.monitor.path_prefix`、`admin.monitor.password`、`admin.monitor.hosts`

需要重启：

- `http.plain_bind`
- `http.tls_bind`
- `http.h3_bind`
- `admin.bind`
- TCP / UDP listener bind 集合
- `http.tls.mode`
- 日志路径和级别核心设置

### 4.3 Windows 后台自启动

Windows 下执行 `service install` 会优先写入当前用户的 HKCU Run，并通过 `wscript //B //Nologo` 隐藏启动 proxysss；这样登录时不会弹出 cmd。安装时还会清理旧版本留下的直接执行型计划任务，避免两个入口重复启动。

```powershell
proxysss service install
proxysss service status
proxysss service start
proxysss service stop
```

如果旧计划任务曾经以 `HighestAvailable` 权限创建，普通 PowerShell 可能无法删除它；只需用“以管理员身份运行”的 PowerShell 执行一次 `proxysss service install`，之后保持 HKCU Run 单一入口即可。`service status` 会提示重复入口或仍有弹窗风险的旧任务。

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

默认 GitHub Actions CI 只负责全平台二进制打包，禁止自动或手动 workflow 运行测试、smoke 或性能压测。上面的 benchmark 在本机 Docker 或原生 amd64 Docker 运行，硬校验容器为 Ubuntu 24.04 x86_64；它在容器内构建当前 checkout，并默认把 HTTP/HTTPS/static/SSE/WebSocket/TCP/UDP/透明 QCP 一起按 1x/2x/4x 放大，逐档要求零错误和严格 `>1.0`。arm64 daemon 的结果会标记 `emulated-amd64`，不能冒充物理 x86。严格入口默认把检测到的 CPU 按约 1/4 网关、1/4 后端、其余客户端分配，并拒绝 cpuset 重叠，物理网络 WSS 结论还要从独立 client 主机运行 `scripts/benchmark-cross-host-wss.sh`：它会检查三台机器的 machine-id 不同，并用远端 systemd cgroup 强制网关 `4 CPU / 300k nofile`，保存同 SHA、`nginx -V`、cgroup memory current/peak、每连接成本、主机指纹和原始样本。内存上限仅在声明生产预算时通过 `GATEWAY_MEMORY_MAX` 设置，默认不以固定物理 RAM 拒绝结果。v1.3.5 UDP fast path 的历史专项结果是 `udp-stream 4.045x`：`proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`，两边 0 错误。

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
- `balanced`：静态大文件与实时连接混合负载的可选配置，须按实际混合矩阵验证；产品默认仍为 `small`。
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

项目要求已经明确：后续所有性能优化都要压测，而且要无副作用。generic SSE、静态、HTTP reverse proxy、TCP、UDP 都应该一起看。通用发布矩阵默认不包含特殊协议，严格 Ubuntu 24 Docker 矩阵另含透明 QCP；这些测试验证 UDP 转发，不代表 proxysss 或 nginx 终止 KCP/QCP 帧。

### 7.3 管理面一上来就开公网写操作

默认的 loopback + `enable_write_ops=false` 是刻意设计的安全姿势。自动化需要时再显式开启。

### 7.4 把管理员密码直接给手机

手机/桌面客户端只应使用 `admin.monitor` 的独立密码。它绑定 `monitor:read` 会话和固定查询接口，即使服务器管理员为了自动化开启了写操作，监控凭据也不能改路由、证书、用户或配置。

## 8. 继续往下看什么

如果你刚跑起来：

- 先读 `README.md`
- 再看 `docs/ARCHITECTURE.md`
- 然后按本文档的场景段落复制配置

如果你已经在做正式网关：

- 去看 `docs/ARCHITECTURE.md`
- 去看 `nginx-to-proxysss.md`
- 做 Linux 调优和 mixed-load benchmark

## CDN 回源、安全下载与幂等验证

`static_sites` 新增 `hide_dotfiles`（默认 true）、`autoindex_max_entries`（默认 1000）、`cache_control`、`security.origin_token`、`security.allowed_peers`、`security.signed_url` 和 `rate_limit`。

完整说明：[CDN 回源与安全下载](CDN-ORIGIN.md)；面向人的入口：[HTML 文档](cdn-origin.html)。使用 `static-sign --site cdn --path /assets/file.bin --ttl-secs 120` 签发短期 URL，密钥从 YAML 读取。

`test.cmd` 连续验证两轮；临时数据和最新报告放在 `.tmp/`，依赖放在 `.cache/`，编译缓存放在 `target/`，本地包固定为 `dist/proxysss-local.zip`，重复运行覆盖并清理 staging。


## 安全开关与跨系统性能适配

`proxysss config security` 输出安全开关、默认值和建议；CDN 的 enabled/origin_token_enabled/allowed_peers_enabled 与 FTP 各类策略支持独立停用并保留参数。`config performance` 探测 Windows IOCP、macOS kqueue、Linux epoll 与实际 socket 能力。Windows/macOS 保持现有调度并按系统适配 socket，Linux 保留独立数据运行时并继续发行版/CPU 自适应。runtime.performance 只在启动时应用，变更需重启。Docker 验证固定命名 `proxysss-verify`，前后清理同名项目容器，覆盖项目内报告。完整说明见 [安全与性能指南](SECURITY-PERFORMANCE.md)。


## 管理界面与内置文档

管理后台默认位于 `http://127.0.0.1:7777/`，使用中文导航、状态概览、表格与表单；原始 JSON 放在可折叠的诊断区。登录、会话、配置脱敏和默认只读限制仍由 Rust 管理 API 执行。管理监听器也提供 `/docs` 与 `/docs.html`，与公共监听器使用同一个内置文档模板。模板和 CSS/JavaScript 编译进单个二进制，不依赖外部前端运行时。公开 HTML 文档与内置文档共用排版与复制按钮。

`README.md` 为英文入口，`README-CN.md` 为对应中文入口。Windows 后台安装使用隐藏启动器与登录自启动；本机管理界面仅监听 loopback，公共 80 首页保持简洁的欢迎文字和 GitHub / GitHub Docs 两个链接。

## 原生复杂下载

静态服务支持单段/多段 Range（最多 16 段）、重叠合并、有界流式读取、HEAD 与条件请求优先级，无需脚本钩子；反代下载由上游决定表示语义。详见 [CDN 下载说明](CDN-ORIGIN.md)。
