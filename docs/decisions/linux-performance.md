# Linux 性能决策与已拒绝实验

本页保存从原根 AGENTS.md 移出的性能约束与历史测量说明，供修改调度、sendfile 和实时 relay 前查阅。历史比值没有本次提交的原始工件，不构成当前性能结论；当前机制以 [架构](../ARCHITECTURE.md) 和源码为准，新的优化仍须通过 [完整混合验证](../VALIDATION.md)。

## 2026-09-21：UDP 生命周期正确性修复后的全混合检查，未过性能门槛

提交 f4d10e6 修复 UDP 并发容量预留、退休资源、停止监听和新会话热更新。Ubuntu 24.04 x86_64 两轮各 366 项功能测试、Clippy、文档和 CLI 通过；独立 24 客户端探针把 max_associations=1 时的实际 24 个会话降为 1 个。该正确性修复保留，不把它描述为性能优化成功。

标准 Docker Desktop Linux VM 6/6/12 核隔离矩阵的 1×/2×/4× 大文件吞吐比为 0.704/0.619/0.752；静态小文件 p99 比为 3.078/2.689/1.968，4× 游戏 TCP p50/p95/p99 比为 1.180/1.245/1.216，WebSocket 为 1.044/1.209/1.269。全量 198 个样本零错误，132 个等负载样本最低完成率 99.8061%；完成率通过不抵消下载和延迟失败，也不能把单轮变化归因于 UDP 生命周期修改。

4× cgroup current/peak：proxysss 89518080/278081536，nginx 208834560/361988096 bytes。18 秒活跃、162 秒矩阵 wall、330 秒总耗时。完整原始工件 `.benchmark/direct-ubuntu24-amd64/goal-udp-lifetime/`，功能日志、二进制摘要和独立审计 `.benchmark/udp-lifetime-functional/`，容量复现 `.benchmark/udp-cap-baseline/` 与 `.benchmark/udp-cap-fixed/`。未进行 Windows 验收、push 或发布；Docker 数据不是独立三主机发布证据。

## 2026-09-21：报告完成率改为最差单轮，保留

报告器按原始成功次数、时长和目标计算每轮完成率，汇总保存最低值及轮次；吞吐和延迟继续取中位数。样本缺失、参数或双方目标不一致、缺失最低完成率及重复聚合均拒绝通过。回归覆盖真实 1032/1056 样本落在双方任一轮的情况，并穿过 JSONL、汇总 JSON、Markdown/HTML 和门槛返回值。Go 报告器测试纳入本地 Ubuntu 功能入口。

Ubuntu 24.04 x86_64 功能检查通过：两轮各 354 项 Rust 测试、Clippy/格式/文档/Go、CPU 探针和 CLI。在相同系统用新报告器重算 goal-fair-ready 与 goal-http-metrics 的完整三尺度原始矩阵，共 396 个样本、其中 264 个等负载样本；12 份阶段报告仍全部未通过严格性能门槛。仅检查完成率时，准确拒绝 goal-fair-ready 的 1× nginx QCP（97.73%），以及 goal-http-metrics 的 4× proxysss/nginx QCP（90.91%/95.55%）；其余尺度通过完成率检查。重算不产生新性能证据，未改变发生器计划、数据路径、目标或 98% 门槛。原始 SHA256、报告和重算程序保存在 `.benchmark/per-sample-gate-replay/`，功能工件在 `.benchmark/per-sample-gate-functional/`。

## 2026-09-21：下载与 realtime 按 fd 共用 epoll 就绪集合，未接受

候选 d0f9118 将 balanced 下载与实时 socket 放到同一 level-triggered epoll 集合，按内核返回顺序逐 fd 推进：每个就绪下载最多 1 MiB，实时端点最多 8 次 16 KiB 读取。取消独立下载 FIFO 和“完整实时批次之后仅一个下载轮”的限制；新下载只注册不立即发送，旧事件消费后才安装新 fd。nice 3、CPU 预算、取消及文件版本边界保留，small/bulk 路径不变。这是对 a9f5c56 诊断中工作份额下降的具体替代设计，不是恢复原先的整批配额。

Ubuntu 24.04 x86_64 功能入口两轮各 356 项测试、Clippy/格式/文档/Go、真实 CPU 探针和 CLI 全过。新增实际 epoll 测试确认三个就绪下载各自推进且不超过上限、注册不发送、取消移除监听；共享 owner 同时处理阻塞下载、三个完整 4 MiB 下载、32 次双向 echo、半关闭尾数据和取消后的新下载。

