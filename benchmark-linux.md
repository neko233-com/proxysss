# proxysss Linux benchmark（官方 Linux 口径）

> 中文 first。官方 Linux benchmark 现在默认就是“全协议 mixed-load 对比”，不是只看一个 static quick screenshot。

## 1. 先看结论

- **这页现在同时给两样东西：官方 Linux mixed-load benchmark 口径 + 已有历史跑分基线。**
- **历史真机基线已经有了，不该藏着不写。** Ubuntu 24.04 LTS 真机 mixed-load 对标 `nginx` 的聚合结果是 **`1.287x`**；Docker `ubuntu:24.04` 验证聚合是 **`1.059x`**。
- 真机里最能代表生产长连接价值的场景已经很强：`game-long-connection` **`2.732x`**、`tcp-stream` **`2.741x`**、`websocket-long-connection` **`1.319x`**。
- `new-api-sse` 历史真机基线是 **`0.849x`**，Docker 24 核校验是 **`1.014x`**。这正好说明为什么 SSE 优化不能只盯一个局部数字，而必须和所有兄弟协议一起看。
- **单场景 quick benchmark 仍然可以保留，但它只是诊断工具，不再代表官方发布口径。**

## 2. 历史跑分基线（直接对标 nginx）

### 2.1 真机 Ubuntu 24.04 LTS（v1.3.0）

| Scenario | Ratio | proxysss ops/s | nginx ops/s | Errors |
| --- | ---: | ---: | ---: | ---: |
| `cdn-hot-update` | **`1.074x`** | 4134.17 | 3847.67 | 0 |
| `game-long-connection` | **`2.732x`** | 4729.17 | 1731.25 | 0 |
| `https-static-small` | `0.829x` | 908.83 | 1096.17 | 0 |
| `kcp-style-udp` | `0.988x` | 1549.08 | 1568.17 | 0 |
| `new-api-sse` | `0.849x` | 94.50 | 111.33 | 0 |
| `reverse-proxy` | **`1.054x`** | 2450.50 | 2325.42 | 0 |
| `static-large` | **`1.180x`** | 19.67 | 16.67 | 0 |
| `static-small` | `0.893x` | 4226.42 | 4733.42 | 0 |
| `tcp-stream` | **`2.741x`** | 4699.25 | 1714.50 | 0 |
| `udp-stream` | `0.855x` | 1533.08 | 1793.33 | 0 |
| `websocket-long-connection` | **`1.319x`** | 1303.67 | 988.42 | 0 |

- 真机聚合：`proxysss 25648.34 ops/s` vs `nginx 19926.35 ops/s` → **`1.287x`**
- 真机环境：Ubuntu 24.04 LTS，`proxysss tune linux --apply` 后 mixed-load，`QUICK=1`，每场景 12 秒。

### 2.2 Docker `ubuntu:24.04` 验证

| Scenario | Ratio |
| --- | ---: |
| `game-long-connection` | **`1.429x`** |
| `tcp-stream` | **`1.438x`** |
| `reverse-proxy` | **`1.200x`** |
| `https-static-small` | **`1.269x`** |
| `new-api-sse` | **`1.014x`** |
| `static-small` | `1.000x` |
| `cdn-hot-update` | `0.978x` |
| `websocket-long-connection` | `0.924x` |
| `kcp-style-udp` | `0.783x` |
| `udp-stream` | `0.861x` |

- Docker 聚合：`248015 / 234176` → **`1.059x`**
- Docker 上 UDP / KCP 受容器 `rmem_max` 环境限制噪声更大，所以生产判断以真机 Linux 主机为准。

> 上面这些数字来自仓库里已经存在的 mixed-load 历史报告，只是之前官方 benchmark 页没有把它们直接展示出来。现在应该把它们作为“历史基线”放在台面上，便于对照后续 SSE / HTTP2 / TCP / UDP 优化是不是整体前进。

## 3. 官方 Linux benchmark 现在对比哪些协议

默认 mixed matrix 需要一起跑这些场景：

- `static-small`
- `static-large`
- `cdn-hot-update`
- `https-static-small`
- `reverse-proxy`
- `new-api-sse`
- `websocket-long-connection`
- `game-long-connection`
- `tcp-stream`
- `udp-stream`
- `kcp-style-udp`

这也是为什么仓库一直强调：**性能优化必须无副作用**。如果 SSE 更快了，但 WebSocket、TCP、UDP、静态或 reverse proxy 退了，那不算成功。

## 4. GitHub Actions 官方工件现在应该长什么样

Linux 官方 benchmark 工件名仍然是：

- `benchmark-results-linux`

但它的内容应该来自：

- `.benchmark/runs/all-scenarios/results.json`
- `.benchmark/runs/all-scenarios/summary.md`
- `.benchmark/runs/all-scenarios/summary.html`

也就是说：

- `results.json` 给机器、脚本、agent 用
- `summary.md` 给仓库里快速 diff / 审阅用
- `summary.html` 给人直接打开看

## 5. Windows benchmark 还要不要留

要留，但角色不同。

- **Linux mixed-load**：官方发布口径，必须比较所有协议
- **Windows throughput smoke**：本地 / CI 烟雾测试，用来发现 bench 命令、构建链和基础 throughput 有没有炸

不要把 Windows quick smoke 当成正式性能发布结论。

## 6. GitHub-hosted Linux runner 和专门 Linux 主机的区别

这里要把话说清楚：

- **GitHub-hosted Linux benchmark** 现在也会比较所有协议
- 但它的 `kcp-style-udp` 在 summary 里默认是 **diagnostic**
- 真正的 `KCP / realtime UDP` 强门槛，仍然应该看专门调优过的 Linux 主机

原因不是“把差结果藏起来”，而是 GitHub-hosted runner 的 UDP / realtime 噪声太大，容易把 KCP 结论带偏。我们保留它在表里，是为了看趋势；我们不把它当 hosted runner 上的最终裁判，是为了不让错误环境替代真实环境。

## 7. 正式 Linux 发布怎么跑

```bash
proxysss tune linux --apply --profile latency --max-connections 200000
PROXY_BIN=/usr/local/bin/proxysss QUICK=1 DURATION_SECS=12 MIXED_MATRIX=1 \
  bash scripts/benchmark-all-scenarios.sh
```

判断重点不是“某一项截图好不好看”，而是：

- 关键长连接路径守住公平比值底线
- static / reverse / SSE 保持软门槛和低错误
- sibling 场景不能因为一次局部优化而明显退化
- 聚合 mixed ratio 也要站住

## 8. 单场景 quick benchmark 现在放到哪里

单场景 quick benchmark 仍然有价值，但只应该放在这些用途里：

- 定位热点
- 验证某个局部 fast path 有没有明显改善
- 在 Windows / 本地环境做低成本 smoke

它不应该继续承担“官方 Linux benchmark”这个角色。

## 9. 为什么这次必须改

如果官方 benchmark 只比 static small：

- 会误导人以为发布门槛只看一个 HTTP 小文件数字
- 会淡化 `SSE / WebSocket / TCP / UDP / KCP-style` 的 release gate 地位
- 会和仓库里的 AGENTS / 架构文档 / 生产硬化文档口径打架

所以这次不是“补一页文档”，而是把 **benchmark workflow、artifact、文档口径** 统一回“全协议 Linux mixed-load”。

## 10. 推荐阅读

- HTML 入口：`docs/benchmark-linux.html`
- mixed-load 历史报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`
- 总入口：`docs/index.html`
