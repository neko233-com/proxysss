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
| `src/plain_relay.rs` | balanced 明文 TCP/WebSocket 的每连接就绪任务、共享临时缓冲与背压存储 |
| `src/runtime_tuning.rs`、`src/linux_cpu.rs`、`src/linux_tune/` | 系统能力、cgroup cpuset、运行时/socket 适配与显式主机调优 |
| `src/script.rs`、`src/ts_transpile.rs` | 进程内 QuickJS、TypeScript 类型剥离、扩展调用与热重载 |
| `src/security.rs`、`src/acme/`、`src/filecloud/` | 访问/限流/管理鉴权、证书与 DNS provider、文件服务 |
| `src/install.rs` | 安装、升级、后台服务与平台集成 |

普通转发使用连接私有状态、分片表、原子计数器和有界池，配置通过 ArcSwap 发布。共享 Mutex/RwLock 只用于控制面、证书操作和首次缓存填充协调，不放在普通 HTTP/TCP/UDP 转发路径上。

性能验证的数据流独立于网关：HTTP/1 发生器每个 worker 持有独立的预连接 client，HTTP/2 使用共享多路复用 client；下载逐块消费和计数，检查响应状态及夹具长度，不聚合整个大文件。共同绝对开始时间不可由迟到进程自行后移，双方任一轮未完成等负载目标都会失败；CDN 发布预检位于测量前。完整边界与诊断方法见 [验证流程](VALIDATION.md)。

## HTTP 请求路径

配置边界：`reverse_proxy.routes` 用 `hosts`，`domain_routes` 用 `domains`，两者每条路由都直接指向 `upstream`。后者不是引用其他服务的嵌套路由编排器。`static_sites` 同时匹配可选 `hosts` 与路径；单个 `webdav` 按路径匹配，可通过 `auth.enabled/username/password` 开启内建 Basic 认证，认证先于路径检查和任何文件操作。具体可执行片段与边界见 [配置指南](CONFIGURATION.md)，不能将忽略的未知字段视为能力实现。

1. 公共 HTTP 默认监听 80，TLS/H2/H3 默认使用 443。accept 与握手由监听运行时负责，H2/H3 分别管理多路复用流。
2. 判断协议与配置路由，处理内建服务、ACME challenge、管理 HTTPS/monitor。用户公共路由优先于欢迎页兜底；默认 `/` 仅显示 `Welcome to proxysss`、GitHub 与 GitHub Docs 链接。
3. 执行配置的访问控制、限流和连接上限。静态源站额外检查真实 TCP 对端、令牌/签名及路径边界；HEAD、Range、缓存和条件响应不能绕过授权。
4. 分发静态、WebDAV、域名/反代/AI 路由或脚本。静态条件按 If-Match / If-Unmodified-Since、If-None-Match / If-Modified-Since、If-Range 的优先级执行，再选择完整或部分内容。
5. 代理路径查询缓存和内存上游池，结合负载均衡、主动健康、被动隔离和重试选点。Consul/etcd/Nacos 发现结果通过控制面更新上游池，普通请求不逐次查询注册中心。
6. 发送正文；WebSocket upgrade 后保持双向转发，SSE/AI 响应保留背压，gRPC 使用 HTTP/2。动态压缩受类型/大小限制，206 不进行整对象压缩。
7. 更新分片指标，按配置记录访问/错误日志；流式正文发送后遇到文件截断则终止传输，不补成成功短正文。

快路径只在语义等价时启用。raw reverse/SSE 保留默认转发头并复用请求/响应缓冲与上游 lane。条件请求和 Range 进入完整静态处理。启用热重载/管理更新时不复用连接上的免策略静态响应；不可变无策略 H2 使用连接 OnceLock 和只读 ArcSwap/FxHashMap 响应快照。

原始 SSE 转换为 TLS/HTTP2 下游响应前先按 HTTP/1 消息边界解码：chunked 去除大小行、扩展与终止分隔符，消费并丢弃 trailer；Content-Length 精确计数，无长度响应才以 EOF 结束。大小/元数据行最多 16 KiB，trailer 总量最多 64 KiB；长度冲突、未支持的 transfer coding、分块错误或正文提前 EOF 会终止响应。每次仍按下游需求借用池缓冲并返回 Bytes 切片，不积累完整 chunk 或 SSE 事件，完成或取消释放上游。SSE 不再压缩，但保留上游已有 Content-Encoding；移除编码头而保留压缩字节会破坏正文。

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

TCP 接受连接后执行流访问/限流和路由，选择内存上游池；没有脚本、亲和、健康和额外候选时走单上游直接路径。默认 TCP_NODELAY，连接阶段受 connect_timeout_ms 限制。半关闭先冲刷未发送字节，再传播写关闭，保留反向数据。

UDP 按客户端建立上游 association；session_ttl_secs 管理空闲时间，max_associations 限制高频重连规模。创建去重使用分片 pending 集合，清理按时间、创建数和容量压力节流。已建 association 使用 worker 本地缓存，TTL 最多每秒刷新一次，数据报使用有界缓冲池。

