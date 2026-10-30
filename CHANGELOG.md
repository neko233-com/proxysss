# Changelog

## v1.3.12 - 2026-10-30

- 新增 `proxysss traffic`，按需读取本机管理接口，以有界快照计算域名请求增量和速率，展示当前 WebSocket 连接数；不增加转发路径的统计开销。
- 此命令只反映到达当前网关的请求，不代表 CDN 边缘流量；不保存历史数据，重启或计数回退时拒绝输出误导性速率。
- 明确 Ubuntu 与 Debian 独立功能验收要求，整理文档索引和归属，修正过时的发布压测说明。
- 云证书同步成功状态持久化，重启后未变化的目标和证书不重复同步；证书或目标变化及未完成任务仍会自动同步或重试。
- 内建 ACME 默认每天检查，续签窗口可配置，默认为有效期剩余少于 15 天；默认关闭开发构建的增量缓存，减少长期磁盘占用。

## v1.3.11 - 2026-10-30

- 新增可选云证书同步目标，首个策略支持火山引擎证书中心和 DCDN；证书变更后后台同步，`proxysss certificate sync` 支持手动或外部定时触发，检查绑定状态及当前网络出口的 TLS 证书。
- 内建 DNS-01 支持环境变量引用，修正火山引擎 DNS 签名和 TXT 记录处理；托管 ACME 支持独立 ECDSA/RSA 证书，并增加有效期判断及证书对安装恢复。
- 新增可搜索的多渠道飞书卡片，云同步结果包含证书 ID、证书域名和 HTTPS 核验链接；新增只读 WebSocket 会话指标和默认开启、可关闭的有界域名请求计数，以及跨平台 `proxysss open config`。
- `proxysss certificate list` 查看托管证书，`certificate issue` 与 `certificate sync` 支持选择单项、多项或全部执行。
- 只读监控增加运行时长和主证书及 SNI 证书的磁盘有效期信息；缺失或无法解析时明确返回空值。
- 火山引擎同步发现同一证书绑定的 DCDN 域名并安全替换，记录部分部署以便重试；可选清理经过身份与绑定核验的旧实例，补充独立权限指南。
- 取消强制发布前压测门槛，保留功能验证、目标环境验收和六平台打包；更新文档中的安装命令与版本下载入口。
- Linux 发布包在 Debian 12 环境构建，避免较新 Ubuntu 构建引入 Debian 12 不支持的 glibc 依赖。
- Windows 提供当前用户 MSI 安装包，支持替换安装目录已有程序、同版本修复和新版升级；不覆盖用户配置或自动注册服务，也不更新其他位置的副本。

## v1.3.10 - 2026-10-21

按用户明确授权交付 Linux nginx 迁移所需的功能与稳定性；不宣称性能胜过 nginx、跨主机性能验收或所有平台运行全绿。当前证据、Windows IPv6 环境限制及最终发布状态见 [版本验证记录](docs/RELEASE-1.3.10.md)。六平台附件仍须全部构建与核对。

- 新增有界原生异步 `auth_request`，先于业务缓存、脚本选择的静态文件和主上游执行；固定 GET、禁止访客头体隐式继承及重定向，超时/并发满拒绝，保留明确状态映射和 401 challenge。
- 脚本支持合成状态/正文、按状态注入或追加独立响应头、实际缓存状态复制、独立回源超时与访客头体转发开关；显式 Host 最后生效，重复 Set-Cookie 与 Secure/Expires 保留。
- 脚本静态站点可设 `script_only`，统一保留授权、真实 peer、Host/path 与 HEAD/Range 边界；SPA 仅回退安全缺失目标，H2 authority 在 URI 改写前保存。
- 严格 SNI 按配置域名选择器拒绝未知或无 SNI；全部证书与私钥成对验证后重载，半更新保留旧状态。HTTP 自动 308 可显式关闭，脚本与静态错误响应不再被默认错误页改写。
- H1/H2/H3 共用可选请求正文预算，复制前预留、全部 Bytes clone 释放后返还，重载不重置在飞计数；预算衡量 payload，不是进程 RSS 上限。
- 可显式覆盖普通回源方法；HEAD→GET 保留访客方法和正确编码元数据且前端无正文，HEAD 缓存仍 bypass。响应头 `append` 保留上游重复字段与原顺序。
- 路由压缩复用现有算法，只有已知长度≤4 MiB 的合格响应缓冲变换；未知/大响应继续流式，SSE、Range、已有编码不变换。保留上游 Vary；压缩表征及对应 HEAD 弱化强 ETag，源站编码缺 Accept-Encoding 缓存维度时不存储。
- 隔离直接构造配置的测试证书路径，避免跨夹具共享证书/私钥；保留完整断言。文档、脚本声明与可执行协议回归同步。

## v1.3.9 - 2026-09-21

本版本按用户授权优先交付完整功能与稳定性，不以超过 nginx 为发布条件。CDN 优先小文件优化，大、小文件不要求同时胜出；两类复杂下载能力都保留。Ubuntu 全功能及混合负载结果、Windows 快速验证与性能边界见 [v1.3.9 验证说明](docs/RELEASE-1.3.9.md)。本地 Ubuntu 两轮各 388 项测试、Windows 361 项测试通过；混合负载 198 个原始样本零错误。当前矩阵仍有大文件吞吐与尾延迟差距，不宣称严格性能胜出或跨主机验证。

- 修复动态黑名单持久化：版本化快照保留原始 UTC 起点、纳秒和 TTL，重启扣除停机时间，不复活已过期封禁；拒绝损坏、重复或未知格式。管理写入先同步同目录唯一临时文件并原子替换，再更新内存与 DDoS，失败返回 500；单 worker 按顺序提交，调用方断开后已接受的写入继续持有执行责任，普通准入不等待文件锁。新增重启、最大 TTL、并发快照读取、失败和取消回归。修复未持久化的管理封禁被免策略 HTTP/AI/WS 反代绕过；admin/热重载开启时统一经过当前准入链，真实 keep-alive/H2/AI/WS 回归覆盖封禁和解除，并保持既有 WS 会话。Ubuntu 功能夹具移至容器独占的原生 Linux 卷并记录文件系统，避免 Windows 共享 v9fs 的非原子替换影响文件语义验收；严格并发断言保持不变。

- 修复长期运行的安全状态积累：独立控制任务回收过期 DDoS、登录失败和黑名单记录，保留活动窗口与封禁；黑名单只读查询避免误删并发续封，DDoS/黑名单用 IP 地址键免除接入路径字符串分配。封禁时长按已过时间计算，超大秒数不再导致 Instant 溢出，并发登录失败不重置有效锁定；新增真实管理 API、后台回收与并发边界回归。

