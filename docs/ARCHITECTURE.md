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
7. Proxy request/response (including WebSocket upgrade, generic SSE streaming, New API-compatible routes, and gRPC-over-h2), or serve static files with bounded memory cache, mmap-backed hot objects on supported builds, and a Linux plain-HTTP fast lane whose cache/sendfile behavior follows `runtime.performance.traffic_profile`.
8. Optionally compress response and write access log entry. Successful requests on manual-reload deployments skip the extra post-dispatch config lock used only for error-page decoration or live logging changes.

## Stream path (TCP, UDP, KCP, and QCP datagrams)

1. Accept a TCP connection or UDP datagram on a configured `tcp.listeners[]` / `udp.listeners[]` bind. On Linux with `runtime.performance.enabled=true`, TCP stream binds fan out across multiple `SO_REUSEPORT` accept workers on the dedicated stream runtime, and UDP binds fan out across CPU-adaptive `SO_REUSEPORT` datagram workers with a shared association table.
2. Enforce stream access control and shared-zone rate limits where configured.
3. Select an upstream from `upstream` / `upstreams` using the active load-balancing and health state, or use the direct single-upstream TCP fast path when scripts, affinity, active health, passive health, and extra upstream candidates are all disabled.
4. TCP disables Nagle by default (`nodelay: true`), applies `connect_timeout_ms`, then selects an independent relay profile: latency-sensitive game/MQTT/tool streams use small fixed relay buffers and parallel one-way pumps, while explicit bulk/file stream protocols may use Linux `splice` zero-copy when runtime performance tuning is enabled.
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

## Performance notes

- Async Tokio runtime with tuned keepalive connection pooling via `reqwest` for HTTPS/fallback upstreams and a Hyper HTTP/1 fast client for ordinary `http://` reverse proxy traffic. Server connections explicitly enable HTTP/1 vectored writes and HTTP/2 adaptive windows for high-throughput static and proxy responses.
- The plain-HTTP raw reverse-proxy and raw SSE fast lanes preserve the default `X-Forwarded-*` / `Forwarded` chain and `proxysss-ai-*` metadata headers, so default `reverse_proxy` and `ai_proxy` routes can use the low-allocation path without disabling observability.
- Plain HTTP, TLS HTTP, TCP stream, and UDP listener binds use multiple workers on Linux when `runtime.performance.enabled=true`, backed by `SO_REUSEPORT`; worker counts are derived from detected CPU cores instead of user configuration or fixed caps, so larger machines automatically get more accept/runtime parallelism. Portable/unknown platforms keep a single accept/datagram loop.
- TCP listeners are opened through explicit sockets with `SO_REUSEADDR`, a large listen backlog, and `TCP_NODELAY` on accepted HTTP/TLS/admin/stream connections where latency matters. TCP stream workers run on the dedicated `proxysss-stream` runtime and keep simple single-upstream sessions out of the generic upstream planner when no policy surface needs it.
- `runtime.performance` is default-on. Linux hosts use portable socket tuning; Ubuntu 24.x additionally enables the extreme socket policy (`TCP_QUICKACK`, `TCP_NOTSENT_LOWAT`, `TCP_USER_TIMEOUT`). Older Ubuntu/Debian/unknown distros keep the portable path and log the downgrade reason at startup.
- Hot-path shared state avoids one global lock: rate limits, cache zones, sticky affinity, and upstream runtime state use sharded maps; raw HTTP upstream keepalive uses a lock-free bounded queue; and the native single-upstream fast path skips upstream runtime lookups entirely when active and passive health are disabled. Mutex/RwLock usage is reserved for control-plane reload/certificate state and one-time static-cache fill coordination, not ordinary HTTP/TCP/UDP forwarding.
- `forward_headers: false` on native HTTP routes disables automatic `X-Forwarded-*` / `Forwarded` insertion for nginx-parity and high-throughput deployments that do not need that metadata.
- `services.ai_proxy.routes[*].emit_metadata_headers: false` skips `proxysss-ai-*` upstream metadata headers for nginx-parity SSE paths while preserving native path rewrite and provider routing.
- Native HTTP route resolution borrows global and per-route compression/cache/rate-limit policy on the hot path; owned policy copies are only made for work that must outlive the request task.
- Linux GNU builds use jemalloc as the global allocator to reduce header, routing, and cache bookkeeping overhead under highly concurrent edge workloads.
- Direct TCP listeners, KCP UDP listeners, and QCP UDP listeners keep payloads transparent; protocol labels are observability hints, not hot-path parsers. QCP support is therefore an independent edge-forwarding listener for neko233-com/QCP services, not QCP frame termination inside proxysss.
- TCP stream proxying has its own execution pool and relay profile selector instead of sharing HTTP/static tuning. With `runtime.performance.enabled=true`, TCP listeners run on a dedicated `proxysss-stream` Tokio runtime so long-lived connections are not starved by HTTP/static/SSE workloads. The default latency profile uses 16KB relay buffers and parallel one-way pumps for request/response long connections; the bulk profile is reserved for file/backup-style streams and can use Linux `splice(socket -> pipe -> socket)` to avoid user-space data copies when it is actually beneficial.
- Static files below the in-memory threshold use a bounded in-process cache with short mtime/size revalidation for hot objects; large cached objects use mmap-backed `Bytes` where available to reduce heap allocation and copy pressure. `runtime.performance.traffic_profile` controls the static fast-lane tradeoff: default `small` favors cached small-file/HTTP2/SSE/TCP/UDP feedback, `bulk` favors Linux sendfile/zero-copy large transfers, and `balanced` prepares both. Config load preloads eligible static index/top-level files into the bounded cache or opens sendfile descriptors according to that profile.
- Plain HTTP reverse-proxy, generic SSE/New API-compatible streaming, and no-policy WebSocket requests enter raw data lanes when the route has no script/plugin/cache/compression/rate-limit/retry/health bookkeeping on the hot path. Raw reverse keeps a per-downstream upstream lane, rewrites prefixes without reparsing `Uri`, filters hop-by-hop headers as bytes, omits redundant `Content-Length: 0`, and coalesces response head plus small fixed bodies into one downstream write. Raw SSE writes byte-level response heads, then relays the upstream body as connection-close byte passthrough to minimize first-token latency. Raw WebSocket forwards the upgrade and tunnels bytes before the general Hyper upgrade path for simple `ws://` routes.
- `scripts/benchmark-all-scenarios.sh` is the manual Linux production performance gate. Default GitHub Actions CI is packaging-only and does not run benchmark/smoke/performance jobs automatically. The script's default mode runs all nginx-comparable generic gateway features at the same time per gateway: CDN/static delivery, HTTPS static, HTTP reverse proxy, generic SSE, WebSocket long connections, game/TCP long connections, generic TCP stream, and UDP stream. nginx runs the full concurrent wave first, then proxysss runs the full concurrent wave. The default critical gate uses a fairness-adjusted `0.97` ratio floor for aggregate mixed load plus WebSocket, game/realtime TCP, and generic UDP because proxysss includes built-in policy and gateway surfaces that nginx usually needs extra modules or configuration to approximate; static/reverse/generic SSE must stay above the soft floor with low errors; HTTPS static and static-large remain diagnostic unless explicitly promoted for a TLS/static or bulk-transfer release. New API provider routes and KCP/QCP special UDP encapsulations are excluded from the current performance benchmark matrix. Serial or single-scenario runs are diagnostic only, and strict `>1.0` gates are opt-in. The v1.3.5 UDP fast path diagnostic result for `udp-stream` is `4.045x` on Docker Ubuntu 24 (`127742.75` vs `31577.33` ops/s, 0 errors).
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