标准 6/6/12 核完整三尺度仍失败：大文件吞吐比 0.588/0.591/0.723，1× 游戏 TCP p99 比 2.171，2× TCP p99 为 1.131，4× 游戏 TCP p50/p95 为 1.179/1.101、WebSocket p50/p95/p99 为 1.067/1.160/1.193。CDN 与小静态部分尾延迟降低不抵消下载与长连接问题；不同轮次等负载目标变化也不支持精确因果归因。4× cgroup current/peak bytes：proxysss 87777280/229318656，nginx 209424384/366350336。

18 份原始文件共 198 个样本零错误；132 个等负载样本中，1× nginx QCP 第一轮为 1032/1056（97.73%），第二轮 1056/1056。独立原始审计发现此失败，原汇总却用中位吞吐计算出 98.86%，没有报告逐轮失败。此证据确认报告实现与“每轮 98%”文字不一致，后续修复必须保留最差单轮完成率、验证样本数和目标一致性，不能降低门槛或仅修改文字。

候选经 3eb60da 完整撤回至 a9a495b，HTTP 统计及先前功能修复保留。18 秒活跃、162 秒矩阵 wall、331 秒总耗时。工件 `.benchmark/direct-ubuntu24-amd64/goal-fair-ready/`，功能/原始审计/manifest 在 `.benchmark/fair-ready-functional/`，固定二进制 SHA256 `dc9970fc690fa09ec4887bd5ea55e733f530b539d2890e95fc8ba5cfc5ce0e8c`。该结果只否定本次共享工作分配；未执行 Windows、push 或发布。

## 2026-09-21：HTTP 统计正确性修复保留，混合门槛未通过

提交 fd275ed 补齐静态缓存/sendfile、原始反代、SSE、WS/WSS 请求计数，统一 4xx/5xx 响应计数并消除异常路径重复累加。Ubuntu 24.04 x86_64 两轮各 354 项测试、Clippy、格式、文档/Go、实际 CPU 范围与 HTTP/TCP/UDP 探针、CLI 检查通过；真实 HTTP/1、TLS/H2/H3 与 WebSocket 测试确认快速路径、回退和升级期间的计数。该修复是监控正确性要求，不能通过漏记请求获得性能。

无记录器标准 6/6/12 核、1×/2×/4× 矩阵仍失败。大文件吞吐比为 0.779/0.637/0.766；CDN p99 比 3.229/2.333/1.852，小静态 p99 为 3.334/2.246/1.913，反代 p99 为 2.177/1.801/1.267。4× WebSocket p50/p95/p99 为 1.142/1.185/1.079。不同轮次波动不能证明该计数修复造成或解决了既有调度问题。

18 份原始文件共 198 个样本零错误，但 132 个等负载样本中有两个未达完成率门槛：4× 透明 QCP 第一轮 proxysss 为 960/1056（90.91%），nginx 为 1009/1056（95.55%），第二轮双方均为 1056/1056。不能只报告零错误或合并后的完成率来掩盖失败。实际间隔 90738 μs，第 11 个同步 tick 位于 998118 μs，距截止仅 1.882 ms；这是末轮定时敏感性的线索，尚未证实客户端定时器或网关是根因，不据此降低门槛。

4× cgroup current/peak bytes：proxysss 89624576/282320896，nginx 202149888/370860032。18 秒活跃、163 秒矩阵 wall、331 秒总耗时。工件 `.benchmark/direct-ubuntu24-amd64/goal-http-metrics/`，功能日志、全量原始审计及 manifest 在 `.benchmark/http-metrics-functional/`；固定二进制 SHA256 `f51ef95c08b9682d87830b73d0c0f75264d8ca771ebd0bf9a8150e66ffb73eef`。Docker Desktop VM 数据不是独立三主机证据，未进入 Windows 验证、push 或发布。

## 2026-09-21：balanced 下载与 realtime 共用 native owner，未接受

