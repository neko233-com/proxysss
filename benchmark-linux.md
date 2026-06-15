# proxysss Linux benchmark（官方 Linux 口径）

> 中文 first。这个文档讲的是“当前官方 GitHub Actions Linux benchmark 快照”，不是拿单场景截图冒充正式发布结论。

## 1. 先看结论

- **最新官方 Linux quick benchmark** 里，`proxysss` 对静态小文件 HTTP/1.1 热路径已经明显超过 `nginx`。
- 这份快照来自 **GitHub Actions Linux** 产物，适合回答“现在主干的 Linux 小文件热路径大概到什么水平了”。
- **它不是正式发版唯一依据。** 正式性能发布仍然必须看 Linux mixed-load：静态、reverse proxy、New API/SSE、WebSocket、TCP、UDP、KCP-style 一起压，不能只挑一个好看的数字。

## 2. 当前官方快照

- Generated: `2026-06-15 13:46:39 UTC`
- Source: GitHub Actions `benchmark-results-linux`
- proxysss version: `proxysss 1.3.2`
- Workload: static `index.html` over HTTP/1.1
- Concurrency: `128`
- Duration: `10s`

| Gateway | ops/sec | vs nginx | MiB/s | p50 ms | p95 ms | p99 ms | success | errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| proxysss | 76099.60 | **2.366x** | 13.14 | 1.58 | 2.85 | 3.71 | 760996 | 0 |
| nginx | 32158.10 | 1.000x | 5.55 | 4.07 | 4.61 | 5.65 | 321581 | 0 |

## 3. 这个结果说明了什么

- `proxysss` 的 **静态小文件热路径**、预热、缓存命中、连接处理和 Go benchmark helper 链路当前是健康的。
- `0 errors` 说明这次官方 Linux quick run 至少没有靠“错误换吞吐”。
- `2.366x` 是一个很好的 **单场景快照**，适合放在“当前主干 Linux throughput 状态”这一层。

## 4. 这个结果不应该被拿来说明什么

- 它**不能单独证明** `reverse proxy`、`AI/SSE`、`WebSocket`、`TCP`、`UDP`、`KCP-style` 全部都已经同步更快。
- 它**不能替代** mixed-load release gate。
- 它**不能推翻**“性能优化必须无副作用”的团队约束。

换句话说：

- 单场景 quick benchmark 用来发现热点、看方向。
- mixed-load benchmark 用来决定“这次优化能不能真正发布”。

## 5. 正式 Linux 发布口径

正式发布请继续按下面顺序来：

```bash
proxysss tune linux --apply --profile latency --max-connections 200000
PROXY_BIN=/usr/local/bin/proxysss QUICK=1 DURATION_SECS=12 MIXED_MATRIX=1 \
  bash scripts/benchmark-all-scenarios.sh
```

发布判断重点不是“某一项有没有破 1.0”，而是：

- 关键长连接路径要守住公平比值底线。
- static/reverse/SSE 要维持软门槛和低错误。
- sibling 场景不能因为一次局部优化而明显退化。

## 6. 为什么现在要单独补这份文档

之前仓库里已经有一份更深的 Ubuntu 24 mixed-load 报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`。

但它讲的是：

- 更早一轮版本
- 更重的 mixed-load 场景
- 更接近“发布结论”的口径

而这份新文档讲的是：

- **最新主干**
- **GitHub Actions Linux 官方 quick snapshot**
- 给人看得懂的、和 HTML 页面一致的解释入口

两者不是互相替代，而是互相补位。

## 7. 和这次 SSE / HTTP2 优化的关系

这次优化的要求不是“只把 SSE 数字拉高”，而是：

- SSE raw fast lane 保持 `X-Forwarded-*` / `Forwarded` 语义不丢
- AI route 的 `proxysss-ai-*` metadata header 不丢
- HTTP/2 默认调优继续提升
- 不能为了一个流式路径让兄弟场景观测性、兼容性或稳定性倒退

这也是为什么 benchmark 文档里必须把 **no-side-effect optimization** 写清楚。

## 8. 推荐阅读

- HTML 入口：`docs/benchmark-linux.html`
- mixed-load 历史报告：`docs/BENCHMARK-ubuntu24-vs-nginx.md`
- 总入口：`docs/index.html`
