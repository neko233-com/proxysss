# proxysss 架构

proxysss 用单个 Rust 二进制承担通用边缘网关职责：协议终止、路由、静态文件、反向代理、流转发、安全策略、日志与管理。可选 TypeScript/JavaScript 扩展负责业务决策；普通网关请求无需脚本。面向用户的交互入口是 [架构 HTML](architecture.html)。

## 模块职责

| 模块 | 所有权与职责 |
| --- | --- |
| `src/main.rs` | CLI、命令运行时与能力检查 |
| `src/config.rs`、`src/config/performance.rs` | 单 YAML 配置、默认值、校验、重载指纹和流量 profile |
| `src/gateway.rs` | 监听器、HTTP 分发、上游池、缓存、健康状态、TCP/UDP 与管理 API |
| `src/static_security.rs` | 静态源站令牌、真实路径边界、签名 URL 和目录规则 |
| `src/static_hosts.rs` | 无分配的静态域名模式匹配、端口/IPv6 处理与字段校验 |
| `src/filesystem_boundary.rs` | WebDAV/FileCloud 已有目标与新建目标父目录的真实路径边界检查 |
| `src/file_download.rs` | 共享文件条件请求；WebDAV/FileCloud 的有界完整/Range 下载 |
| `src/file_io.rs` | 文件版本核对与严格长度读取；防止替换混版、增长越界和截断被当作成功 |
| `src/static_ranges.rs` | Range 解析、重叠合并、单段读取与多段流式下载 |
| `src/http1_body.rs` | 原始 SSE 转换为 Hyper 响应前的有界 HTTP/1 正文解码 |
| `src/stream_reactor.rs`、`src/sendfile_reactor.rs` | Linux 明文实时流 epoll relay、大文件 sendfile 作业与完成通知 |
| `src/runtime_tuning.rs`、`src/linux_cpu.rs`、`src/linux_tune/` | 系统能力、启动 CPU affinity/quota、运行时/socket 适配与显式主机调优 |
| `src/script.rs`、`src/ts_transpile.rs` | 进程内 QuickJS、TypeScript 类型剥离、扩展调用与热重载 |
| `src/security.rs`、`src/acme/`、`src/filecloud/` | 访问/限流/管理鉴权、证书与 DNS provider、文件服务 |
| `src/install.rs` | 安装、升级、后台服务与平台集成 |

普通转发使用连接私有状态、分片表、原子计数器和有界池，配置通过 ArcSwap 发布。共享 Mutex/RwLock 只用于控制面、证书操作和首次缓存填充协调，不放在普通 HTTP/TCP/UDP 转发路径上。

性能验证的数据流独立于网关：HTTP/1 发生器每个 worker 持有独立 client，每条连接预先完整读取两次真实资源响应；HTTP/2 在共享多路复用连接上预热两次。下载逐块消费和计数，检查响应状态及夹具长度，不聚合整个大文件。有限 SSE 夹具允许 keep-alive，发生器用 `--drain-response` 读到 HTTP 正文结束后复用各自连接，仍报告首块延迟；HTTP/SSE 测量截止取消未完成响应，迟到完成不计入成功。共同绝对开始时间不可由迟到进程自行后移，双方任一轮未完成等负载目标都会失败；CDN 发布预检位于测量前。每波记录 taskset 后的真实 CPU affinity、worker 环境值与发生器报告值，汇总前核对，并独立保留各轮记录。完整边界与诊断方法见 [验证流程](VALIDATION.md)。

Go 报告器聚合原始重复样本时，吞吐/延迟使用中位数，错误保留最大值，完成率保留最差单轮及序号；样本数和固定负载参数必须一致，门槛不能从中位吞吐推算逐轮完成率。该流程由本地 Docker 功能入口中的 Go 回归保护，具体字段见验证流程。

## HTTP 请求路径

配置边界：`reverse_proxy.routes` 用 `hosts`，`domain_routes` 用 `domains`，两者每条路由都直接指向 `upstream`。后者不是引用其他服务的嵌套路由编排器。`static_sites` 同时匹配可选 `hosts` 与路径；单个 `webdav` 按路径匹配，可通过 `auth.enabled/username/password` 开启内建 Basic 认证，认证先于路径检查和任何文件操作。具体可执行片段与边界见 [配置指南](CONFIGURATION.md)，未知字段在配置解码阶段拒绝，不能用未实现字段声明能力。

1. 公共 HTTP 默认监听 80，TLS/H2/H3 默认使用 443。accept 与握手由监听运行时负责，H2/H3 分别管理多路复用流。
2. 判断协议与配置路由，处理内建服务、ACME challenge、管理 HTTPS/monitor。用户公共路由优先于欢迎页兜底；默认 `/` 仅显示 `Welcome to proxysss`、GitHub 与 GitHub Docs 链接。
3. 执行配置的访问控制、限流和连接上限。静态源站额外检查真实 TCP 对端、令牌/签名及路径边界；HEAD、Range、缓存和条件响应不能绕过授权。
4. 分发静态、WebDAV、域名/反代/AI 路由或脚本。静态条件按 If-Match / If-Unmodified-Since、If-None-Match / If-Modified-Since、If-Range 的优先级执行，再选择完整或部分内容。
5. 代理路径查询缓存和内存上游池，结合负载均衡、主动健康、被动隔离和重试选点。Consul/etcd/Nacos 发现结果通过控制面更新上游池，普通请求不逐次查询注册中心。
6. 发送正文；WebSocket upgrade 后保持双向转发，SSE/AI 响应保留背压，gRPC 使用 HTTP/2。动态压缩受类型/大小限制，206 不进行整对象压缩。
7. 更新分片指标，按配置记录访问/错误日志；流式正文发送后遇到文件截断则终止传输，不补成成功短正文。

