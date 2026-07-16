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
- Linux `runtime.performance.enabled=true` 下，从 cgroup effective cpuset 发现完整 CPU 集；默认 `small` 的 HTTP/1.1、HTTP/2/TLS、WebSocket、TCP、UDP 与透明 QCP 共用一个 CPU-sized Tokio runtime，每个允许核心一个 nice 0 worker，并允许 ready connection 在这些 per-core shard 之间 work-steal。Tokio LIFO slot 保留。global injection queue 每 31 次 poll、I/O driver 每 16 次 poll 检查；8-poll 实验降低了 4x HTTP 吞吐且没有修复尾延迟。低密度 HTTP/TLS 显式 yield 摊到每 256 响应；高密度 plain HTTP 使用每 128 响应的公平边界。静态文件重验证独立使用 256-hit budget，不复用 scheduler fairness 常量。WebSocket 使用 readiness-bounded 64-poll normal relay turn。持续 UDP burst 仍每 8 包有界让出。`balanced` UDP 继续复用该 runtime，TLS 使用 `ceil(cpuset cores / 8)` 的 bounded runtime，并以 nice 5 运行，避免持续 H2 在共享 cpuset 的某一核心上把 nice-0 HTTP/realtime shard 推入 p95/p99；`bulk` 使用专用 transfer owner。HTTP 请求统计使用每 data worker 独占的 cache-line padded counter，采集时汇总；配置 hot reload 通过 ArcSwap 原子发布。
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
- HTTP/2 在 ALPN 已确认后直接使用 Hyper H2 server builder；默认 `small` 的 TLS handshake、H2 与 plain HTTP 共用相同 per-core data shard。warm-up 从 bounded static cache 生成 immutable `Bytes` 响应表，并通过 ArcSwap 发布只读 FxHashMap；同路径 H2 请求不获取 DashMap shard。每条 H2 connection 使用一个本地 `FuturesUnordered` executor 驱动 stream future，避免每请求一次 Tokio spawn/global-queue hop；每完成一个 ready stream 就 cooperative yield，防止热 H2 connection 长驻 Tokio LIFO slot 并阻塞同 shard 的 HTTP/1、SSE、WebSocket、TCP、UDP/QCP。文件过期后继续 stale-while-revalidate 并原子替换有界快照。
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. Plain/TLS fast-lane request readers receive socket bytes directly into persistent per-connection `BytesMut` spare capacity。warm-up 为小型静态对象预构建完整 HTTP/1 keep-alive 响应 bytes，首条连接无需重复格式化 header 和拷贝 body；同一 keep-alive connection 后续 exact static GET head 命中 connection response cache，一秒 revalidation clock 每 256 次 exact hit 检查一次。反向代理/AI 上游池按 CPU 并行预拨，按 pool 去重，并受每池与全局上限约束。低密度 tiny cached static/TLS connection 每 256 个响应显式 cooperative yield；高密度 plain HTTP 使用每 128 响应的 saturated fairness boundary。WebSocket relay 使用 64-poll normal budget。HTTP/2 使用固定大窗口与 warm-up immutable snapshot。raw reverse 每次请求本身都会经过 upstream/downstream readiness，因此不叠加周期性 yield；它复用 parsed request、serialized upstream request、per-downstream upstream lane、response buffer 与 4 KiB framing cache。只有尚未收全的 body 才进入 bounded `ByteBufferPool` relay。Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough。Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path。
- `scripts/benchmark-ubuntu24-amd64-docker.sh` 是禁止 GitHub Actions 性能压测后的本机/原生 Docker 入口：它硬校验 controller 与被测镜像为 Ubuntu 24.04 x86_64；Windows Docker Desktop 可从 Git Bash 使用本地 `npipe://`，Linux/macOS 使用 Unix socket。wrapper 会把 `TRAFFIC_PROFILE`、吞吐/延迟运行顺序、隔离 subnet 与角色 IP 原样传入并写入 fingerprint，便于复现默认 `small`、官方 `balanced` 及双向顺序检查。脚本在容器内构建当前 checkout，再把 gateway、backend、load client 分配到互不重叠的 cpuset/cgroup。每尺度只启动 1 个 backend 和两边 gateway；非被测 gateway 在共享 cpuset 上 pause。每个 wave 只启动 1 个 client 容器，内部保留 11 个独立协议进程，避免容器生命周期淹没 1 秒样本。这样 proxysss 更快的 closed-loop TCP/UDP 不会让 client/backend 多消耗 CPU、反向饿死同轮 HTTP。HTTP、HTTPS、static-large、SSE、WebSocket、TCP、UDP 与透明 QCP 按 1×/2×/4× 一起放大；每档同时判 mixed saturation 吞吐和 equal-offered-load p50/p95/p99，要求零错误、逐场景及聚合严格胜出，原始证据保存在 `.benchmark/direct-ubuntu24-amd64/`。arm64 Docker daemon 使用 `linux/amd64` 模拟时会记录 `execution_mode=emulated-amd64`；两边承受相同模拟成本，但该结果不能冒充物理 x86 证据。arm64 daemon 的 release 构建必须使用 Zig + cargo-zigbuild 在宿主原生速度交叉编译，并在 Ubuntu 24 amd64 容器执行同一 ELF 后才开始测量，不能回退到 QEMU 内编译。
- `scripts/benchmark-all-scenarios-isolated.sh` 默认每尺度运行 1 次 saturation、2 次反向 candidate order 的 equal-load：完整 1×/2×/4× active measurement 为 18 秒并硬限制在 20 秒内，latency report 使用两次样本的 median。inactive gateway 使用 Docker pause；每次 unpause 后双方都先经过默认 250 ms、排除在 active measurement 外的 settle window，再安排 future start。双方使用相同端口、cpuset、nofile、somaxconn、共享 sysctl、协议面与场景 CPU 分区。equal-load 取较慢 saturation 的 25%，要求至少完成 98% 可执行 target 且 proxysss 每个 percentile 严格更低。
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