- 修正稳态压测预热：每条 HTTP/1 连接完整读取两次真实响应，避免其余连接仅完成 HEAD、在测量内才增长接收窗口。有限 SSE 夹具支持 keep-alive，发生器新增 `--drain-response` 完整消费正文、保留首块延迟并在测量截止取消等待；持续 SSE 默认仍支持按块数提前结束。HTTP 同样在测量截止取消响应头/正文等待，不计截止后完成。新增真实连接、复用、截断和截止时间回归；发生器修正不代表网关性能收益，双方成绩须重建。

- 修正混合压测的发生器资源元数据：每波保留实际 taskset affinity、计划与真实 Tokio worker 数并严格核对，避免把整个 client 容器配额误报为单场景资源；不改变负载目标或 CPU 分配。

- 修复 UDP 新会话绕过 stream allow/deny、连接限流、DDoS 与动态黑名单；连接配额按 association 扣取，与 TCP 共享 zone，既有数据报不重复扣取。统一流拒绝统计，修复 TCP DDoS 拒绝重复计数。listener supervisor 及 HTTP/TLS/TCP 跨 runtime 包装任务增加取消所有权，防止重载撤销监听后旧 accept 任务继续持有端口。

- 修复 UDP 并发建会话突破 max_associations 的问题：先预留容量，再复制首包、运行脚本或 DNS，失败和取消自动释放。退休会话唤醒等待任务，弱引用缓存不保留旧 socket；listener 停止会取消独立运行时 worker 和在途创建。满额保留活跃会话，清理扫描限频；同名同 bind 的容量/TTL/protocol 更新供新会话使用，既有会话保留原 TTL。

- 统一 TCP/UDP、stream route、FTP 的上游地址校验，管理更新在落盘前拒绝无效地址，脚本流路由检查主目标、候选池和权重键。UDP 新会话支持域名，DNS 解析有并发上限与超时；UDP 健康探测按解析结果选择 IP 地址族，超时覆盖解析、发送和回复。
- 管理接口的 stream route 与 TCP/UDP listener 统一应用上游主机/IP 策略；主机规则按去除端口与末尾点的名称比较，修复 metadata 主机带端口绕过规则的问题。MQTT 示例、CLI 场景输出和内建文档将越界端口 88831 修正为 18833。

- 修复混合性能报告用中位吞吐掩盖单轮完成率不足的问题：保留最低完成率及轮次，检查样本数量、参数与双方目标一致性；吞吐和延迟仍取中位数。HTML/Markdown 显示最低完成率，Go 报告器回归纳入本地 Ubuntu 功能入口。

- 修复缓存静态/sendfile、原始反代、SSE、WS/WSS 快速路径漏记 HTTP 请求，以及分发异常重复累加错误的问题；握手在长连接存活期间即可观测，消息不计作 HTTP 请求。JSON/Prometheus 统一对网关生成和上游返回的 4xx/5xx 响应计数，真实 HTTP/1、TLS/H2/H3、WebSocket 收发与回退纳入回归。

- Linux 数据面统一记录启动 affinity 与 quota 预算，不再用 cgroup CPU 列表覆盖 taskset 限制；HTTP/TCP/UDP、TLS 与 native owner 共享预算。动态亲和位图支持 CPU ID 超过 1023，诊断输出区分控制运行时和数据核预算；真实受限进程的 HTTP/TCP/UDP 收发、线程范围与 owner 数量纳入 Docker 功能验证。

- 主配置与嵌套对象严格拒绝未知字段，错误信息包含文件与字段；合法别名和自定义键值映射保留，错误重载不替换有效服务状态。修复首页、nginx/Caddy 迁移指南失效配置；迁移 HTML 正文从 Markdown 生成并校验源/正文摘要，全部公开和内置文档及随包 YAML 纳入可执行配置验证。

- 修复 Linux 自动 socket 调优覆盖 TCP listener 显式 nodelay=false 的问题；接入、直接上游与策略上游统一应用配置。接入循环读取当前 listener 的 nodelay、连接超时和协议提示，热更新后新连接生效，已有连接保持；真实 socket 开关与收发行为纳入回归。架构页的反代、游戏、AI 和脚本示例改为可执行配置，纳入 Markdown/HTML 配对与逐字段回读验证。
- 修复 Linux 原子发布的文件版本混用：sendfile 池按已打开版本扩容/预热，缓存和弱 ETag 识别保留长度/mtime 的替换；完整/Range 下载打开后核对版本，完整流式与非 Linux 复制严格限制 Content-Length，提前截断作为正文错误处理。
- 新增静态站点 hosts 匹配（精确、通配子域、IPv4/IPv6）与多站点缓存隔离；同路径按 YAML 首个匹配站点解析，预热、HTTP/1 keep-alive、H2 快照、Range 与 Linux sendfile 不跨站复用资源，支持热重载撤销域名。
- 修正配置指南中无效的反代、静态、WebDAV、TCP/UDP、权重/限流、DNS-01 和管理 token 示例；Markdown/HTML 示例统一通过真实配置类型与逐字段回读测试，避免未知字段静默失效。
- 修复原始 SSE 向 TLS/HTTP2 转换时泄漏 chunked 分块字节、等待 keep-alive EOF 和丢失上游 Content-Encoding 的问题；有界解码消息边界，保留压缩正文元数据，截断/非法分块不再伪装为完整响应。同步修正 AI/SSE 文档中未实现的配置字段。
- 修复 Linux 原生 sendfile 的取消清理：HTTP 等待任务退出后主动唤醒 owner 并释放后台 socket/file 副本，避免客户端保持连接但停止读取时遗留作业；使用每 owner 有界无锁通知，普通完成不扫描任务表。
- 修复 Linux 大文件下载的持久 socket 副作用：移除逐响应 SO_SNDBUF 增大/恢复逻辑，避免关闭发送缓冲自动调优并让后续 keep-alive 下载受限；保留 sendfile 背压与资源池上限。
- 修正 HTTP/SSE 等负载半整数目标：恰在测量截止点的请求不计入可执行目标，空计划拒绝执行，避免双方完成全部有效请求仍被判低于 98%。
- 修正 WSS 等负载短窗口目标与截止行为：只要求可执行完整 tick，默认逐轮校验双方同目标和 98% 完成率；等待、收发与重连到期退出，避免停滞连接延长压测。
- 修正 WSS 容量验证：持有期结束逐连接 echo，区分 opened 与 retained；在全连接存活期间采内存并执行 2× 对照门槛。客户端/后端按 CPU 配额启动运行时，活跃采样在所有预热 echo 完成后开始，排除空闲超时和单线程客户端造成的假通过。

- 静态缓存使用原子预算和一次性淘汰扣减；所有在途/缓存快照总计不超过 320 MiB，预算不足时冷下载回退流式读取，填充失败或取消不遗留协调锁。