后续仅用保留的候选二进制执行 epoll/sendfile 诊断，没有恢复生产代码。原生记录器自检验证正文、偏移、错误码和 epoll 返回值，完整标准三尺度矩阵记录 proxysss 360326 / nginx 176467 个工作区间，无溢出或未完成记录。饱和时 native owner 每轮平均 ready fd 为 13.1/25.2/53.0，每发送轮仍约 1 MiB，工作 CPU 约 1699/1646/1669 ms，其中 sendfile 为 706/486/311 ms，占比从 41.6% 降到 18.6%。HTTP owner 的工作 CPU 同时维持约 3.3 秒；nginx 每轮发送合计字节随密度增长。该证据支持重新分配每连接的处理机会，不证明只调整切片或 nice 可以达标。work wall−CPU 包含程序自身等待与 Tokio park，不能误称纯 runqueue wait，尤其不能等同 UDP 请求时延。仪器化结果不作发布证据，完整边界、源代码、自检与原始工件保存在 `.benchmark/goal-shared-reactor-calls/`。

候选 a9f5c56 将 balanced 大文件交给已预热的 nice 3 realtime epoll owner，取消额外 sendfile 线程池和 HTTP owner 上的直接大段发送。每轮先消费整个实时事件批次，再从去重 FIFO 推进一个下载、最多 1 MiB；下载接收每轮 32 个，取消队列每轮 256 项，旧事件先于新注册处理。就绪节点内嵌在作业状态，完成/取消以 O(1) 摘除，保持文件版本、背压和完成通知语义。该形态不同于此前把 native relay 整批交给 HTTP Tokio runtime 的实验。

Ubuntu 24.04 x86_64 功能入口通过两轮各 351 项测试、Clippy、格式、文档/Go 和 CLI 检查。新增真实 socket 测试覆盖同一 owner 上的下载背压、32 次双向转发、半关闭后的尾数据、取消及后续完整下载；另覆盖下载 FIFO 去重、轮转及取消后 fd 槽位复用。真实 taskset=0,23 网关使用 2 个 HTTP owner 与 2 个共享 owner，没有独立 sendfile owner，11 个线程均未越界，HTTP/TCP/UDP 正文验证通过。功能通过不构成性能准入。

同提交的无记录器全混合 1×/2×/4× 矩阵失败：Ubuntu 24.04 native-amd64 Docker Desktop VM，标准 6/6/12 核。大文件吞吐分别只有 nginx 的 0.475/0.306/0.244 倍；1×/2× SSE p99 比为 1.412/1.907，2× UDP p99 为 2.341，4× TCP p50 为 1.280，WebSocket p50/p95/p99 为 1.105/1.159/1.108。18 份原始文件共 198 个样本零错误，132 个等负载样本最低完成率 0.998022412657。4× cgroup current/peak bytes：proxysss 88199168/228802560，nginx 209932288/373411840。

4× 小静态 p99 比虽降为 0.649，但同轮大文件等负载仅 24 ops/s，前一基线矩阵为 73 ops/s；不能把较轻大文件竞争下的局部尾延迟当作已证明的网关收益。当前证据只否定这一共享资源分配，不证明所有共享 reactor 设计不可行；进一步设计应先量化实时批次与下载获得的处理机会，不直接重复调小切片、提高优先级或恢复被拒绝的独立线程池。

候选已完整撤回，运行时代码恢复到 6b969df；此前 CPU 资源修复和功能覆盖保留。18 秒活跃、163 秒矩阵 wall、334 秒总耗时。原始工件 `.benchmark/direct-ubuntu24-amd64/goal-shared-native/`，功能日志与原始样本审计器 `.benchmark/shared-native-functional/`，固定二进制 `.benchmark/goal-wss/binaries/proxysss-a9f5c56`，SHA256 `51c76b5165349b158adfe8bbbf69397fa03f965e237ca4c63967c276eb324cf9`。这些是失败实验工件，未进入 Windows 验证、push 或发布。

## 2026-09-21：CPU 资源语义修复，性能门槛仍未通过

提交 7dc3e82 修复 taskset 启动范围被外层 cpuset 扩大、数据面忽略 quota 预算的问题。349 项 Linux 测试连续两轮、Clippy/格式/文档/CLI 和真实网关 CPU 探针通过；额外 cpuset=0–3、taskset=0,3、quota=1.5/0.5 CPU 容器中，基础预算均为 1，HTTP/native relay/sendfile owner 各 1，所有线程范围和 HTTP/TCP/UDP 正文验证通过。功能证据保存在 `.benchmark/cpu-resources-verification/`。

