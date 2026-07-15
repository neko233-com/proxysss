# proxysss architecture

proxysss is a single Rust binary that replaces nginx/Caddy-style edge duties: protocol termination, routing, load balancing, policy enforcement, and observability. It also covers transparent MQTT/IoT edge patterns while keeping protocol-specific broker logic upstream. Optional TypeScript plugins extend business logic without sitting on every hot-path byte.

## Layers

```
Clients ──► proxysss core (Rust/async)
              ├─ HTTP/HTTPS/H2/H3 listeners
              ├─ TCP/UDP stream listeners (games, MQTT/IoT, KCP, QCP, CoAP)
              ├─ Route matcher + policy chain
              ├─ Upstream pool + health state
              └─ Admin API + metrics
                    ▲
                    │ hot reload
              proxysss.yaml + scripts/plugins
```

| Component | Responsibility |
| --- | --- |
| `gateway` | Listeners, protocol handling, proxy loops, cache, compression |
| `config` | YAML schema, validation, defaults, reload fingerprint |
| `script` | Embedded QuickJS + in-process TypeScript strip for hooks |
| `install` | Background service, init layout, updater integration |
| `admin` (in gateway) | Dashboard, stats, upstream drain, automation upserts |

## Request path (HTTP)

1. Accept connection on plain/TLS/H3 bind.
2. Optional automatic HTTP→HTTPS redirect for managed TLS domains.
3. Serve `/metrics`, `/.well-known/acme-challenge/*`, built-in `/`, `/docs`, `/healthz`. The built-in `/` fallback is a zero-asset `Welcome to proxysss` page with only GitHub and GitHub Docs links; configured user routes still take precedence.
4. Enforce `services.access_control` and `services.rate_limit`.
5. Match static site, WebDAV, domain route, reverse-proxy route, or script hook.
6. Apply cache lookup, upstream selection (LB algorithm + health), retries, and passive quarantine.
7. Proxy request/response (including WebSocket upgrade, generic SSE streaming, New API-compatible routes, and gRPC-over-h2), or serve static files with bounded memory cache, mmap-backed hot objects on supported builds, Range/206 resumable downloads, and a Linux plain-HTTP fast lane whose cache/sendfile behavior follows `runtime.performance.traffic_profile`.
8. Optionally compress response and write access log entry. Successful requests on manual-reload deployments skip the extra post-dispatch config lock used only for error-page decoration or live logging changes.

## Stream path (TCP, UDP, KCP, and QCP datagrams)

1. 在配置的 `tcp.listeners[]` / `udp.listeners[]` bind 接收连接或数据报。Linux 性能模式按 cpuset 建立 `SO_REUSEPORT` worker。balanced 原生 WebSocket/TCP epoll relay 每 4 CPU 一个 nice +5 owner；UDP 与 plain HTTP 复用 per-core shard，并在每 8 个热关联数据报后 cooperative yield，避免 datagram 独占 Tokio queue。TLS 使用 `ceil(cpuset cores / 8)`、nice 0 的单/少 owner crypto runtime。配置通过 ArcSwap 原子发布；balanced 大文件留在 HTTP owner，bulk 才启用 sendfile reactor。
2. Enforce stream access control and shared-zone rate limits where configured.
3. Select an upstream from `upstream` / `upstreams` using the active load-balancing and health state, or use the direct single-upstream TCP fast path when scripts, affinity, active health, passive health, and extra upstream candidates are all disabled.
4. TCP 默认关闭 Nagle（`nodelay: true`）并应用 `connect_timeout_ms`。Linux 性能模式下，游戏、MQTT/tool 与通用实时流优先使用 CPU 自适应分片的原生 `epoll` relay，并保留有界 buffer pool 的 Tokio 回退路径；明确的 bulk/file 协议才使用带 `SPLICE_F_MORE` 的 Linux `splice` 零拷贝。
5. UDP creates a transparent client association to the selected upstream; each datagram refreshes `session_ttl_secs`.
6. Idle UDP associations are pruned with a throttled shared prune state, and `max_associations` caps churn-heavy KCP, QCP, game, and voice fleets so the listener cannot grow unbounded.

MQTT/IoT traffic uses the same stream path: MQTT TCP on `1883`, MQTT TLS passthrough/SNI on `8883`, MQTT over WebSocket through HTTP reverse proxy routes, and CoAP-style UDP through `udp.listeners`.

## Upstream health model