UDP 清理在同一分片操作内核对 association 的 Arc 身份，旧 reader、发送失败或容量清理不能误删同地址的新会话。替换会话会停用旧实例；reader 检查自身活跃状态和 TTL，观察到停用后退出并停止转发迟到回复，不借用新实例的活跃时间维持旧 socket。本地缓存判定过期前读取共享刷新时间，避免其他 worker 的活跃会话被旧缓存关闭；容量候选已刷新或替换时跳过本次淘汰。

MQTT TCP、TLS SNI 透传、MQTT WebSocket、CoAP、游戏语音、KCP 风格与透明 QCP 都复用这些转发路径。proxysss 不实现 MQTT broker，QCP 透明转发不代表帧终止。FTP 控制与主动/被动数据连接执行各自策略，启用时默认控制端口为 21。

## Linux 调度

CPU 数从 cgroup effective cpuset 探测，不从已绑核线程误推全局资源，也不硬编码小型 worker 上限。SO_REUSEPORT listener fanout 与数据核数成比例，socket 分配交给 Linux。

| traffic_profile | HTTP/TLS | 实时流与 UDP | 静态文件 |
| --- | --- | --- | --- |
| small（默认） | HTTP/1、TLS/H2 共用每允许核心一个 worker 的 Tokio runtime | TCP/WebSocket/UDP 同运行时，可 work stealing | 热小对象缓存；sendfile 稀疏时 8 MiB、超过数据核数时 2 MiB 切片 |
| balanced | 明文 HTTP 每核独立单线程 runtime；TLS/H2 在 nice 5 有界运行时 | 明文 TCP/WebSocket 每连接一个就绪任务，与接收连接的 HTTP 分片共用队列；UDP 用 ceil(cores/2) nice 3 Tokio worker；WSS 留在 TLS runtime | 直接 sendfile，每 2 MiB 让出同一任务队列 |
| bulk | 优先大流传输 | 显式 bulk TCP 可使用 splice | 优先 sendfile 描述符，可持续使用 reactor |

balanced 不再为明文实时连接启用独立线程或整组事件任务，也不使用请求后短自旋。small 保留 64-poll relay 公平边界，shared UDP 每 4 包让出。

balanced 大于等于 8 MiB 的完整对象直接使用 sendfile，每 2 MiB 让出 HTTP 分片的任务队列，高密度时也保留该边界。长连接按各自读写就绪调度，每方向至多 16 个 I/O 步骤（最后一次读可追加一次写），不把整组连接作为一个任务连续处理。每个实际执行线程借用一个 16 KiB 临时池缓冲；只有未写出的字节才为该方向保留一个池缓冲，写完立即归还。暂时无流量的连接不占用正文缓冲；半关闭先冲刷待发送字节，再传播写关闭。

TCP 直接留在接收连接的分片。明文 WebSocket 完成升级和首批字节转发后，在当前分片注册 socket 副本并创建独立 relay 任务，原 HTTP 解析状态随之释放；失败则使用已有异步回退，TLS/WSS 不使用该明文路径。取消 relay 任务会释放 socket、待发送缓冲和就绪注册。这里只描述候选实现的资源归属，性能准入仍须完整混合实测。

bulk 仍使用独立 native sendfile/stream reactor。独立文件描述符从有界池借还；EAGAIN 保存进度并形成背压。静态 sendfile 不设置 SO_SNDBUF，保留 Linux 发送缓冲自动调优；手动设置再写回数字不能清除用户锁，会影响同一 keep-alive 连接后续下载。bulk reactor 提前清除 TCP_CORK 后通知 owner，避免重复 syscall。

明文 HTTP I/O event interval 为 8，shared-small 为 16，global queue interval 为 31。H2 每连接用本地 FuturesUnordered 驱动流，每个完成流后让出。静态 freshness 为 2 秒，nice-10 控制运行时重验证并原子发布快照。改变策略前阅读 [性能决策](decisions/linux-performance.md)，验证方法见 [验证流程](VALIDATION.md)。

原生 sendfile 的完成 future 同时拥有取消责任。HTTP 等待任务退出时关闭完成通道，经每 owner 的有界无锁取消队列和 eventfd 唤醒 reactor；队列溢出才扫描已放弃作业，普通完成不扫描。清理前检查完成通道确已关闭，防止旧 fd 通知误取消复用该数字的新下载。未注册作业和连续发送轮也检查取消；已阻塞的作业无需等待客户端再次产生 I/O 即可释放 socket/file 副本。

## 启动、预热和重载

启动加载一个 YAML（默认 proxysss.yaml，支持 -config/--config/-c），校验后探测系统/socket 能力，记录启用与跳过原因。Ubuntu 24.x 使用支持的极限参数，旧/未知系统明确降级；持久 sysctl 修改只由显式 `proxysss tune linux --apply` 执行，保留 SSH 安全、备份与失败回滚。