快路径只在语义等价时启用。raw reverse/SSE 保留默认转发头并复用请求/响应缓冲与上游 lane。条件请求和 Range 进入完整静态处理。启用 admin 或热重载时，HTTP/AI/WS 反代与静态响应都经过当前策略链，不进入绕过准入的免策略快路径；黑名单磁盘持久化关闭不代表内存封禁关闭。HTTP keep-alive 的后续请求和新 WS 握手受更新后的封禁约束，已建立 WS/TCP relay 沿用既有连接生命周期。不可变无策略 H2 使用连接 OnceLock 和只读 ArcSwap/FxHashMap 响应快照。

原始 SSE 转换为 TLS/HTTP2 下游响应前先按 HTTP/1 消息边界解码：chunked 去除大小行、扩展与终止分隔符，消费并丢弃 trailer；Content-Length 精确计数，无长度响应才以 EOF 结束。大小/元数据行最多 16 KiB，trailer 总量最多 64 KiB；长度冲突、未支持的 transfer coding、分块错误或正文提前 EOF 会终止响应。每次仍按下游需求借用池缓冲并返回 Bytes 切片，不积累完整 chunk 或 SSE 事件，完成或取消释放上游。SSE 不再压缩，但保留上游已有 Content-Encoding；移除编码头而保留压缩字节会破坏正文。

脚本保护的静态站点用 `script_only: true` 排除原生选站及预热/HTTP1/TLS/H2 快缓存；脚本显式选择
`proxysss://static-site/<name>` 后，核心继续执行 Host/path、真实 peer 和文件授权。可选 `spa_fallback`
只在合法目标不存在时回退根内单个 index 文件，保留原有目录、链接、HEAD/Range 安全边界。详见配置指南。

## CDN 和复杂下载

WebDAV 与 FileCloud 使用共享文件系统边界检查：已有目标解析真实路径，新建目标解析最近存在的父目录后追加缺失部分；使用 symlink_metadata 识别悬空链接，不能用 exists 跳过它们。WebDAV 在阻塞任务池检查源路径、COPY/MOVE 目标及递归 COPY 的每个子项；越界、悬空链接、复制到自身/子目录或内部循环返回 403。COPY 以深度优先目录迭代器遍历，只保存当前祖先，允许根目录内部的有效链接；删除/移动仍使用原始路径，避免误删链接指向的文件。PROPFIND 和 FileCloud 列表/搜索隐藏不满足边界的子项。递归 COPY 不是事务，遇到后续错误时根目录内已完成的部分可能保留。

这些检查要求配置根目录及其链接由可信运维管理，不是基于目录描述符的原子沙箱；检查与文件打开之间仍有时间窗口，不能据此允许不可信本地进程并发改写链接。文件服务的路径边界检查不进入普通 HTTP 反代、静态缓存或 TCP/UDP 转发路径。

WebDAV/FileCloud 下载共用静态服务的条件请求规则与 Range 解析、缓冲池。完整小文件最多预读 64 KiB，大文件与多段响应按消费进度读取，每次最多 64 KiB，空闲池总计最多 8 MiB；取消或截断释放描述符与缓冲。HEAD 不读取正文且忽略 Range。FileCloud 只对 GET/HEAD 提供下载，认证下载返回 private, no-store；公开下载按 cdn_cache_secs 缓存，不给可覆盖文件自动添加 immutable。下载路径只解码一次，API/JSON 中的路径保留字面 + 与 %，中文文件名通过 filename* 编码。

WebDAV PROPFIND 支持 Depth 0/1，以流式 XML 返回目录，不积累全部条目；目录的默认/infinity 深度返回 403 propfind-finite-depth。COPY 支持 Depth 0/infinity，MOVE 要求 infinity，Overwrite F 遇到已有目标返回 412；覆盖替换整个目标集合，创建返回 201、覆盖返回 204。缺少父集合返回 409，MKCOL 的非空请求体返回 415；跨服务器 Destination 返回 502。方法检查不等于完整 WebDAV 扩展支持，锁与自定义属性能力必须按实际实现声明。

静态站点先按 YAML 顺序同时匹配 hosts/path，再进入授权与文件处理。多站点或配置 hosts 时，预热路由键由配置内站点序号和路径组成，H2 快照与连接缓存使用同一个键；相同路径在不同根目录不互相覆盖。单个无 hosts 站点继续借用路径作为键。热重载清空路由/H2 映射并重新预热，可变配置不使用连接级免策略缓存。文件正文仍按真实文件路径共享有界快照，站点授权、缓存响应头和条件处理独立执行。

