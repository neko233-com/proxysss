# Linux 性能决策与已拒绝实验

本页保存从原根 AGENTS.md 移出的性能约束与历史测量说明，供修改调度、sendfile 和实时 relay 前查阅。历史比值没有本次提交的原始工件，不构成当前性能结论；当前机制以 [架构](../ARCHITECTURE.md) 和源码为准，新的优化仍须通过 [完整混合验证](../VALIDATION.md)。

## 不可重复的已知回归

- 不把 balanced 的 H2 放回共享实时运行时，不恢复 32-stream H2 批处理；这些实验曾压低其他协议吞吐、增加尾延迟。
- 不恢复 ceil(cores/4) nice 0 的实时池、nice 5 realtime、密集短轮询或每 socket SO_INCOMING_CPU 强绑定。
- 不无条件启用 balanced 全核 sendfile reactor，不给长连接永久加深 2/8 MiB SO_SNDBUF；根据活跃传输数选择发送路径，完成/取消恢复临时队列。
- 不把启动 runtime 批量建立的上游 socket 当作数据分片长期连接；池预热有界，数字 IP 只解析一次。
- 不以 HTTP 连接数或请求耗时推断大文件压力；以活跃传输数判断。不能以省内存为由接受跨场景性能退化。

## 迁移保留的原始约束

以下原文保留具体参数与历史取舍便于追溯；第二段已经取代第一段中的旧 realtime owner 数和密集 probe。维护当前文档时不得复制旧方案再追加“当前修订”。

- The default `small` traffic profile keeps HTTP/1.1, HTTP/2/TLS, WebSocket, TCP, UDP, and transparent QCP on one CPU-sized Tokio data runtime with one worker per allowed core and work stealing across those per-core shards. `balanced` maps every plain HTTP `SO_REUSEPORT` accept loop to its own pinned one-thread Tokio runtime, gives TLS/H2 one CPU-adaptive bounded runtime at nice 5, and gives both UDP/QCP and post-connect policy-free realtime TCP/plain WebSocket CPU-adaptive pools sized at `ceil(cores/2)` with nice 3. Listener fanout leaves reuseport/RFS placement to Linux; forcing `SO_INCOMING_CPU` per socket was rejected after it worsened mixed HTTP tails and 2x static-large. The per-core plain runtimes use an 8-poll I/O event interval; the shared `small` runtime retains 16 polls. Established TCP/WebSocket pairs use the native epoll relay; after a genuinely blocked client-request tick, an owner performs one poll through 16 pairs, two through 32 pairs, or 32 through 128 pairs. The high-density budget spans the short backend scheduling gap seen at scale 4; sparse 1x/2x traffic retains its smaller syscall budget. Upstream reply events never schedule another spin, and continuously-ready saturation never enables this path. UDP keeps Tokio socket ownership. The rejected `ceil(cores/4)` stream pool at nice 0 fell to 0.85-0.98x nginx and worsened realtime p95/p99, so do not restore that shape. Raising balanced realtime nice from 3 to 5 reduced realtime throughput without improving static-large, so do not repeat that trade. Putting continuously runnable H2 back onto the shared balanced runtime caused 16-21 ms cross-protocol tails and up to a 0.225x HTTPS saturation ratio. Plain static keeps an explicit 256-response low-density / 128-response high-density fairness boundary; raw reverse/SSE rely on their bounded upstream/downstream readiness points. Plain fast-lane HTTP and raw plain-upstream response heads use bounded `try_read_buf`/`try_write` calls before readiness registration, removing generic async-trait/cooperative bookkeeping without disabling Tokio fairness around an entire exchange. Balanced static objects at or above 8 MiB use direct sendfile at sparse/equal load and high pressure, or a prewarmed nice-0 native sendfile reactor only while current active large transfers are above the data-plane core count and at most four per core. The first sparse transfers yield once so sibling owners register before pressure classification. Idle/equal-load remains direct, 1x/2x saturation can use the reactor, and dense 4x mixed traffic stays on HTTP owners. Reactor handoff prefers the current pinned HTTP CPU, falls back to round-robin only when no matching owner exists, and attempts sendfile immediately before registering EPOLLOUT. Active job lookup uses a fd-indexed slot table rather than a hash map. Adaptive balanced jobs use the full response as their event budget; real `EAGAIN` remains the fairness/backpressure boundary instead of an artificial 16 MiB epoll round. Reactor completion clears `TCP_CORK` before waking the HTTP owner, so final bytes are flushed without waiting for another Tokio scheduling round. Warm-up validates the static target, starts eligible reactor owners outside live traffic, and seeds a bounded lock-free descriptor pool per hot large file; concurrent responses check out independent file descriptions and return them after sendfile, while surplus descriptors close automatically when the pool is full. This avoids cross-connection `struct file` contention, per-response synchronous open jitter, and unbounded idle file-descriptor retention. Direct sendfile attempts progress before registering writable readiness, and partial progress survives `EAGAIN`. At more than four active transfers per data-plane core, natural `EAGAIN`/readiness boundaries replace the otherwise explicit chunk-edge cooperative yield, avoiding a redundant dense-wave scheduler round while preserving backpressure. Balanced sendfile temporarily requests a bounded 1 MiB queue only when global active transfers exceed half the detected data-plane cores, then restores the prior numeric value on completion or cancellation; equal-load/idle traffic retains Linux autotuning. Per-connection request timing and HTTP connection counts are not valid pressure signals because pooled clients may dispatch many workers over fewer keep-alive connections. Do not add persistent per-large-connection deep queues: measured 2 MiB and 8 MiB `SO_SNDBUF` experiments reduced low-scale static-large or worsened mixed WebSocket/static tails. A full-core always-on balanced sendfile reactor caused 4x mixed QCP errors and cross-protocol p95/p99 regressions; balanced reactor use must remain active-transfer-bounded, while bulk may use it unconditionally. Balanced dedicated UDP relies on Tokio socket scheduling; shared-small UDP retains its 4-packet fairness boundary. Static file revalidation uses a 2-second freshness window and one nice-10 control runtime. Freshness timestamp and in-flight claim are shared atomics, so stale readers never take a DashMap write lock; unchanged metadata refreshes shared freshness without rebuilding the H2 snapshot. Small-profile WebSocket relays use the normal 64-poll readiness-bounded turn. The global injection queue remains at 31 polls. HTTP/2 uses fixed large windows plus a warm-up-built immutable ArcSwap/FxHashMap response snapshot. Each H2 connection drives stream futures through one local `FuturesUnordered` executor instead of one Tokio spawn/global-queue operation per request and yields after every completed ready stream; a dedicated 32-stream batch was rejected because its CFS consumption collapsed 4x CDN/static/reverse throughput even though H2 itself was fast. Do not put same-path H2 requests behind a DashMap shard or populate the snapshot on first live traffic. Per-request metrics use shard-local padded counters. Small-profile sendfile uses 8 MiB chunks only while active transfers do not exceed detected data-plane cores, then automatically returns to the mixed-safe 2 MiB slice.

