# proxysss 性能开发交接计划

## 当前交接点

分支：`main`

当前性能候选父提交：`1995e1a Rebalance TLS weight against mixed throughput`

该提交已经通过：

```bash
cargo fmt --all
cargo test --locked --no-run
git diff --check
```

但它在本轮交接前**尚未运行 Docker benchmark**。不要把它当成已通过候选。它基于以下结构：

- `LINUX_STREAM_REACTOR_ENABLED=false`：plain WebSocket 与 realtime TCP 不再交给额外原生 epoll 线程，改为留在 CPU 自适应的每核 Tokio HTTP/I/O 分片，减少 CFS 调度实体和短样本尾延迟。
- HTTP/UDP/realtime 使用每核 I/O 分片；TLS 使用有界、低权重的独立 Tokio runtime，避免 HTTPS 被普通/实时任务完全饿死。
- 2 核 gateway cpuset 下，balanced TLS runtime 使用 `ceil(cores / 2)=1` 个 worker。
- 当前 balanced TLS worker 为 `nice 7`；数据 runtime `event_interval=16`。
- 约 16 MiB benchmark 静态文件走 32 MiB 以下的共享、256 MiB 有界静态缓存；32 MiB 及以上继续走流式/sendfile。

新电脑开始后先执行：

```bash
git switch main
git pull --ff-only origin main
git status --short --branch
rustup target add x86_64-unknown-linux-gnu
cargo install cargo-zigbuild
zig version
docker version
```

确保 Docker 至少提供 4 个 CPU；当前本机典型分配为 gateway `0-1`、backend `2-3`、client `4-7`。

## 已确认的权威事实

### 一分钟反馈链路已经完成

`scripts/benchmark-ubuntu24-amd64-docker.sh` 当前默认使用：

- `DURATION_SECS=3`
- `BENCHMARK_REPETITIONS=1`
- `LOAD_SCALES="1 2 4"`
- `CLIENT_START_LEAD_MS=750`
- `EQUAL_LOAD_FRACTION=0.25`
- `MAX_VALIDATION_SECS=60`
- serial isolated saturation 默认关闭

完整 1x/2x/4x 严格矩阵已多次稳定在 `51-52s` validation time；交叉编译和镜像准备不计入该计时。部分旧文档仍写“2 秒样本”，这是陈旧口径，最终收敛后必须改成与脚本一致的 3 秒。

arm64 Docker 路径使用 Zig + cargo-zigbuild 在宿主交叉编译 `x86_64-unknown-linux-gnu` release ELF，再在 Ubuntu 24 amd64 容器执行同一二进制；不要退回 QEMU 内编译。

### 当前最重要的性能证据

以下数据均来自本机 Docker、Ubuntu 24 x86_64 容器、2 核 gateway cpuset、scale 1 全 11 场景并发。完整筛选报告已归档到 `performance-evidence/development/local-docker/`。

#### `2be6ba4`：统一 realtime 到每核 I/O 分片

- saturation：除 HTTPS 外的 10 个场景吞吐均领先 nginx `1.291x-1.740x`，聚合 `1.424x`。
- HTTPS saturation：`0.395x`，说明统一分片时 TLS 被普通/实时任务饿死。
- equal-load：大多数 p95/p99 明显领先；仍有轻微 p50 缺口：game `1.027x`、SSE `1.032x`、HTTPS `1.157x`、reverse `1.034x`、TCP `1.004x`、WebSocket `1.048x`。
- 结论：减少数据面线程是正确主方向，剩余核心是给 TLS 一个受控份额。

#### `3bf5b54`：TLS 独立 worker，nice 0（已证伪）

- HTTPS saturation 达到 `18.609x`。
- 其他 10 个场景仅 `0.165x-0.414x`。
- 结论：TLS 独立队列有效，但 nice 0 会垄断 2 核混合负载，禁止恢复该权重。

#### `3005fde`：TLS 独立 worker，nice 10，event interval 8

- 除 HTTPS 外的 10 个 saturation 场景全部领先，最低 static-large `1.008x`，其余 `1.022x-1.162x`，聚合 `1.051x`。
- HTTPS saturation `0.570x`。
- equal-load 的 p50 全部领先；仅剩 p95：game `1.438x`、static-large `1.016x`、TCP `1.097x`。
- validation time `17s`（只跑 scale 1）。
- 结论：nice 10 已保护普通路径，但 TLS 份额不足。

#### 当前 `1995e1a`：nice 7，event interval 16

- 这是由上述数据推导的下一候选：Linux CFS nice 10 到 nice 7 的权重约增加 1.95 倍，理论上可把 HTTPS 从 `0.570x` 推向约 `1.11x`。
- event interval 从 8 恢复到 16，目的是补回普通路径至少约 5% 的 saturation 余量。
- 该推导必须由 Docker 实测确认；若失败，以实测为准，不要把理论写成结果。

## 已证伪方案，禁止重复浪费时间