完整小对象使用有界缓存和预构建 HTTP/1 keep-alive 响应，大文件流式发送；Linux 合适的明文完整响应使用 sendfile。Range 按偏移读取所需字节，不读取整个原文件。HEAD 忽略 Range，只返回完整表示元数据。

正文缓存持有自有不可变 Bytes 快照，不直接 mmap 可变源文件；每次填充最多 32 MiB，读取前后核对文件版本，源文件随后截断不会让 TLS/压缩访问失效映射。版本包含长度、修改/创建时间；Unix 还包含设备、inode 和纳秒 ctime，保留长度和 mtime 的原子替换仍会更新缓存与弱 ETag。H2 的文件重验证在控制运行时执行；文件删除、越界或重验证失败会使连接快照失效并退回正常分发，避免永久发送已撤下资源。

完整流式与 Range 下载在打开文件后核对其版本，发现元数据与所开文件不一致时失败，不拼接不同发布版本。完整流式正文和非 Linux 明文文件复制都严格限制到声明长度：文件增长不会发送多余字节，提前 EOF 是正文错误；HTTP/3 因而重置失败流而非发送成功 FIN。背压和取消仍由读取方控制，不增加预读生产任务。

Linux sendfile 池每个版本保留一个锚点描述符。预热和池耗尽时通过 /proc/self/fd 重新打开该锚点，以取得同一 inode 的独立文件描述符；路径被替换或删除后仍发送原版本。procfs 不可用时复制锚点并继续使用显式发送偏移，保持正确性。空闲池仍有固定容量，多余归还立即关闭；额外锚点随旧缓存及最后一个响应释放。这不是不可变文件系统快照，就地覆盖仍需运维避免。

明文 HTTP/1 连接缓存每次命中都检查单调时间上的 2 秒 freshness，不按请求次数推迟检查。低频复用同一连接仍会触发共享后台重验证，不能因尚未积累 256 次访问而长期保留旧版本；过期不等于每次同步 stat，文件刷新仍由现有控制运行时合并执行。

正文缓存使用原子预留的 256 MiB 上限；进程内全部静态快照（含正在填充、已淘汰但响应仍引用的正文）另受 320 MiB 硬上限约束。最后一个 Bytes 引用释放时归还预算，预算耗尽的冷下载立即回退流式读取，预热跳过该对象。缓存淘汰只扣减一次，填充协调锁随成功、失败或取消清理；缓存命中不重新争用预算。此上限不包括 socket、TLS、压缩输出与其他网关内存。

Range 最多接受 16 个区间，合并重复/相邻/重叠区间，保留未合并部分的请求顺序。单段返回 206 和 Content-Range，多段使用 multipart/byteranges、分段 Content-Range 和精确总 Content-Length。有效集合的越界部分被跳过，全部不可满足返回 416；格式错误、未知单位或超限集合忽略 Range。

多段响应只持有一个文件描述符，HTTP 消费正文才触发读取。大范围每次最多借用 64 KiB 池缓冲，Bytes 持有所有权直到下游释放，独立空闲池最多 8 MiB；取消释放文件与缓冲，无后台生产线程。弱元数据 ETag 不满足 If-Match 强比较，存在文件可用 `*`；If-Range 只接受精确匹配且稳定 60 秒的修改日期，否则退回完整 200。部署资源宜写新文件后原子替换，避免就地修改正在发送的内容。详细说明见 [CDN 下载](CDN-ORIGIN.md)。

## 游戏与 socket 长连接

管理 API 的 TCP/UDP listener 与 stream route 使用相同目标策略：主机名去除端口与末尾点后比较规则，数字地址检查 IP/CIDR，校验先于落盘和重载。该控制面检查不做 DNS 解析，不等于 DNS 重绑定防护，也不限制本地可信配置与脚本的内网目标。

`src/stream_endpoint.rs` 统一流目标语法，配置加载/管理更新与脚本返回路由时校验主目标、候选和权重键，不在普通转发中重复检查。UDP 域名仅在新 association 建立时进入有界系统 DNS 解析，数字 IP 保持直接路径；最多 64 个在途解析、5 秒等待，超时后许可仍由实际系统解析任务持有，防止遗留任务无界增长。取首个解析地址并按其地址族绑定，已有会话继续复用 socket。健康探测共用地址处理，单个期限覆盖 DNS、发送与回复；详细输入格式与 DNS 生效边界见 [配置指南](CONFIGURATION.md)。

TCP 接受连接后读取当前 listener 配置，再执行流访问/限流和路由，选择内存上游池；没有脚本、亲和、健康和额外候选时走单上游直接路径。nodelay 默认 true，同时作用于接入与上游 socket；显式 false 保留 Nagle 合并，自动系统调优不覆盖该选择。同名同 bind listener 的 nodelay、connect_timeout_ms、protocol 更新应用于随后建立的连接，已建立会话保留原选项与上游。连接阶段受 connect_timeout_ms 限制。半关闭先冲刷未发送字节，再传播写关闭，保留反向数据。

