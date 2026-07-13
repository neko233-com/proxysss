# proxysss Linux benchmark（官方 Linux 口径）

> 中文 first。默认 CI 现在只做全平台打包；性能 benchmark 改回手动/专机口径。最新矩阵只跑通用网关场景：static、reverse proxy、generic SSE、WebSocket、TCP、UDP；不跑 New API 专属路由，也不把 KCP/QCP 这类特殊 UDP 封装放进 nginx 对标。

## 1. 先看结论

- **v1.3.5 UDP 专项优化已经有新结果：** Docker Ubuntu 24 UDP-only 官方脚本路径跑到 **`4.045x`**，`proxysss 127742.75 ops/s` vs `nginx 31577.33 ops/s`，两边 0 错误。
- **历史 mixed-load 基线也保留，但它不是 v1.3.5 UDP fast path 的新结论。** Ubuntu 24.04 LTS 真机 mixed-load 对标 `nginx` 的历史聚合结果是 **`1.287x`**；Docker `ubuntu:24.04` 历史 mixed-load 聚合是 **`1.059x`**。
- 真机里最能代表生产长连接价值的场景已经很强：`game-long-connection` **`2.732x`**、`tcp-stream` **`2.741x`**、`websocket-long-connection` **`1.319x`**。
- 历史报告里的 `new-api-sse` 已经不再是当前矩阵名称；最新脚本改为通用 `generic-sse`，只验证普通 SSE/HTTP 流式反代能力。
- **单场景 quick benchmark 仍然可以保留，但它只是诊断工具，不再代表官方发布口径。**

## 2. v1.3.5 UDP fast-path 结果

这次 UDP 优化验证的是 nginx 可公平转发的 `udp-stream`，不是 KCP/QCP 语义。命令口径是官方 Go helper + `scripts/benchmark-all-scenarios.sh`，只用 `SCENARIO_FILTER=udp-stream` 缩小到 UDP 单项：

| 场景 | Ratio | proxysss ops/s | nginx ops/s | Errors |
| --- | ---: | ---: | ---: | ---: |
| `udp-stream` | **`4.045x`** | 127742.75 | 31577.33 | 0 |

这说明 `0.855x` / `0.861x` 那组旧数字不能再当成当前 UDP fast path 的结论。它们是 v1.3.0 附近 mixed-load 历史报告里的 UDP 行，当时 UDP 还没有现在的 sharded pending-session dedupe、池化 recv buffer、批量 response drain、policy-free single-upstream direct fast path。

KCP 和 QCP 仍然是两套独立 UDP listener 能力，但不进入性能 benchmark 矩阵。当前 benchmark 只覆盖普通 UDP 转发；KCP/QCP 的协议语义由各自上游服务和功能验证负责，不拿 nginx 做伪对照。

## 3. 历史 mixed-load 跑分基线（旧矩阵，仅供对照）

下面两张表来自旧矩阵，里面的 `new-api-sse`、`kcp-style-udp` 是历史场景名，不代表最新 benchmark 仍然运行这些专项场景。最新脚本已经改成 `generic-sse`，并且默认排除 KCP/QCP 特殊 UDP 封装。

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
> KCP 和 QCP 现在作为独立 UDP listener 能力维护，但默认 nginx 对标 benchmark 只比较 nginx 能公平转发的 `udp-stream`。QCP/KCP 不再出现在当前性能矩阵里，也不拿 nginx 伪装成协议对照组。

## 4. 官方 Linux mixed-load benchmark 对比哪些协议

默认 mixed matrix 需要一起跑这些场景：

- `static-small`
- `static-large`
- `cdn-hot-update`
- `https-static-small`
- `reverse-proxy`
- `generic-sse`
- `websocket-long-connection`
- `game-long-connection`
- `tcp-stream`
- `udp-stream`

这也是为什么仓库一直强调：**性能优化必须无副作用**。如果 SSE 更快了，但 WebSocket、TCP、UDP、静态或 reverse proxy 退了，那不算成功。KCP/QCP 作为 proxysss 独立 UDP listener 能力保留，但不进入当前性能 benchmark 矩阵。

### 4.1 三阶段公平判定，不能混用吞吐与延迟口径