- Superseding the rejected dense-poll, timed-wait, and 8 MiB event-round experiments described above: balanced realtime uses one soft-affinity native owner per allowed CPU. With the resulting lower pair density, blocked-request owners use one immediate probe at 1-16 pairs, two at 17-32, and none above 32; dense traffic relies on ordinary epoll wakeups instead of stealing HTTP CPU. Adaptive sendfile retains the full-response event budget. At no more than two active transfers per data-plane core it selects an evenly spaced half of the prewarmed owners, reducing all-core contention without adding an epoll round; higher pressure restores every owner. Sendfile reactor completion reports its early `TCP_CORK` clear to the HTTP owner, which must not repeat that syscall.

## 2026-09-20：短读后提前结束 native relay 批次，未接受

候选提交 cff1210 把短读且完整写出后的下一次 recv/EAGAIN 改为返回 epoll。Ubuntu 24 x86_64、6 核 gateway、三角色独立 cpuset 的完整 1×/2×/4× 混合矩阵均通过吞吐和零错误，但等负载延迟全部未过；2× UDP p99 比值 1.707，4× 反代 p99 1.097、大文件 p99 1.038。对照提交 1de0a8c 自身也未通过全部延迟门槛，因此这些单轮数据不能证明精确因果或稳定收益。按无跨场景退化的准入规则，恢复原先 recv 排空行为，仅保留背压/半关闭回归测试。原始本地工件：.benchmark/direct-ubuntu24-amd64/goal-native-short-read 与 goal-safe-static-snapshot；未作为发布证据上传。


## 2026-09-20：raw WSS 连续就绪批次从 64 缩为 8，未接受

以 dbe1382 为基线，仅把 raw WSS TLS 转发单方向的连续 poll 上限缩为 8。Ubuntu 24 amd64 Docker，gateway/backend/client 使用独立 0-3/4-7/8-15 cpuset；4096 条 WSS，两个相反候选顺序、每网关每轮 5 秒。两版都通过 284 项 Linux 测试与 Clippy，压测双方错误为 0，但候选吞吐中位数 89993.5 ops/s，低于基线 95140.8；p95 从 264.914 增至 289.979 ms，p99 从 306.605 增至 331.665 ms。nginx p99 为 83.626/83.598 ms，基本稳定。proxysss 活跃 current/peak 约 187/190 MB，两版近似，不能以更小批次为名接受吞吐/延迟退化。

