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
3. Serve `/metrics`, `/.well-known/acme-challenge/*`, built-in `/`, `/docs`, `/healthz`.
4. Enforce `services.access_control` and `services.rate_limit`.
5. Match static site, WebDAV, domain route, reverse-proxy route, or script hook.
6. Apply cache lookup, upstream selection (LB algorithm + health), retries, and passive quarantine.
7. Proxy request/response (including WebSocket upgrade, generic SSE streaming, New API-compatible routes, and gRPC-over-h2), or serve static files with bounded memory cache, mmap-backed hot objects on supported builds, Range/206 resumable downloads, and a Linux plain-HTTP fast lane whose cache/sendfile behavior follows `runtime.performance.traffic_profile`.
8. Optionally compress response and write access log entry. Successful requests on manual-reload deployments skip the extra post-dispatch config lock used only for error-page decoration or live logging changes.

## Stream path (TCP, UDP, KCP, and QCP datagrams)

1. 在配置的 `tcp.listeners[]` / `udp.listeners[]` bind 接收连接或数据报。Linux 且 `runtime.performance.enabled=true` 时，listener 按 `sched_getaffinity` 检测到的可用 CPU 核建立 `SO_REUSEPORT` accept worker；游戏小帧、通用 TCP 与 plain WebSocket 交给按 cpuset 线性扩展的原生 `epoll` relay reactor：实时优先的 `small` profile 每 2 个可用 CPU 一个 owner，混合/大流量的 `balanced`、`bulk` 每 4 个可用 CPU 一个 owner，为 HTTP/TLS/UDP 保留更多 scheduler 预算。每条连接只由一个 reactor owner 维护 backpressure 与 half-close，避免每帧跨 Tokio 调度；`EPOLLHUP`/`EPOLLRDHUP` 与 `EPOLLIN` 同时出现时先 drain 尾部数据再传播 half-close，不能丢弃游戏/WebSocket 最后一帧。每 worker 不超过 4 对低密度连接时只做极短 reply spin，中高密度完全依赖 `epoll` batch；不可 handoff 或非 Linux 时回退到有界 Tokio relay，bulk 流保留 `splice` 路径。HTTP accept/runtime 仍保留完整 per-core fanout，不因 stream reactor 扣减 worker。UDP 同样按 CPU 自适应 fanout，并共享 association 表。
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
- Linux `runtime.performance.enabled=true` 下，plain HTTP、TLS HTTP、TCP stream、UDP listener 都由 `SO_REUSEPORT` 多 worker 承载；worker 数以 `sched_getaffinity` 的 cpuset 为准，不设固定小上限。plain `ws://` 与小帧 TCP 进入原生 `epoll` relay：默认实时优先 `small` profile 每 2 个可用 CPU 一个 owner，`balanced`/`bulk` 每 4 个 CPU 一个 owner，避免单 reactor 和每帧 async 调度瓶颈，并按 traffic profile 给 HTTP/TLS/UDP 保留混合负载调度空间；WSS 因 rustls 语义保留多核 Tokio 有界 relay。正式 4c 包络以 4096 active WSS 和 20k idle WSS 为默认可验证目标，内存以 current/peak/每连接成本报告，只有实际生产预算才设上限。TLS 低密度连接复用 HTTP shard；达到每 base shard 64 条活跃 TLS 连接时，额外 TLS shard 才弹性启动。
- TCP listener 通过显式 socket 设置 `SO_REUSEADDR`、大 backlog 和 `TCP_NODELAY`。无 script/affinity/health/多 upstream 的单上游连接直接进入 fast path，不经过通用 upstream planner。实时 profile 保留 `QUICKACK/NODELAY/USER_TIMEOUT`，收发队列交给 Linux autotuning；深队列与 `TCP_NOTSENT_LOWAT` 只用于 HTTP/gateway profile。
- `runtime.performance` is default-on. Linux hosts use portable socket tuning; Ubuntu 24.x additionally enables the extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`). Older Ubuntu/Debian/unknown distros keep the portable path and log the downgrade reason at startup.
- Hot-path shared state avoids one global lock: rate limits, cache zones, sticky affinity, and upstream runtime state use sharded maps; raw HTTP upstream keepalive uses a lock-free bounded queue; and the native single-upstream fast path skips upstream runtime lookups entirely when active and passive health are disabled. Mutex/RwLock usage is reserved for control-plane reload/certificate state and one-time static-cache fill coordination, not ordinary HTTP/TCP/UDP forwarding.
- `forward_headers: false` on native HTTP routes disables automatic `X-Forwarded-*` / `Forwarded` insertion for nginx-parity and high-throughput deployments that do not need that metadata.
- `services.ai_proxy.routes[*].emit_metadata_headers: false` skips `proxysss-ai-*` upstream metadata headers for nginx-parity SSE paths while preserving native path rewrite and provider routing.
- Native HTTP route resolution borrows global and per-route compression/cache/rate-limit policy on the hot path; owned policy copies are only made for work that must outlive the request task.
- Linux GNU builds use jemalloc as the global allocator to reduce header, routing, and cache bookkeeping overhead under highly concurrent edge workloads.
- Direct TCP listeners, KCP UDP listeners, and QCP UDP listeners keep payloads transparent; protocol labels are observability hints, not hot-path parsers. A policy-free UDP worker keeps a local upstream/config snapshot and refreshes it once per second after reload, so ordinary datagrams do not acquire the dynamic configuration lock. QCP support is therefore an independent edge-forwarding listener for neko233-com/QCP services, not QCP frame termination inside proxysss.
- TCP stream 有独立 relay profile。Linux 性能模式下，游戏小帧与通用 1 KiB realtime TCP 默认使用 CPU 自适应分片的原生 `epoll` relay，每条连接由单一 reactor owner 维护显式 backpressure 和 half-close；handoff 不可用或非 Linux 时使用多核 Tokio 双向 relay 与有界 `ByteBufferPool`，bulk/file/backup profile 才使用 `splice(socket -> pipe -> socket)`。
- plain WebSocket 在 HTTP shard 完成握手后，Linux 性能模式默认 handoff 到 CPU 自适应分片的原生 `epoll` relay，失败时回退到多核 Tokio relay；WSS 使用有界 Tokio relay 加 rustls/AWS-LC。空闲长连接不会预占每方向固定大 buffer。4c 的生产参考包络默认验证 20k idle WSS，活跃消息规模验证到 4096，并分别报告握手与消息 p50/p95/p99、内存 current/peak 与每连接成本；它不是固定 RAM 准入门槛。
- HTTP/2 在 ALPN 已确认后直接使用 Hyper H2 server builder，不再走 HTTP/1 自动探测；TLS accept、handshake 与 H2 connection 全程留在同一个 shard，避免跨 runtime 搬运和 wake。`SO_REUSEPORT` 把不同连接分散到 CPU 自适应 shard。小静态文件进入有界内存缓存，H2 热对象以 immutable `Bytes` 直接应答，并使用 stale-while-revalidate。Range 精确返回 `206`/`416`；大文件 Linux fast lane 使用缓存 fd + `sendfile`。`balanced`/`bulk` 在 CPU-local HTTP shard 上用最多 2 MiB 的初始非阻塞 drain 填满已 cork response 的 socket window（与 nginx 现代 `sendfile_max_chunk` 默认值对齐），只有真正产生 backpressure 的剩余 body 才把 duplicated socket/file fd 交给每核一个、随 cpuset 线性扩展的有界原生 epoll sendfile reactor。reactor 每事件最多 drain 16 MiB；`small` profile 上限始终不超过 2 MiB。`sendfile(2)` 使用每响应独立的显式 offset，duplicated file descriptor 只负责跨线程生命周期。worker 绑定自己的 CPU group，handoff 按当前 HTTP shard 所在 CPU 稳定映射。HTTP shard 保留连接与 keep-alive 所有权并等待 completion，因此 bulk writable drain 不再占住通用 HTTP scheduler；queue 满或 handoff 失败时从已发送 offset 继续走 Tokio readiness。默认 `small` 仍做 125µs 公平 pacing；native reactor completion 后立即读取下一条 keep-alive 请求，不再重复 cooperative yield，普通 fast lane 仍每 32 个响应公平让出。要求同一轮把 small/realtime 与 bulk 都设为严格 gate 时，可显式使用 `TRAFFIC_PROFILE=balanced`，但必须重新通过全部混合场景，不能只报大文件单项。
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. 同一 keep-alive connection 重复的 exact static GET head 在首次校验后直接命中 connection response cache，直到 revalidation deadline 前不再重复 UTF-8/header parse 与 route lookup；exact raw reverse request 同样复用已验证的 parsed request，避免每次重建 target/path/forwarding header。Tiny cached static and raw reverse connections amortize explicit cooperative yield over 32 completed requests；socket read/write 未就绪时仍会自然让出 runtime。Raw reverse keeps a per-downstream upstream lane, rewrites prefixes without reparsing `Uri`, filters hop-by-hop headers as bytes, omits redundant `Content-Length: 0`, and coalesces response head plus small fixed bodies into one downstream write. Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough to minimize first-token latency. Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path for simple `ws://` routes.
- `scripts/benchmark-all-scenarios.sh` 支持 `BENCHMARK_REPETITIONS` 与交替 `RUN_ORDER`；多轮 mixed saturation 对吞吐和 percentile 取中位数，对错误取最大值，避免单次共享 runner 抖动决定结论。Ubuntu 24 x86_64 workflow 的 `load_scale` 会同时放大 HTTP、HTTPS、static-large、SSE、WebSocket、TCP 与 UDP 并发，用于验证规模增长，不能只放大优势路径；static-large 严格基线从每核 2 条流开始，双方完全相同，避免每核仅 1 条时 `SO_REUSEPORT` hash placement 决定稀疏样本。`scripts/benchmark-all-scenarios-isolated.sh` 是 4c 单网关的严格 Linux 对照入口：gateway、backend、client 使用互不重叠的 cpuset/cgroup 和独立容器网络命名空间，默认 4+4+8 CPU 分配先做 16-core 预检；nginx 1.31.2 mainline 与 proxysss 接受同一 workload。mixed saturation、equal-offered-load 各默认交错运行 4 次，serial isolated saturation 运行 3 次并按指标取中位数，任一轮错误仍保留最大值；equal load 使用较慢方饱和吞吐的固定比例严格判 p50/p95/p99 与完成率。`MIXED_SCENARIOS` 可做不改变方法学的单项诊断。`benchmark-websocket-production-gate.sh` 另测多尺度 WSS active 与 20k idle hold；`benchmark-cross-host-wss.sh` 从独立 client host 把同 SHA binary 布置到 gateway/backend，并以远端 systemd cgroup 强制 4 CPU、300k nofile，保留 cgroup memory current/peak 与每连接成本、主机/`nginx -V` 指纹及原始样本，再严格复跑 WSS 吞吐、p50/p95/p99 和容量。`MemoryMax` 只在声明了生产内存预算时显式设置，不以任意固定 RAM 阈值拒绝证据。Docker role isolation 不能冒充三台物理机。
- 默认 nginx 对照矩阵不包含 KCP/QCP 协议专用封装；`EXTENDED_REALTIME=1` 只把 `protocol: qcp` 的透明 UDP 转发加入同一 mixed wave，与 nginx 等价 UDP stream 比较并接受同一严格门禁。这个结果证明 edge forwarding，不代表 proxysss 在热路径解析或终止 QCP frame。
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