`scripts/benchmark-all-scenarios-isolated.sh` 把 4c gateway、backend、client 固定到互不重叠的 CPU set/cgroup，并用同一 Docker bridge 上的独立网络命名空间传输。默认记录 cgroup current/peak、容器资源快照和每连接成本；只有声明真实生产预算时才传 Docker/systemd 内存上限。对照 nginx 固定为当前 mainline `1.31.2`，以 `-O3 -fno-plt` 构建并启用 HTTP SSL/H2/stream；proxysss 必须使用 Linux release binary。

它分三阶段输出 JSON、Markdown、HTML 与百分比表。mixed saturation 与 equal offered load 默认交错运行 4 次，serial isolated saturation 默认 3 次，并对 ops/s、p50、p95、p99 取中位数；任一轮出现错误时聚合结果保留最大错误数，不能用中位数掩盖不稳定：

1. `mixed saturation`：十个场景同时跑，只判聚合吞吐和错误，不拿两边不同实际吞吐下的饱和延迟硬比。
2. `isolated saturation`：每次只跑一个场景并交替先后顺序，判单场景最大吞吐/容量。
3. `equal offered load`：从两边较慢的饱和吞吐取 70%，固定发送间隔；双方必须完成至少 98% 目标负载，并且 proxysss 的 p50/p95/p99 都严格低于 nginx。

```bash
PROXY_BIN=target/release/proxysss \
STRICT_SUPERIORITY=1 DURATION_SECS=30 \
bash scripts/benchmark-all-scenarios-isolated.sh
```

Docker role isolation 解决的是同进程、同 cgroup、同 CPU 抢占混淆，不等于三台物理机。脚本会先拒绝角色 cpuset 重叠或默认 16 核包络之外的机器；公网 RTT、NIC/IRQ/RSS、跨机丢包结论仍要在独立 gateway/backend/client 主机复跑。

### 4.2 WebSocket 容量与延迟要分开验证

`ops/s` 是一条已建立 WebSocket 上的回显消息轮次，不是并发连接数。原生 benchmark 还提供只建连、保持、采样握手延迟的容量模式：

```bash
proxysss bench websocket \
  --url ws://gateway.example.com/gateway/ws \
  --connections 20000 \
  --hold-connections \
  --connect-workers 128 \
  --connect-timeout-ms 10000 \
  --connect-retries 4 \
  --duration-secs 30
```

4c 的默认生产参考目标是 20k idle WSS 与最多 4096 条活跃消息连接。它不是固定 RAM 准入门槛：报告内存 current/peak 和每连接成本，以无 OOM、无持续增长、缓冲池有界为判据；只有部署方已声明内存预算才启用上限。单个 IPv4 源地址到单个后端 `IP:port` 仍只有有限临时端口；若主动把目标提高到单源端口范围以上，必须增加压测源 IP 和后端 `IP:port`（或代理源 IP 池）。这是 TCP 四元组限制，nginx 和 proxysss 都不能绕过。

容量成功只说明可保持足够多连接；低延迟与消息吞吐仍要单独用普通 `proxysss bench websocket` 以及 mixed-load 矩阵验证。

### 4.3 单网关 WSS 隔离 Docker 验证

对 Rust 游戏网关，先跑下面的生产门禁。它会把 nginx/proxysss 网关固定在同一组 `4 CPU`，把 4 个回源和多个客户端放到独立 cgroup 与网络命名空间；默认按 256/1024/4096 active WSS 和 5k/10k/20k idle WSS 逐级验证，四轮 Latin-square 平衡候选顺序与地址哈希，并比较吞吐、p50/p95/p99、握手延迟、错误、RSS/cgroup peak 和每连接成本。内存上限仅在声明实际生产预算时启用：

```bash
proxysss tune linux --apply
bash scripts/benchmark-websocket-production-gate.sh
```

默认容量上限 20k，仍使用多个客户端地址和 4 个后端 `IP:port`，避免把端口耗尽误判成网关失败。参考压测机至少要有 16 CPU / 32 GiB；默认 4 gateway + 4 backend + 8 client 的 cpuset 预检会在不满足时失败，而不是悄悄让角色抢 CPU。该脚本刻意使用生成的自签名 WSS fixture，并仅在压测客户端加 `--insecure`；它验证 TLS/WSS 数据路径，不是证书信任测试，真实 ACME 由 Docker 场景测试与公网证书测试单独覆盖。

