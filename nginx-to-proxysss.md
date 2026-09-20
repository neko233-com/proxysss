# nginx 迁移到 proxysss

本教程把 nginx 的网站、静态下载与 stream 配置迁到单个 proxysss.yaml。先确认路径和请求头语义，再接入证书、集群与长连接，最后使用实际流量验收。

## 1. 迁移前确认

先列出域名、路径、源站、TLS 证书、请求头改写和 TCP/UDP 端口，再选择配置项。示例中的域名、源站、目录和凭据需要替换为自己的值。每个 YAML 块可单独解析；组合时合并同一个根节点，不要重复声明 services 或 http。

| 迁移需求 | proxysss 配置 |
| --- | --- |
| 整站、API 前缀、WebSocket、普通 SSE | services.reverse_proxy.routes |
| 静态站点和游戏资源下载 | services.static_sites |
| 域名代理与独立证书策略 | services.domain_routes |
| 加权上游、健康检查与重试 | 路由 upstreams/upstream_weights 与根 load_balance |
| 游戏、数据库、MQTT、设备流量 | tcp.listeners / udp.listeners |
| 可选业务路由和插件 | script.entry 与 plugins.auto_load_dir |

HTTP 默认监听 80，HTTPS/H2/H3 使用 443，管理端默认 127.0.0.1:7777 且只读。TLS 监听地址不等于已经获得公开可信证书。未命中用户路由的根路径返回只含 GitHub 和 GitHub Docs 链接的欢迎页。

未知配置字段会被拒绝。反代使用 hosts/path_prefix/upstream；domain_routes 使用 domains/path_prefix/upstream；listener 直接指定 upstream 或字符串数组 upstreams。不支持嵌套 match、service 引用或 listener.routes。

## 2. 网站、API 和资源下载

### 2.1 整站与 API 前缀

在 nginx 中，带 URI 的 proxy_pass 会替换 location 匹配到的前缀，例如：

```nginx
location /api/ {
    proxy_pass http://127.0.0.1:8080/;
}
location / {
    proxy_pass http://127.0.0.1:3000;
}
```