UDP 按客户端建立上游 association。分片 pending 集合去重后，listener 私有原子计数器在首包复制、任务创建、脚本和 DNS 前预留容量；max_associations 覆盖在途创建及尚未释放的会话。许可随成功的 association 转移，失败、取消和最后一个所有者退出时自动释放；普通数据报不操作容量计数器。满额拒绝新会话，不按容量压力淘汰活跃玩家，过期扫描最多每秒一次。

TCP 新连接与 UDP 新 association 统一经过 stream 连接限流、动态黑名单、DDoS 和 stream allow/deny。UDP 在 pending 去重后、容量预留及脚本/DNS 前执行准入检查；直接数字目标和扩展路由没有绕过入口。两种协议按客户端 IP 共用 stream zone，既有 UDP 数据报只转发，不重复消耗连接配额或执行准入策略。DDoS 辅助方法只记录封禁，拒绝由调用入口统一计数一次。配置与生效边界见 [配置指南](CONFIGURATION.md)。

worker 本地缓存使用 Weak 引用，转发期间才取得 Arc，TTL 最多每秒刷新一次，数据报使用有界缓冲池。退休标记广播唤醒等待回复或可写事件的任务；reader 的释放守卫负责移除自身版本、归还缓冲与上游统计。listener 退出先禁止新发布，停止时同时取消独立运行时中的接收 worker 和 JoinSet 管理的在途创建，清空并退休已有会话；发布前后均检查关闭状态，避免停机期间重新插入孤儿会话。

同名同 bind 的容量、TTL 和 protocol 更新由 worker 最多一秒的配置快照供新会话使用；既有会话保留其创建时的 TTL 和上游。降低容量不主动断开旧会话，旧会话仍占名额，直到总量低于新值才允许新建。0 明确表示不设容量上限。

UDP 清理在同一分片操作内核对 association 的 Arc 身份，旧 reader 和发送失败不能误删同地址的新会话。替换会话会停用旧实例；reader 检查自身活跃状态和 TTL，观察到停用后退出并停止转发迟到回复，不借用新实例的活跃时间维持旧 socket。本地缓存判定过期前读取共享刷新时间，避免其他 worker 的活跃会话被旧缓存关闭。TTL 由客户端数据报刷新，后端持续回复不能推迟固定检查期限；持续就绪的回复每批最多追加读取 64 包后让出，保证取消与过期检查获得执行机会。

MQTT TCP、TLS SNI 透传、MQTT WebSocket、CoAP、游戏语音、KCP 风格与透明 QCP 都复用这些转发路径。proxysss 不实现 MQTT broker，QCP 透明转发不代表帧终止。FTP 控制与主动/被动数据连接执行各自策略，启用时默认控制端口为 21。

## Linux 调度

进程入口在数据线程绑核前记录主线程的启动 CPU affinity；内核返回值已经受 cpuset 限制，也保留 taskset 的更窄范围，不能用外层 cgroup CPU 列表扩大它。数据核预算取此集合与 Rust available_parallelism 的较小值，后者考虑可读取的 cgroup CPU quota，向下取完整 CPU、最少一核。HTTP/TCP/UDP fanout、TLS/UDP runtime、native relay/sendfile owner 和缓存池共同使用这份启动快照；各 profile 仍按其比例分配，因此预算不是整个进程的线程总数上限。没有 quota 时随全部允许核心扩展，不设置小型 worker 上限。

绑核使用动态位图，支持 CPU ID 超过 1023；失败时记录原因并保留继承 mask。已绑单核的 worker 不重新探测全局资源，soft-affinity realtime owner 只能恢复到启动允许集合。资源快照在重启时刷新，不在普通转发或热重载时读 procfs。config performance 的 linux_cpu 分别展示 allowed_cpu_ids 和 data_plane_workers；gateway_runtime_workers=1 指控制运行时。依据：[Linux affinity](https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html)、[Rust available_parallelism](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html)。

| traffic_profile | HTTP/TLS | 实时流与 UDP | 静态文件 |
| --- | --- | --- | --- |
| small（默认） | HTTP/1、TLS/H2 共用按数据核预算扩展 worker 的 Tokio runtime | TCP/WebSocket/UDP 同运行时，可 work stealing | 热小对象缓存；sendfile 稀疏时 8 MiB、超过数据核数时 2 MiB 切片 |
| balanced | 明文 HTTP 每核独立单线程 runtime；TLS/H2 在 nice 5 有界运行时 | 无策略 TCP/明文 WebSocket 按数据核预算建立 nice 3 soft-affinity epoll owner；UDP 用 ceil(cores/2) nice 3 Tokio worker；WSS 留在 TLS runtime | 按活跃传输压力选择 HTTP owner 或预热 sendfile reactor |
| bulk | 优先大流传输 | 显式 bulk TCP 可使用 splice | 优先 sendfile 描述符，可持续使用 reactor |

balanced 实时 owner 仅在真正阻塞后的请求 tick 短探测：1–16 对连接一次、17–32 对两次、超过 32 对依赖 epoll；上游回复和持续就绪流量不额外自旋。small 保留 64-poll relay 公平边界，shared UDP 每 4 包让出。