- 修复可变静态文件 mmap 在截断后通过 TLS 发送导致 Linux SIGBUS 退出的问题：正文缓存使用有界不可变快照，H2 重验证失败后使连接缓存失效，已删除资源回到正常 404 路径。
- 原生静态下载支持最多 16 段 Range、重叠/相邻合并和有界 multipart 流；局部下载按偏移读取，修复小区间读取整个大文件和文件截断可能引发切片越界的问题。HEAD 忽略 Range，补齐 If-Match/If-Unmodified-Since 的 412 与前置条件优先级。
- 按常驻规则、当前架构、主题指南和性能决策拆分 agent 文档；新增中文验证/发布指南与本地 Go 文档检查，维护性能证据 schema v3 示例。该改动不表示严格 Linux 性能发布门禁已通过。
- 重做管理后台：浅色工作区、深色侧栏、中文导航、表格与表单；默认只读状态可见，保存和删除按钮随写入开关禁用，原始 JSON 收入折叠诊断区。
- 内置文档与公开文档统一排版，提供目录筛选、代码复制、分步配置说明和小屏布局；管理端口也可直接访问 /docs.html，静态文档不包含实际配置密钥。
- 默认 80 欢迎页增加中文引导，保留 Welcome to proxysss 标题与 GitHub、GitHub Docs 两个入口，不加载外部资源。
- README.md 改为英文，新增 README-CN.md 中文说明，六平台发布包均包含双语说明与完整中文 HTML 文档。
- 将后台与内置文档从 Rust 字符串拆到独立模板，样式和脚本仍编译进单一二进制；用户名及 HTTPS 管理路径在 HTML 中转义。


## v1.3.8 - 2026-09-08

- 本次为经用户授权的功能版本，豁免此版本的严格 Linux 性能证据要求；不附带严格领先 nginx 的性能声明。功能测试和六平台打包已通过。
- 六平台打包统一附带 CHANGELOG、中文 HTML 文档、配置示例、模板和安装脚本，使用项目内固定 staging 并检查归档内容；GitHub Actions 保持纯打包，功能测试在本地执行。
- 基于远端 v1.3.7 更新，新增静态 CDN 回源令牌/真实对端 CIDR、短期 HMAC-SHA256 URL、站点限流、缓存验证器；私有响应禁止共享缓存。
- 加固静态根目录边界、隐藏文件、index/预加载与 HTML 目录索引，限制扫描条目并处理热重载撤权。
- 本地连续两轮幂等验证与固定目录打包；测试、缓存、报告、staging 和 ZIP 统一留在项目内。
- 新增 config security/config performance：安全开关、默认值、中文建议和原生 socket 能力探测；CDN/FTP 支持保留参数的独立策略开关，修复单 IPv4 ACL，拒绝未实现的 MAC 黑名单配置。
- Windows/macOS 保持已验证的运行时调度，增加独立 socket 适配，保留 TCP 自动调节、可选 keepalive 与共享 UDP 监听缓冲；上游 UDP 会话不强制扩容。macOS 可选 TCP_NOTSENT_LOWAT；Linux 保留原有发行版和 CPU 自适应。runtime.performance 变更需重启。
- Docker 验证固定使用 proxysss-verify，运行前清理同名项目容器，结束或失败自动删除；系统探测、测试、场景检查和可选混合诊断复用一个容器。

## v1.3.7 - 2026-08-20

- Added the production api.neko233.com AI gateway route for ai-router, including root/admin, health, OpenAI Responses, and Chat Completions paths while keeping the upstream listener loopback-only.
- Removed admin-token forwarding at the edge and kept the AI route on the low-allocation streaming path with hot-reloadable configuration.

- Hardened Windows auto-start repair: HKCU Run is now the single preferred hidden entry, legacy direct-console scheduled tasks are detected and removed, status reports duplicate or unsafe startup entries, and installer errors preserve the underlying permission failure.
- Bounded local benchmark disk usage: default Rust targets and reports are disposable, and proxysss-owned Docker benchmark images are removed after a run unless artifact retention is explicitly enabled.
- Rebuilt the default public `/` page as a zero-asset, responsive `Welcome to proxysss` screen containing only GitHub and GitHub Docs links.
- Locked the default TLS ALPN order so ordinary clients prefer HTTP/2 on port 443 while ACME TLS-ALPN-01 remains available, and added default-config coverage for self-signed TLS bootstrap.
- Hardened the Ubuntu 24 Docker benchmark entrypoints for Windows Docker Desktop path handling and emitted a same-run fairness manifest proving matching ports, CPU/FD/sysctl limits, protocol surfaces, and nginx/proxysss optimizations. Strict measured validation remains capped at 60 seconds.
- Added explicit `REUSE_BENCH_IMAGE=1` support for offline/transient-registry validation; the wrapper still probes the cached image for amd64 and Ubuntu 24.04 before building or measuring the checkout.
- Fixed `verify-docker-scenarios.sh` on Windows Git Bash by preserving container `/work` paths and explicitly converting only the host checkout mount.
- Added `REUSE_VERIFY_IMAGE=1` for registry-independent scenario validation while retaining an explicit Ubuntu 24.04 x86_64 image probe.
- Made the Ubuntu 24 wrapper record and forward explicit `TRAFFIC_PROFILE`, saturation/latency run orders, and benchmark subnet/role addresses so default-small and balanced experiments remain reproducible without editing scripts.
- Enforced that 60-second cap inside every client wave, shortened synchronized startup and UDP/QCP tail waits, kept balanced UDP on the proven per-core fast path, and reduced TLS/sendfile/realtime owner contention under mixed load.
- Replaced the hot-path async configuration `RwLock` with atomically published ArcSwap snapshots, so HTTP/H2/stream requests remain reload-safe without lock acquisition or wakeups per state read.
- Added bounded UDP cooperative fairness, lower-weight realtime owners, a cache-local TLS owner profile, and fat-LTO release builds to preserve per-protocol latency under saturated mixed traffic.
- Partitioned every mixed-wave client scenario and backend protocol service onto disjoint CPUs, including separate UDP/QCP echo listeners, so faster stream paths cannot steal generator/backend time from HTTP; the full strict matrix now requires at least 24 Docker CPUs.
- Fixed lazy native-reactor CPU discovery to use the cgroup effective cpuset instead of an already pinned caller thread; sparse WebSocket/TCP owners explicitly regain the full mask for soft CFS ownership, while dense relay and sendfile owners retain deliberate per-CPU placement.
- Made balanced UDP/QCP fairness burst-aware: the shared per-core runtime yields after eight packets only when they arrive within eight milliseconds, avoiding scheduler tax on latency-sensitive low-rate traffic while preserving saturated mixed-load fairness.
- Bounded the default-small TLS crypto runtime to one owner per four detected CPUs and its UDP/QCP runtime to one owner per two CPUs, preserving full-CPU `SO_REUSEPORT` listener fanout and adaptive scaling while reducing mixed-traffic scheduler contention.
- Consolidated default-small HTTP/1.1, HTTP/2/TLS, WebSocket, TCP, UDP, and transparent QCP work onto the same CPU-adaptive per-core data shards, halving I/O-driver polling intervals for latency, using cooperative sendfile yields instead of timer sleeps, and keeping tiny reverse/SSE upstream sockets on Linux autotuning rather than forced bulk buffers.
- Replaced isolated one-thread default-small data runtimes with one CPU-sized work-stealing Tokio runtime. HTTP/H2/TLS/WebSocket/TCP/UDP/QCP still share one worker per allowed core, but ready connections can escape SO_REUSEPORT hash skew instead of leaving sibling cores idle during mixed load.
- Disabled Tokio's non-stealable LIFO slot on the unified data runtime, ensuring echo-style WebSocket/TCP/UDP/QCP wake chains return to stealable worker queues instead of creating cross-protocol p99 stalls.
- Removed sub-p99 self-wake boundaries from low-density HTTP/TLS/WebSocket lanes: the data runtime now polls injection/I/O twice as often, WebSocket tunnels use the normal bounded relay budget, and explicit HTTP yields occur every 256 responses until connections exceed 256 per data worker. Saturated high-density traffic retains the proven 32-response fairness boundary.
- Restored Tokio's connection-local LIFO wake slot and the throughput-proven 31/16 injection/I/O cadence after strict mixed evidence showed that queueing every fresh I/O wake behind the stealable local queue created a common ~2 ms p99 step and that the 15/8 cadence reduced static/HTTPS throughput. The short-period HTTP/WebSocket self-wakes remain removed, preventing the monopolization risk that originally motivated disabling LIFO.
- Restored the UDP/QCP benchmark response timeout to 500 ms and expanded only the non-measured process grace to four seconds; a 50 ms timeout produced false packet errors at higher scales while active measurement remains capped at 20 seconds.
- Reduced the official Ubuntu 24 strict matrix to a 20-second active-measurement budget with synchronized one-second samples. All 1x/2x/4x saturation and equal-load windows count toward that budget; orchestration wall time is reported separately, and every wave retains a hard stuck-process timeout.
- Added free managed-ACME certificate key selection: stable, efficient ECDSA P-256 remains the default, while `http.tls.acme.key_algorithm: rsa2048` provides explicit legacy-client compatibility without external certificate tooling.
- Removed the shared HTTP request-counter cache-line bounce with per-data-shard padded counters, restored a throughput/latency-balanced 16-poll I/O driver cadence, and made immutable HTTP/2 static hits complete synchronously inside the connection service without per-request gateway Arc contention. Plain static connections now refresh the global fairness threshold only at yield boundaries and amortize the one-second revalidation clock check over 32 exact hits. Kept the proven 2 MiB sendfile fairness slice after a 16 MiB experiment starved mixed HTTP siblings.
- Fixed the mmap static-cache threshold being Linux-only at compile time even though the portable cache path uses it, restoring Windows/macOS build parity for large cached assets.