1. 共享缓存大 body 交给两条 nice 0 原生 epoll 写线程（`502a5c1`）：scale 1 realtime/TLS 吞吐退到约 `0.70-0.88x`，CPU 争抢加重。
2. TLS 使用每核两条独立 worker、nice 5 的旧方案（`52b3e01`）：小文件、CDN、SSE、反代整体下降，scale 1 聚合仅 `0.972x`。
3. balanced 大文件启用独立 native sendfile reactor（`cf562e2`）：static-large 没有稳定过线，同时明显抢 TLS CPU。
4. realtime 每核 nice 0 owner 会明显拖慢 HTTP；每批无条件 `thread::yield_now()` 又会把 realtime saturation 砍半。
5. realtime 每核 nice 5/6 owner 的 p99 有改善，但 p50 与总体 CFS 竞争变差；统一到现有 I/O 分片的结果更好。
6. 对大文件连接应用完整 `tune_tcp_stream_for_gateway`：显式 `SO_SNDBUF` 会破坏 Linux autotune，static-large 反而退化。
7. 只修某一个场景再跑一小时的流程不可接受。修改必须针对共享根因，快速先跑 scale 1 全混合，只有 scale 1 全部接近或通过才跑默认三尺度。

## 新电脑执行顺序

### Step A：先验证当前 nice 7 候选

```bash
LOAD_SCALES=1 scripts/benchmark-ubuntu24-amd64-docker.sh
```

记录 saturation 每行 ratio、equal-load 每行 p50/p95/p99 ratio、错误数和 `validation_elapsed_secs`。

决策规则：

- 如果 HTTPS 与其他 10 条 saturation 全部 `>1.0`，不要再调 TLS 权重，直接修剩余 latency 行。
- 如果 HTTPS 仍低于 1，而普通路径有足够余量，按 `nice 7 -> 6` 小步增加 TLS 权重；每次只接受全 11 场景同时改善的结果。
- 如果 HTTPS 过线但 static-large/static-small/TCP/UDP 下降到 1 以下，不能放宽 gate；应限制 TLS 每轮连续处理预算，而不是增加更多线程。
- scale 1 未通过时不要浪费时间跑 2x/4x。

### Step B：收敛剩余 p95 根因

nice 10 的最近证据只剩 game、TCP、static-large p95。优先检查共享根因：

1. game/TCP 在 `LINUX_STREAM_REACTOR_ENABLED=false` 时走 `tokio::io::copy_bidirectional`，与 HTTP/TLS 共用每核 runtime。检查 `DATA_RUNTIME_EVENT_INTERVAL`、relay task 自唤醒和读写 batch；目标是降低 p95，不新增常驻高权重线程。
2. game 与 TCP 同时打到 `18200`，payload 分别为 256 B 和 1024 B。任何优化都要同时验证两行，避免只偏向一种帧大小。
3. static-large 约 16 MiB，具有 4/8/16 条并发连接。共享 `Bytes` 缓存能让 saturation 接近或超过 nginx，但单个 `write_all` 可能形成长尾。若切片，只允许在同一 Tokio task 内有限 cooperative yield；新增 native body writer 已证伪。
4. p95/p99 在 3 秒 amd64 模拟样本中有噪声。不能放宽 `<1.0` gate；最终通过后连续复跑一次完整矩阵确认不是偶然。

### Step C：跑最终默认矩阵

scale 1 全部通过后运行：

```bash
scripts/benchmark-ubuntu24-amd64-docker.sh
```

必须看到：

```text
==> all strict Ubuntu 24 x86_64 Docker scales passed in ...s
```

检查完整报告：

```bash
latest=$(ls -dt .benchmark/direct-ubuntu24-amd64/* | head -1)
cat "$latest/host-fingerprint.txt"
for scale in 1 2 4; do
  cat "$latest/scale-$scale/saturation-summary.md"
  cat "$latest/scale-$scale/equal-load-summary.md"
  cat "$latest/scale-$scale/scale-$scale-nginx-gateway-memory-final.txt"
  cat "$latest/scale-$scale/scale-$scale-proxysss-gateway-memory-final.txt"
done
```

完整 validation 必须 `<=60s`；三档每一行严格通过；内存 current/peak 不超过 nginx 2 倍。

### Step D：同步文档和完整验证

架构最终确定后，至少同步：

- `AGENTS.md`
- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/architecture.html`
- `docs/BENCHMARK-ubuntu24-vs-nginx.md`
- `docs/benchmark-linux.html`

把仍写“2 秒样本”的内容改为脚本真实默认 `3 秒`，并说明 timer 排除 build/setup/warm-up、完整矩阵实测不超过 60 秒。

最终运行：

```bash
cargo fmt --all -- --check
cargo test --locked
go test scripts/benchmark-helper.go scripts/benchmark-helper_test.go
bash -n scripts/benchmark-ubuntu24-amd64-docker.sh
bash -n scripts/benchmark-all-scenarios-isolated.sh
git diff --check
git status --short --ignored
```

最后提交并推送：

```bash
git fetch origin
git status --short --branch
git push origin main
```

## 提交与证据纪律

- benchmark 只接受干净工作树；每个 `.benchmark/direct-ubuntu24-amd64/<run-id>` 必须映射到唯一提交。
- `.benchmark/`、`.ssh/`、`target/`、证书、日志和本地 `proxysss.yaml` 不提交。
- 筛选后的历史报告必须更新到 `performance-evidence/development/local-docker/`；不提交 9 GiB 交叉编译产物、镜像上下文和临时二进制。
- 不删除失败证据来制造成功叙述；不把 scale 1 诊断写成完整通过；不把 emulated-amd64 写成 native x86。
- 最终 push 前确认 `origin/main` 没有新提交；若有，只允许 `git pull --ff-only` 或明确处理冲突，禁止 destructive reset。
