# proxysss 性能开发交接计划

## 当前交接点

分支：`main`

当前代码候选：`6580c82 Cache exact H2 static routes per connection`

相对 `origin/main` 的性能实现主线：

- `c67dad7`：data runtime 使用 `global_queue_interval=31`、`event_interval=8`；TLS HTTP/1 静态快路径用 rustls vectored writer 同时提交 header 与共享 `Bytes`，不再复制正文。
- `e553c48`：预加载的 H2 static route 合并为一次 `DashMap` lookup，正文通过 `ArcSwap` 无锁共享并 stale-while-revalidate。
- `47a9a18`：默认 `balanced` 在共享 256 MiB/256-entry 上限内预载 32 MiB 以下静态正文；strict matrix 的约 16 MiB large fixture 在配置加载阶段准备，正式样本不支付首次加载成本。
- `6580c82`：同一 H2 connection 的首个 exact-path stream 完成 route lookup，后续同路径 stream 命中 connection-local `OnceLock`；没有引入全局锁。

当前默认 `balanced` 数据面：

- `LINUX_STREAM_REACTOR_ENABLED=false`：plain WebSocket 与 realtime TCP 留在 CPU 自适应的 per-core Tokio HTTP/I/O shard，减少额外 CFS runnable 实体与跨 runtime wake。
- HTTP/TCP/UDP runtime：每核 shard，`global_queue_interval=31`、`event_interval=8`。
- TLS runtime：`ceil(cpuset cores / 2)` workers、nice +7；2 核 gateway cpuset 下为 1 worker。
- balanced sendfile reactor 关闭；32 MiB 及以上对象由 connection owner 按 Tokio writable readiness 流式发送，显式 `bulk` profile 才启用独立 reactor。

`6580c82` 已完成：

```bash
cargo fmt --all
cargo test --locked lock_free_h2_route_cache_shares_body_and_coalesces_revalidation -- --nocapture
git diff --check
```

H2 connection-local cache 尚未完成 Docker matrix，不能写成已验证结果。

## 权威严格矩阵

入口：

```bash
scripts/benchmark-ubuntu24-amd64-docker.sh
```

当前脚本默认：

- `DURATION_SECS=3`
- `BENCHMARK_REPETITIONS=1`
- `LOAD_SCALES="1 2 4"`
- `CLIENT_START_LEAD_MS=750`
- `EQUAL_LOAD_FRACTION=0.25`
- `MIN_TARGET_ACHIEVEMENT=0.98`
- `MAX_VALIDATION_SECS=60`
- serial isolated saturation 关闭

proxysss 是 AOT binary，不做 JIT warm-up。构建、镜像/容器准备、确定性的配置加载缓存准备与 readiness 在 strict validation timer 之前完成；计时内只包含正式 mixed saturation、equal-load 和报告/gate，完整 1×/2×/4× 必须不超过 60 秒。

本轮工作机是 Windows x86_64 + Docker Desktop Linux/amd64，而 `TARGET.md` 的唯一完成标准明确要求本机 Mac arm64 Docker + `execution_mode=emulated-amd64`。因此这里的 Ubuntu 24 amd64 结果可用于实现收敛和回归诊断，但即使三尺度全绿，也不能冒充最终 TARGET 证据；最后仍需在目标 Mac arm64 上对同一提交复跑并归档原始报告。

## 最近有效证据

### `c67dad7`：vectored TLS static + event interval 8

scale 1、Ubuntu 24 amd64 Docker、11 场景并发：

- saturation 11 行全部领先；聚合 `1.582x`。
- 代表性比例：CDN `1.241x`、game `2.332x`、SSE `2.152x`、HTTPS `1.577x`、QCP `1.894x`、reverse `1.664x`、static-large `1.009x`、static-small `1.207x`、TCP `2.240x`、UDP `2.175x`、WebSocket `2.193x`。
- equal-load 只剩 HTTPS p50 `1.181x`、reverse p50 `1.008x` 未过；其他 percentile 通过。
- 零错误，validation 约 19 秒，proxysss memory 明显低于 nginx。

### `e553c48`：H2 route/payload 无锁合并

一次较干净的 scale 1：

- saturation 仅 static-large `0.929x` 未过；聚合 `1.334x`。
- equal-load 仅 HTTPS p50 `1.113x`、UDP p99 `1.047x`、WebSocket p99 `1.154x` 未过。
- proxysss current/peak 约 26/76 MiB，nginx 约 182/240 MiB，远低于 2x 内存边界。

### `47a9a18`：balanced 预载 16 MiB fixture

