# caddy 迁移到 proxysss

这份文档回答的是另一个很常见的问题：

`Caddyfile` 里的常见写法，在 `proxysss` 里怎么落地？

它同样遵循官方文档规范：

- `中文 first`
- 必要术语保留 `English`
- 对应 HTML 子页：`docs/caddy-to-proxysss.html`

## 1. 先建立迁移心智

### 1.1 Caddy 和 proxysss 的思路差别

`Caddy` 的体验很强在于：

- 上手快
- `Caddyfile` 简洁
- 自动 HTTPS 默认很好用

`proxysss` 的目标不是复制 `Caddyfile`，而是提供更完整的 nginx 级网关能力，并把这些能力统一在单个 YAML 中：

- HTTP / HTTPS / HTTP2 / HTTP3
- static files
- reverse proxy
- AI / SSE
- WebDAV
- TCP / UDP
- MQTT / IoT
- 热重载、watchdog、metrics、admin API

一句话理解：

- `Caddy` 迁移更像从“简洁站点代理”迁到“更完整的通用网关”

### 1.2 Caddyfile 常见能力，对应到哪里

| Caddy 思路 | proxysss 更常用的落点 | 什么时候用 |
| --- | --- | --- |
| `example.com { reverse_proxy ... }` | `services.reverse_proxy.routes` | 普通站点反代 |
| `handle_path /api/*` | `match.path_prefix` + `strip_prefix` | API 前缀改写 |
| `file_server` | `services.static_sites` | 静态站点 |
| 自动 HTTPS | `http.tls.mode: acme_managed` | 默认推荐 |
| `header` | `set_headers` / `strip_headers` | 请求头操作 |
| `reverse_proxy` + LB | `upstreams` + `load_balance` | 集群代理 |
| `layer4` 场景 | `tcp.listeners` / `udp.listeners` | 原生 TCP / UDP |

## 2. 新手先迁最常见的 6 类 Caddy 配置

### 2.1 反代整个站点

Caddy：

```caddy
app.example.com {
    reverse_proxy 127.0.0.1:9000
}
```

proxysss：

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443

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

这表示：

- 根路径和全部子路径都转给后端
- 80/443 是默认公网入口

### 2.2 `handle_path /api/*`

Caddy：

```caddy
app.example.com {
    handle_path /api/* {
        reverse_proxy 127.0.0.1:8080
    }
}
```

proxysss：

```yaml
services:
  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["app.example.com"]
          path_prefix: "/api"
        strip_prefix: true
        upstreams:
          - url: "http://127.0.0.1:8080"
```

为什么这里要特别注意：

- `handle_path` 的核心语义是去掉前缀再转发
- 在 proxysss 里对应 `strip_prefix: true`

### 2.3 静态站点 `file_server`

Caddy：

```caddy
www.example.com {
    root * /srv/www
    file_server
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
- 纯静态前端

### 2.4 自动 HTTPS / WSS

Caddy 很多人喜欢，就是因为自动 HTTPS 体验顺手。proxysss 的单域名 WSS 路径同样只需要给出域名。

proxysss 里推荐这样写：

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443
  tls:
    auto_https:
      domains: [wss.example.com]
```

这会使用内建 managed ACME 的正式 TLS-ALPN-01，无需 certbot、acme.sh、DNS API 或邮箱；A/AAAA 必须指向网关，且公网 443 可达。邮箱仍可通过 `http.tls.auto_https.email` 选填，以接收到期和安全通知。旧的显式 HTTP-01（需 80）/ TLS-ALPN-01 / DNS-01 配置保持兼容。

如果是泛域名：

```yaml
http:
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

### 2.5 自定义 `header`

Caddy：

```caddy
app.example.com {
    reverse_proxy 127.0.0.1:8080 {
        header_up X-Tenant-Id tenant-a
        header_up X-Env production
    }
}
```

proxysss：

```yaml
services:
  reverse_proxy:
    routes:
      - name: api
        match:
          hosts: ["app.example.com"]
          path_prefix: "/"
        upstreams:
          - url: "http://127.0.0.1:8080"
        set_headers:
          x-tenant-id: tenant-a
          x-env: production
```

### 2.6 同一域名下前后端分离

Caddy：

```caddy
app.example.com {
    handle_path /api/* {
        reverse_proxy 127.0.0.1:8080
    }

    reverse_proxy 127.0.0.1:3000
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

## 3. 高手迁移：从站点代理走向完整网关

### 3.1 负载均衡 + 健康检查

Caddy：

```caddy
api.example.com {
    reverse_proxy 10.0.0.11:8080 10.0.0.12:8080
}
```

proxysss：

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

### 3.2 SSE / AI 代理

如果你之前只是把流式 AI 请求当成普通 `reverse_proxy` 用 `Caddy` 转发，迁到 proxysss 时建议改用 `services.ai_proxy`：

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

这类配置更贴合：

- `OpenAI-compatible`
- `New API`
- `SSE`
- 长响应 token 流

### 3.3 Caddy `layer4` 思路迁到 TCP / UDP

TCP：

```yaml
tcp:
  listeners:
    - name: mqtt-tcp
      bind: 0.0.0.0:1883
      nodelay: true
      routes:
        - name: broker
          upstreams:
            - addr: "10.0.0.30:1883"
```

UDP：

```yaml
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

这里的关键差别是：

- proxysss 明确把 TCP / UDP 暴露成一等公民能力面
- 不需要依赖额外插件体系才能表达这些流量

### 3.4 WebDAV 与文件协作

如果你过去在 `Caddy` 上只做简单静态文件，现在想加可控写入或协作入口，可以直接用 `WebDAV`：

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

## 4. Caddy 用户迁移时最容易踩的坑

### 4.1 以为 proxysss 只是另一个 `Caddyfile`

不是。proxysss 的目标是更完整的通用网关，所以会有更明确的能力面划分。

### 4.2 以为所有事情都该塞进一条 `reverse_proxy`

不是。静态站点、AI 代理、TCP、UDP、WebDAV 都应该落到各自能力面。

### 4.3 自动 HTTPS 很顺手，就忽略了运维边界

迁到 proxysss 后，除了自动证书，你还应该一起看：

- `reload-plan`
- `capabilities`
- `warm-up`
- mixed-load benchmark

### 4.4 只看单站点 HTTP 成绩，不看全局

proxysss 的性能优化和迁移结论都要遵循同一条纪律：

- 后续所有性能优化都要压测
- 必须是无副作用优化
- SSE、static、HTTP reverse proxy、TCP、UDP、KCP-style、QCP 要一起看
- 涉及公网/NIC 延迟时必须用 `scripts/benchmark-cross-host-wss.sh` 在独立 client/gateway/backend 主机复跑，Docker cpuset 数据不能冒充跨机结果

## 5. 一句迁移建议

如果你是从 `Caddy` 迁到 `proxysss`：

1. 先迁最简单的站点 `reverse_proxy`
2. 再拆 `static_sites`、`ai_proxy`、`tcp.listeners`、`udp.listeners`
3. 最后补 TLS、health、cache、benchmark

这样你会更快理解 proxysss 的能力面，而不是把它误用成另一个语法不同的 `Caddyfile`。
