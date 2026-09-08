# 安全开关、配置建议与系统性能适配

proxysss 的安全策略按服务和功能独立控制。开关关闭时保留参数，重新开启无需重填规则；管理认证、路径边界、HTTP 解析正确性和 HTML 转义属于基础实现约束，不提供绕过它们的总开关。默认值用于通用网关启动，不等于每种公网场景的推荐值。上游证书校验与插件启用的默认值已统一：省略整个配置段或只填写部分字段，不会暗中切换为跳过证书校验或启用插件。

## 先检查当前配置

```bash
proxysss -c proxysss.yaml config security
proxysss -c proxysss.yaml config performance
proxysss -c proxysss.yaml config reload-plan
```

`config security` 输出可解析的 YAML：每项包含路径、当前值、默认值、开关方式和中文建议。静态站点还输出鉴权实际生效状态。不会打印密码、令牌、签名密钥。`config performance` 只读检查当前 OS、版本、CPU、原生 I/O 后端和 socket 参数支持情况，不修改配置或主机设置；不带 `-c` 且无配置文件时使用默认值生成建议。

## 安全配置如何开关

| 配置面 | 开关与默认行为 | 配置建议 |
| --- | --- | --- |
| 管理 API 输入校验 | `security.validate_admin_mutations: true` | 生产保留。关闭不绕过配置语法和类型校验。 |
| 上游 SSRF 限制 | `security.block_ssrf_targets: true`；`blocked_upstream_hosts` / `blocked_upstream_cidrs` 为规则 | 保留默认防护并按实际网络收紧管理 API 的目标范围；不是 DNS 重绑定的完整防线。 |
| HTTP/1 歧义防护 | `security.reject_ambiguous_http1: true` | 保留。底层解析器的协议检查始终存在。 |
| DDoS 连接速率封禁 | `security.ddos.enabled: false` | 公网按峰值开启；用 `max_connections`、`window_secs`、`burst`、`ban_secs` 控制窗口和惩罚，CDN 出口会合并计数。 |
| 动态黑名单 | `security.dynamic_blacklist.enabled: false` | 需要临时封禁时开启；保护持久化路径和管理写接口。 |
| MAC 黑名单 | `security.mac_deny` 非空会报错 | 旧字段没有接入转发链，不再静默接受；使用 IP/CIDR，或在主机 L2 防火墙执行。MAC 不跨路由器。 |
| HTTP/stream ACL | `services.access_control.http.enabled` / `stream.enabled` 默认 false | 内网 API、数据库和 MQTT 运维入口按实际对端开启；维护 allow/deny 列表。 |
| HTTP/stream 限流 | `services.rate_limit.http.enabled` / `stream.enabled` 默认 false | 按容量选择 fixed_window/token_bucket/leaky_bucket；限制突发与窗口，HTTP `max_connections: 0` 表示不设并发上限。 |
| 路由 ACL/限流 | 反向代理/域名路由可覆盖限流；TCP SNI 路由另有 ACL | 关闭路由覆盖项仍可能继承全局策略，不等于关闭全局防护。 |
| CDN 静态鉴权 | `static_sites[].security.enabled: true` | 总开关控制回源令牌、对端白名单、签名；没有配置令牌/规则时不会自动添加认证。 |
| CDN 回源令牌 | `security.origin_token_enabled: true`；`origin_token` 默认空 | 填独立的至少 32 字节随机令牌，CDN 覆盖对应 header；关闭时保留密钥但不验证。 |
| CDN 回源 IP | `security.allowed_peers_enabled: true`；`allowed_peers` 默认空 | CDN 提供稳定回源 CIDR 时配置；只读取 TCP 对端，不信任转发 IP header。 |
| 私有 URL 签名 | `security.signed_url.enabled: false` | 私有源站下载开启；`max_ttl_secs` 默认 300，范围 1–86400；响应 no-store。CDN 命中缓存时仍须边缘鉴权。 |
| 站点限流 | `static_sites[].rate_limit.enabled: false` | 独立于鉴权总开关；禁用站点覆盖不关闭全局 HTTP 限流。 |
| 目录浏览 | `autoindex: false`、`hide_dotfiles: true` | 仅为公开下载目录开启索引；`autoindex_max_entries` 默认 1000、上限 10000。隐藏文件保护建议保持。 |
| 管理服务 | `admin.enabled: true`、`loopback_only: true` | 默认 loopback:7777；不需要管理接口可关闭。管理员口令和令牌必须替换为强凭据。 |
| 配置读取与写入 | `admin.expose_config: false`、`enable_write_ops: false` | 授权自动化需要时分别开启；配置读取仍脱敏。 |
| HTTPS 管理入口 | `admin.https.enabled: false` | 公网运维按需开启并限定 hosts；始终认证。 |
| 管理登录防暴力 | `admin.auth_rate_limit.enabled: true` | 保持开启，用 max_failures/window_secs/lockout_secs 适配共享出口人数。 |
| 上游 TLS 校验 | `http.allow_insecure_upstreams: false` | 默认和生产建议均为 false，校验 HTTPS/WSS 上游证书；true 仅用于明确接受自签证书的环境。 |
| 按需签发证书 | `http.tls.on_demand.enabled: false` | 动态域名场景才启用，配 allow/ask_url 与签发速率限制。 |
| WebDAV | `services.webdav.enabled: false`，`allow_write: true` | 公开只读设置 allow_write=false；可写入口另配身份验证/ACL。 |
| FileCloud | 服务默认关闭；上传/删除/移动/建目录各有 `allow_*` | 设置强密码和 HTTPS。只读部署关闭所有写操作；私有下载设置 require_auth_for_download=true，按需限制 max_upload_bytes/session_ttl_secs。 |
| FTP IP ACL | `services.ftp.access_control_enabled: true` | 配 allow/deny 后执行；关闭保留规则。FTP 服务默认关闭。 |
| FTP 命令/传输 | `command_policy_enabled`、`transfer_policy_enabled` 默认 true | 各自控制全局及该类用户规则；只读账号只允许必要命令和下载。 |
| FTP 用户策略 | `user_policy_enabled: true` | false 仅跳过 user_policies，全局命令/传输策略继续执行。 |
| FTP 登录与带宽 | `max_login_attempts: 0`、`limit_rate: 0` | 0 表示不设相应上限；公网建议设置登录次数与实际带宽预算。 |
| 缓存 PURGE | 各级 `cache.allow_purge` 默认 true，缓存本身默认关闭 | 公网缓存建议关闭 PURGE，或通过 ACL/受控路由保护。 |
| 插件管理 | `plugins.allow_admin_manage: true` | 不需要远程管理插件时关闭；开启仍需管理员认证与对应写权限。 |
| 脚本资源边界 | `script.timeout_ms`、`memory_limit_mb`、`max_stack_size_kb` | 随脚本启用生效，按合法脚本的耗时、内存与递归深度配置；保留资源上限。 |
| TS/插件扩展 | `script.enabled`、`plugins.enabled` 默认 false | 仅加载可信扩展；Referer/UA/bot-score、业务身份、CSRF 由应用、CDN WAF 或脚本代理路由处理。 |

