# proxysss TypeScript How To Use

proxysss 的 TS 脚本运行在内嵌 QuickJS 引擎里，不依赖 node、deno 或 tsc。脚本是可选扩展层，不是主配置面；HTTP/HTTPS/TCP/UDP/FTP/WebDAV 的常规入口能力优先建议走单个 YAML 配置文件。

## 推荐文件

- `gateway.ts`: 主 fallback 路由脚本
- `plugins/*.ts`: 可选插件脚本
- `proxysss-script.d.ts`: 类型声明，写脚本前先引用它

## 在脚本里引用类型声明

主脚本：

```ts
/// <reference path="./proxysss-script.d.ts" />
```

插件脚本：

```ts
/// <reference path="../proxysss-script.d.ts" />
```

## 脚本导出格式

脚本必须 `export default` 一个对象：

```ts
/// <reference path="./proxysss-script.d.ts" />

const plugin: ProxysssPlugin = {
  name: "gateway",
  priority: -1000000,
  enabled: true,

  access(message, current) {
    if (message.ctx.path === "/healthz") {
      return { upstream: "proxysss://healthz" };
    }
    return current;
  },
};

export default plugin;
```

## 可用 hook

- `access`: HTTP 路由阶段，适合 path/host 级反代决策
- `balancer`: HTTP 上游选择阶段，适合在已有路由上改 upstream/upstreams/headers
- `preread`: TCP/UDP 路由阶段，适合基于 listener/player_id/首包预览决定上游
- `log`: 观测钩子，能看到最终 message + route
- `init_worker`: 插件加载时调用，可读取 `spec.config`
- `onDispose`: 插件卸载/重载时调用

## 最常用字段

HTTP：

- `message.ctx.host`
- `message.ctx.method`
- `message.ctx.path`
- `message.ctx.query`
- `message.ctx.remote_addr`
- `message.ctx.player_id`
- `message.ctx.headers`

TCP/UDP：

- `message.listener`
- `message.ctx.protocol`
- `message.ctx.remote_addr`
- `message.ctx.player_id`
- `message.ctx.first_packet_preview`
- `message.ctx.payload_len`

## 返回值怎么写

最小可用：

```ts
return { upstream: "http://127.0.0.1:8080" };
```

带 upstream pool：

```ts
return {
  upstream: "http://127.0.0.1:8080",
  upstreams: [
    "http://127.0.0.1:8080",
    "http://127.0.0.1:8081",
  ],
};
```

带 header 和 path 改写：

```ts
return {
  upstream: "http://127.0.0.1:9000",
  rewrite_path: "/internal/api",
  set_headers: {
    "x-proxysss-plugin": "custom-business-route",
    "x-tenant-id": "tenant-a",
  },
  strip_headers: ["x-remove-me"],
};
```

## 什么时候用 YAML，什么时候用 TS

优先用 YAML：

- 固定 host/path 反代
- 整站域名转发
- 静态文件、WebDAV、FTP、TCP、UDP
- TLS、健康检查、缓存、限流、访问控制

再用 TS：

- 业务相关 header 注入
- player_id / tenant / device 维度的选路
- 对接特殊 API 兼容层
- 插件式灰度/实验逻辑

## 常见误区

- 不要把常规 reverse proxy 全搬进 TS，YAML 更稳定也更快。
- 不要假设有 `fetch`、`fs`、`process`、`node` 内建模块；这是内嵌 QuickJS，不是 Node.js。
- hook 应保持同步和快速；返回 Promise 不会按 Node 风格长期异步驱动整个数据面。
- 额外 header 请优先写在 `set_headers`，不要依赖不在类型声明中的自定义字段名。

## 和文档联动

- nginx 对照与常见反代案例见 `nginx-to-proxysss.md`
- 运行时/内建文档页见 `http://localhost/docs.html` 或 `http://localhost/docs`
- 泛域名证书使用内建 managed DNS-01：`http.tls.mode: acme_managed` + `http.tls.acme.challenge: dns01` + `http.tls.acme.dns.provider`（`cloudflare` / `aliyun_cn` / `aliyun_intl` / `tencent` / `volcengine` / `aws` / `azure` / `google`）。无云 token 时用 `auto_https`（HTTP-01/TLS-ALPN-01），均不依赖 acme.sh