同提交的无记录器完整矩阵仍失败：Ubuntu 24.04 native-amd64 Docker Desktop VM，标准 6/6/12 核，198 原始样本零错误、双方等负载完成率通过。大文件吞吐比 0.700/0.630/0.740；CDN p99 比 3.041/2.261/1.896，小静态 p99 为 3.149/2.560/2.225。4× TCP p50/p95/p99 为 1.105/1.080/1.032，WebSocket 为 1.118/1.151/1.082。保留资源配置的正确性修复，但不能声称 CPU 发现修复解决了既有吞吐或尾延迟问题，也不能把不同轮次波动当作因果收益。

4× cgroup current/peak bytes：proxysss 90849280/279580672，nginx 209395712/372604928。18 秒活跃、163 秒矩阵 wall、330 秒总耗时，包含 2m17s release 构建。工件 `.benchmark/direct-ubuntu24-amd64/goal-cpu-resources/`，固定二进制 SHA256 `efb96f789b211cabc7d059bf903bf2afa3fd011ce7fdc37d20fbdeb64ee484b7`。未进入 Windows、push 或发布。

## 2026-09-21：sendfile 墙钟与线程 CPU 诊断

固定基线 5427540，在 Ubuntu 24.04 native-amd64 Docker Desktop VM 使用标准 0–5/6–11/12–23 CPU 分配。C preload 仅记录两种网关的 sendfile/sendfile64，客户端与后端不变；有界 mmap 保存开始时刻、墙钟、线程 CPU、字节数、线程和 fd，真实 socket 自检覆盖完整内容、EOF、EBADF。18 个混合窗口全部捕获，proxysss/nginx 总记录为 14913/296483，无丢弃或未完成记录；198 个原始协议样本零错误。活跃测量 18 秒，矩阵 wall 163 秒。

等负载 proxysss 全部 sendfile 位于 HTTP owner，单次请求最多 16 MiB，线程 CPU p99 约 3.05–3.29 ms。nginx 也用 sendfile 发送小文件，不能混用全部调用分位数；只筛 requested≥1MiB 时，其 CPU p99 约 0.605–0.733 ms。这个证据确认 HTTP owner 有毫秒级同步 CPU 占用。饱和时部分 HTTP-owned 调用另有数毫秒 wall−CPU 差值，native sendfile owner 的对应 p99 较小；不能据此认定独立 reactor 必然更优。

wall−CPU 不是纯调度等待，也不是请求延迟；按开始时刻归属窗口，末次调用可能跨越结束边界。时钟调用和 trace 映射会影响吞吐及内存，因此此轮失败矩阵只作诊断，不能当发布证据。既有每 2 MiB 让出和全核常驻 reactor 的混合回归仍有效，下一方案须同时解释 HTTP 单次占用和实时 owner 竞争。

工件 `.benchmark/goal-sendfile-calls/` 保存原始 trace、18 个窗口时间、按线程/大调用分组报告与 collector 源码/二进制/摘要。基线二进制 SHA256：`566fde7932ef6c07c45b87da2cba7be4ab158560759a17b5686a5b82cc09f02c`。后续 CPU 发现修复处理 taskset/quota 配置语义，不应宣称它已解决这里的大文件性能瓶颈。

## 2026-09-21：balanced HTTP 分片允许 CPU 迁移，未接受

候选 6e48083 保留每允许 CPU 一个独立单线程 HTTP runtime、连接/socket/reactor 所有权、实时 native owner 和原 sendfile 边界，只将 HTTP worker 从单核硬绑定改为可在完整获准 CPU 集内迁移。349 项 Linux 功能测试连续两轮、Clippy、格式、文档与 CLI 检查通过，新增真实线程 mask 检查。该方案用于检验 SO_REUSEPORT 长连接分布不均的影响，并未证明这是主要瓶颈。

标准 Ubuntu 24.04 native-amd64 Docker Desktop VM，gateway/backend/client CPU 为 0–5/6–11/12–23，完整 1×/2×/4× 混合矩阵仍失败。198 个原始样本零错误，双方等负载完成率通过；大文件吞吐比为 0.677/0.584/0.765，其余吞吐通过。CDN p99 比为 2.988/2.486/1.789，静态小文件 p99 为 3.117/2.316/1.578；4× WebSocket p50/p95/p99 为 1.081/1.193/1.190，普通 TCP p50/p95 为 1.143/1.012。不能用部分尺度的大文件比值提高宣称跨场景优化成功，也不能从不同轮次推断精确因果差值。

