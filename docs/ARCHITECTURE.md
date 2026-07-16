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

1. 在配置的 `tcp.listeners[]` / `udp.listeners[]` bind 接收连接或数据报。Linux 性能模式从 cgroup effective cpuset（而非可能已被 pin 的当前线程）发现完整 CPU 集，并据此建立全核 `SO_REUSEPORT` listener fanout；flow placement 交给 Linux reuseport/RFS 自适应，避免 per-socket `SO_INCOMING_CPU` 强绑造成混合 HTTP 尾延迟。默认 `small` 把 HTTP/1.1、HTTP/2/TLS、WebSocket、TCP、UDP 与透明 QCP 放进同一个 CPU-sized Tokio runtime；`balanced` 把每个 plain HTTP accept loop 固定到独立单线程 Tokio runtime，TLS/H2 放到 nice 5 bounded runtime，UDP/QCP 放到 `ceil(cores/2)` nice 3 runtime，并在透明 TCP/plain WebSocket 完成 connect/upgrade 后交给 `ceil(cores/2)` nice 3 native epoll relay。plain HTTP runtime 每 8 次 task poll 检查 I/O，shared-small 保持 16 次；8 MiB 以上 balanced 静态响应从有界 lock-free pool 独占 file description，根据 active sendfile transfer 压力由连接所属 HTTP owner 或预热 native reactor 发送，完成后归还。配置通过 ArcSwap 原子发布。
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
- Linux `runtime.performance.enabled=true` 下，从 cgroup effective cpuset 发现完整 CPU 集；默认 `small` 的 HTTP/1.1、HTTP/2/TLS、WebSocket、TCP、UDP 与透明 QCP 共用一个 CPU-sized Tokio runtime，每个允许核心一个 nice 0 worker，并允许 ready connection 在这些 per-core shard 之间 work-steal。`balanced` 将每个 plain HTTP `SO_REUSEPORT` accept loop 映射到固定 CPU 的独立单线程 Tokio runtime，消除跨核 work stealing 和共享 LIFO slot 引起的明文 HTTP 尾延迟；这些 plain runtime 每 8 次 task poll 检查 I/O，shared-small runtime 保持 16 次。TLS/H2 使用 nice 5 bounded runtime；UDP/QCP 使用 `ceil(cpuset/2)` nice 3 runtime；policy-free realtime TCP 和 plain WebSocket 在 connect/upgrade 后进入 `ceil(cpuset/2)` nice 3 native epoll relay。blocked client-request tick 在每 owner 不超过 16/32/128 对时分别追加 1/2/32 次有界 nonblocking reply poll；reply event 不继续 spin，持续 ready 的 saturation 也不启用该路径。nice 5 realtime 实验降低吞吐且未改善 static-large，已回退。plain fast lane 与 raw plain-upstream response head 在注册 readiness 前先执行有界 `try_read_buf` / `try_write`，减少单请求 async trait/cooperative bookkeeping，但不会像已回退的整段 `unconstrained` 实验那样绕过整个 exchange 的公平性。8 MiB 以上 balanced 静态对象在 current active sendfile transfer 大于数据核数且不超过每核 4 条时交给预热的 nice-0 native reactor；稀疏/equal-load 与更高压力直接留在 HTTP owner。首批 sparse transfer cooperative yield 一次，让 sibling large response 完成登记；bulk 可始终使用 reactor。reactor handoff 优先选择当前 pinned HTTP CPU 对应 owner，无法匹配时才 round-robin；active jobs 使用 fd-indexed slot table，job 在 EPOLLOUT 注册前直接尝试 sendfile，adaptive balanced job 以完整 response 为 event budget，真实 `EAGAIN` 才是公平与背压边界。完成时 reactor 先清除 `TCP_CORK` 再唤醒 HTTP owner，末尾字节不等待下一轮 Tokio 调度。warm-up 在 live traffic 前启动 eligible reactor，并为每个热大文件注入有界 lock-free descriptor pool。并发响应独占 file description，完成后归还；池满时 surplus descriptor 自动关闭，避免共享 `struct file`、per-response synchronous open jitter 和无界 idle FD retention。direct sendfile 在注册 writable readiness 前先尝试推进，`EAGAIN` 后保留 partial progress。超过每数据核 4 条 active transfer 时，natural `EAGAIN`/readiness boundary 替代 chunk-edge cooperative yield，避免 dense wave 多一次 scheduler round。全局 active sendfile transfer 超过数据核一半时，balanced 才在当前 queue 较小时临时请求 1 MiB `SO_SNDBUF`，结束或取消时恢复原数值；equal-load/idle 保留 Linux autotuning。per-connection request timing 与 HTTP connection count 不能作为压力信号，因为 pooled client 可让多个 worker 共用更少的 keep-alive connection。2 MiB 与 8 MiB 深队列实验已造成 low-scale static-large 或混合尾延迟回归，不恢复。曾失败的全核 always-on balanced reactor 在 4× 造成 QCP 错误和跨协议尾延迟，因此 active-transfer pressure band 是必须约束。`ceil(cpuset/4)` nice 0 的 stream 实验在 1×/4× 将 TCP/WebSocket 压到 0.85-0.98x nginx，并恶化 p95/p99，已拒绝。统一 balanced TLS 的实测会产生 16-21 ms 跨协议尾峰并把 HTTPS saturation 压到最低 0.225x，因此不得重新合并。global injection queue 每 31 次 poll 检查。plain static 采用低密度 256 / 高密度 128 响应 fairness boundary；raw reverse/SSE 依靠有界 upstream/downstream readiness。balanced 专用 UDP 依靠 Tokio socket scheduling；shared-small UDP 保留 4 包边界。静态文件使用 2 秒 freshness window，metadata/body 重验证进入单独 nice-10 control runtime。freshness timestamp 与 in-flight claim 为共享原子状态，stale reader 不获取 DashMap 写锁；metadata 未变化时只刷新共享 freshness，不重建 H2 snapshot。small-profile WebSocket 使用 readiness-bounded 64-poll normal relay turn。HTTP 请求统计使用每 data worker 独占的 cache-line padded counter，采集时汇总；配置 hot reload 通过 ArcSwap 原子发布。
- 上述 32 次 nonblocking poll 是 `bc47c75` 已拒绝实验，不是当前实现。当前 1-16/17-32 对 owner 保留 1/2 次 immediate probe；33-128 对改用一次 50 µs、可被任意 socket event 中断的 `epoll_pwait2`，跨过 backend scheduling gap 时不忙占 HTTP CPU。sendfile reactor 提前清除 `TCP_CORK` 后会把状态传回 HTTP owner，避免 owner 重复 syscall。
- TCP listener 通过显式 socket 设置 `SO_REUSEADDR`、大 backlog 和 `TCP_NODELAY`。无 script/affinity/health/多 upstream 的单上游连接直接进入 fast path，不经过通用 upstream planner。实时 profile 保留 `QUICKACK/NODELAY/USER_TIMEOUT`，收发队列交给 Linux autotuning；深队列与 `TCP_NOTSENT_LOWAT` 只用于 HTTP/gateway profile。
- `runtime.performance` is default-on. Linux hosts use portable socket tuning; Ubuntu 24.x additionally enables the extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`). Older Ubuntu/Debian/unknown distros keep the portable path and log the downgrade reason at startup.
- Hot-path shared state avoids one global lock: rate limits, cache zones, sticky affinity, and upstream runtime state use sharded maps; raw HTTP upstream keepalive uses a lock-free bounded queue，checkout 会先用 nonblocking read 丢弃已 EOF、报错或残留脏数据的 idle socket，预热也会同时持有目标数量后再归还，避免长启动屏障后的首个 SSE/reverse 请求命中 stale fd。默认 small 的 raw reverse/SSE upstream socket 使用 Linux autotuning 与 low-latency socket 选项，不强制 8 MiB bulk queue；the native single-upstream fast path skips upstream runtime lookups entirely when active and passive health are disabled. Mutex/RwLock usage is reserved for control-plane reload/certificate state and one-time static-cache fill coordination, not ordinary HTTP/TCP/UDP forwarding.
- `forward_headers: false` on native HTTP routes disables automatic `X-Forwarded-*` / `Forwarded` insertion for nginx-parity and high-throughput deployments that do not need that metadata.
- `services.ai_proxy.routes[*].emit_metadata_headers: false` skips `proxysss-ai-*` upstream metadata headers for nginx-parity SSE paths while preserving native path rewrite and provider routing.
- Native HTTP route resolution borrows global and per-route compression/cache/rate-limit policy on the hot path; owned policy copies are only made for work that must outlive the request task.
- Linux GNU builds use jemalloc as the global allocator to reduce header, routing, and cache bookkeeping overhead under highly concurrent edge workloads.
- Direct TCP listeners, KCP UDP listeners, and QCP UDP listeners keep payloads transparent; protocol labels are observability hints, not hot-path parsers. A policy-free UDP worker keeps a local upstream/config snapshot and refreshes it once per second after reload, so ordinary datagrams do not acquire the dynamic configuration lock. QCP support is therefore an independent edge-forwarding listener for neko233-com/QCP services, not QCP frame termination inside proxysss.
- TCP stream 有独立 relay profile。默认 `small` 的 realtime/game/TCP 与 plain WebSocket 留在统一多核 Tokio data runtime，每条连接使用单 future 双向 relay、显式 half-close 与有界 `ByteBufferPool`；`balanced` 在建连/升级后把 policy-free TCP 与 plain WebSocket 交给 sparse native epoll relay，fd-indexed state、bounded pending buffer pool 和显式 backpressure 避免长期 Tokio relay task 挤占 HTTP。bulk/file/backup profile 使用 `splice(socket -> pipe -> socket)`。
- 原生 realtime relay 对一次 level-triggered readiness 最多连续 drain 8 个 16 KiB read batch，合并 WebSocket 小帧、游戏帧和普通 TCP 已排队数据的 epoll/hash dispatch；达到 batch 上限就交还 event loop，partial write 立即启用显式 pending/backpressure。owner 只有在 `epoll_wait` 确实阻塞后收到客户端请求时才追加 reply probe：不超过 16/32 对使用 1/2 次 immediate poll；33-128 对使用一次 50 µs、可被任意 socket event 中断的 `epoll_pwait2`。上游回包事件不再安排下一轮等待，持续 ready 的 saturation 也不启用该路径。普通数据事件只在本端已观察到 EOF 后才执行完整 pair-finished 查找，避免每帧额外查询两端状态。
- plain WebSocket 在 HTTP shard 完成握手后，Linux 性能模式默认 handoff 到 CPU 自适应分片的原生 `epoll` relay，失败时回退到多核 Tokio relay；WSS 使用有界 Tokio relay 加 rustls/AWS-LC。空闲长连接不会预占每方向固定大 buffer。4c 的生产参考包络默认验证 20k idle WSS，活跃消息规模验证到 4096，并分别报告握手与消息 p50/p95/p99、内存 current/peak 与每连接成本；它不是固定 RAM 准入门槛。
- HTTP/2 在 ALPN 已确认后直接使用 Hyper H2 server builder；默认 `small` 的 TLS handshake、H2 与 plain HTTP 共用相同 per-core data shard，`balanced` 则进入独立低权重 TLS runtime。warm-up 从 bounded static cache 生成 immutable `Bytes` 响应表，并通过 ArcSwap 发布只读 FxHashMap；同路径 H2 请求不获取 DashMap shard。每条 H2 connection 使用一个本地 `FuturesUnordered` executor 驱动 stream future，避免每请求一次 Tokio spawn/global-queue hop，并在每个完成 stream 后 cooperative yield；32-stream dedicated batch 会在共享 cgroup 内抢占 CFS，已因 4× CDN/static/reverse 回归而移除。文件过期后继续 stale-while-revalidate，刷新工作在 nice-10 control runtime 完成后原子替换有界快照。
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. Plain/TLS fast-lane request readers receive socket bytes directly into persistent per-connection `BytesMut` spare capacity。warm-up 为小型静态对象预构建完整 HTTP/1 keep-alive 响应 bytes，首条连接无需重复格式化 header 和拷贝 body；同一 keep-alive connection 后续 exact static GET head 命中 connection response cache，一秒 revalidation clock 每 256 次 exact hit 检查一次。反向代理/AI 上游池每个去重 pool 只预拨一对 socket，避免从启动 runtime 大批创建、再跨 reactor 长期使用；数字 IP 在 pool 创建时预解析为 `SocketAddr`，冷连接不重复走字符串地址解析。低密度 tiny cached static/TLS connection 每 256 个响应显式 cooperative yield；高密度 plain HTTP 使用每 128 响应的 saturated fairness boundary。WebSocket relay 使用 64-poll normal budget。HTTP/2 使用固定大窗口与 warm-up immutable snapshot。raw reverse 每次请求本身都会经过 upstream/downstream readiness，因此不叠加周期性 yield；它复用 parsed request、serialized upstream request、per-downstream upstream lane、response buffer 与 4 KiB framing cache。只有尚未收全的 body 才进入 bounded `ByteBufferPool` relay。Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough。Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path。
- `scripts/benchmark-ubuntu24-amd64-docker.sh` 是禁止 GitHub Actions 性能压测后的本机/原生 Docker 入口：它硬校验 controller 与被测镜像为 Ubuntu 24.04 x86_64；Windows Docker Desktop 可从 Git Bash 使用本地 `npipe://`，Linux/macOS 使用 Unix socket。wrapper 会把 `TRAFFIC_PROFILE`、吞吐/延迟运行顺序、隔离 subnet 与角色 IP 原样传入并写入 fingerprint，便于复现默认 `small`、官方 `balanced` 及双向顺序检查。脚本在容器内构建当前 checkout，再把 gateway、backend、load client 分配到互不重叠的 cpuset/cgroup。每尺度只启动 1 个 backend 和两边 gateway；非被测 gateway 在共享 cpuset 上 pause。每个 wave 只启动 1 个 client 容器，内部保留 11 个独立协议进程，避免容器生命周期淹没 1 秒样本。这样 proxysss 更快的 closed-loop TCP/UDP 不会让 client/backend 多消耗 CPU、反向饿死同轮 HTTP。HTTP、HTTPS、static-large、SSE、WebSocket、TCP、UDP 与透明 QCP 按 1×/2×/4× 一起放大；每档同时判 mixed saturation 吞吐和 equal-offered-load p50/p95/p99，要求零错误、逐场景及聚合严格胜出，原始证据保存在 `.benchmark/direct-ubuntu24-amd64/`。arm64 Docker daemon 使用 `linux/amd64` 模拟时会记录 `execution_mode=emulated-amd64`；两边承受相同模拟成本，但该结果不能冒充物理 x86 证据。arm64 daemon 的 release 构建必须使用 Zig + cargo-zigbuild 在宿主原生速度交叉编译，并在 Ubuntu 24 amd64 容器执行同一 ELF 后才开始测量，不能回退到 QEMU 内编译。
- `scripts/benchmark-all-scenarios-isolated.sh` 默认每尺度运行 1 次 saturation、2 次反向 candidate order 的 equal-load：完整 1×/2×/4× active measurement 为 18 秒并硬限制在 20 秒内，latency report 使用两次样本的 median。inactive gateway 使用 Docker pause；每次 unpause 后双方都先经过默认 1000 ms、排除在 active measurement 外的 settle window，再安排 future start。plain HTTP/1 pool、HTTPS/H2 session、两次 GET/SSE 请求及每连接一次 WebSocket/TCP/UDP/QCP echo 都在 active window 前完成。双方使用相同端口、cpuset、nofile、somaxconn、共享 sysctl、协议面与场景 CPU 分区。equal-load 取较慢 saturation 的 25%；同步 UDP/QCP target 保留 1 ms completion guard，TCP 保留 2 ms，WebSocket 为 framing 与两次用户态协议转换保留 5 ms，要求至少完成 98% 可执行 target 且 proxysss 每个 percentile 严格更低。
- 20 秒上限按 active measurement window 累加；每个 wave 启动前必须预留完整采样时长，并由 GNU `timeout` 另设 4 秒独立 process grace，卡死会停止容器内发生器并失败。默认 future-start lead 为 2000 ms，确保 4× 下 11 个独立发生器完成连接池与协议预热后再进入共享时间戳；plain HTTP/1 双方并发建立与测量 concurrency 相同的连接池，HTTPS/H2 继续预连单个 multiplexed session。预连与 lead 不计入 active measurement，UDP/QCP response timeout 为 500 ms，1 秒测量窗口不缩短。
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