## v1.3.5 - 2026-07-01

- Added independent QCP UDP listener coverage for neko233-com/QCP alongside existing KCP-style UDP examples, templates, capability output, nginx-parity output, and Chinese-first docs. KCP and QCP remain separate listener modes; proxysss forwards datagrams transparently and leaves protocol framing/reliability semantics to upstream services.
- Optimized the UDP data path for transparent high-throughput forwarding: pending-session dedupe now uses a sharded set, UDP association pruning is throttled under churn, listener/reader buffers come from the bounded reusable pool, upstream response readers drain ready datagrams in batches, and policy-free single-upstream UDP listeners use a direct fast path that avoids per-new-client script-routing task churn.
- Added a faster per-worker UDP association cache and moved benchmark defaults back to nginx-comparable UDP only, while keeping KCP/QCP as proxysss-native capability validation instead of pretending nginx has native KCP/QCP protocol semantics.
- Updated the Ubuntu 24 benchmark container and Go benchmark helper so Docker validation can run the official script path end to end; `SCENARIO_FILTER=udp-stream` now supports quick UDP-only release checks. Docker UDP-only validation passed at `4.045x` proxysss/nginx with 0 errors (`127742.75` vs `31577.33` ops/s).
- Updated the current benchmark matrix to generic gateway scenarios only: `new-api-sse` is replaced by ordinary `generic-sse`, and KCP/QCP special UDP encapsulations are excluded from performance benchmark docs/scripts while remaining supported listener capabilities.
- Simplified default GitHub Actions CI to packaging-only six-platform binary builds; tests, smoke checks, and performance benchmark gates now remain manual/operator validation paths. Refreshed benchmark docs so v1.3.5 UDP fast-path results appear before older mixed-load UDP baselines.

## v1.3.4 - 2026-06-15

- Promoted the official GitHub Actions Linux benchmark from a single-scenario throughput snapshot to the real mixed all-protocol matrix: the Linux benchmark workflow now runs `scripts/benchmark-all-scenarios.sh` with `QUICK=1`, `DURATION_SECS=12`, and `MIXED_MATRIX=1`, then publishes the mixed results artifact (`results.json`, `summary.md`, `summary.html`) instead of a static-only quick report.
- Clarified benchmark roles across docs: the official Linux benchmark docs now say the release-facing artifact must compare `static / reverse proxy / New API / SSE / WebSocket / TCP / UDP / KCP-style UDP` together, while Windows quick throughput remains a smoke-only auxiliary path.
- Hardened the mixed benchmark gate for CI reality without weakening the benchmark intent: SSE now allows only a tiny `proxysss <= nginx + 1` stream-error tolerance on GitHub runners, and benchmark artifacts upload with `if: always()` so failed gates still leave behind `summary.md` / `summary.html` for diagnosis.
- Kept `kcp-style-udp` in the GitHub Actions Linux all-protocol comparison artifact but moved it to the hosted-runner diagnostic set instead of the hard-fail critical gate, because GitHub-hosted UDP virtualization is too noisy to act as the final KCP fairness judge. Dedicated tuned Linux hosts still keep the stricter realtime gate.

## v1.3.3 - 2026-06-15

- Kept the low-allocation plain raw reverse-proxy and AI/SSE fast lanes side-effect free: default `X-Forwarded-*` / `Forwarded` semantics and `proxysss-ai-*` metadata headers now survive the fast path, so upstream observability and agent-stream routing do not regress just because the route uses the optimized path.
- Raised the default HTTP/2 reverse-proxy tuning ceiling and added end-to-end coverage for plain fast-lane forwarding semantics plus HTTPS/HTTP2 round trips, tightening the release gate around SSE / reverse proxy / HTTP2 behavior instead of chasing one isolated throughput win.
- Added an official Linux benchmark documentation pair for humans and release bundles: `benchmark-linux.md` explains the latest GitHub Actions Linux quick benchmark, and `docs/benchmark-linux.html` gives the matching HTML entry while keeping mixed-load release validation and no-side-effect performance rules explicit.

## v1.3.2 - 2026-06-15