balanced 大于等于 8 MiB 的对象，仅在活跃传输大于数据核数且不超过每核四条时用 reactor；其他压力区间直接 sendfile。每核不超过两条时选均匀分布的半数预热 owner，更高压力恢复全部 owner。独立文件描述符从有界池借还；EAGAIN 保存进度并形成背压。静态 sendfile 不设置 SO_SNDBUF，保留 Linux 发送缓冲自动调优；手动设置再写回数字不能清除用户锁，会影响同一 keep-alive 连接后续下载。reactor 提前清除 TCP_CORK 后通知 owner，避免重复 syscall。

明文 HTTP I/O event interval 为 8，shared-small 为 16，global queue interval 为 31。H2 每连接用本地 FuturesUnordered 驱动流，每个完成流后让出。静态 freshness 为 2 秒，nice-10 控制运行时重验证并原子发布快照。改变策略前阅读 [性能决策](decisions/linux-performance.md)，验证方法见 [验证流程](VALIDATION.md)。

原生 sendfile 的完成 future 同时拥有取消责任。HTTP 等待任务退出时关闭完成通道，经每 owner 的有界无锁取消队列和 eventfd 唤醒 reactor；队列溢出才扫描已放弃作业，普通完成不扫描。清理前检查完成通道确已关闭，防止旧 fd 通知误取消复用该数字的新下载。未注册作业和连续发送轮也检查取消；已阻塞的作业无需等待客户端再次产生 I/O 即可释放 socket/file 副本。

## 启动、预热和重载

启动加载一个 YAML（默认 proxysss.yaml，支持 -config/--config/-c），先严格解码全部配置对象，再校验语义；未知字段连同配置文件名返回给 CLI/管理调用者，禁用的服务也不能携带未实现字段。请求头、脚本 env、DNS credentials 和 upstream_weights 等开放映射继续接受自定义键，已有显式别名保留。解码失败发生在构建/发布新动态状态之前，重载保留当前路由、缓存与 warm 状态。校验后探测系统/socket 能力，记录启用与跳过原因。Ubuntu 24.x 使用支持的极限参数，旧/未知系统明确降级；持久 sysctl 修改只由显式 `proxysss tune linux --apply` 执行，保留 SSH 安全、备份与失败回滚。

监听前预热 hot index/顶层静态对象、小对象完整响应和 sendfile 描述符；每个去重反代/AI 上游池仅预拨一对连接，数字 IP 在池创建时解析。Tokio socket 归创建 reactor 所有，不把启动 runtime 大批 socket 作为其他 runtime 长期 lane。`/healthz` 暴露 warm 状态，重载重新预热。

默认手动 reload，文件监视需显式启用；主 YAML、主脚本与自动加载插件参与监视。指纹确定 reload/restart 范围，配置通过 ArcSwap 发布。

火山引擎 DNS-01 按挑战创建独立 TXT 并只删除本次记录 ID，允许根域名和通配符的验证值同时存在，不覆盖同名记录。签名日期使用 UTC 日历时间；云 API 返回业务错误时不能当作成功。

内建 ACME 的周期检查先核对证书与私钥、域名及有效期；距离到期超过 30 天的有效证书不会重复下单。缺失、不匹配或进入续期窗口时申请新证书；签发后重载失败则按检查间隔重试重载，不为重载重试重复签发。管理员手动签发仍可显式发起新订单。

周期、CLI 手动和按需 ACME 订单通过进程内锁及 ACME 缓存目录中的跨进程锁串行执行；不同缓存目录不互相协调。任务取消或进程中断时的 DNS 记录恢复仍需单独处理。

托管证书安装先校验证书与私钥，再将新证书对写入受限权限的临时恢复日志，逐文件同步并原子替换；跨进程锁协调证书对安装，重启前或下轮检查重放未完成的安装。恢复日志不保存旧证书备份；损坏时明确报错，不把不匹配的证书对发布。托管输出路径不支持符号链接；不同缓存目录或未共享锁的主机之间不协调签发。

DNS-01 配置可声明额外托管证书，共享账户和 DNS provider，但分别检查和签发各自域名、算法与文件；首次启动可在准备 SNI 前签发缺失项。SNI 映射仍由 certificates 显式配置，不因添加额外证书自动更改路由。

通知属于控制面，多个飞书渠道并发发送；Webhook 和签名密钥按环境变量引用，日志不记录其值。每渠道最多尝试三次，不提供持久化重试。证书签发、网关加载及云端部署是不同状态，通知不能把网关加载当作云端生效。

云证书同步属于控制面，配置按目标选择厂商策略和启用状态，当前策略覆盖火山引擎证书中心与 DCDN。后台每 60 秒检查证书变化，命令行可单次调用；两者使用相同的证书路径跨进程锁，失败重试不会阻塞转发。以配置域名的现有证书绑定发现同项目其他 DCDN 域名；新证书须覆盖全部发现域名，冲突时停止并在写入前回读。API 不提供条件更新，检查与写入之间仍有并发窗口。云端去重导入后检查绑定状态和当前出口的 TLS 叶证书。月度时间由 cron 等外部调度器触发；该检查不证明所有 CDN 边缘节点一致。变更配置不会取消已经发起的云 API 请求。

