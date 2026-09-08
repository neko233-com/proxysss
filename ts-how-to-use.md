# proxysss TypeScript How To Use

proxysss 的 TypeScript 运行时是内嵌 QuickJS，不依赖 node、deno 或 tsc。

最重要的一句话先写在最前面：

`TS 脚本是可选扩展层，不是主配置面。`

也就是说：

- 固定的 HTTP / HTTPS / TCP / UDP / FTP / WebDAV / TLS / 缓存 / 限流，优先走 YAML
- 只有当你真的需要业务感知、租户感知、player_id 感知、特殊上游选择时，再进入 TS
- 默认 `/` 的 `Welcome to proxysss` 页面是 Rust 内建 fallback，不属于脚本 API；用户路由优先，不要为了改默认页把 TS 放进热路径

## 1. 两种读法

如果你是新手：

1. 先抄一个最小插件
2. 确认你知道脚本该放哪里
3. 只用 `access` 或 `balancer` 先做一个很小的改动

如果你是高手：

1. 直接看 hook 能力面
2. 再看返回值结构
3. 最后看 TCP / UDP `preread`、生命周期和常见误区

## 2. 先把脚本文件摆对

推荐文件布局：

- `gateway.ts`：主 fallback 路由脚本
- `plugins/*.ts`：可选插件脚本
- `proxysss-script.d.ts`：类型声明

为什么这样分：

- `gateway.ts` 适合全局入口或兜底逻辑
- `plugins/*.ts` 适合模块化业务规则
- `proxysss-script.d.ts` 让你写脚本时至少有补全和类型提示

## 3. 在脚本里引用类型声明

主脚本：

```ts
/// <reference path="./proxysss-script.d.ts" />
```

插件脚本：

```ts
/// <reference path="../proxysss-script.d.ts" />
```

如果你没有先引用类型声明，最直接的后果就是：

- 编辑器没有类型提示
- 你更容易写出不在 API 表面上的字段名

## 4. 第一个最小可用插件

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

这段代码做了什么：

- `name` 是插件名，建议稳定且可读
- `priority` 决定执行顺序，数值越小越先执行
- `enabled` 控制是否启用
- `access` 在 HTTP 路由阶段生效
- `return current` 表示不改变当前决策

适合谁先从这个例子开始：

- 你想验证脚本是否真的被加载
- 你只需要给某个特殊 path 做小范围改写

## 5. 什么时候该用 YAML，什么时候该用 TS

优先用 YAML：

- 固定 host/path 反代
- 整站域名转发
- 静态文件、WebDAV、FTP、TCP、UDP
- TLS、健康检查、缓存、限流、访问控制

再用 TS：

- 业务相关 header 注入
- player_id / tenant / device 维度选路
- 对接特殊 API 兼容层
- 插件式灰度、实验、亲和路由

一句经验法则：

如果这条规则脱离业务上下文后依然成立，它大概率应该写在 YAML，而不是 TS。

## 6. 最常用 hook 一次讲清

### 6.1 `access`

适合：HTTP 路由阶段做 host/path 级决策。

```ts
const plugin: ProxysssPlugin = {
  name: "tenant-header",
  enabled: true,

  access(message, current) {
    if (message.ctx.host === "api.example.com" && message.ctx.path.startsWith("/tenant-a/")) {
      return {
        ...current,
        rewrite_path: message.ctx.path.replace("/tenant-a", "/api"),
        set_headers: {
          "x-tenant-id": "tenant-a",
        },
      };
    }
    return current;
  },
};

export default plugin;
```

什么时候好用：

- 你有极少量特殊路径想动态加 header
- 你不想为一条业务规则把整份 YAML 搞复杂

### 6.2 `balancer`

适合：路由已经命中，只想改上游选择或追加某些 header。

```ts
const plugin: ProxysssPlugin = {
  name: "gray-release",
  enabled: true,

  balancer(message, current) {
    const user = message.ctx.headers["x-user-id"];
    if (user && Number(user) % 10 === 0) {
      return {
        ...current,
        upstream: "http://10.0.0.12:8080",
        set_headers: {
          "x-release-bucket": "canary",
        },
      };
    }
    return current;
  },
};

export default plugin;
```