参数能否执行还取决于所属服务是否启用。关闭某项策略不等于请求一定放行：其他 ACL、鉴权和限流仍可能拒绝。静态站点按 YAML 顺序匹配，具体路径放在 `/` 前；私有数据使用独立目录，不能放在公开根目录内。

## 推荐的部署组合

CDN 公开源站：回源令牌 + 回源 CIDR + 源站速率限制，默认关闭目录索引；边缘负责每位访客的限速、盗刷判定与费用封顶。公开资源明确设置 cache_control。

私有下载：上述源站约束 + 短期签名，保持 no-store；或由 CDN 对每次访客请求（含缓存命中）验证其自身签名，再通过回源令牌取资源。不要把“签名带 query”误当成共享缓存已受保护。

管理/数据库入口：管理 API 留在 loopback，远程走受控 HTTPS 入口；开启登录防暴力；按需启用配置读取和写入。TCP SNI/stream 使用真实对端 ACL 和连接速率限制。

## Windows、macOS、Linux 性能适配

```yaml
runtime:
  performance:
    enabled: true
    adaptive_system: true
    socket_extreme: true
    profile: edge
    traffic_profile: small
    log_on_start: true
    windows:
      enabled: true
      tcp_keepalive_secs: 60
      tcp_keepalive_interval_secs: 15
      tcp_send_buffer_bytes: 0
      tcp_receive_buffer_bytes: 0
      udp_buffer_bytes: 262144
      tcp_notsent_lowat_bytes: 0
    macos:
      enabled: true
      tcp_keepalive_secs: 60
      tcp_keepalive_interval_secs: 15
      tcp_send_buffer_bytes: 0
      tcp_receive_buffer_bytes: 0
      udp_buffer_bytes: 262144
      tcp_notsent_lowat_bytes: 0
```

Windows 使用 Tokio/Mio 的 IOCP。新增适配在建 TCP socket 时应用系统 keepalive，在建共享 UDP 监听 socket 时设置有界收发缓冲，上游逐对端会话保留系统默认值；TCP 缓冲默认 0，保留系统自动调节。没有修改注册表、netsh、电源计划或网络设备配置。`tcp_notsent_lowat_bytes` 在 Windows 必须为 0。