域名请求计数默认开启，从显式列表和站点、路由、证书等配置中的精确 Host 建立最多 256 项的有界表，显式列表优先；快路径和统一请求入口共用分片原子计数。关闭时不做 Host 查找。热重载保留仍配置的域名计数，移除项后不再暴露。WS/WSS 只计握手，未匹配的 Host 不增加新标签；它不包括 CDN 边缘命中请求。

| 可热更新 | 需重启 |
| --- | --- |
| 路由/上游池、服务发现映射、静态站点、WebDAV、FTP upstream、脚本/插件与相应策略 | HTTP/TLS/H3 bind、admin bind/enabled、TCP/UDP listener 名称与 bind 集合、FTP bind/enabled、TLS mode、日志格式/级别/路径、runtime.performance |

listener supervisor 持有每个监听任务的取消守卫；移除监听、失败重试或 supervisor 退出时不会仅丢弃 JoinHandle 而分离后台任务。HTTP、TLS（含弹性 accept）、TCP、UDP 的跨 runtime 包装任务也持有取消守卫，包含尚未开始轮询即被取消的情况。撤销监听停止新 accept/recv；已建立的 TCP/HTTP 会话继续自行结束，UDP 会话由 listener 生命周期统一退休。进程退出仍由操作系统回收最终资源，不把撤销监听等同于强制中断全部 TCP 下载。

token 认证 API 可持久化允许的路由、listener、TLS、filecloud 和黑名单变更；实际应用边界仍由 reload-plan 决定。explain、capabilities、watched-scripts、routes、reload-plan、nginx-parity 是 agent 检查入口。

## 管理、监控和扩展

动态黑名单持久化使用版本 1 JSON，保留每条 IP 的原始 UTC 秒/纳秒和 TTL；启动严格解析，扣除停机时间后重建单调时间有效期，过期记录跳过。文件缺失可从空状态启动，启用持久化时读取/格式失败则拒绝启动。管理增删在控制面排队，信号量随阻塞 worker 持有到完成；每实例只有一个文件 worker。writer 锁串行化完整快照，唯一临时文件与目标同目录，写完 sync_all、关闭后 rename，成功才发布内存及 DDoS 状态。写入失败返回 500，保留原内存和已提交文件；未启用持久化返回 persisted=false。已交给 worker 的操作在调用方断开后继续完成，取消排队等待则不提交。

普通黑名单检查和自动 DDoS 封禁只访问分片表，不等待上述控制锁。每次成功管理更新保存当时收集到的活动表；快照采集后新增的自动封禁仍在内存，下次管理快照才保存。配置热更新选定随后请求使用的持久化开关/路径，保留当前活动表；外部文件只在进程启动时读取。UTC 时钟用于跨重启换算，运行期间按单调时间到期；重复保存不重新开始 TTL。完整文件可见性依赖底层文件系统提供同目录原子替换；状态目录应使用 Linux 本地磁盘或原生 Docker volume，不能对 Windows 共享 v9fs 作同等承诺。未承诺跨进程同时写同一文件或断电后的目录项事务。

`security_cleanup` 在控制运行时每秒读取当前配置，清理过期的 DDoS 窗口、admin/monitor 登录失败窗口和动态黑名单；它由现有 supervisor 管理，独立于热重载、主动健康开关，无需额外数据线程。清理在各 DashMap 分片内检查并删除，保留仍活动的窗口和封禁；普通黑名单查询只读，避免旧读者删除并发替换的新封禁。DDoS/黑名单按 IpAddr 查表，接入检查不再分配 IP 字符串。封禁记录使用起点与秒数计算剩余时间，外部超大秒数不会溢出未来 Instant；并发登录失败不能清除或延长已生效的锁定。

admin 默认只监听 127.0.0.1:7777，enable_write_ops=false、expose_config=false。token 经 `proxysss token show/set` 检查和轮换，一般配置展示脱敏。loopback 管理面可引导 ACME/TLS，admin.https 在主 TLS listener 提供相同 `/v1/*` API。

admin.monitor 是独立 HTTPS 只读入口，使用专用密码与 monitor:read 短期会话。登录后仅提供 health、summary、stats、SSL 状态，拒绝写接口，不暴露管理 token、私钥、DNS 凭据或配置写入能力。

监控在控制面读取运行时长、已有协议/故障计数与进程资源；SSL 状态按证书文件去重、最多读取每个文件 1 MiB 后解析叶证书有效期，失败返回 null。它反映磁盘文件，不能代替监听器或云边缘实际 TLS 核验，不进入请求转发热路径。

HTTP 请求计数由实际接管请求的处理路径负责。普通 HTTP/TLS/H2/H3、缓存命中、sendfile、原始反代和 SSE 都对同一请求只记录一次；快速路径探测后交还普通处理器时，由后者计数。WS/WSS 握手在响应开始前计数，升级后的消息不增加 HTTP 请求数，不能等长连接关闭才记录握手。请求计数保留分片原子计数器，不引入数据面全局锁。

