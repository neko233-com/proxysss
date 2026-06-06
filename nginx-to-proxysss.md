# nginx to proxysss

这个文档专门回答一个问题：现有 nginx 常见配置，在 proxysss 里该怎么写。

原则：

- 常规入口职责优先用 YAML
- 业务相关分流和补充 header 再交给 TS 插件
- proxysss 追求 nginx 同级行为，但配置表达更直接

## 1. 反代某个 API 前缀

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
  tls_bind: 0.0.0.0:443

services:
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      strip_prefix: true
```

说明：

- `strip_prefix: true` 对应 nginx `location /api/` + `proxy_pass .../`
- 常见 `Host` / `X-Real-IP` / `X-Forwarded-*` 会自动补齐

## 2. 反代某个域名 = 后面整个服务器

这是前后端分离项目里最常见的整站反代。

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
  domain_routes:
    - name: app
      domains: [app.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
      strip_prefix: false
```

说明：

- 这会把 `/` 以及所有子路径一起交给后端服务器
- `strip_prefix: false` 对应整站透传，后端自己处理路径

## 3. 整个域名反代，后端子路径也要一起处理

典型场景：React/Vue 前端 + `/api/*` 后端接口都在同一台应用服务器。

nginx：

```nginx
server {
    listen 80;
    server_name spa.example.com;

    location / {
        proxy_pass http://127.0.0.1:9000;
    }
}
```

proxysss：

```yaml
services:
  domain_routes:
    - name: spa
      domains: [spa.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:9000
```

后端收到的仍然是：

- `/`
- `/dashboard`
- `/api/user/profile`
- `/assets/app.js`

## 4. 同一域名下前后端分离

例如：

- `/` 走前端静态或 SSR 服务
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
  domain_routes:
    - name: backend-api
      domains: [app.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      strip_prefix: true

    - name: frontend-app
      domains: [app.example.com]
      path_prefix: /
      upstream: http://127.0.0.1:3000
      strip_prefix: false
```

proxysss 会自动选最长匹配前缀，所以 `/api/*` 会优先命中 `backend-api`。

## 5. 自定义增加额外 header

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
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      strip_prefix: true
      set_headers:
        x-tenant-id: tenant-a
        x-env: production
```

如果要删 header：

```yaml
      strip_headers:
        - x-legacy-header
        - x-debug-token
```

## 6. 负载均衡 upstream pool

nginx：

```nginx
upstream api_upstream {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}

location /api/ {
    proxy_pass http://api_upstream/;
}
```

proxysss：

```yaml
load_balance:
  algorithm: rendezvous

services:
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      upstreams:
        - http://127.0.0.1:8080
        - http://127.0.0.1:8081
      strip_prefix: true
```

## 7. TLS / 自动 HTTPS

nginx 常见上法通常要手配证书或配外部 certbot/acme.sh。

proxysss：

```yaml
http:
  tls:
    auto_https:
      enabled: true
      domains: [example.com, www.example.com]
      email: admin@example.com
      production: true
```

## 8. 健康检查与维护态

proxysss 可直接在配置里打开：

```yaml
load_balance:
  active_health:
    enabled: true
    http_enabled: true
    tcp_enabled: true
    path: /healthz
    failure_threshold: 2
    success_threshold: 2
    alert_webhooks:
      - https://ops.example.com/webhooks/proxysss

runtime:
  maintenance_state:
    enabled: true
    path: ./runtime/maintenance-state.json
```

路由级覆写：

```yaml
services:
  domain_routes:
    - name: api
      domains: [api.example.com]
      path_prefix: /api
      upstream: http://127.0.0.1:8080
      active_health:
        path: /readyz
        failure_threshold: 3
        success_threshold: 2
```

## 9. 静态文件

nginx：

```nginx
location /assets/ {
    root /srv/www;
    autoindex off;
}
```

proxysss：

```yaml
services:
  static_sites:
    - name: assets
      path_prefix: /assets
      root: /srv/www/assets
      index_files: [index.html, index.htm]
      autoindex: false
```

## 10. WebDAV

proxysss 直接内建：

```yaml
services:
  webdav:
    enabled: true
    path_prefix: /dav
    root: ./webdav
    allow_write: true
```

## 11. TCP / UDP

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      upstreams: [127.0.0.1:9000, 127.0.0.1:9001]

udp:
  listeners:
    - name: realtime
      bind: 0.0.0.0:7001
      upstreams: [127.0.0.1:9100, 127.0.0.1:9101]
```

## 12. FTP

```yaml
services:
  ftp:
    enabled: true
    bind: 0.0.0.0:21
    upstream: 127.0.0.1:2121
    native_control: true
    passive_port_start: 50000
    passive_port_end: 50100
```

## 13. 错误页 / 404

```yaml
http:
  error_pages:
    enabled: true
    show_details: false
    pages:
      - status: 404
        content_type: text/html; charset=utf-8
        body: |
          <html>
            <body>
              <h1>{{status}} {{reason}}</h1>
              <p>proxysss could not match this route.</p>
            </body>
          </html>
```

## 14. include 子配置

```yaml
include:
  enabled: true
  required: true
  files:
    - ./conf.d/http.yaml
    - ./conf.d/streams.yaml
```

## 15. 什么时候还用 TS

YAML 已覆盖常规网关入口职责；TS 更适合：

- 自定义业务 header
- 特殊 tenant / player / device 路由
- API 兼容层
- 插件式业务逻辑
