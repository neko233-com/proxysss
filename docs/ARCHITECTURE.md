# proxysss 架构

proxysss 用单个 Rust 二进制承担通用边缘网关职责：协议终止、路由、静态文件、反向代理、流转发、安全策略、日志与管理。可选 TypeScript/JavaScript 扩展负责业务决策；普通网关请求无需脚本。面向用户的交互入口是 [架构 HTML](architecture.html)。

## 模块职责

| 模块 | 所有权与职责 |
| --- | --- |
| `src/main.rs` | CLI、命令运行时与能力检查 |
| `src/config.rs`、`src/config/performance.rs` | 单 YAML 配置、默认值、校验、重载指纹和流量 profile |
| `src/gateway.rs` | 监听器、HTTP 分发、上游池、缓存、健康状态、TCP/UDP 与管理 API |
| `src/static_security.rs` | 静态源站令牌、真实路径边界、签名 URL 和目录规则 |
| `src/static_ranges.rs` | Range 解析、重叠合并、单段读取与多段流式下载 |
| `src/stream_reactor.rs`、`src/sendfile_reactor.rs` | Linux 明文实时流 epoll relay、大文件 sendfile 作业与完成通知 |
| `src/runtime_tuning.rs`、`src/linux_cpu.rs`、`src/linux_tune/` | 系统能力、cgroup cpuset、运行时/socket 适配与显式主机调优 |
| `src/script.rs`、`src/ts_transpile.rs` | 进程内 QuickJS、TypeScript 类型剥离、扩展调用与热重载 |
| `src/security.rs`、`src/acme/`、`src/filecloud/` | 访问/限流/管理鉴权、证书与 DNS provider、文件服务 |
| `src/install.rs` | 安装、升级、后台服务与平台集成 |

普通转发使用连接私有状态、分片表、原子计数器和有界池，配置通过 ArcSwap 发布。共享 Mutex/RwLock 只用于控制面、证书操作和首次缓存填充协调，不放在普通 HTTP/TCP/UDP 转发路径上。

## HTTP 请求路径

1. 公共 HTTP 默认监听 80，TLS/H2/H3 默认使用 443。accept 与握手由监听运行时负责，H2/H3 分别管理多路复用流。
2. 判断协议与配置路由，处理内建服务、ACME challenge、管理 HTTPS/monitor。用户公共路由优先于欢迎页兜底；默认 `/` 仅显示 `Welcome to proxysss`、GitHub 与 GitHub Docs 链接。
3. 执行配置的访问控制、限流和连接上限。静态源站额外检查真实 TCP 对端、令牌/签名及路径边界；HEAD、Range、缓存和条件响应不能绕过授权。
4. 分发静态、WebDAV、域名/反代/AI 路由或脚本。静态条件按 If-Match / If-Unmodified-Since、If-None-Match / If-Modified-Since、If-Range 的优先级执行，再选择完整或部分内容。
5. 代理路径查询缓存和内存上游池，结合负载均衡、主动健康、被动隔离和重试选点。Consul/etcd/Nacos 发现结果通过控制面更新上游池，普通请求不逐次查询注册中心。
6. 发送正文；WebSocket upgrade 后保持双向转发，SSE/AI 响应保留背压，gRPC 使用 HTTP/2。动态压缩受类型/大小限制，206 不进行整对象压缩。
7. 更新分片指标，按配置记录访问/错误日志；流式正文发送后遇到文件截断则终止传输，不补成成功短正文。

快路径只在语义等价时启用。raw reverse/SSE 保留默认转发头并复用请求/响应缓冲与上游 lane。条件请求和 Range 进入完整静态处理。启用热重载/管理更新时不复用连接上的免策略静态响应；不可变无策略 H2 使用连接 OnceLock 和只读 ArcSwap/FxHashMap 响应快照。

## CDN 和复杂下载

完整小对象使用有界缓存和预构建 HTTP/1 keep-alive 响应，大文件流式发送；Linux 合适的明文完整响应使用 sendfile。Range 按偏移读取所需字节，不读取整个原文件。HEAD 忽略 Range，只返回完整表示元数据。

正文缓存持有自有不可变 Bytes 快照，不直接 mmap 可变源文件；每次填充最多 32 MiB，读取前后校验文件长度与修改时间，源文件随后截断不会让 TLS/压缩访问失效映射。H2 的文件重验证在控制运行时执行；文件删除、越界或重验证失败会使连接快照失效并退回正常分发，避免永久发送已撤下资源。