WebSocket 成功升级后分别记录累计会话和当前活动会话；失败握手不计入会话。异步转发通过生命周期守卫释放活动计数，Linux 原生 reactor 接管时由 reactor 持有该守卫直到连接结束，无需为每个连接增加等待任务。此计数不包含普通 TCP 或升级后的消息数。

HTTP 错误计数只记录已确定响应状态的 4xx/5xx，包括网关生成和上游返回的响应；统一完成器与直接响应路径各自负责一次，异常转为 502 后不能再次累加。没有形成 HTTP 响应的连接失败不计作错误响应，正文中断也不把已经记录的响应再计一次，传输异常通过错误日志排查。JSON 与 Prometheus 使用相同计数源，字段与运维口径见[配置指南](CONFIGURATION.md)。

QuickJS 与 swc_ts_fast_strip 内嵌进程，无外部 Node/Deno/tsc。routeHttp/routeTcp/routeUdp 与插件钩子负责可选业务扩展；核心边缘策略保持原生实现。主动健康覆盖 HTTP/TCP 和显式 UDP 探测，被动失败可隔离上游；watchdog 观察关键后台任务，metrics 暴露重启/失败计数。

管理 HTML/CSS、内置文档和站点资源嵌入二进制。管理 `/docs` 与 `/docs.html` 经过管理传输检查；公开文档导航只链接 HTML。详细配置见 [配置指南](CONFIGURATION.md)，扩展见 [脚本指南](../ts-how-to-use.md)，交付顺序见 [验证与发布](VALIDATION.md)。

## 流式协议与入站正文边界

HTTP/1 原始上游响应使用共享分帧校验：拒绝冲突 Content-Length、TE/CL 共存和不支持的 Transfer-Encoding；跳过有界数量的信息响应，再转发最终响应。chunked 扫描先确认片段边界，再发送有效前缀，不把尾随的另一响应混入正文；固定长度或 chunked 提前 EOF 作为失败关闭，异常连接不能回池。

HTTP 上游等待采用连接/响应头期限与逐帧正文空闲期限，持续 SSE、下载不受总时长限制，下游背压不消耗上游空闲预算。WebSocket 非 101 响应仍完整转发其固定长度、chunked 或 EOF 正文。具体配置见 [HTTP 正文边界](CONFIGURATION.md)。

HTTP/2 的快速路径仅处理已经结束的入站正文，不能仅凭 GET 或缺少 Content-Length 推断无正文。HTTP/3 每个请求流独立驱动，慢上传/下载不会串行阻塞同连接其他请求；逐帧发送保持 QUIC 背压并保留响应 trailers，正文生产失败会重置流而非发送正常 FIN。HTTP/3 使用与其他 HTTP 路径相同的访问日志和错误装饰。入站正文具有单请求大小与空闲期限，失败正文不会进入业务分发；当前入站正文仍按请求缓冲，不宣称已实现大上传零拷贝。