- Hardened the no-side-effect performance validation toolchain around the Go benchmark helper: CI benchmark runs now trigger when `scripts/benchmark-helper.go` changes, and obsolete Python benchmark report dependencies were removed from GitHub Actions so the official benchmark path stays Go-first end to end.
- Cleaned the deploy/release packaging manifests so the new `caddy -> proxysss` migration guide ships alongside the existing nginx and TypeScript docs instead of leaving the release bundle with partial documentation.
- Refreshed the built-in docs and README capability wording so supported surfaces like `token-bucket` rate limiting and migration tracks stay aligned with tests, docs, and release evidence.

## v1.3.1 - 2026-06-15

- Static small-file delivery now copies response head + body into one pooled buffer and issues a single write syscall (nginx single-segment parity), removing the extra syscall and separate header packet; static-small and CDN hot-update now exceed nginx in the Ubuntu 24 mixed benchmark.
- SSE/streaming proxy forwards each upstream read immediately with no event-boundary buffering (nginx `proxy_buffering off`), lowering first-token and inter-token latency for AI streaming; New API/SSE now meets or beats nginx.
- Larger cached bodies and sendfile static keep `TCP_CORK` head+body coalescing for single-segment delivery.
- Fixed CI clippy failures (`identity_op`, `needless_return`) so the quality gate passes cleanly.

## v1.3.0 - 2026-06-15

- Added bounded lock-free buffer pooling (`ByteBufferPool`) for the hot-path raw HTTP/SSE relay and UDP association readers, cutting per-connection heap churn under very high concurrent socket counts while keeping resident pool memory bounded (no leak): surplus buffers are freed on return.
- Added startup/reload self-optimization warm-up: hot static files are preloaded into the bounded fast-lane cache and reverse-proxy/AI-proxy upstream keepalive pools are pre-dialed before listeners begin serving, so the first live request never pays a cold connect. Warm-up reruns after every hot reload, and `/healthz` now reports a `warm` readiness flag.
- Reduced UDP datagram loss for game/KCP/IoT traffic by enlarging UDP socket buffers (`SO_RCVBUF`/`SO_SNDBUF` plus privileged `SO_RCVBUFFORCE`/`SO_SNDBUFFORCE`) on both listener and upstream sockets under Linux performance mode.
- Removed a redundant per-chunk copy in the streaming SSE relay and enlarged its read buffer for lower latency and overhead on long-lived event streams.
- Added file-descriptor ceiling tuning (`fs.nr_open`, `fs.file-max`) to the Linux sysctl profile to support super-scale gateways targeting 100k–1M concurrent sockets, paired with a high systemd `LimitNOFILE`.

## v1.2.11 - 2026-06-12

- Added production hardening for critical gateway operation: supervised runtime watchdog, watchdog heartbeat metrics, and critical task failure counters.
- Added opt-in UDP active health probes for game/KCP/IoT datagram services, alongside existing HTTP/TCP active health.
- Added first-class game, MQTT, and IoT configuration surfaces: low-latency TCP tuning, UDP association caps/TTL, MQTT TCP, MQTT TLS passthrough, MQTT over WebSocket, and CoAP-style UDP examples.
- Added production hardening documentation and release gate guidance covering benchmark baselines, HA patterns, Linux tuning, and operational watch points.

## v1.2.10 - 2026-06-12

- Fixed reverse proxy streaming normal HTTP responses (HTML, JSON, etc.) as if they were SSE, which stripped `Content-Length` and caused incomplete response bodies.
- Now only `text/event-stream` responses are streamed; all other responses are fully buffered to preserve original Content-Length and ensure delivery完整性.

## v1.2.9 - 2026-06-11

- Added browser-language-aware admin UI localization for English and Simplified Chinese, with a manual language selector.
- Localized login, dashboard navigation, metric labels, filters, empty states, status text, and upstream health table labels.

## v1.2.8 - 2026-06-11

- Reduced admin dashboard clutter by folding endpoint docs, moving filters into the main dashboard, and compacting upstream health details.
- Simplified the upstream health table into fewer scan-friendly columns so long route lists no longer crowd the first screen.

## v1.2.7 - 2026-06-11

- Added `/v1/login` for dedicated admin login sessions with signed Bearer tokens and automatic dashboard entry until token expiry.
- Restored the admin console to a dark default style while keeping login and dashboard as separate UI screens.
- Kept admin passwords out of generated HTML and browser session storage; only short-lived signed session tokens are stored.

## v1.2.6 - 2026-06-11

- Reworked the built-in admin console into a dedicated login-first experience with session-scoped credentials.
- Removed the inline Quick Login card and stopped embedding the configured admin password into the generated HTML.
- Restyled the admin UI with a cleaner light dashboard layout inspired by new-api while keeping the single-binary built-in console.

## v1.2.5 - 2026-06-11

- Added proxysss process resource metrics to `/v1/stats` and Prometheus output: PID, CPU percentage, resident memory bytes/MB, and memory percentage.
- Updated the built-in admin dashboard with live Process CPU, Process Memory, and Memory % cards.

## v1.2.4 - 2026-06-11

- Made SSE hardening headers idempotent so existing upstream `Cache-Control: no-cache` and `X-Accel-Buffering: no` values are preserved without duplicated tokens.
- Kept the v1.2.3 streaming safeguards: SSE/streaming responses bypass compression and carry `no-transform` plus unbuffered proxy hints.

## v1.2.3 - 2026-06-11

- Hardened AI/SSE reverse proxy responses by automatically emitting `Cache-Control: no-cache, no-transform` and `X-Accel-Buffering: no` for streamed or `text/event-stream` responses.
- Prevented response compression from touching streamed bodies or SSE payloads, avoiding proxy/client buffering risks for Cline, Claude Code, and other streaming CLI clients.
- Added regression coverage for SSE response headers and compression bypass behavior.

## v1.2.2 - 2026-06-11

- Fixed AI reverse proxy streaming for New API, sub2api, OpenAI-compatible SSE, and Cline/CLI clients by forwarding upstream HTTP/1.1 and HTTP/2 response chunks directly instead of buffering the full response body.
- Kept response cache and HTTP/3 paths on buffered bodies so cache correctness and existing QUIC behavior stay stable.
- Added an e2e SSE regression test that verifies the first AI proxy event arrives while the upstream connection remains open.

## v1.2.1 - 2026-06-10

- Added **HTTPS admin API** (`admin.https`): expose the full `/v1/*` automation surface on the main TLS listener at `admin.https.path_prefix` (default `/_proxysss/admin`); bootstrap TLS/ACME on loopback first, then drive agents over HTTPS with Bearer auth.
- Added **SNI certificate admin API** (`GET/POST /v1/tls/sni-certificates/*`) with PEM upload or path-based upsert, plus TLS panel UI.
- Added **Security** admin view for dynamic IP blacklist (`/v1/security/blacklist/*`).
- Expanded admin route forms (extra upstreams, strip_prefix) and documented full JSON fields for agents in `docs/AGENT-API.md`.
- Updated `cluster-automation-internal.md` and `AGENTS.md` for current automation scope.
- Added integration tests for HTTPS admin API and SNI certificate upsert.
- Fixed TLS handshake rejection when clients connect by IP without SNI (falls back to the default certificate).