监听前预热 hot index/顶层静态对象、小对象完整响应和 sendfile 描述符；每个去重反代/AI 上游池仅预拨一对连接，数字 IP 在池创建时解析。Tokio socket 归创建 reactor 所有，不把启动 runtime 大批 socket 作为其他 runtime 长期 lane。`/healthz` 暴露 warm 状态，重载重新预热。

默认手动 reload，文件监视需显式启用；主 YAML、主脚本与自动加载插件参与监视。指纹确定 reload/restart 范围，配置通过 ArcSwap 发布。

| 可热更新 | 需重启 |
| --- | --- |
| 路由/上游池、服务发现映射、静态站点、WebDAV、FTP upstream、脚本/插件与相应策略 | HTTP/TLS/H3 bind、admin bind/enabled、TCP/UDP listener 名称与 bind 集合、FTP bind/enabled、TLS mode、日志格式/级别/路径、runtime.performance |

token 认证 API 可持久化允许的路由、listener、TLS、filecloud 和黑名单变更；实际应用边界仍由 reload-plan 决定。explain、capabilities、watched-scripts、routes、reload-plan、nginx-parity 是 agent 检查入口。

## 管理、监控和扩展

admin 默认只监听 127.0.0.1:7777，enable_write_ops=false、expose_config=false。token 经 `proxysss token show/set` 检查和轮换，一般配置展示脱敏。loopback 管理面可引导 ACME/TLS，admin.https 在主 TLS listener 提供相同 `/v1/*` API。

admin.monitor 是独立 HTTPS 只读入口，使用专用密码与 monitor:read 短期会话。登录后仅提供 health、summary、stats、SSL 状态，拒绝写接口，不暴露管理 token、私钥、DNS 凭据或配置写入能力。

QuickJS 与 swc_ts_fast_strip 内嵌进程，无外部 Node/Deno/tsc。routeHttp/routeTcp/routeUdp 与插件钩子负责可选业务扩展；核心边缘策略保持原生实现。主动健康覆盖 HTTP/TCP 和显式 UDP 探测，被动失败可隔离上游；watchdog 观察关键后台任务，metrics 暴露重启/失败计数。

管理 HTML/CSS、内置文档和站点资源嵌入二进制。管理 `/docs` 与 `/docs.html` 经过管理传输检查；公开文档导航只链接 HTML。详细配置见 [配置指南](CONFIGURATION.md)，扩展见 [脚本指南](../ts-how-to-use.md)，交付顺序见 [验证与发布](VALIDATION.md)。

## 流式协议与入站正文边界

HTTP/1 原始上游响应使用共享分帧校验：拒绝冲突 Content-Length、TE/CL 共存和不支持的 Transfer-Encoding；跳过有界数量的信息响应，再转发最终响应。chunked 扫描先确认片段边界，再发送有效前缀，不把尾随的另一响应混入正文；固定长度或 chunked 提前 EOF 作为失败关闭，异常连接不能回池。

HTTP 上游等待采用连接/响应头期限与逐帧正文空闲期限，持续 SSE、下载不受总时长限制，下游背压不消耗上游空闲预算。WebSocket 非 101 响应仍完整转发其固定长度、chunked 或 EOF 正文。具体配置见 [HTTP 正文边界](CONFIGURATION.md)。

HTTP/2 的快速路径仅处理已经结束的入站正文，不能仅凭 GET 或缺少 Content-Length 推断无正文。HTTP/3 每个请求流独立驱动，慢上传/下载不会串行阻塞同连接其他请求；逐帧发送保持 QUIC 背压并保留响应 trailers，正文生产失败会重置流而非发送正常 FIN。HTTP/3 使用与其他 HTTP 路径相同的访问日志和错误装饰。入站正文具有单请求大小与空闲期限，失败正文不会进入业务分发；当前入站正文仍按请求缓冲，不宣称已实现大上传零拷贝。

Linux native TCP/WebSocket 与 sendfile owner 先消费完整个 epoll_wait 返回批次，再安装待接收的连接/下载，避免当前批次中的旧 ERR/HUP 作用于复用 fd 的新任务。每轮最多安装 256 个任务，有剩余则通过 eventfd 唤醒下一轮，已有流量先获得 I/O 处理机会；该边界只限制接收队列，不减少 CPU 自适应 owner 数，也不改变既有数据发送预算或背压。依据 [Linux epoll 事件缓存说明](https://www.man7.org/linux/man-pages/man7/epoll.7.html)；不能仅因 EPOLL_CTL_DEL 已调用，就假定用户态已取回的事件也被删除。

原始 HTTP 上游空闲池给每个 Tokio socket 记录所属 runtime ID。checkout 在同一 runtime 内保持直接复用；跨 runtime（包括控制面预热转入数据面、不同 HTTP shard 之间复用）时先解除旧 reactor 注册，使用非阻塞 peek 排除已关闭或带残留数据的连接，再把同一 TCP 连接注册到当前 reactor。避免 socket 移动后仍依赖旧运行时唤醒；不额外预拨连接、不改变池容量，也不把每次普通转发变成重连。
