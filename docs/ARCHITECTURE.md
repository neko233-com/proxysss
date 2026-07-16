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

1. 在配置的 `tcp.listeners[]` / `udp.listeners[]` bind 接收连接或数据报。Linux 性能模式从 cgroup effective cpuset（而非可能已被 pin 的当前线程）发现完整 CPU 集，并据此建立全核 `SO_REUSEPORT` listener fanout。默认 `small` 把 HTTP/1.1、HTTP/2/TLS、WebSocket、TCP、UDP 与透明 QCP 放进同一组 CPU 自适应 per-core data shard，避免额外 TLS/UDP runtime 与 HTTP 争抢 CFS 时间；UDP 每 8 个持续 burst 数据报 cooperative yield。`balanced` UDP 同样复用 shard，但 TLS 保持 `ceil(cpuset cores / 8)` 的 bounded runtime；`bulk` 才启用独立 transfer owner/sendfile reactor。配置通过 ArcSwap 原子发布。
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

普通公网 WSS 只需配置 `http.tls.auto_https.domains: [wss.example.com]`：非空域名列表会选择免费的内建 managed ACME、正式 Let's Encrypt、默认 TLS-ALPN-01 与 ECDSA P-256 证书密钥。无需外部证书客户端、DNS API 凭据或账号邮箱；公网 DNS 必须指向网关且 443 可达。网关负责临时 `acme-tls/1` 证书、证书持久化、续期与 reload；`email` 仅用于到期/安全通知。极老旧客户端可用 `http.tls.acme.key_algorithm: rsa2048`，显式 managed HTTP-01（需 80）、TLS-ALPN-01、DNS-01 与 legacy external ACME 仍受支持。

`services.service_discovery` is a control-plane declaration for Consul, etcd, and Nacos registries. Registry mappings identify which HTTP route, domain route, TCP listener, or UDP listener should receive discovered upstreams; automation/admin writes refresh the YAML upstream pools and then reload. The ordinary data plane still selects from in-memory upstream pools, so HTTP/1.1, HTTP/2/gRPC, WebSocket, TCP, and UDP forwarding do not perform per-request registry network calls.

## Performance notes