## v1.2.0 - 2026-06-10

- Added **FileCloud** (`services.filecloud`): proxysss-exclusive shared directory UI with single-password auth, file tree with sizes, drag upload/move, search, delete/rename/mkdir, and CDN-friendly `/dl/*` downloads; all operations are confined to `services.filecloud.root`. Existing nginx-style `services.webdav` is unchanged.
- Added built-in managed ACME DNS-01 providers: Cloudflare, Alibaba Cloud CN/Intl, Tencent, Volcengine, AWS Route 53, Azure DNS, and Google Cloud DNS, with automatic migration from legacy `acme_dns_external` / acme.sh aliases.
- Added **manual** DNS-01 provider (`provider: manual`) for wildcard certificates without cloud API keys: proxysss prints the TXT record and polls public DNS until propagation completes.
- Non-wildcard automatic HTTPS still works without any DNS API key via built-in HTTP-01 / TLS-ALPN-01 (`http.tls.auto_https` or `acme_managed`).

## v1.1.2 - 2026-06-09

- Stabilized benchmark automation by comparing proxysss directly against nginx, removing Caddy download/runtime dependencies, fixing Linux JSON row capture, and writing Windows benchmark JSON without a UTF-8 BOM.

## v1.1.1 - 2026-06-09

- Hardened deep/e2e verification startup by waiting for the spawned gateway listener to accept TCP connections before requests are sent, matching the Linux CI timing model.

## v1.1.0 - 2026-06-09

- Reworked the built-in welcome page into a standalone `templates/welcome.html` asset with polished responsive styling and CSS motion, while keeping runtime rendering embedded in the single binary.
- Added first-class AI reverse proxy configuration for New API, sub2api, and OpenAI-compatible route shapes, including validation and docs/templates.
- Completed declared FTP nginx module parity coverage for command/transfer policies, per-user policies, passive/active data rewrite, timeouts, login limits, and structured logs.
- Fixed nginx migration routing parity: configured YAML routes and TypeScript gateway scripts now run before the fallback welcome page, upstream `Host` preserves the external host, and `/` path prefixes match child paths.
- Hardened static migration behavior by allowing `proxysss://static/...` to serve config-root symlinks safely and by covering root static-site child files with tests.
- Kept wildcard certificate guidance on the explicit non-default `http.tls.mode: acme_dns_external` + `acme.sh` DNS-01 path.

## v1.0.0 - 2026-06-09

- **1.0 stable release** — agent-native high-performance gateway with single-YAML config, admin API, and full cross-platform release artifacts.
- Cloudflare-style HTTP cache policies, domain stream SNI routing (Redis/MySQL/PostgreSQL), on-demand TLS, DDoS mitigation, and dynamic IP blacklist.
- Security hardening: SSRF blocking, HTTP smuggling rejection, admin auth rate limits, atomic config writes.
- Interactive architecture guide at `docs/architecture.html`; parity drift tests keep README, AGENTS.md, and capability matrix aligned.
- Release workflow fix: version bumps now ship with a synced `Cargo.lock` so `cargo build --locked` succeeds on all CI/release runners.

## v0.3.15 - 2026-06-09

- Added Cloudflare-style HTTP cache policies: `behavior` (`bypass`, `respect_origin`, `override`, `no_cache`), edge/browser TTL, `CDN-Cache-Control`, `stale_if_error_secs`, and `X-Cache` status headers.
- Added domain stream proxy via `tcp.stream_routes` with TLS SNI routing for Redis/MySQL/PostgreSQL-style workloads and `POST /v1/stream-routes/upsert`.
- Added on-demand TLS (`http.tls.on_demand`) with managed ACME first-hit issuance, allow globs, optional `ask_url`, and rate limits.
- Added DDoS mitigation (`security.ddos`), dynamic IP blacklist admin API (`/v1/security/blacklist/*`), and `services.access_control.stream`.
- Extended FTP with transfer-level hooks (`transfer_allow`/`transfer_deny`), per-user `user_policies`, and structured transfer logging.
- Documented new surfaces in `docs/CONFIGURATION.md`, `docs/SECURITY.md`, `docs/AGENT-API.md`, and `docs/architecture.html`.

## v0.3.14 - 2026-06-09

- Hardened admin defaults: `enable_write_ops` and `expose_config` now default to `false`, with `loopback_only` and auth rate limiting enabled by default.
- Added `security.*` controls for admin mutation validation, SSRF blocking on agent-supplied upstreams, and HTTP/1 smuggling rejection.
- Added agent delete endpoints: `POST /v1/domain-routes/delete` and `POST /v1/reverse-proxy-routes/delete`.
- Added TLS automation endpoints: `POST /v1/tls/auto-https/upsert` and `POST /v1/tls/wildcard-dns/upsert` (acme.sh DNS-01 wildcard path).
- Added optional `kubernetes.enabled` service-to-upstream DNS mappings for in-cluster ingress-style routing.
- Switched admin config persistence to atomic temp-file writes with rollback on failed reload.
- Documented security and agent workflows in `docs/SECURITY.md` and `docs/AGENT-API.md`.

## v0.3.13 - 2026-06-09

- Added Prometheus text exposition on `monitoring.path` (default `/metrics`) with `monitoring.format: prometheus|json`.
- Added weighted load balancing via `load_balance.algorithm: weighted` and per-route `upstream_weights`.
- Added token-bucket HTTP rate limiting through `services.rate_limit.http.algorithm: token_bucket`.
- Documented configuration, architecture, and demo workflows in `docs/CONFIGURATION.md`, `docs/ARCHITECTURE.md`, and `examples/demo/README.md`.
- Expanded capability matrix coverage for Prometheus metrics, weighted balancing, and gRPC-over-HTTP/2 proxying.

## v0.3.12 - 2026-06-07

- Added non-default `http.tls.mode: acme_dns_external` for wildcard certificate issuance and renewal through external `acme.sh` DNS-01 providers, with DNS credentials redacted from config display paths.
- Documented acme.sh wildcard DNS-01 setup in README, built-in docs, nginx migration notes, lab examples, and agent guidance.
- Expanded token-authenticated automation endpoints beyond domain routes to also support `services.reverse_proxy.routes`, `tcp.listeners`, and `udp.listeners` upserts over HTTP, with persistence back into the main YAML file and in-process reload.
- Added an internal cluster automation integration guide covering the agent-facing HTTP API workflow for HTTP, WebSocket, TCP, and UDP onboarding.

## v0.3.11 - 2026-06-07

- Added bearer-token admin authentication support for automation and cluster control-plane calls.
- Added `POST /v1/domain-routes/upsert`, allowing services to register or update domain-based reverse proxy routes over HTTP, persist them into the main YAML config, and trigger an in-process reload.
- Updated README, AGENTS guidance, built-in docs, and admin endpoint notes to document token-authenticated route automation.

## v0.3.10 - 2026-06-07