- 后续两次 scale 1 均受到宿主 `wasm-opt`/Tuanjie/Weixin 构建负载干扰；不能作为最终证据。
- 在相对较轻的一次中 static-large 已过线，只有 static-small saturation `0.995x`；说明预载方向有效，但仍需在宿主安静时复跑同一 commit。

## 已证伪方案

不要重复恢复下列候选：

1. balanced TLS nice `7 -> 6`（`e895b8f`，后由 `8085769` 回退）：TLS 抢占兄弟路径，整体更差。
2. TLS H2 `event_interval 8 -> 4`（`2836212`，后由 `6b73263` 回退）：H2 p50 略有改善，但 static/realtime 吞吐被抢走。
3. TLS global queue interval `31 -> 8`（`06fe7d0`，后由 `83318f1` 回退）：增加注入队列检查开销，没有形成全矩阵收益。
4. exact TLS HTTP/1 connection cache（`3cc2f91`，后由 `1983fd3` 回退）：不是 H2 benchmark 路径，且 mixed 结果退化。
5. TLS 两 worker/nice 6（`aa2ecfa`，后由 `a77e0e2` 回退）：2 核包络中增加 scheduler 竞争。
6. balanced 独立 native sendfile reactor：static-large 没有稳定过线，并抢占 TLS/realtime CPU。
7. 额外 realtime native epoll owner：短样本的 p50/p99 与兄弟吞吐不如统一 per-core Tokio I/O shard。

所有单场景、单尺度与受宿主构建负载污染的结果只作诊断，不能替代 strict matrix。

## 下一步执行顺序

### Step A：完成文档口径并提交

当前工作树中的文档变更已把陈旧样本时长统一为脚本真实的 3 秒，明确 AOT 无 JIT warm-up、strict timer 排除 build/setup/readiness，并同步当前 per-core Tokio relay、TLS nice +7、balanced preload 与 sendfile 边界。

提交前检查：

检查所有官方 benchmark 入口不再保留旧时长口径，并运行 `git diff --check`。

### Step B：宿主安静后先跑 scale 1

不要终止用户的后台构建进程。先确认没有多核 `wasm-opt`、`wasm-emscripten-finalize`、`emcc/python` 或类似编译任务，再运行：

```bash
LOAD_SCALES=1 MAX_VALIDATION_SECS=60 scripts/benchmark-ubuntu24-amd64-docker.sh
```

同一提交至少连续复跑一次；第二次复用 AOT release artifact，不重复把 build 噪声带进判断。记录：

- saturation 11 行 ratio 与 aggregate；
- equal-load 11 行 p50/p95/p99 与双方完成率；
- 零错误；
- cgroup current/peak/每连接成本；
- `validation_elapsed_secs`。

只有 scale 1 所有严格 gate 都过，才跑三尺度。若 H2 p50 仍未过，不再改变 TLS runtime 权重/轮询间隔；优先从 H2 response 构造中的 per-stream header/state 成本继续消除，且每次都用 11 场景 mixed 回归。

### Step C：完整 1×/2×/4×

```bash
MAX_VALIDATION_SECS=60 scripts/benchmark-ubuntu24-amd64-docker.sh
```

必须看到：

```text
==> all strict Ubuntu 24 x86_64 Docker scales passed in ...s
```

三档每一行 throughput 和 p50/p95/p99 都严格通过、双方完成率至少 98%、零错误、validation `<=60s`、proxysss memory current/peak/每连接成本都 `<=2x nginx`。通过后立即用同一提交连续复跑完整 matrix，排除 3 秒样本偶然噪声。

### Step D：目标 Mac arm64 最终复现

在 `TARGET.md` 指定的 Mac arm64 Docker 上，对同一提交运行默认入口。必须记录 `execution_mode=emulated-amd64`，使用 Zig + cargo-zigbuild 在宿主原生速度交叉编译 release ELF，再在 Ubuntu 24 amd64 容器运行；禁止 QEMU 内编译，禁止 SSH 远程主机。

### Step E：归档、全量测试与推送

筛选同一最终提交的原始报告到 `performance-evidence/development/local-docker/`，不提交 `.benchmark/`、镜像上下文、binary、密钥或日志。然后运行：

```bash
cargo fmt --all -- --check
cargo test --locked
go test scripts/benchmark-helper.go scripts/benchmark-helper_test.go
bash -n scripts/benchmark-ubuntu24-amd64-docker.sh
bash -n scripts/benchmark-all-scenarios-isolated.sh
git diff --check
git status --short --ignored
git fetch origin
git push origin main
```

只有同一最终提交的 Mac arm64 原始报告满足 `TARGET.md` 全部条目、全量测试通过、工作树干净并已推送 `origin/main`，才能更新 goal 为 complete 并宣称“全面超过 nginx”。