Linux native TCP/WebSocket 与 sendfile owner 先消费完整个 epoll_wait 返回批次，再安装待接收的连接/下载，避免当前批次中的旧 ERR/HUP 作用于复用 fd 的新任务。每轮最多安装 256 个任务，有剩余则通过 eventfd 唤醒下一轮，已有流量先获得 I/O 处理机会；该边界只限制接收队列，不减少 CPU 自适应 owner 数，也不改变既有数据发送预算或背压。依据 [Linux epoll 事件缓存说明](https://www.man7.org/linux/man-pages/man7/epoll.7.html)；不能仅因 EPOLL_CTL_DEL 已调用，就假定用户态已取回的事件也被删除。

原始 HTTP 上游空闲池给每个 Tokio socket 记录所属 runtime ID。checkout 在同一 runtime 内保持直接复用；跨 runtime（包括控制面预热转入数据面、不同 HTTP shard 之间复用）时先解除旧 reactor 注册，使用非阻塞 peek 排除已关闭或带残留数据的连接，再把同一 TCP 连接注册到当前 reactor。避免 socket 移动后仍依赖旧运行时唤醒；不额外预拨连接、不改变池容量，也不把每次普通转发变成重连。


## 可执行路径示例

以下片段与 HTML 架构页对应，经过真实配置解析、校验及逐字段回读。反代直接使用 hosts/path_prefix/upstream；TCP/UDP listener 直接指定 upstream；脚本必须显式 enabled 并指定 entry。完整参数教程见 [配置指南](CONFIGURATION.md)。

### HTTP 监听与反向代理

```yaml
http:
  plain_bind: 0.0.0.0:80
  tls_bind: 0.0.0.0:443

services:
  reverse_proxy:
    routes:
      - name: app
        hosts: ["app.example.com"]
        path_prefix: "/"
        upstream: "http://127.0.0.1:9000"
```

### 游戏 TCP 与透明 QCP

```yaml
tcp:
  listeners:
    - name: game-tcp
      bind: 0.0.0.0:7000
      nodelay: true
      connect_timeout_ms: 1500
      upstream: "10.0.1.10:7000"

udp:
  listeners:
    - name: game-udp
      bind: 0.0.0.0:7001
      protocol: qcp
      session_ttl_secs: 45
      max_associations: 150000
      upstream: "10.0.1.10:7001"
```

### 进程内脚本与插件入口

```yaml
script:
  enabled: true
  entry: "./gateway.ts"

plugins:
  enabled: true
  auto_load_dir: "./plugins"
```

### 独立反向代理片段

```yaml
services:
  reverse_proxy:
    routes:
      - name: app
        hosts: ["app.example.com"]
        path_prefix: "/"
        upstream: "http://127.0.0.1:9000"
```

### New API 与流式 AI 请求

```yaml
runtime:
  performance:
    enabled: true
    traffic_profile: small

services:
  ai_proxy:
    enabled: true
    routes:
      - name: llm-edge
        provider: new-api
        match_host: ai.example.com
        path_prefix: /v1
        upstream: http://127.0.0.1:3000
        rewrite_base_path: /v1
        forward_headers: true
        emit_metadata_headers: true
```

### MQTT 与 CoAP 上游

```yaml
tcp:
  listeners:
    - name: mqtt-tcp
      bind: 0.0.0.0:1883
      upstream: 127.0.0.1:18831

udp:
  listeners:
    - name: coap
      bind: 0.0.0.0:5683
      session_ttl_secs: 30
      max_associations: 50000
      upstream: 127.0.0.1:56831
```

## 授权与 TLS 状态发布

异步授权由 src/auth_request.rs 持有单个 HTTP client 和 128 个共享许可，调用仍属于原请求 future；不创建脱离请求的授权任务。路由决策先完成策略/限流，随后授权，再进入缓存/静态/主代理。拒绝路径不读取业务上游，认证响应正文不累计。脚本合成状态/响应头和单路由超时由核心执行，详情见配置指南。TLS 状态保存已解析且 keys_match 验证的默认/SNI keys 与同一状态的指纹；supervisor 只依赖该指纹换监听，不以磁盘半更新提前撤销旧监听。失败的续期保留旧状态。strict SNI 只接受显式配置选择器，不扩张证书 SAN 或 on-demand 白名单。

## 聚合请求体预算

`http.max_buffered_request_body_bytes` 默认 0（关闭 admission 限额），可限制同一个 Gateway 的在飞 buffered payload 总字节数。
HTTP/1、HTTP/2、HTTP/3 与管理 HTTP 共用计数；每个 chunk 复制前预留，预算不足立即 503，不排队，也不执行脚本或业务上游。
读取失败、超时、取消归还已预留字节；成功的 Bytes 使用持有 RAII reservation 的 owner，所有 clone（包括上游客户端持有的正文）释放后才返还。
重载只修改限额，不重建在飞计数；降低到已有占用以下不撤销已收正文，后续新增字节拒绝，0 模式也持续计数以支持之后开启。
例如保留旧站点 1 GiB 单体上限时，可设 max_request_body_bytes=1073741824 与 max_buffered_request_body_bytes=2147483648，另按机器容量留足运行余量。
该限额衡量 buffered payload，不是进程 RSS 或整个 HTTP/QUIC/TLS buffer 的硬上限；分配器 spare capacity、解析/解码/业务复制和其它状态仍占内存。
不把此功能宣称为 nginx 磁盘临时文件或端到端 streaming 上传；仍需实际负载与进程内存限制共同验证。

`http.redirect_http_to_https` 默认 true；证书域名命中时原生 308 先于脚本。需要脚本保留各站点明文跳转/代理策略时显式 false，不改变 TLS/SNI 和 ACME challenge。

普通 HTTP 代理在脚本与固定 GET 授权之后解析可选 `upstream_method`，不改变访客 ctx.method 或 WebSocket；访客 HEAD 显式 GET 回源时仅取头部并丢弃上游正文，HTTP/1、HTTP/2、HTTP/3 均保持 HEAD 无正文和 GET 元数据。HEAD 仍绕过 GET 缓存。响应头对象 `append: true` 在上游同名字段之后追加独立字段，默认仍替换；Set-Cookie 不折叠，cookie_secure 在最终头集合上逐条执行。

脚本 compression 省略/null 时继承全局，显式对象覆盖整个策略，enabled:false 可关闭。上游仅在已知 Content-Length≤4 MiB、达到 min_length 且 MIME 合格时为压缩收集正文；SSE 优先保持流式，未知或更大响应流式发送并跳过变换，206/已有编码也不变换。Vary 在编码协商前按完整表示资格补充，保留上游 token；HEAD 用表示长度，206 用 Content-Range total。非 identity 源站编码未以 Accept-Encoding 区分缓存键时不入缓存，流式响应也不入整块缓存。该有界收集不是 nginx 的端到端流式 gzip，详细契约见配置指南。
HEAD 不读取正文计算压后长度：协商压缩时设置 GET 对应编码、删除未知派生长度，identity 保留原长度。压缩表征及其 HEAD 元数据将强 ETag 弱化，已有弱标签不重复处理，identity/206/已有编码保留原 ETag。

未知/大表示的合格 MIME 仍保留 Accept-Encoding Vary，但保持流式并返回不可变换资格；HEAD 不宣称本次 GET 实际不会执行的编码。已知低于 min_length 的表示不增加该 Vary。