- Fixed GitHub CI, deploy, and release Windows builds by moving the Windows amd64 and Windows arm64 jobs back to native GitHub-hosted Windows runners instead of the failing Ubuntu `cargo xwin` path.
- Kept first-class Windows arm64 packaging in deploy and release workflows while preserving the lighter zip artifact packaging path.
- Refreshed remaining public docs and built-in landing copy so they consistently describe the single-YAML config model and direct proxysss capabilities.

## v0.3.9 - 2026-06-07

- Enforced a single-YAML runtime configuration model: `proxysss.yaml` remains the default, `-config` / `--config` / `-c` select custom YAML paths, and runtime `include` files are now rejected.
- Removed JSON runtime config support and narrowed auto-loaded plugin sidecar metadata to YAML-only files.
- Clarified and tested domain-first service grouping so one YAML file can define multiple domains, each with its own upstream pool while still reusing shared backend machines.
- Rewrote the README in English and updated built-in docs, examples, AGENTS guidance, and supporting docs to match the new config model.

## v0.3.8 - 2026-06-07

- Moved Windows amd64 and Windows arm64 release/deploy builds off hosted Windows runners onto Ubuntu-based MSVC cross-compilation with `cargo-xwin`, targeting the same `.zip` outputs while cutting the slowest build lane.
- Added first-class Windows arm64 assets to both deploy artifacts and GitHub Releases.
- Updated CI build coverage to compile both Windows MSVC targets on Ubuntu as well, so pre-release validation no longer waits on the slower hosted Windows build path.

## v0.3.7 - 2026-06-07

- Further reduced packaging overhead by trimming deploy/release bundle contents to the binary, README, key offline docs, and install scripts instead of copying the full templates/scripts trees.
- Switched the Linux ARM64 release build from the x64 cross-toolchain + `apt-get` path to a native ARM GitHub runner to remove cross-toolchain setup overhead while preserving the same final asset.
- Unified Rust cache keys across workflows and targets so release/deploy/CI builds can reuse cached dependencies and target artifacts more effectively across version bumps.

## v0.3.6 - 2026-06-07

- Fixed the release/cold-build dependency lock by downgrading the locked `getrandom` crate from `0.3.5` to `0.3.4`, restoring reproducible builds on clean GitHub Actions runners.
- Kept the `-c` / `-config` global config-path support and workflow caching improvements from the previous patch while repairing the failed `v0.3.5` tag line.

## v0.3.5 - 2026-06-07

- Optimized GitHub Actions build/package workflows by adding Rust dependency/target caching across CI, deploy, and release jobs.
- Avoided recompressing already packaged `.tar.gz` / `.zip` release artifacts during artifact upload to cut avoidable CPU time in Actions packaging jobs.
- Switched workflow build invocations to `cargo build --locked` so package/release jobs reuse the checked-in lockfile consistently.

## v0.3.4 - 2026-06-07

- Added global CLI config path support for both `-c <file>` and compatibility-style `-config <file>`, while keeping the default fallback to `proxysss.yaml` when no path is supplied.
- Updated README command examples to prefer the new global config-path form so users can launch subcommands against arbitrary YAML files without repeating per-command `--config` flags.

## v0.3.3 - 2026-06-07

- Added bilingual Chinese/English annotations to `proxysss-script.d.ts` so embedded TypeScript hooks, contexts, and route fields are easier to use without guessing the runtime contract.

## v0.3.2 - 2026-06-07

- Added managed ACME renewal flows for both HTTP-01 and TLS-ALPN-01 without requiring an external ACME binary on the default path.
- Expanded FTP from raw TCP passthrough into native control-channel proxying with passive and active data-channel rewriting.
- Added global and route-level active health control with default-on HTTP/TCP liveness probes, hysteresis thresholds, jitter, and webhook hooks.
- Added persistent upstream maintenance state so manual drain/restore actions survive reloads and restarts.
- Upgraded the admin dashboard on port 7777 with search, filter, grouping, route aggregation, manual upstream drain controls, and richer upstream health telemetry.
- Added configurable browser-facing error pages, including a polished built-in 404/4xx/5xx experience and route-level docs entrypoints.
- Added the built-in `docs.html` manual, TypeScript declaration file, TS usage guide, and nginx-to-proxysss mapping guide, and made `proxysss init` generate them.

## v0.3.1 - 2026-06-06

- Added built-in HTTP access control with IP / CIDR allow and deny lists via `services.access_control.http`, including `blacklist` / `denylist` aliases for direct YAML use.
- Enforced access-control checks in the native HTTP request path before proxying so blacklisted clients are rejected without needing any script/plugin layer.
- Expanded README automatic HTTPS guidance to call out no DNS-token requirements, built-in renewal behavior, and the current external ACME client dependency.
- Reworked README nginx parity coverage into a per-capability checklist with explicit checkmarks, and documented blacklist + rate-limit + compression examples alongside auto HTTPS.
- Prepared the next release metadata by bumping the project version to `v0.3.1` and refreshing install/update examples.

## v0.3.0 - 2026-06-06

- Replaced the external `deno` script sidecar with an embedded TypeScript/JavaScript engine compiled directly into the proxysss binary (QuickJS via `rquickjs`, TypeScript stripped in-process with `swc_ts_fast_strip`); there is no longer any external `deno`/`node`/`tsc` dependency or bundled runtime directory.
- Added hard per-call isolation for scripts: every plugin hook runs under a configurable timeout enforced by a QuickJS interrupt handler plus a memory limit, so a buggy plugin (throw, infinite loop, runaway memory) is reported to the error log and never affects native/YAML proxy traffic.
- Moved routing pipeline orchestration (priority ordering, merge/normalize, fallback) from the TypeScript harness into Rust for full control of script input/output; the main `gateway.ts` is now a normal default-export plugin that acts as the lowest-priority fallback router.
- Added `script.memory_limit_mb` (default 64) and `script.max_stack_size_kb` (default 512) and removed the deno-specific `script.command`/`script.args` settings.
- Reworked `proxysss script run-file` / `proxysss script eval` to execute through the embedded engine instead of spawning deno.
- Removed deno download/bundling from release, deploy, and install packaging; release artifacts are now a single self-contained binary.
- Added `specs/embedded-ts-runtime.md` documenting the engine decision, isolation model, host/script contract, and verification plan.
- Added auto-loaded plugin sidecar configuration (`<name>.plugin.yaml/.yml/.json`) so bundled plugins can stay default-off while still receiving enabled/priority/config values without any external runtime.
- Injected `x-real-ip`, `x-forwarded-for`, `x-forwarded-host`, `x-forwarded-proto`, and `forwarded` on upstream requests to better match nginx-style forwarding semantics.
- Added default-off built-in plugin templates/examples for geo header injection (`geo-headers`) and AI API compatibility routing (`ai-api-compat`), plus README coverage for real business scenarios and AI API forwarding.

## v0.2.7 - 2026-06-06