正文缓存使用原子预留的 256 MiB 上限；进程内全部静态快照（含正在填充、已淘汰但响应仍引用的正文）另受 320 MiB 硬上限约束。最后一个 Bytes 引用释放时归还预算，预算耗尽的冷下载立即回退流式读取，预热跳过该对象。缓存淘汰只扣减一次，填充协调锁随成功、失败或取消清理；缓存命中不重新争用预算。此上限不包括 socket、TLS、压缩输出与其他网关内存。

Range 最多接受 16 个区间，合并重复/相邻/重叠区间，保留未合并部分的请求顺序。单段返回 206 和 Content-Range，多段使用 multipart/byteranges、分段 Content-Range 和精确总 Content-Length。有效集合的越界部分被跳过，全部不可满足返回 416；格式错误、未知单位或超限集合忽略 Range。

多段响应只持有一个文件描述符，HTTP 消费正文才触发读取。大范围每次最多借用 64 KiB 池缓冲，Bytes 持有所有权直到下游释放，独立空闲池最多 8 MiB；取消释放文件与缓冲，无后台生产线程。弱元数据 ETag 不满足 If-Match 强比较，存在文件可用 `*`；If-Range 只接受精确匹配且稳定 60 秒的修改日期，否则退回完整 200。部署资源宜写新文件后原子替换，避免就地修改正在发送的内容。详细说明见 [CDN 下载](CDN-ORIGIN.md)。

## 游戏与 socket 长连接

TCP 接受连接后执行流访问/限流和路由，选择内存上游池；没有脚本、亲和、健康和额外候选时走单上游直接路径。默认 TCP_NODELAY，连接阶段受 connect_timeout_ms 限制。半关闭先冲刷未发送字节，再传播写关闭，保留反向数据。

UDP 按客户端建立上游 association；session_ttl_secs 管理空闲时间，max_associations 限制高频重连规模。创建去重使用分片 pending 集合，清理按时间、创建数和容量压力节流。已建 association 使用 worker 本地缓存，TTL 最多每秒刷新一次，数据报使用有界缓冲池。

MQTT TCP、TLS SNI 透传、MQTT WebSocket、CoAP、游戏语音、KCP 风格与透明 QCP 都复用这些转发路径。proxysss 不实现 MQTT broker，QCP 透明转发不代表帧终止。FTP 控制与主动/被动数据连接执行各自策略，启用时默认控制端口为 21。

## Linux 调度

CPU 数从 cgroup effective cpuset 探测，不从已绑核线程误推全局资源，也不硬编码小型 worker 上限。SO_REUSEPORT listener fanout 与数据核数成比例，socket 分配交给 Linux。

| traffic_profile | HTTP/TLS | 实时流与 UDP | 静态文件 |
| --- | --- | --- | --- |
| small（默认） | HTTP/1、TLS/H2 共用每允许核心一个 worker 的 Tokio runtime | TCP/WebSocket/UDP 同运行时，可 work stealing | 热小对象缓存；sendfile 稀疏时 8 MiB、超过数据核数时 2 MiB 切片 |
| balanced | 明文 HTTP 每核独立单线程 runtime；TLS/H2 在 nice 5 有界运行时 | 无策略 TCP/明文 WebSocket 每允许 CPU 一个 nice 3 soft-affinity epoll owner；UDP 用 ceil(cores/2) nice 3 Tokio worker；WSS 留在 TLS runtime | 按活跃传输压力选择 HTTP owner 或预热 sendfile reactor |
| bulk | 优先大流传输 | 显式 bulk TCP 可使用 splice | 优先 sendfile 描述符，可持续使用 reactor |

balanced 实时 owner 仅在真正阻塞后的请求 tick 短探测：1–16 对连接一次、17–32 对两次、超过 32 对依赖 epoll；上游回复和持续就绪流量不额外自旋。small 保留 64-poll relay 公平边界，shared UDP 每 4 包让出。

balanced 大于等于 8 MiB 的对象，仅在活跃传输大于数据核数且不超过每核四条时用 reactor；其他压力区间直接 sendfile。每核不超过两条时选均匀分布的半数预热 owner，更高压力恢复全部 owner。独立文件描述符从有界池借还；EAGAIN 保存进度并形成背压。静态 sendfile 不设置 SO_SNDBUF，保留 Linux 发送缓冲自动调优；手动设置再写回数字不能清除用户锁，会影响同一 keep-alive 连接后续下载。reactor 提前清除 TCP_CORK 后通知 owner，避免重复 syscall。

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