候选已完整撤回，未进入发布或混合矩阵；这组单项失败足以拒绝候选，不证明根因已定位。基线与候选都有很低的 p50 和较高的 p95/p99，下一步应检查高并发调度与压测端分布，不重复降低同一转发预算。工件：.benchmark/goal-wss/sat-dbe1382-summary.{json,md}、sat-poll8-summary.{json,md}、runs/isolated-websocket/sat-{dbe1382,poll8}-r{1,2}；补丁和二进制 SHA 见 wss-poll8-provenance.json。

## 2026-09-20：移除静态响应的临时 SO_SNDBUF，保留自动调优

早期 balanced sendfile 曾在活跃传输超过半数核心时请求 1 MiB SO_SNDBUF，响应完成/取消再写回旧数字。该做法不能恢复 Linux 自动调优：设置 SO_SNDBUF 会保留用户锁，wmem_max 还可能令请求的增大被压低。这一发现取代本页历史原文中的 scoped 1 MiB 队列建议；当前静态路径不再修改 SO_SNDBUF，背压仍由 sendfile 的 EAGAIN/readiness、有限并行 owner 和有界缓存/描述符池提供。依据：[Linux tcp_wmem](https://kernel.org/doc/html/latest/networking/ip-sysctl.html)。

Ubuntu 24 的 MSS=1460 独立 socket 复现中，初始87040 bytes，自动调优可增长到4194304；旧增大/恢复序列后的缓冲始终87040。真实网关复验固定1 CPU、两条 MSS=1460 keep-alive 连接并发两轮64 MiB下载，使用 NETLINK_INET_DIAG 读取服务端缓冲且验证完整正文。旧网关 dbe1382 的一条连接始终87040，另一条由522240变为425984；修复版两条都从87040自然增长到478720并在复用时保留。此例证明跨请求状态修复，单项耗时不作为性能收益或发布证据。

本地复现工件：.tmp/sndbuf-autotune-mtu1500-probe.go/.log、.tmp/sendfile-keepalive-probe.go、.tmp/goal-sendfile-autotune-{baseline,fixed}-probe.log。原始程序与所有数据应随最终证据归档；完整混合回归仍是保留修复并发布的必要检查。

## 2026-09-20：balanced 临时 TCP_NOTSENT_LOWAT 256 KiB，未接受

候选 1934bc4 在活跃传输超过半数数据核时临时限制未发送队列，完成/取消恢复原值（含 0 的命名空间继承语义），不改变 SO_SNDBUF。285 项 Linux 测试、Clippy 与两条 keep-alive 完整下载验证通过，但不能证明跨场景性能收益。

Ubuntu 24 amd64 Docker、6 核 gateway、角色隔离的 1×/2×/4× 全混合矩阵共 18 秒有效测量、125 秒矩阵 wall time。各场景零错误，CDN 吞吐比为 0.994/0.946/0.963；4× 游戏 TCP p50、普通 TCP p95、WebSocket p99 的延迟比为 1.320/1.155/1.176（大于 1 更慢）。基线 1d776d0 也未全过，因此不能用单轮差异推断精确因果；候选没有获得准入证据，已撤回，保留移除 SO_SNDBUF 用户锁的正确性修复。工件：.benchmark/direct-ubuntu24-amd64/goal-sendfile-lowat256 与 goal-sendfile-autotune；这两次失败均不能作为发布成功证据。

## 2026-09-20：每核 realtime owner 提升到 nice 0，未接受

候选 52fcfa8 仅将 balanced native TCP/WebSocket owner 从 nice 3 改为 0，保留每允许核心一个 owner、现有 epoll 批次和探测边界；这是全核 owner 实验，与历史被拒绝的四分之一核心 nice 0 池不同。287 项 Linux 测试、Clippy 与构建通过，但严格全混合矩阵未过。4× CDN/HTTPS/大文件吞吐比为 0.963/0.955/0.962，TCP 等负载完成率约 96%，低于 98% 门槛，WebSocket p99 为 nginx 的 1.177 倍。

4× cgroup current/peak 为 proxysss 86335488/283049984、nginx 200294400/383815680 bytes；内存低于 2 倍不抵消性能失败。基线 b6b486a 的矩阵也未全过（4× nginx 透明 QCP 有 3 次错误），不能从单轮差异推断准确因果。候选已撤回为 nice 3，下一步应先区分线程调度等待和数据转发成本，不重复无证据地调整优先级。候选工件 .benchmark/direct-ubuntu24-amd64/goal-realtime-nice0（18 秒活跃、122 秒矩阵 wall），基线 goal-sendfile-cancel；均不是发布通过证据。

## 2026-09-20：只读线程调度诊断，不修改默认参数

运行时基线 e3e0d60（Linux 二进制 SHA256 `8317728fc3b972c6fa34e2426e720acc87c7d707ebfddfd43913a60999b233a1`），Ubuntu 24 amd64 Docker Desktop VM。网关/后端/客户端固定 0–3/4–9/10–21 CPU，两个只读 Go 观察器使用独立 22–23 CPU 与 cgroup，加入各网关 PID namespace；nginx 采 master 和全部 worker。每 50 ms 读取 `/proc/PID/task/TID/schedstat`，未修改宿主机 sysctl。先以单 CPU 双忙线程验证运行和等待计数实际增长，不能因为 `kernel.sched_schedstats=0` 就把可读计数当作无效或零等待。

全部 11 个场景按 1×/2×/4× 混合运行，18 秒有效测量。按记录的绝对开始时间取前一采样点，实际计数窗口约 986–1037 ms，保留两端偏移。4× 等负载两轮 native realtime owner 累计运行 107/113 ms、运行队列等待 33/31 ms；nginx 全部 worker 累计等待约 5.5/2.3 ms。4× 饱和 native realtime owner 的累计等待约 2943 ms，多个线程同时等待，绝不能把这个和请求 p99 等同。该结果支持继续检查线程调度竞争，但不能确定每个长尾请求的根因，也不能据此恢复已拒绝的 nice 0 或密集自旋。

本轮仍未全过：1× 大文件吞吐比 0.946，2× UDP p99 比 2.121，4× 反代 p99 比 1.437，部分其他百分位亦未过。4× current/peak：proxysss 71139328/207339520、nginx 142864384/275632128 bytes。观察器与不同 CPU 包络使它仅能作诊断，不能替代无观测干扰的正式矩阵或三主机发布证据。原始计数、网关数据、每窗口汇总及偏移保存在 `.benchmark/goal-scheduler-e3/`；采集、时间线与汇总源码为 `.tmp/observe-linux-scheduler.go`、`.tmp/benchmark-scheduler-diagnostic.sh`、`.tmp/report-scheduler-diagnostic.go`，应随最终工件归档。当前数据面参数保持不变。

## 2026-09-20：稀疏 balanced direct sendfile 每 2 MiB 让出，未接受

新发生器固定 7681946 的四角色 CPU 诊断完成 18 波、54 个角色窗口，198 个原始场景样本零错误；4/6/12 核与独立观察器 CPU 22–23 的资源配置只用于定位，不能替代标准 6 核网关门槛。等负载 sendfile reactor 未观察到运行增量，大文件在 HTTP owner 直接发送。源码原有 16 MiB 公平预算与夹具大小相同，中途让出条件无法约束这类完整响应。客户端大文件仍可能占满其单 CPU 分区，不能宣称发生器约束已全部排除。

候选 cb5f4df 将非密集 direct sendfile 改为每 2 MiB 让出，移除按源端口及每三次响应选择让出的状态；超过每核四条活跃传输时保留 16 MiB 与 EAGAIN 边界，reactor 的完整响应事件预算不变。333 项 Linux 测试、格式与 Clippy 通过，但 Ubuntu 24 amd64 Docker 标准 6/6/12 核全混合 1×/2×/4× 验证失败（18 秒活跃、180 秒矩阵 wall）。198 条原始样本零错误；大文件吞吐比为 0.702/0.637/0.722，仍未胜出。静态 HTTP p99 改善，但 2× 游戏 TCP p50/p95 比为 1.191/1.137，WebSocket p95/p99 在 2× 为 1.119/1.018、4× 为 1.132/1.115，不能接受局部改善伴随长连接延迟退步。

4× cgroup current/peak 为 proxysss 84430848/283869184、nginx 208941056/381706240 bytes；内存通过不能抵消性能失败。候选已撤回。基线自身也未全过，两次矩阵的参考吞吐与等负载目标有变化，不能把单轮比值变化当作精确因果证据；下一次应整批检查 HTTP/direct sendfile 与实时 owner 调度关系，不重复单独缩小直接发送切片。诊断工件 `.benchmark/goal-rolecpu-768/`，候选工件 `.benchmark/direct-ubuntu24-amd64/goal-direct-fair-2m/`。