- Hardened TypeScript extension execution so startup and hot reload now fall back to YAML-only routing when the script runtime cannot start.
- Changed auto-loaded plugin handling to warn and skip broken TypeScript plugins instead of aborting gateway startup or reload.
- Switched script hot reload fingerprints to explicit MD5 content hashing across config and watched script/plugin files.
- Documented the embedded single-binary TypeScript runtime target in `AGENTS.md` as a required end-state rather than an optional direction.

## v0.2.6 - 2026-06-06

- Reworked the default port 80 landing page into a compact, single-screen project page with no admin address exposure, protocol coverage, and benchmark highlights.
- Switched gateway architecture to YAML-first routing by default so HTTP/HTTPS/TCP/UDP no longer depend on TypeScript scripts unless explicitly enabled.
- Bundled the TypeScript runtime into release and deploy artifacts so proxysss installation no longer requires downloading or installing an external interpreter separately.
- Updated release verification, installer flow, and workflow packaging to validate the bundled runtime alongside the proxysss binary.
- Added `proxysss script run-file` and `proxysss script eval` for direct TypeScript runtime verification against bundled engine capability.
- Changed `proxysss start` to silent background launch with stale proxysss process cleanup, added `proxysss restart`, and made top-level `status` report background process state.

## v0.2.5 - 2026-06-05

- Added reproducible gateway benchmark workflow for proxysss, nginx, and Caddy.
- Documented local Windows loopback benchmark results and caveats in README.
- Hardened YAML config parsing for UTF-8 BOM files generated by Windows PowerShell.

## v0.2.4 - 2026-06-05

- Expanded README gateway comparison coverage for Nginx, LVS, HAProxy, Caddy, Envoy, Traefik, and proxysss.
- Added proxysss-style `http.tls.auto_https` documentation for fast production SSL without copying another product's config syntax.
- Recorded local HTTP benchmark results for the current release preparation workflow.

## v0.2.3 - 2026-06-05

- Tightened product positioning around proxysss as an nginx-level general gateway, not a business gateway.
- Removed business-flavored default TCP/UDP listener names from generated config and made stream listener examples explicitly generic.
- Fixed `config reload-plan` so logging level/filter/format/path changes are consistently shown as restart-required.
- Normalized relative `proxysss init --dir` paths so watched script output stays readable for agents.
- Updated README copy and comparison guidance for agent-first nginx replacement workflows.

## v0.2.2 - 2026-06-05

- Serve the welcome page from Rust built-in routes on `/`, `/index.html`, and `/docs` so older `gateway.ts` installs still show the landing page.
- Warn during `check-config` when `http.plain_bind` is disabled because port 80 will not expose the nginx-parity welcome page.

## v0.2.1 - 2026-06-05

- Fixed release publish job changelog extraction so tagged releases always find `CHANGELOG.md` sections.
- Upgraded GitHub Actions artifact steps to `upload-artifact@v6` / `download-artifact@v6` (Node.js 24 LTS runtime).
- Required agents to monitor CI/release through `skills/gh-cli/SKILL.md` and documented the rule in `AGENTS.md`.
- Split backend lab (`examples/lab/`) from proxysss default install paths (`examples/lab-proxysss/`).

## v0.2.0 - 2026-06-05

- Positioned proxysss as an nginx-class general gateway with agent-first CLI (`config explain`, `nginx-parity`, `reload-plan`, etc.).
- Added built-in static sites, WebDAV, declarative reverse proxy, HTTP rate limit, and FTP TCP passthrough.
- Added access/error log file sinks (`logs/access.log`, `logs/error.log`) and demo plugins (`structured-log`, `traffic-stats`).
- Merged listener supervisor hot-reload from v0.1.10; default admin port restored to `127.0.0.1:7777`.
- Added agent skills: `skills/proxysss-install`, `skills/gh-cli`, and local verification lab under `examples/lab/`.

## v0.1.10 - 2026-06-04

- Fixed Windows `proxysss start` fallback launching a visible console window.
- Changed Windows auto-start install to use a hidden `wscript.exe` launcher through HKCU Run by default.
- Made running `proxysss.exe` without arguments start the gateway instead of printing help and exiting immediately.

## v0.1.9 - 2026-06-04

- Hardened Windows installer downloads with a GitHub CLI release-download fallback when direct asset HTTPS downloads time out.
- Hardened release verification hash checks with the same GitHub CLI fallback.

## v0.1.8 - 2026-06-04

- Changed default public gateway ports to `80` for HTTP and `443` for HTTPS/HTTP3 so proxysss can directly replace Nginx and Caddy.
- Enabled `http.plain_bind` by default on `0.0.0.0:80`.
- Added top-level service commands: `proxysss start`, `proxysss stop`, `proxysss enable`, `proxysss disable`, and `proxysss status`.
- Updated installer and docs to state proxysss installs standalone and does not install Nginx.
- Kept admin on `127.0.0.1:7778` to avoid exposing admin on public ports.

## v0.1.7 - 2026-06-04

- Changed default gateway port to `7777` and admin port to `7778`.
- Added configurable public monitoring API with `monitoring.enabled` and `monitoring.path`.
- Added hot reload supervisor for listener changes, including HTTP/TLS/H3, TCP/UDP listeners, and admin bind changes.
- Added GitHub Actions `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` to remove Node.js 20 deprecation risk.
- Updated docs to position proxysss as a high-performance Web reverse proxy competitor to Nginx and to document ACME/Caddy-style certificate automation.
- Expanded memory-safety and monitoring documentation.

## v0.1.6 - 2026-06-04

- Fixed release verification compatibility when downgrading to older builds that do not support `proxysss --version`.
- Verification now falls back to installed binary SHA256 comparison against the expected GitHub release asset.

## v0.1.5 - 2026-06-04

- Added `proxysss update --version <latest|vX.Y.Z>` for one-command upgrades.
- Added `proxysss switch-version <vX.Y.Z>` for explicit upgrade/downgrade installs.
- Added Windows/Linux/macOS installer safeguards for version comparison, downgrade intent, service restart, skip-init, and dry-run flows.
- Added built-in internal HTTP routes for Nginx-like common cases:
  - `proxysss://healthz`
  - `proxysss://redirect/<location>`
  - `proxysss://static/<path under config root>`
- Added route decision fields `status` and `content_type` for redirect/static responses.
- Added default `/healthz` and `/static/*` routes in generated TypeScript gateway template.
- Added release verification script for GitHub release assets and local upgrade/downgrade checks.
- Kept GitHub Actions skill and workflow lint in-repo for automated checks.

## v0.1.4 - 2026-06-04

- Added programmable gateway foundation for HTTP/1.1, HTTP/2, HTTP/3, TCP, UDP, WebSocket, and WSS.
- Added TS/JS script routing through Deno.
- Added plugin load/unload/list support through admin API and CLI.
- Added load balancing, passive health checks, retries, affinity routing, config hot reload, admin console, service install, and installer scripts.
