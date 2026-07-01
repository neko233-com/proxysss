# proxysss Linux benchmark（官方 Linux 口径）

> 中文 first。默认 CI 现在只做全平台打包；性能 benchmark 改回手动/专机口径。发布前需要性能证据时，跑 Linux mixed-load；定位 UDP fast path 时，可以先跑 `SCENARIO_FILTER=udp-stream`。

## 1. 先看结论

- **v1.3.5 UDP 专项优化已经有新结果：** Docker Ubuntu 24 UDP-only 官方脚本路径跑到 **`4.045x`**，`proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`，两边 0 错误。
- **历史 mixed-load 基线也保留，但它不是 v1.3.5 UDP fast path 的新结论。** Ubuntu 24.04 LTS 真机 mixed-load 对标 `nginx` 的历史聚合结果是 **`1.287x`**；Docker `ubuntu:24.04` 历史 mixed-load 聚合是 **`1.059x`**。
- 真机里最能代表生产长连接价值的场景已经很强：`game-long-connection` **`2.732x`**、`tcp-stream` **`2.741x`**、`websocket-long-connection` **`1.319x`**。
- `new-api-sse` 历史真机基线是 **`0.849x`**，Docker 24 核校验是 **`1.014x`**。这正好说明为什么 SSE 优化不能只盯一个局部数字，而必须和所有兄弟协议一起看。
- **单场景 quick benchmark 仍然可以保留，但它只是诊断工具，不再代表官方发布口径。**

## 2. v1.3.5 UDP fast-path 结果

这次 UDP 优化验证的是 nginx 可公平转发的 `udp-stream`，不是 KCP/QCP 语义。命令口径是官方 Go helper + `scripts/benchmark-all-scenarios.sh`，只用 `SCENARIO_FILTER=udp-stream` 缩小到 UDP 单项：

| 场景 | Ratio | proxysss ops/s | nginx ops/s | Errors |
| --- | ---: | ---: | ---: | ---: |
| `udp-stream` | **`4.045x`** | 127742.75 | 31577.33 | 0 |

这说明 `0.855x` / `0.861x` 那组旧数字不能再当成当前 UDP fast path 的结论。它们是 v1.3.0 附近 mixed-load 历史报告里的 UDP 行，当时 UDP 还没有现在的 sharded pending-session dedupe、池化 recv buffer、批量 response drain、policy-free single-upstream direct fast path。

KCP 和 QCP 仍然是两套独立 UDP listener 能力：KCP 用 `protocol: kcp`，neko233-com/QCP 用 `protocol: qcp`。它们不进入默认 nginx head-to-head，因为 nginx 没有原生 KCP/QCP 协议语义。

## 3. 历史 mixed-load 跑分基线（直接对标 nginx）

### 3.1 真机 Ubuntu 24.04 LTS（v1.3.0）

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

### 3.2 Docker `ubuntu:24.04` 历史 mixed-load 验证

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
- Docker 上 UDP 和 KCP 受容器 `rmem_max` 环境限制噪声更大，所以生产判断以真机 Linux 主机为准。

> 上面这些数字来自仓库里已经存在的 mixed-load 历史报告，只是之前官方 benchmark 页没有把它们直接展示出来。现在应该把它们作为“历史基线”放在台面上，便于对照后续 SSE / HTTP2 / TCP / UDP 优化是不是整体前进；不要把其中的旧 UDP 行误读成 v1.3.5 UDP fast path 的当前结果。
> KCP 和 QCP 现在作为独立 UDP listener 能力维护，但默认 nginx 对标 benchmark 只比较 nginx 能公平转发的 `udp-stream`。QCP 结论看 proxysss 自身协议服务验证，不拿 nginx 伪装成 QCP 对照组。

## 4. 官方 Linux mixed-load benchmark 对比哪些协议

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

这也是为什么仓库一直强调：**性能优化必须无副作用**。如果 SSE 更快了，但 WebSocket、TCP、UDP、静态或 reverse proxy 退了，那不算成功。KCP/QCP 作为 proxysss 独立 UDP listener 能力单独验证，不进入默认 nginx head-to-head gate。

## 5. CI 和 benchmark 的边界

默认 GitHub Actions CI 已经按发布要求收敛为纯打包：六个平台 release binary 构建、打包、上传 artifact，不再自动跑 test、smoke 或性能 benchmark。

性能 benchmark 仍然保留在脚本里，但从默认 CI 移到手动/专机路径：

- `scripts/benchmark-all-scenarios.sh`：正式 Linux mixed-load 入口
- `SCENARIO_FILTER=udp-stream`：定位 UDP fast path 的专项入口
- `.benchmark/runs/all-scenarios/results.json` / `summary.md` / `summary.html`：手动 benchmark 输出

## 6. Windows benchmark 还要不要留

脚本可以留，但默认 CI 不再跑。

- **Linux mixed-load**：官方发布口径，必须比较所有协议
- **Windows throughput smoke**：本地诊断工具，用来发现 bench 命令、构建链和基础 throughput 有没有炸

不要把 Windows quick smoke 当成正式性能发布结论。

## 7. GitHub-hosted Linux runner 和专门 Linux 主机的区别

这里要把话说清楚：

- **GitHub-hosted Linux benchmark** 只适合手动诊断或临时验证，不再是默认 CI 发布门槛
- `KCP`、`QCP` 是 proxysss 独立 UDP listener 能力，不默认拿 nginx 做协议对照
- 真正的 `realtime UDP` 强门槛，仍然应该看专门调优过的 Linux 主机
- Docker / WSL2 容器如果缺 `/proc/sys/net/core/rmem_max`，脚本会自动把 UDP error tolerance 放宽到 `proxysss <= nginx + 16` 并在 summary 明示；真 Linux 主机默认仍是 `+4`

原因不是“把差结果藏起来”，而是 GitHub-hosted runner 的 UDP / realtime 噪声太大，且 nginx 没有 KCP/QCP 语义；默认 benchmark 不把错误环境或错误对照组包装成最终裁判。

## 8. 正式 Linux 发布怎么跑

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

## 9. 单场景 quick benchmark 现在放到哪里

单场景 quick benchmark 仍然有价值，但只应该放在这些用途里：

- 定位热点
- 验证某个局部 fast path 有没有明显改善
- 在 Windows / 本地环境做低成本 smoke

它不应该继续承担“官方 Linux benchmark”这个角色。

## 10. 为什么这次必须改

如果官方 benchmark 只比 static small：

- 会误导人以为发布门槛只看一个 HTTP 小文件数字
- 会淡化 `SSE`、`WebSocket`、`TCP`、`UDP` 的 release gate 地位，也会把 KCP/QCP 这类 proxysss 独立能力误写成 nginx 对标项
- 会和仓库里的 AGENTS / 架构文档 / 生产硬化文档口径打架

所以 benchmark 文档要同时讲清楚两件事：默认 CI 是纯打包；性能判断仍然走 Linux mixed-load 和必要的单场景诊断，不把旧历史数字当成当前优化结果。

## 11. 推荐阅读

- HTML 入口：`docs/benchmark-linux.html`
- mixed-load 历史报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`
- 总入口：`docs/index.html`
