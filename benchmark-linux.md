# proxysss Linux benchmark（官方 Linux 口径）

> 中文 first。官方 Linux benchmark 现在默认就是“全协议 mixed-load 对比”，不是只看一个 static quick screenshot。

## 1. 先看结论

- **官方 GitHub Actions Linux benchmark 必须同时对比所有核心协议面。**
- 这条官方产物现在应该回答的是：`proxysss` 在 Linux 上和 `nginx` 做同波次 mixed-load 时，`HTTP / HTTPS / reverse proxy / New API / SSE / WebSocket / TCP / UDP / KCP-style UDP` 有没有一起站住。
- **单场景 quick benchmark 仍然可以保留，但它只是诊断工具，不再代表官方发布口径。**

## 2. 官方 Linux benchmark 现在对比哪些协议

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

## 3. GitHub Actions 官方工件现在应该长什么样

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

## 4. Windows benchmark 还要不要留

要留，但角色不同。

- **Linux mixed-load**：官方发布口径，必须比较所有协议
- **Windows throughput smoke**：本地 / CI 烟雾测试，用来发现 bench 命令、构建链和基础 throughput 有没有炸

不要把 Windows quick smoke 当成正式性能发布结论。

## 5. 正式 Linux 发布怎么跑

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

## 6. 单场景 quick benchmark 现在放到哪里

单场景 quick benchmark 仍然有价值，但只应该放在这些用途里：

- 定位热点
- 验证某个局部 fast path 有没有明显改善
- 在 Windows / 本地环境做低成本 smoke

它不应该继续承担“官方 Linux benchmark”这个角色。

## 7. 为什么这次必须改

如果官方 benchmark 只比 static small：

- 会误导人以为发布门槛只看一个 HTTP 小文件数字
- 会淡化 `SSE / WebSocket / TCP / UDP / KCP-style` 的 release gate 地位
- 会和仓库里的 AGENTS / 架构文档 / 生产硬化文档口径打架

所以这次不是“补一页文档”，而是把 **benchmark workflow、artifact、文档口径** 统一回“全协议 Linux mixed-load”。

## 8. 推荐阅读

- HTML 入口：`docs/benchmark-linux.html`
- mixed-load 历史报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`
- 总入口：`docs/index.html`