不同 proxy_pass URI、正则 location 与 rewrite 的组合需要分别核对，参见 [nginx 官方代理文档](https://nginx.org/en/docs/http/ngx_http_proxy_module.html#proxy_pass)。

对应的 proxysss 配置把较长的 /api 前缀交给 API，其余路径交给前端：

```yaml
services:
  reverse_proxy:
    routes:
      - name: backend-api
        hosts: [app.example.com]
        path_prefix: /api
        strip_prefix: true
        upstream: http://127.0.0.1:8080
        set_headers:
          x-environment: production
        strip_headers: [x-debug-token]
      - name: frontend
        hosts: [app.example.com]
        path_prefix: /
        upstream: http://127.0.0.1:3000
```

/api/orders?limit=10 转成 /orders?limit=10；/api 本身也匹配并转成 /，/apix 不匹配。proxysss 使用路径段前缀和最长匹配，不能机械照搬原网关的正则、通配或自动重定向规则。只有整站反代时保留 frontend 一条即可。

set_headers/strip_headers 修改发往上游的请求头，不是响应头指令。默认 forward_headers 补充转发头，迁移后应在源站核对 Host、客户端地址与实际路径。WebSocket 和普通 SSE 可沿用反代路由。

### 2.2 静态站点和游戏资源

```yaml
services:
  static_sites:
    - name: game-assets
      hosts: [cdn.example.com]
      path_prefix: /assets
      root: /srv/game-assets
      index_files: [index.html]
      autoindex: false
      hide_dotfiles: true
      cache_control: public, max-age=300
```

/assets/client.zip 对应 /srv/game-assets/client.zip。多站点按 YAML 首个 hosts/path 匹配项选站；不同根目录的相同 URL 缓存隔离。根站点把 path_prefix 设为 /。目录字段为 root。

完整下载流式发送并保留背压；GET 支持单段和最多 16 段 Range，HEAD 与条件请求按 HTTP 语义处理。发布采用写新文件后原子替换，避免就地覆盖正在发送的资源。目录浏览默认关闭；回源令牌、签名 URL、授权和缓存规则见 [CDN 下载指南](docs/CDN-ORIGIN.md)。

### 2.3 自动 HTTPS 与 WSS

先让 A/AAAA 指向网关，并确保公网 443 可达。下面的声明申请证书；还需保留实际反代或静态路由提供业务内容。

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  tls:
    auto_https:
      domains: [wss.example.com]
```

内建 managed ACME 默认使用正式 TLS-ALPN-01 和 ECDSA P-256，不依赖 certbot/acme.sh，也不要求邮箱。显式 HTTP-01 需要公网 80；旧客户端可选择 rsa2048。泛域名改用 DNS-01：

```yaml
http:
  tls_bind: 0.0.0.0:443
  tls:
    mode: acme_managed
    acme:
      domains: [example.com, "*.example.com"]
      challenge: dns01
      key_algorithm: ecdsa_p256
      dns:
        provider: cloudflare
        credentials:
          api_token: REPLACE-WITH-DNS-API-TOKEN
```

credentials 的键由 provider 决定，示例占位符不能申请真实证书。保护本地配置权限；普通配置显示会脱敏。证书申请、续期和外部 provider 的边界见 [配置指南](docs/CONFIGURATION.md)。

## 3. 集群、游戏长连接与文件协作

### 3.1 域名代理

```yaml
services:
  domain_routes:
    - name: api-domain
      domains: [api.example.com]
      path_prefix: /api
      strip_prefix: true
      upstream: http://127.0.0.1:8080
```

domain_routes 直接代理到上游，并支持路由证书策略；它不把 static:homepage 或 reverse_proxy:api 作为服务引用。静态站点仍独立放在 static_sites。

### 3.2 权重、健康检查、缓存和限流

```yaml
load_balance:
  algorithm: weighted
  active_health:
    enabled: true
    path: /healthz
    interval_secs: 5
    timeout_ms: 1200
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
        rate_limit:
          enabled: true
          requests: 200
          window_ms: 1000
          burst: 400
```

权重是流量倾斜，不是绝对配额。根 load_balance 管理算法、主动/被动健康和重试；单条路由可用 active_health 覆盖检查参数。只为可共享响应开启缓存；respect_origin 判断源站是否允许存储，边缘存储时间仍使用 ttl_secs。限流不代替鉴权。

### 3.3 游戏、数据库、MQTT 与 UDP

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      protocol: game_tcp
      nodelay: true
      connect_timeout_ms: 2000
      upstream: 10.0.1.10:9000
udp:
  listeners:
    - name: realtime-udp
      bind: 0.0.0.0:7001
      session_ttl_secs: 45
      max_associations: 150000
      upstream: 10.0.1.10:9001
```

nodelay 同时控制接入与上游 socket，显式 false 不被自动系统调优覆盖。TCP 半关闭先冲刷待发送数据，再传播写关闭；UDP 按客户端维护有上限的 association。MQTT TCP/TLS 透传、MQTT over WebSocket、CoAP、KCP 风格和透明 QCP 使用相应 TCP/UDP/HTTP 路径；网关不充当 MQTT broker，也不终止 QCP 帧。

### 3.4 AI 与 SSE

```yaml
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

上游是实际 New API/sub2api 或兼容 API 服务。持续 SSE 按下游消费转发，上游正文以空闲期限计时，不使用整个流的总时长超时。AI route 没有 transport.flush_interval_ms、tcp_nodelay 或 upstreams 字段。通用集群选择使用普通反代配置，模型与账号逻辑由上游或可选脚本处理。

### 3.5 WebDAV

```yaml
services:
  webdav:
    enabled: true
    path_prefix: /dav
    root: /srv/shared
    allow_write: true
    auth:
      enabled: true
      username: operator
      password: REPLACE-WITH-A-PRIVATE-PASSWORD
```

公网使用 HTTPS。Basic 认证先于文件操作，目录与复制/移动受 root 边界约束；大文件下载和 Range 有界流式发送。目录深度、覆盖和错误语义不等于完整 WebDAV 扩展，不支持 LOCK/UNLOCK/PROPPATCH。FTP 使用 services.ftp，FileCloud 使用 services.filecloud；详细配置见 [文件服务说明](docs/CONFIGURATION.md)。

## 4. 验收与切换

先用自己的路径、认证、上传、断点续传和长连接流量验证行为，再切换生产入口。check-config 检查配置，不能证明源站可达、证书申请成功或性能通过。

```bash
proxysss -config proxysss.yaml check-config
proxysss -config proxysss.yaml config explain
proxysss -config proxysss.yaml config routes
proxysss -config proxysss.yaml config reload-plan
proxysss -config proxysss.yaml config nginx-parity --format yaml
```

Linux 验证使用 scripts/verify-docker-scenarios.sh 与 scripts/benchmark-ubuntu24-amd64-docker.sh。后者在 Ubuntu 24.04 amd64 中隔离网关、后端和客户端资源，同时运行静态/CDN、反代、SSE、WebSocket、TCP、UDP 与透明 QCP 的 1×/2×/4× 负载。记录错误、目标完成率、吞吐、延迟和双方内存；单项成功不代表全场景通过，Docker 数据不能冒充独立物理主机证据。GitHub Actions 只打包，测试与性能验证在本地完成。

Linux 通过后，本机 Windows 快速运行 cargo test --locked。正式发布仍须同提交的功能、性能和发布证据；不要根据本文示例或能力清单声称已胜过其他网关。操作流程见 [验证指南](docs/VALIDATION.md)。

## 5. 后续阅读

- [完整配置与默认值](docs/CONFIGURATION.md)：配置字段、服务组合和管理 API。
- [CDN 下载与发布](docs/CDN-ORIGIN.md)：源站鉴权、文件版本、Range 和缓存。
- [架构与重载边界](docs/ARCHITECTURE.md)：运行时所有权、数据路径和脚本扩展。
- [系统适配与安全开关](docs/SECURITY-PERFORMANCE.md)：Linux/Windows/macOS 的实际调优范围。
- [TypeScript 脚本](ts-how-to-use.md)：进程内脚本与插件，不依赖外部 Node/Deno。