- **Active probes**: periodic HTTP `GET`, TCP connect, or opt-in UDP payload probes per `load_balance.active_health`.
- **Passive quarantine**: consecutive proxy failures trip `quarantine_secs` cooldown.
- **Manual drain**: admin API marks upstreams disabled; state can persist in `runtime.maintenance_state`.
- **Runtime watchdog**: supervised background loops emit heartbeat metrics and can restart after unexpected task failure.
- **Runtime performance plan**: startup reads `runtime.performance`, detects the OS/distro, logs the selected policy once per process start, applies Linux socket tuning on accepted HTTP/TLS/admin/stream sockets and stream upstream sockets, and preloads eligible static hot files/sendfile descriptors according to `traffic_profile`.

## Configuration model

One YAML file is intentional: agents and humans can reason about the entire edge in one document. Cluster nodes self-register through bearer-token `POST /v1/domain-routes/upsert` (and sibling endpoints), which persists back to the same file and reloads in process. Manual reload through `POST /v1/reload` is the default operating model; background file watching is opt-in with `runtime.hot_reload.enabled: true`.

普通公网 WSS 只需配置 `http.tls.auto_https.domains: [wss.example.com]`：非空域名列表会选择内建 managed ACME、正式 Let's Encrypt 与默认 TLS-ALPN-01。无需外部证书客户端、DNS API 凭据或账号邮箱；公网 DNS 必须指向网关且 443 可达。网关负责临时 `acme-tls/1` 证书、证书持久化、续期与 reload；`email` 仅用于到期/安全通知。显式 managed HTTP-01（需 80）、TLS-ALPN-01、DNS-01 与 legacy external ACME 仍受支持。

`services.service_discovery` is a control-plane declaration for Consul, etcd, and Nacos registries. Registry mappings identify which HTTP route, domain route, TCP listener, or UDP listener should receive discovered upstreams; automation/admin writes refresh the YAML upstream pools and then reload. The ordinary data plane still selects from in-memory upstream pools, so HTTP/1.1, HTTP/2/gRPC, WebSocket, TCP, and UDP forwarding do not perform per-request registry network calls.

## Performance notes