4× cgroup current/peak：proxysss 87699456/282824704、nginx 209354752/375361536 bytes。18 秒活跃、163 秒矩阵 wall、336 秒总耗时（含 2m17s release 构建）。候选已由 a53d848 完整撤回，保留 34d42d0 的严格配置和官方示例修复。工件 .benchmark/direct-ubuntu24-amd64/goal-http-affinity/；固定候选二进制 SHA256 为 299cf1f10e25b4f2b90dfa36c1e09129159879dcccf5afd81f91ec77f6a02005。本轮不是发布通过证据。
## 2026-09-21：balanced 每连接独立任务与直接 sendfile，未接受

候选 49b4782 为每条明文 TCP/WebSocket 连接建立独立 Tokio 就绪任务，留在接收连接的 HTTP 分片；每方向至多 16 个 I/O 步骤，线程共享 16 KiB 临时缓冲，背压时才保留独立缓冲。balanced 下载直接 sendfile，每 2 MiB 让出同一队列。339 项 Linux 测试连续两轮、Clippy、格式、文档与配置检查通过，覆盖半关闭、背压、取消和 WebSocket 交接。

标准 Ubuntu 24.04 native-amd64 Docker Desktop VM，gateway/backend/client CPU 为 0–5/6–11/12–23，完整 1×/2×/4× 混合矩阵失败。198 个原始样本零错误，双方等负载完成率通过；大文件吞吐比 0.814/0.700/0.763，其余吞吐通过。游戏 TCP p99 比为 1.346/1.357/1.184，普通 TCP 为 1.552/1.307/1.435，WebSocket 为 1.446/1.039/1.386；静态小文件 p99 为 1.545/1.475/1.405。共享任务队列没有证明跨协议尾延迟改善，不能仅凭部分 HTTP 相对比值降低接受该设计。

4× cgroup current/peak：proxysss 59404288/247631872、nginx 209674240/374738944 bytes。18 秒活跃、162 秒矩阵 wall、202 秒总耗时。候选已由 cf03b23 完整撤回，保留此前文件版本与下载边界正确性修复。工件 `.benchmark/direct-ubuntu24-amd64/goal-pair-tasks/`；固定二进制 SHA256 为 `98e80089432c2f781f06ce76ead1d9e128849c68d38d9d026565bb31651fcdac`。不同轮次的参考吞吐和目标会变化，不能将单轮差值解释为精确因果收益。

## 2026-09-21：balanced native relay 与 sendfile 作为 HTTP 分片任务，未接受

候选 35afe39 将 balanced 每核 native relay 和 sendfile 的子 epoll 交给同核 HTTP Tokio runtime 的 AsyncFd 驱动。relay 每轮至多 64 个事件和 64 个接入，sendfile 每轮一个作业、2 MiB；处理后让出 Tokio。保留半关闭、背压、取消清理与 fd 复用顺序，336 项 Linux 测试两轮、fmt、Clippy 与场景配置检查全部通过，但功能通过不能证明调度公平。

Ubuntu 24.04 amd64 Docker Desktop VM，gateway/backend/client 分别使用 0–5/6–11/12–23 CPU，标准 1×/2×/4× 全混合矩阵失败。198 个原始样本全部零错误；大文件吞吐比为 0.377/0.209/0.165，静态小文件为 0.812/0.615/0.539，CDN 为 0.837/0.679/0.580，反代为 0.880/0.661/0.527，SSE 为 0.786/0.579/0.482。1× 游戏 TCP 与 WebSocket 分别约为 nginx 的 5.78 倍和 5.32 倍，不能抵消 HTTP 与下载退步。4× TCP 等负载完成率 0.976，低于 0.980；多个延迟门槛亦未通过。

4× cgroup current/peak：proxysss 82624512/202162176、nginx 209272832/380526592 bytes。18 秒活跃测量、163 秒矩阵 wall、202 秒总耗时。候选已撤回到此前数据路径，保留用户确认的验收约束和完整工件 `.benchmark/direct-ubuntu24-amd64/goal-cooperative-shards/`。不能将实时吞吐改善作为全场景优化，也不能通过忽略静态场景或降低门槛接纳该架构。

此前同一标准资源配置的 perf 诊断位于 `.benchmark/goal-perf-standard/`，使用 7681946 二进制；观察器与客户端共享 CPU，仅用于定位。等负载 proxysss 采样 CPU 总量低于 nginx，但这不足以证明线程调度是唯一根因；候选失败也不能证明所有协同调度架构都不可行。下一次改动必须解释工作分配与事件公平性，不能只再次修改一个轮询预算。

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