- Async Tokio runtime with tuned keepalive connection pooling via `reqwest` for HTTPS/fallback upstreams and a Hyper HTTP/1 fast client for ordinary `http://` reverse proxy traffic. Server connections explicitly enable HTTP/1 vectored writes；HTTP/2 使用固定的大 stream/connection window，避免 small-object mixed load 为 adaptive-window 调节付出额外热路径成本。
- The plain-HTTP raw reverse-proxy and raw SSE fast lanes preserve the default `X-Forwarded-*` / `Forwarded` chain and `proxysss-ai-*` metadata headers, so default `reverse_proxy` and `ai_proxy` routes can use the low-allocation path without disabling observability.
- Linux `runtime.performance.enabled=true` 下，从 cgroup effective cpuset 发现完整 CPU 集；默认 `small` 的 HTTP/1.1、HTTP/2/TLS、WebSocket、TCP、UDP 与透明 QCP 共用一个 CPU-sized Tokio runtime，每个允许核心一个 nice 0 worker，并允许 ready connection 在这些 per-core shard 之间 work-steal。Tokio LIFO slot 保留，用于 connection-local I/O 的即时唤醒。global injection queue 每 31 次 poll、I/O driver 每 16 次 poll 检查。低密度 HTTP/TLS 依靠 socket readiness 与 Tokio cooperative budget 自然让出，显式 HTTP yield 在每 data worker 少于 64 条连接时摊到每 256 响应；达到该混合负载密度后使用每 128 响应的公平边界，使 yield 事件低于 p99，又避开 256-response overload knee。WebSocket 每轮最多推进 8 个 relay poll，短帧队列可直接排空，持续可读 tunnel 仍不能占用 bulk relay 的 64-step budget。持续 UDP burst 仍每 8 包有界让出。`balanced` UDP 继续复用该 runtime，TLS 使用 `ceil(cpuset cores / 8)` 的 bounded runtime；`bulk` 使用专用 transfer owner。HTTP 请求统计使用每 data worker 独占的 cache-line padded counter，采集时汇总；配置 hot reload 通过 ArcSwap 原子发布。
- TCP listener 通过显式 socket 设置 `SO_REUSEADDR`、大 backlog 和 `TCP_NODELAY`。无 script/affinity/health/多 upstream 的单上游连接直接进入 fast path，不经过通用 upstream planner。实时 profile 保留 `QUICKACK/NODELAY/USER_TIMEOUT`，收发队列交给 Linux autotuning；深队列与 `TCP_NOTSENT_LOWAT` 只用于 HTTP/gateway profile。
- `runtime.performance` is default-on. Linux hosts use portable socket tuning; Ubuntu 24.x additionally enables the extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`). Older Ubuntu/Debian/unknown distros keep the portable path and log the downgrade reason at startup.
- Hot-path shared state avoids one global lock: rate limits, cache zones, sticky affinity, and upstream runtime state use sharded maps; raw HTTP upstream keepalive uses a lock-free bounded queue，checkout 会先用 nonblocking read 丢弃已 EOF、报错或残留脏数据的 idle socket，预热也会同时持有目标数量后再归还，避免长启动屏障后的首个 SSE/reverse 请求命中 stale fd。默认 small 的 raw reverse/SSE upstream socket 使用 Linux autotuning 与 low-latency socket 选项，不强制 8 MiB bulk queue；the native single-upstream fast path skips upstream runtime lookups entirely when active and passive health are disabled. Mutex/RwLock usage is reserved for control-plane reload/certificate state and one-time static-cache fill coordination, not ordinary HTTP/TCP/UDP forwarding.
- `forward_headers: false` on native HTTP routes disables automatic `X-Forwarded-*` / `Forwarded` insertion for nginx-parity and high-throughput deployments that do not need that metadata.
- `services.ai_proxy.routes[*].emit_metadata_headers: false` skips `proxysss-ai-*` upstream metadata headers for nginx-parity SSE paths while preserving native path rewrite and provider routing.
- Native HTTP route resolution borrows global and per-route compression/cache/rate-limit policy on the hot path; owned policy copies are only made for work that must outlive the request task.
- Linux GNU builds use jemalloc as the global allocator to reduce header, routing, and cache bookkeeping overhead under highly concurrent edge workloads.
- Direct TCP listeners, KCP UDP listeners, and QCP UDP listeners keep payloads transparent; protocol labels are observability hints, not hot-path parsers. A policy-free UDP worker keeps a local upstream/config snapshot and refreshes it once per second after reload, so ordinary datagrams do not acquire the dynamic configuration lock. QCP support is therefore an independent edge-forwarding listener for neko233-com/QCP services, not QCP frame termination inside proxysss.
- TCP stream 有独立 relay profile，但默认 realtime/game/TCP 与 plain WebSocket 都留在统一多核 Tokio data runtime：每条连接使用单 future 双向 relay、显式 half-close 与有界 `ByteBufferPool`，避免额外长期 runnable epoll owner 与 HTTP/H2/UDP worker 竞争 CFS 时间片。Linux 原生 stream reactor 保留为未启用的实验实现，不属于默认生产路径；bulk/file/backup profile 才使用 `splice(socket -> pipe -> socket)`。
- 原生 realtime relay 对一次 level-triggered readiness 最多连续 drain 8 个 16 KiB read batch，合并 WebSocket 小帧、游戏帧和普通 TCP 已排队数据的 epoll/hash dispatch；达到 batch 上限就交还 event loop，partial write 立即启用显式 pending/backpressure。普通数据事件只在本端已观察到 EOF 后才执行完整 pair-finished 查找，避免每帧额外查询两端状态。
- plain WebSocket 在 HTTP shard 完成握手后，Linux 性能模式默认 handoff 到 CPU 自适应分片的原生 `epoll` relay，失败时回退到多核 Tokio relay；WSS 使用有界 Tokio relay 加 rustls/AWS-LC。空闲长连接不会预占每方向固定大 buffer。4c 的生产参考包络默认验证 20k idle WSS，活跃消息规模验证到 4096，并分别报告握手与消息 p50/p95/p99、内存 current/peak 与每连接成本；它不是固定 RAM 准入门槛。
- HTTP/2 在 ALPN 已确认后直接使用 Hyper H2 server builder；默认 `small` 的 TLS handshake、H2 与 plain HTTP 共用相同 per-core data shard，避免 crypto runtime 额外抢占，`balanced` TLS 仍使用每 8 核一个 bounded owner。H2 小静态热对象使用 immutable `Bytes`、固定大窗口与 revalidation-bounded per-path precompiled response snapshot；普通热请求只做一次路径查询并 clone 已编译 body/header，过期时仍回到原 static cache 并触发异步 metadata revalidation。small 大文件保持经过 mixed 验证的 2 MiB `sendfile` cooperative slice；16 MiB ceiling 会占用共享 shard 并拖慢 HTTP siblings，故不采用。`bulk` 才启用全核 reactor。release 使用 fat LTO 与单 codegen unit。所有 profile 变更都必须重跑完整 mixed gate。
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. Plain/TLS fast-lane request readers receive socket bytes directly into the spare capacity of their persistent per-connection `BytesMut` and return the discovered `head_end` to the caller, removing both the temporary 4 KiB block/copy and a duplicate CRLF delimiter scan on every request. 同一 keep-alive connection 重复的 exact static GET head 在首次校验后直接命中 connection response cache，直到 revalidation deadline 前不再重复 UTF-8/header parse 与 route lookup；一秒 revalidation clock 每 128 次 exact hit 检查一次。fairness threshold 同样只在 yield boundary 刷新，不再每请求读取全局连接计数。exact raw reverse request 复用已验证的 parsed request、完整 serialized upstream request bytes 与已选 route/upstream pool，并在 path 分类前命中，避免重复 static/SSE/WebSocket prefix 判断、route/pool 查找以及 target/path/forwarding header 的 String/Vec allocation。低密度 tiny cached static/TLS connection 每 256 个响应才显式 cooperative yield；当总连接数达到每 data worker 64 条时，使用每 128 响应的 saturated fairness boundary，兼顾 2x/4x mixed load 公平性并把显式 yield 事件压到 p99 以下。WebSocket relay 每轮最多推进 8 个 poll step，使短帧队列直接排空，同时限制持续可读 tunnel 对共享 small-profile runtime 的占用。HTTP/2 使用固定大窗口与预编译静态响应快照。raw reverse 每次请求本身都会经过 upstream write/read 与 downstream write readiness，因此不再叠加重复的周期性 yield。Raw reverse keeps a per-downstream upstream lane, rewrites prefixes without reparsing `Uri`, filters hop-by-hop headers as bytes, omits redundant `Content-Length: 0`, and reads every upstream response directly into the spare capacity of a bounded per-connection reusable buffer, avoiding a temporary 4 KiB stack block and copy. Fixed-length small responses reuse that same allocation for parsing and forward the raw head plus already-arrived body in one downstream write；只有尚未收全的 body 才进入 bounded `ByteBufferPool` relay，避免普通小包每响应创建 4 KiB `BytesMut`、拆分 `Bytes` 与二次拼接。Repeated identical upstream response heads up to 4 KiB additionally hit a per-downstream framing cache: partial read first compare the cached prefix, and an exact head reuses status/body framing without delimiter search, `httparse`, or a 64-header scan；head 发生变化时立即退回完整解析并替换 cache，不缓存 response body，超出 4 KiB 的 head 也不会常驻 connection memory。Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough to minimize first-token latency. Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path for simple `ws://` routes.
- `scripts/benchmark-ubuntu24-amd64-docker.sh` 是禁止 GitHub Actions 性能压测后的本机/原生 Docker 入口：它硬校验 controller 与被测镜像为 Ubuntu 24.04 x86_64；Windows Docker Desktop 可从 Git Bash 使用本地 `npipe://`，Linux/macOS 使用 Unix socket。wrapper 会把 `TRAFFIC_PROFILE`、吞吐/延迟运行顺序、隔离 subnet 与角色 IP 原样传入并写入 fingerprint，便于复现默认 `small`、官方 `balanced` 及双向顺序检查。脚本在容器内构建当前 checkout，再把 gateway、backend、load client 分配到互不重叠的 cpuset/cgroup。每尺度只启动 1 个 backend 和两边 gateway；非被测 gateway 在共享 cpuset 上 pause。每个 wave 只启动 1 个 client 容器，内部保留 11 个独立协议进程，避免容器生命周期淹没 1 秒样本。这样 proxysss 更快的 closed-loop TCP/UDP 不会让 client/backend 多消耗 CPU、反向饿死同轮 HTTP。HTTP、HTTPS、static-large、SSE、WebSocket、TCP、UDP 与透明 QCP 按 1×/2×/4× 一起放大；每档同时判 mixed saturation 吞吐和 equal-offered-load p50/p95/p99，要求零错误、逐场景及聚合严格胜出，原始证据保存在 `.benchmark/direct-ubuntu24-amd64/`。arm64 Docker daemon 使用 `linux/amd64` 模拟时会记录 `execution_mode=emulated-amd64`；两边承受相同模拟成本，但该结果不能冒充物理 x86 证据。arm64 daemon 的 release 构建必须使用 Zig + cargo-zigbuild 在宿主原生速度交叉编译，并在 Ubuntu 24 amd64 容器执行同一 ELF 后才开始测量，不能回退到 QEMU 内编译。
- `scripts/benchmark-all-scenarios-isolated.sh` 支持 `BENCHMARK_REPETITIONS` 与交替 `RUN_ORDER`；默认每尺度对每个 gateway/phase 运行 1 个同步 1 秒样本。`validation_elapsed_secs` 累加所有 active client measurement window，完整 1×/2×/4× 默认为 12 秒并硬限制在 20 秒内；build/setup/warm-up、进程启动、结果复制和报告解析不计入该值，`validation_wall_elapsed_secs` 单独记录矩阵编排时间。显式增加 `BENCHMARK_REPETITIONS` 时才取中位数，错误始终取最大值。每次运行还生成 `fairness-config.txt`，记录双方相同的端口、cpuset、nofile、somaxconn、共享内核 sysctl、协议面、配置哈希与场景 CPU 分区，同时列出 nginx 和 proxysss 各自启用的系统优化。GitHub Actions 只允许六平台打包，不得承载或手动触发性能压测。完整 mixed gate 至少需要 24 个 Docker CPU；报告从实际 gateway cpuset 计算核数。每个 wave 只创建 1 个 client 容器，内部 11 个独立协议进程先等待共享 volume；确认该容器运行后，控制器写入统一绝对时间，各进程再把它作为 `--start-at-unix-ms` 交给进程内 worker barrier。每个场景通过 `taskset` 获得互不重叠的 client CPU；backend HTTP、SSE、WebSocket、TCP、UDP、QCP 进程也各自分区，UDP 与 QCP 使用独立 upstream listener，防止更快的 sibling 抢走发生器或后端 CPU。测量窗口不会被 Docker create/start（包括 amd64 模拟启动）缩短或错开。serial isolated saturation 显式启用时使用完整 client cpuset。equal-load 默认使用双方较慢 saturation 的 25%，仍要求双方至少完成 98% target，且 proxysss 每个 percentile 严格更低；HTTP/SSE worker 从 phase 0 起均匀铺满 interval，gate 按完整 aggregate slot 计算整数 target，等待下一个 slot 超过 measurement deadline 时立即退出，不再拖过 process grace。nginx 1.31.2 mainline 使用 `-O3 -fno-plt`、相同 gateway cpuset 和等价协议配置。透明 QCP 通过双方独立的等价 UDP listener 进入同一 wave，只证明 edge forwarding，不声称 frame termination。serial isolated saturation 默认关闭，可显式单样本运行用于单场景诊断，但不能替代 mixed gate。`MIXED_SCENARIOS` 可做不改变方法学的根因诊断。`benchmark-websocket-production-gate.sh` 另测多尺度 WSS active 与 20k idle hold；`benchmark-cross-host-wss.sh` 从独立 client host 把同 SHA binary 布置到 gateway/backend，并以远端 systemd cgroup 强制 4 CPU、300k nofile，保留 cgroup memory current/peak 与每连接成本、主机/`nginx -V` 指纹及原始样本，再严格复跑 WSS 吞吐、p50/p95/p99 和容量。`MemoryMax` 只在声明了生产内存预算时显式设置，不以任意 fixed RAM threshold 拒绝证据。Docker role isolation 不能冒充三台物理机。
- 20 秒上限按 active measurement window 累加；每个 wave 启动前必须预留完整采样时长，并由 GNU `timeout` 另设 4 秒独立 process grace，卡死会停止容器内发生器并失败。默认启动 lead 为 50 ms，UDP/QCP response timeout 为 500 ms；这些编排/尾部时间不计入 active measurement，1 秒测量窗口不缩短。
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