Docker 的 cgroup、网络命名空间和 CPU 集隔离能排除“同一进程/同一容器”干扰，但仍共享同一 Linux 内核；生产发布结论还必须在独立的网关、回源、压测主机上复跑，不能把单机 Docker 结果伪装成跨机网络延迟。

### 4.4 三台独立主机 WSS 严格复跑

从独立的 Linux 压测机运行下面的入口。它把同一个 SHA-256 的 proxysss 二进制复制到 gateway 和 backend，并在 gateway 用 `systemd-run` cgroup 强制 `AllowedCPUs=0-3`、`LimitNOFILE=300000`；CPU 不足、无 systemd cgroup 或角色主机不独立都会明确失败。内存不使用任意物理 RAM 阈值卡死，而是保存 cgroup memory current/peak 与每连接成本；若发布有明确预算，再显式传 `GATEWAY_MEMORY_MAX=8G`（或实际预算）。它保存三台主机的 `uname`/`lscpu`、gateway `nginx -V`、每一轮原始输出与中位数报告；顺序交错四轮，饱和吞吐和 equal-offered-load 的 p50/p95/p99 都必须严格优于 nginx，20k 容量每轮必须零失败。`GATEWAY_ADDR` 是压测机可访问的网关地址，`BACKEND_ADDR` 是网关可访问的回源地址。

```bash
GATEWAY_HOST=gw-ssh BACKEND_HOST=be-ssh \
GATEWAY_ADDR=10.0.0.10 BACKEND_ADDR=10.0.0.20 \
bash scripts/benchmark-cross-host-wss.sh
```

默认使用可跨同架构 Linux 主机运行的 release-fast 二进制。只有三台机器 CPU 特性一致时，才可显式 `BUILD_NATIVE=1` 以 `target-cpu=native` 编译；否则不得以优化名义引入非法指令风险。

## 5. CI 和 benchmark 的边界

默认 GitHub Actions CI 已经按发布要求收敛为纯打包：六个平台 release binary 构建、打包、上传 artifact，不再自动跑 test、smoke 或性能 benchmark。

性能 benchmark 仍然保留在脚本里，但从默认 CI 移到手动/专机路径：

- `scripts/benchmark-all-scenarios.sh`：正式 Linux mixed-load 入口
- `scripts/benchmark-all-scenarios-isolated.sh`：4c8g role-isolated saturation + equal-offered-load 严格对照入口
- `scripts/benchmark-websocket-production-gate.sh`：4c8g 单网关多尺度 WSS active latency + 20k idle 容量角色隔离入口
- `scripts/benchmark-cross-host-wss.sh`：三台独立 Linux 主机 WSS 严格吞吐、p50/p95/p99 与 20k 容量证据入口
- `SCENARIO_FILTER=udp-stream`：定位 UDP fast path 的专项入口
- `.benchmark/runs/all-scenarios/results.json` / `summary.md` / `summary.html`：手动 benchmark 输出

## 6. Windows benchmark 还要不要留

脚本可以留，但默认 CI 不再跑。

- **Linux mixed-load**：官方发布口径，必须比较所有通用网关协议
- **Windows throughput smoke**：本地诊断工具，用来发现 bench 命令、构建链和基础 throughput 有没有炸

不要把 Windows quick smoke 当成正式性能发布结论。

## 7. GitHub-hosted Linux runner 和专门 Linux 主机的区别

这里要把话说清楚：

- **GitHub-hosted Linux benchmark** 只适合手动诊断或临时验证，不再是默认 CI 发布门槛
- `KCP`、`QCP` 是 proxysss 独立 UDP listener 能力，不进入当前 nginx 对标性能矩阵
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
- static / reverse / generic SSE 保持软门槛和低错误
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
- 会淡化 `generic-sse`、`WebSocket`、`TCP`、`UDP` 的 release gate 地位，也会把 KCP/QCP 这类 proxysss 独立能力误写成 nginx 对标项
- 会和仓库里的 AGENTS / 架构文档 / 生产硬化文档口径打架

所以 benchmark 文档要同时讲清楚两件事：默认 CI 是纯打包；性能判断仍然走 Linux mixed-load 和必要的单场景诊断，不把旧历史数字当成当前优化结果。

## 11. 推荐阅读

- HTML 入口：`docs/benchmark-linux.html`
- mixed-load 历史报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`
- 总入口：`docs/index.html`