macOS 使用 kqueue 和系统调度，保留 macOS/Mio 的 SO_NOSIGPIPE 行为；同样支持 keepalive 与共享监听器的有界 UDP 缓冲。可选的 `tcp_notsent_lowat_bytes` 使用 Darwin 的 TCP_NOTSENT_LOWAT，限制待发送数据；默认 0，不擅自改变批量传输与延迟的取舍。没有把 Linux CPU 亲和性、SO_REUSEPORT 扇出或 sysctl 参数套到 macOS。

Linux 延续 epoll、按可用 CPU 扩展的 HTTP/TCP/UDP 并行度、无策略快速转发、静态预热与有界 buffer 池；Ubuntu 24.x 使用已有的极限 socket 策略，旧版/未知发行版明确降级并说明原因。`socket_extreme: false` 关闭极限参数，`adaptive_system: false` 跳过系统 socket 适配，`enabled: false` 关闭可选运行时性能路径。持久主机调优仍需显式 `proxysss tune linux --apply`。

Windows/macOS 保持现有运行时调度，平台 enabled=false 关闭对应平台新增 socket 设置；基础异步 I/O 和监听器配置的 TCP_NODELAY 仍保留。`tcp_keepalive_secs: 0` 跳过 keepalive 设置；缓冲为 0 跳过对应 setsockopt。正数缓冲上限 16 MiB，内核可进一步限制实际值；不要给大量连接盲目分配大缓冲。keepalive idle 上限 86400 秒，interval 范围 1–3600 秒。

启动时记录计划、OS 版本、参数探测成功/失败和 UDP 实际缓冲值；运行时设置失败首次记录告警，保留 OS 回退。能力探测不等于性能证明。`small` 偏向小文件与实时反馈，`bulk` 偏向大文件流式/零拷贝，`balanced` 同时准备两种静态路径；根据真实混合流量选择。

## 生效边界和验证

静态安全开关随 YAML 热重载，新请求重新鉴权；FTP 策略供新控制连接使用。运行时性能路径和 socket 设置在进程启动时固定，修改 `runtime.performance` 后必须重启，普通 reload 会明确拒绝这类变更，避免显示新配置却使用旧参数。管理员启用/bind、监听身份、TLS mode 与日志配置等原有重启边界不变。

`test.cmd` 继续执行项目内两轮幂等功能测试并固定打包。`scripts/verify-macos-adapter.ps1` 对实际平台适配模块与配置类型执行 macOS x86_64/arm64 交叉检查，组件位于 `.cache/macos-sysroot/<Rust版本>`；**这不包含依赖 Apple SDK 的完整 macOS 网关链接，也不代替原生性能测试**。

Docker 验证统一使用固定容器名 `proxysss-verify` 和固定镜像标签，启动前删除该项目遗留的同名容器，完成或失败后删除容器；不创建临时探测容器、匿名卷或每轮新镜像。`scripts/verify-docker-scenarios.ps1 -Performance` 在这一个容器内完成 OS 检查、两轮测试、场景检查与混合诊断。构建缓存复用，报告覆盖 `.tmp/docker-scenarios/`。

本地混合诊断使用 Go 编译工具，连续运行 static-small、static-large、CDN hot-update、HTTPS、HTTP proxy、SSE、WebSocket、game TCP、TCP、UDP。各轮反转关闭/开启顺序，记录原始日志、吞吐、p50/p95/p99、进程当前/峰值内存，输出固定在 `.tmp/platform-performance/<OS>/`，fixture 和子进程自动清理。

```powershell
. ./scripts/project-artifacts.ps1
Initialize-ProjectArtifacts
go build -o .tmp/tools/platform-performance.exe ./scripts/platform-performance/main.go ./scripts/platform-performance/process_windows.go
./.tmp/tools/platform-performance.exe -binary target/release-fast/proxysss.exe -seconds 4 -repetitions 4
```

Windows 当前处理器组至少有 18 个可用逻辑 CPU 时，诊断工具在创建子进程前通过继承的进程亲和性隔离网关、5 类后端与 10 类客户端，具体分配写入 cpu_roles；资源不足或其他系统会明确保留空分配记录。所有 CPU 设置仅作用于测试进程。即使按 CPU 隔离，同机仍共享内核与物理资源，小幅差异可能来自噪声；不得用于声称跨主机或严格优于 nginx。Linux 生产性能结论仍使用原有混合负载和独立角色证据门禁。没有 macOS 实机时只报告交叉检查结果。

参考：[Microsoft TCP/网卡调优](https://learn.microsoft.com/en-us/windows-server/networking/technologies/network-subsystem/net-sub-performance-tuning-nics)、[Windows keepalive](https://learn.microsoft.com/en-us/windows/win32/winsock/sio-keepalive-vals)、[Apple XNU TCP 定义](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet/tcp.h)。

Windows 的多线程扩展及 TCP 调度拆分实验未通过混合长连接回归检查，均已撤回；本次交付不以静态/HTTP 的单项收益换取 TCP 性能下降，也不声称 socket 参数本身必然提高吞吐。