适合场景：

- 灰度
- 亲和路由
- 业务级流量打标

### 6.3 `preread`

适合：TCP / UDP 路由阶段，根据 listener、player_id、首包预览选上游。

```ts
const plugin: ProxysssPlugin = {
  name: "player-affinity",
  enabled: true,

  preread(message, current) {
    if (message.listener === "game-tcp" && message.ctx.player_id) {
      const shard = Number(message.ctx.player_id) % 2;
      return {
        ...current,
        upstream: shard === 0 ? "10.0.1.10:7000" : "10.0.1.11:7000",
      };
    }
    return current;
  },
};

export default plugin;
```

适合：

- 游戏分区
- 设备亲和
- 非 HTTP 的动态流量落点

### 6.4 `log`

适合：观测和统计，不适合做慢操作。

```ts
const plugin: ProxysssPlugin = {
  name: "traffic-stats",
  enabled: true,

  log(message) {
    if (message.ctx.protocol === "http") {
      console.log("[proxysss][http]", message.ctx.host, message.ctx.path);
    }
  },
};

export default plugin;
```

### 6.5 `init_worker` 和 `onDispose`

适合：初始化插件内部状态，以及在卸载/重载时做清理。

```ts
const plugin: ProxysssPlugin = {
  name: "warm-plugin",
  enabled: true,

  init_worker(spec) {
    console.log("plugin loaded", spec.config);
  },

  onDispose() {
    console.log("plugin disposed");
  },
};

export default plugin;
```

## 7. 最常用字段别乱猜

HTTP：

- `message.ctx.host`
- `message.ctx.method`
- `message.ctx.path`
- `message.ctx.query`
- `message.ctx.remote_addr`
- `message.ctx.player_id`
- `message.ctx.headers`

TCP / UDP：

- `message.listener`
- `message.ctx.protocol`
- `message.ctx.remote_addr`
- `message.ctx.player_id`
- `message.ctx.first_packet_preview`
- `message.ctx.payload_len`

建议做法：

- 想用哪个字段，先对照 `proxysss-script.d.ts`
- 不要在代码里拍脑袋发明字段名

## 8. 返回值怎么写

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

理解顺序建议是：

1. 先决定 `upstream`
2. 再决定要不要改 `rewrite_path`
3. 最后才是 `set_headers` / `strip_headers`

## 9. 常见误区

### 9.1 把普通反向代理全搬进 TS

这通常是错误方向。YAML 更稳定、更快、也更容易给团队和 agent 看懂。

### 9.2 把 QuickJS 当 Node.js

不要假设有这些东西：

- `fetch`
- `fs`
- `process`
- `node:*` 内建模块

这是内嵌 QuickJS，不是 Node.js。

### 9.3 在 hook 里做重逻辑或慢逻辑

hook 应保持同步和快速。它是扩展点，不是业务 worker。

### 9.4 用脚本掩盖配置面缺口

如果某个能力应该是通用网关能力，就应该优先推动 YAML / Rust 本体，而不是长期用脚本打补丁。

## 10. 文档怎么配套看

如果你先学整体配置：

- 看 `README.md`
- 看 `docs/CONFIGURATION.md`
- 看 `docs/ARCHITECTURE.md`

如果你在做迁移：

- 看 `nginx-to-proxysss.md`

如果你在写脚本：

- 先看 `proxysss-script.d.ts`
- 再看运行时内建文档页 `http://localhost/docs.html` 或 `http://localhost/docs`

如果你在做 TLS：

- 泛域名证书优先用内建 managed DNS-01：
  - `http.tls.mode: acme_managed`
  - `http.tls.acme.challenge: dns01`
  - `http.tls.acme.dns.provider`

## 11. 一句脚本策略

脚本应该让 proxysss 更灵活，而不是让基础网关能力变得更难找、更难配、更难测。
