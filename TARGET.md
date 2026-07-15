# proxysss 开发目标

## 最终产品目标

proxysss 必须成为可在 Linux 生产环境完整替代 nginx 的通用网关，而不是某一业务的专用网关。业务路由、玩家亲和、定制鉴权等行为放在可选 TypeScript 脚本或插件中；HTTP、HTTPS、HTTP/2、HTTP/3、WebSocket、TCP、UDP、FTP、WebDAV、静态文件、反向代理、日志、热重载、安全策略和管理面属于核心能力。

本阶段只在不破坏这些能力与默认行为的前提下完成数据面性能收敛。不能用删功能、缩场景、降低正确性、隐藏错误或给 nginx 不等价配置的方式制造领先。

## 当前阶段唯一完成标准

在本机 Mac arm64 的 Docker 中，以 Ubuntu 24.04 LTS x86_64 容器同时运行 proxysss、nginx、backend 和 load client。允许 Docker 使用 `linux/amd64` 模拟，但报告必须标记 `execution_mode=emulated-amd64`，不得描述为物理 x86 证据；禁止使用 `.ssh/` 中的远程 Linux 主机。

权威入口：

```bash
scripts/benchmark-ubuntu24-amd64-docker.sh
```

默认严格矩阵必须同时满足：

1. 完整运行 `1x 2x 4x` 三个负载尺度，不允许用单场景或单尺度结果代替最终结论。
2. 每个尺度同时运行 11 个场景：`static-small`、`static-large`、`cdn-hot-update`、`https-static-small`、`reverse-proxy`、`generic-sse`、`websocket-long-connection`、`game-long-connection`、`tcp-stream`、`udp-stream`、`qcp-transparent`。
3. mixed saturation 中，每个场景的 proxysss 吞吐都必须严格高于 nginx，聚合吞吐也必须严格高于 nginx。
4. equal-offered-load 中，双方使用同一个可执行目标速率；双方完成率都至少为 `98%`，proxysss 每个场景的 `p50`、`p95`、`p99` 都必须严格低于 nginx。
5. 所有场景错误数为 `0`，不得容忍 UDP 丢失、WebSocket 重连错误、SSE 中断或 HTTP 失败。
6. 验证计时不包含交叉编译、镜像构建、容器准备和 warm-up；从候选已就绪、严格矩阵开始计时，到三个尺度报告生成结束，必须不超过 `60s`。脚本必须写出 `validation_elapsed_secs` 并在超时后失败。
7. gateway、backend、client 使用互不重叠的 cpuset；两边 gateway 使用相同 gateway cpuset，测试一方时 pause 另一方。nginx 固定为 mainline 1.31.2、`-O3 -fno-plt` 和等价协议配置。
8. 内存证据包含 cgroup v2 current/peak；proxysss 的 current、peak 和每连接成本均不得超过同轮 nginx 的 `2x`，不得出现无界池、OOM 或持续增长。

只有上述条目全部由同一个最终提交的原始报告证明后，才能宣称“全面超过 nginx”。诊断性好结果、历史结果、推测或某一行胜出都不算完成。

## 性能设计约束

- Linux 数据面并发必须随实际 cpuset/CPU 核数自适应，不允许固定小 worker 上限。
- 普通 HTTP/TLS/TCP/UDP 热路径不得引入共享全局锁；优先每连接状态、每核分片、原子、`DashMap` 分片和有界无锁池。
- 小文件保持缓存快路径；大文件保持流式/backpressure，不能按请求复制完整文件或制造内存尖峰。
- realtime TCP/WebSocket、UDP、HTTP/TLS 必须在混合负载下一起优化。只提高一个场景、却让兄弟场景吞吐、延迟或内存退化的修改不接受。
- 允许用不超过 nginx 2 倍的有界内存换取明确吞吐或尾延迟收益，但必须报告 current、peak 和每连接成本。
- 正式 benchmark fixture、解析器与 gate 使用 Go 或原生编译工具，不能引入 Python 作为正式链路依赖。
- GitHub Actions 只负责打包，不运行性能 benchmark。

## 仓库与交付标准

最终交付必须同时具备：

- `cargo test --locked` 全绿；
- `go test scripts/benchmark-helper.go scripts/benchmark-helper_test.go` 全绿；
- benchmark shell 脚本通过 `bash -n`；
- `cargo fmt --all -- --check` 与 `git diff --check` 通过；
- 架构/数据路径变化同步到 `AGENTS.md`、`docs/ARCHITECTURE.md`、`docs/architecture.html`，benchmark 方法同步到 README 与 benchmark 文档；
- 工作树干净，`.benchmark/`、`.ssh/`、`target/`、本地密钥、日志和临时报告不进入 Git；
- 筛选后的历史性能报告保存在 `performance-evidence/development/local-docker/`，可供其他电脑继承分析；
- 最终提交已推送到 `origin/main`。

## 明确非目标

- 不使用远程 SSH Linux 完成本阶段验证。
- 不把 arm64 上的 amd64 模拟结果包装成物理 x86 发布证据。
- 不以单项 microbenchmark、单一 UDP/WebSocket 结果或 isolated saturation 替代 mixed matrix。
- 不通过降低 nginx worker、减少 nginx 功能、给 proxysss 更多 CPU、改变目标负载或放宽 gate 获得“胜出”。
- 不为了兼容旧内部结构保留低性能实现；对外行为与配置正确即可重构内部数据面。