- Async Tokio runtime with tuned keepalive connection pooling via `reqwest` for HTTPS/fallback upstreams and a Hyper HTTP/1 fast client for ordinary `http://` reverse proxy traffic. Server connections explicitly enable HTTP/1 vectored writes and HTTP/2 adaptive windows for high-throughput static and proxy responses.
- The plain-HTTP raw reverse-proxy and raw SSE fast lanes preserve the default `X-Forwarded-*` / `Forwarded` chain and `proxysss-ai-*` metadata headers, so default `reverse_proxy` and `ai_proxy` routes can use the low-allocation path without disabling observability.
- Linux `runtime.performance.enabled=true` 下，plain HTTP 与 balanced UDP 复用每核一个 nice 0 shard；UDP 热关联每 8 包主动让出一次，realtime reactor 每 4 CPU 一个 nice +5 owner。TLS 使用 `ceil(cpuset cores / 8)`、nice 0 的独立 runtime，以更少 owner 提高 crypto cache locality。配置 hot reload 通过 ArcSwap 原子发布。正式 4c 包络以 4096 active WSS 和 20k idle WSS 为默认可验证目标。
- TCP listener 通过显式 socket 设置 `SO_REUSEADDR`、大 backlog 和 `TCP_NODELAY`。无 script/affinity/health/多 upstream 的单上游连接直接进入 fast path，不经过通用 upstream planner。实时 profile 保留 `QUICKACK/NODELAY/USER_TIMEOUT`，收发队列交给 Linux autotuning；深队列与 `TCP_NOTSENT_LOWAT` 只用于 HTTP/gateway profile。
- `runtime.performance` is default-on. Linux hosts use portable socket tuning; Ubuntu 24.x additionally enables the extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`). Older Ubuntu/Debian/unknown distros keep the portable path and log the downgrade reason at startup.
- Hot-path shared state avoids one global lock: rate limits, cache zones, sticky affinity, and upstream runtime state use sharded maps; raw HTTP upstream keepalive uses a lock-free bounded queue，checkout 会先用 nonblocking read 丢弃已 EOF、报错或残留脏数据的 idle socket，预热也会同时持有目标数量后再归还，避免长启动屏障后的首个 SSE/reverse 请求命中 stale fd；the native single-upstream fast path skips upstream runtime lookups entirely when active and passive health are disabled. Mutex/RwLock usage is reserved for control-plane reload/certificate state and one-time static-cache fill coordination, not ordinary HTTP/TCP/UDP forwarding.
- `forward_headers: false` on native HTTP routes disables automatic `X-Forwarded-*` / `Forwarded` insertion for nginx-parity and high-throughput deployments that do not need that metadata.
- `services.ai_proxy.routes[*].emit_metadata_headers: false` skips `proxysss-ai-*` upstream metadata headers for nginx-parity SSE paths while preserving native path rewrite and provider routing.
- Native HTTP route resolution borrows global and per-route compression/cache/rate-limit policy on the hot path; owned policy copies are only made for work that must outlive the request task.
- Linux GNU builds use jemalloc as the global allocator to reduce header, routing, and cache bookkeeping overhead under highly concurrent edge workloads.
- Direct TCP listeners, KCP UDP listeners, and QCP UDP listeners keep payloads transparent; protocol labels are observability hints, not hot-path parsers. A policy-free UDP worker keeps a local upstream/config snapshot and refreshes it once per second after reload, so ordinary datagrams do not acquire the dynamic configuration lock. QCP support is therefore an independent edge-forwarding listener for neko233-com/QCP services, not QCP frame termination inside proxysss.
- TCP stream 有独立 relay profile。Linux 性能模式下，游戏小帧与通用 1 KiB realtime TCP 默认使用 CPU 自适应分片的原生 `epoll` relay，每条连接由单一 reactor owner 维护显式 backpressure 和 half-close；`small` 每 2 个 cpuset CPU 一个 owner，`balanced` 与 `bulk` 每 4 个 CPU 一个 owner，三者都随高核 cpuset 线性增长而不设固定上限。reactor worker 数少于 cpuset 核数时使用 soft CPU ownership，让 Linux 把长寿命 owner 留在当前 CPU，只有出现 runnable imbalance 才迁移；一核一 owner 时才反向硬 pin。这样 2 核/1 owner 不会固定压住 CPU1 的 HTTP/UDP shard 并让 CPU0 空闲。`small` realtime 优先 profile 保持 Linux nice 0；混合 `balanced` owner 使用 nice +3，`bulk` owner 使用 nice +5，让 CFS 在同核 HTTP/file task 可运行时有界让出 CPU，但不会像旧 +12 权重那样在 gateway 尚有空闲预算时仍把 WebSocket/game/TCP wake 延迟到 nginx 之后；空闲时连续处理 epoll batch，不用逐 batch `sched_yield` 制造与 CPU 速度无关的吞吐平台。handoff 不可用或非 Linux 时使用多核 Tokio 双向 relay 与有界 `ByteBufferPool`，bulk/file/backup profile 才使用 `splice(socket -> pipe -> socket)`。
- 原生 realtime relay 对一次 level-triggered readiness 最多连续 drain 8 个 16 KiB read batch，合并 WebSocket 小帧、游戏帧和普通 TCP 已排队数据的 epoll/hash dispatch；达到 batch 上限就交还 event loop，partial write 立即启用显式 pending/backpressure。普通数据事件只在本端已观察到 EOF 后才执行完整 pair-finished 查找，避免每帧额外查询两端状态。
- plain WebSocket 在 HTTP shard 完成握手后，Linux 性能模式默认 handoff 到 CPU 自适应分片的原生 `epoll` relay，失败时回退到多核 Tokio relay；WSS 使用有界 Tokio relay 加 rustls/AWS-LC。空闲长连接不会预占每方向固定大 buffer。4c 的生产参考包络默认验证 20k idle WSS，活跃消息规模验证到 4096，并分别报告握手与消息 p50/p95/p99、内存 current/peak 与每连接成本；它不是固定 RAM 准入门槛。
- HTTP/2 在 ALPN 已确认后直接使用 Hyper H2 server builder；TLS 全程留在 `ceil(cpuset cores / 8)`、nice 0 的 crypto runtime。H2 小静态热对象使用 immutable `Bytes` 与 lock-free 配置快照。balanced 大文件在固定 HTTP owner 上 drain `sendfile`，bulk 启用全核 reactor。release 使用 fat LTO 与单 codegen unit。所有 profile 变更都必须重跑完整 mixed gate。
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. Plain/TLS fast-lane request readers receive socket bytes directly into the spare capacity of their persistent per-connection `BytesMut` and return the discovered `head_end` to the caller, removing both the temporary 4 KiB block/copy and a duplicate CRLF delimiter scan on every request. 同一 keep-alive connection 重复的 exact static GET head 在首次校验后直接命中 connection response cache，直到 revalidation deadline 前不再重复 UTF-8/header parse 与 route lookup；exact raw reverse request 同样复用已验证的 parsed request、完整 serialized upstream request bytes 与已选 route/upstream pool，并在 path 分类前命中，避免重复 static/SSE/WebSocket prefix 判断、route/pool 查找以及 target/path/forwarding header 的 String/Vec allocation。Tiny cached static connections amortize explicit cooperative yield over 32 completed requests；这个批次是 mixed regression boundary，放大到 64 会让 static keep-alive 长时间保持 per-core shard runnable，并压低 TLS/realtime/UDP sibling。raw reverse 每次请求本身都会经过 upstream write/read 与 downstream write readiness，因此不再叠加重复的周期性 yield。Raw reverse keeps a per-downstream upstream lane, rewrites prefixes without reparsing `Uri`, filters hop-by-hop headers as bytes, omits redundant `Content-Length: 0`, and reads every upstream response directly into the spare capacity of a bounded per-connection reusable buffer, avoiding a temporary 4 KiB stack block and copy. Fixed-length small responses reuse that same allocation for parsing and forward the raw head plus already-arrived body in one downstream write；只有尚未收全的 body 才进入 bounded `ByteBufferPool` relay，避免普通小包每响应创建 4 KiB `BytesMut`、拆分 `Bytes` 与二次拼接。Repeated identical upstream response heads up to 4 KiB additionally hit a per-downstream framing cache: partial reads first compare the cached prefix, and an exact head reuses status/body framing without delimiter search, `httparse`, or a 64-header scan；head 发生变化时立即退回完整解析并替换 cache，不缓存 response body，超出 4 KiB 的 head 也不会常驻 connection memory。Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough to minimize first-token latency. Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path for simple `ws://` routes.
- `scripts/benchmark-ubuntu24-amd64-docker.sh` 是禁止 GitHub Actions 性能压测后的本机/原生 Docker 入口：它硬校验 controller 与被测镜像为 Ubuntu 24.04 x86_64；Windows Docker Desktop 可从 Git Bash 使用本地 `npipe://`，Linux/macOS 使用 Unix socket。脚本在容器内构建当前 checkout，再把 gateway、backend、load client 分配到互不重叠的 cpuset/cgroup。每尺度只启动 1 个 backend 和两边 gateway；非被测 gateway 在共享 cpuset 上 pause。每个 wave 只启动 1 个 client 容器，内部保留 11 个独立协议进程，避免容器生命周期淹没 3 秒样本。这样 proxysss 更快的 closed-loop TCP/UDP 不会让 client/backend 多消耗 CPU、反向饿死同轮 HTTP。HTTP、HTTPS、static-large、SSE、WebSocket、TCP、UDP 与透明 QCP 按 1×/2×/4× 一起放大；每档同时判 mixed saturation 吞吐和 equal-offered-load p50/p95/p99，要求零错误、逐场景及聚合严格胜出，原始证据保存在 `.benchmark/direct-ubuntu24-amd64/`。arm64 Docker daemon 使用 `linux/amd64` 模拟时会记录 `execution_mode=emulated-amd64`；两边承受相同模拟成本，但该结果不能冒充物理 x86 证据。arm64 daemon 的 release 构建必须使用 Zig + cargo-zigbuild 在宿主原生速度交叉编译，并在 Ubuntu 24 amd64 容器执行同一 ELF 后才开始测量，不能回退到 QEMU 内编译。
- `scripts/benchmark-all-scenarios-isolated.sh` 支持 `BENCHMARK_REPETITIONS` 与交替 `RUN_ORDER`；默认每尺度对每个 gateway/phase 运行 1 个同步 3 秒样本。`validation_elapsed_secs` 只计候选已就绪后的严格矩阵，排除 build/setup/warm-up，并由 wrapper 硬限制为 60 秒；显式增加 `BENCHMARK_REPETITIONS` 时才取中位数，错误始终取最大值。每次运行还生成 `fairness-config.txt`，记录双方相同的端口、cpuset、nofile、somaxconn、共享内核 sysctl、协议面与配置哈希，同时列出 nginx 和 proxysss 各自启用的系统优化。GitHub Actions 只允许六平台打包，不得承载或手动触发性能压测。role-isolated 默认 4+4+8 CPU，也允许本机 wrapper 按 Docker 可用核数等比例切分；报告从实际 gateway cpuset 计算核数。每个 wave 只创建 1 个 client 容器，内部 11 个独立协议进程先等待共享 volume；确认该容器运行后，控制器写入统一绝对时间，各进程再把它作为 `--start-at-unix-ms` 交给进程内 worker barrier。测量窗口不会被 Docker create/start（包括 amd64 模拟启动）缩短或错开。saturation client 使用完整 client cpuset 以压满更快的 gateway；fixed-rate equal-load 的小包 client 每进程固定 1 个 Tokio I/O worker，static-large 固定 2 个，避免 11 个进程各自按完整 cpuset 扩张后让发生器 timer 跳 tick；调度迟到的 fixed-rate slot 在每 worker 最多一个 in-flight 的边界内补齐，不静默丢弃目标请求。equal-load 默认使用双方较慢 saturation 的 25%，在 11 个隔离 client 同时运行时保留真实 latency headroom；仍要求双方至少完成 98% target，且 proxysss 每个 percentile 严格更低。nginx 1.31.2 mainline 使用 `-O3 -fno-plt`、相同 gateway cpuset 和等价协议配置。透明 QCP 通过双方独立的等价 UDP listener 进入同一 wave，只证明 edge forwarding，不声称 frame termination。serial isolated saturation 默认关闭，可显式单样本运行用于单场景诊断，但不能替代 mixed gate。`MIXED_SCENARIOS` 可做不改变方法学的根因诊断。`benchmark-websocket-production-gate.sh` 另测多尺度 WSS active 与 20k idle hold；`benchmark-cross-host-wss.sh` 从独立 client host 把同 SHA binary 布置到 gateway/backend，并以远端 systemd cgroup 强制 4 CPU、300k nofile，保留 cgroup memory current/peak 与每连接成本、主机/`nginx -V` 指纹及原始样本，再严格复跑 WSS 吞吐、p50/p95/p99 和容量。`MemoryMax` 只在声明了生产内存预算时显式设置，不以任意固定 RAM 阈值拒绝证据。Docker role isolation 不能冒充三台物理机。
- 60 秒 validation deadline 会下传到每个 client wave，并由 GNU `timeout` 按剩余预算硬终止；超时同时停止容器内发生器并失败。默认启动 lead 为 100 ms，UDP/QCP 尾部等待 500 ms，3 秒测量窗口不缩短。
- nginx 对照矩阵不比较 KCP/QCP 协议专用封装；严格本机矩阵只把 `protocol: qcp` 的透明 UDP 转发加入同一 mixed wave，与 nginx 等价 UDP listener 比较并接受同一严格门禁。这个结果证明 edge forwarding，不代表 proxysss 在热路径解析或终止 QCP frame。
- UDP association TTL and caps bound memory under large mobile/game reconnect churn; listener receive buffers are reused from the bounded UDP buffer pool so ordinary datagram forwarding does not allocate a full packet buffer per receive. New-session deduplication uses a sharded pending set instead of one global mutex, and global association pruning is throttled by time/create-count/cap-pressure so a reconnect storm does not scan the whole table for every new association. Once a client association exists, subsequent datagrams use a worker-local association cache and an in-loop fast path that refreshes the global TTL timestamp at most once per second while sending directly to the connected upstream socket, avoiding per-packet routing, request-id allocation, payload copying, task spawning, and global association-table lookups.
- UDP active health is opt-in so opaque KCP/game protocols are not marked unhealthy unless operators configure the expected probe behavior.
- Script hooks are optional and isolated; the default gateway path avoids script calls.
- Compression and cache operate on response bodies with size guards.
- `proxysss tune linux` includes explicit Ubuntu 22.04, 24.04, and 26.04 LTS profiles plus Debian profiles for backlog, BBR/fq, packet budget, and connection churn tuning.

## Extension points

- `script.entry` main module: `routeHttp`, `routeTcp`, `routeUdp` hooks.
- `plugins.auto_load_dir`: prioritized plugin modules with optional `<name>.plugin.yaml` sidecars.
- Admin automation for dynamic route/listener upserts.

## Interactive visualization

Open [architecture.html](./architecture.html) in a browser for an animated first-year-student protocol lab. It explains HTTP, TLS/ACME, WebSocket, gRPC, TCP, UDP, KCP, QCP, MQTT/IoT, FTP, AI API streaming, admin reload, listeners, policy chains, extension hooks, and reload boundaries without external JavaScript dependencies.

## Related docs

- [CONFIGURATION.md](./CONFIGURATION.md) — field-by-field tutorial
- [PRODUCTION-HARDENING.md](./PRODUCTION-HARDENING.md) — release gates, benchmark baselines, HA, and watch points
- [IMPROVEMENT-BACKLOG.md](./IMPROVEMENT-BACKLOG.md) — stability, performance, protocol, security, and operations backlog
- [../nginx-to-proxysss.md](../nginx-to-proxysss.md) — migration mapping
- [../ts-how-to-use.md](../ts-how-to-use.md) — scripting guide
