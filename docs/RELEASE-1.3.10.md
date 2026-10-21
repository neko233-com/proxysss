# v1.3.10 Linux 迁移稳定版本验证

2026-10-21 用户明确要求“没问题就全部发布”。本次版本按 Linux nginx 迁移的功能与稳定性交付，豁免仅限 v1.3.10 的严格性能胜出及跨主机性能证据，不豁免生产 Linux 功能失败、产物错误或六平台打包失败。不宣称 nginx 性能胜出、完整流式 gzip 等价、所有平台运行全绿或长期线上稳定。对应 [HTML 记录](release-1.3.10.html) 同步维护。

## 当前验收与产物身份

- 功能源码候选 `52b3259`：Linux Bookworm x86_64 原生 Docker release 测试 419/419 通过，测试执行 59.53 秒；独立原生 Linux 文件系统夹具，未把 Windows 共享目录语义当作 Linux 本地文件系统。
- 该候选 Linux 二进制 SHA256：`98bbb4c074700869e79826c033283db11136756d947089e99c64089e207d673f`。它是版本元数据更新前的功能候选，不能冒充 v1.3.10 的最终平台附件。
- 对该实际二进制，在 2 CPU、768 MiB、无外网、只读根文件系统且无 capabilities 的独占容器中，既有 16 项独立 Go race 黑盒全部通过；新增真实 H1/H2 HEAD gzip/identity、强弱 ETag、Vary、响应头追加/替换、Range、SSE 及未知长度 EOF 前传输通过。原始 HTTP/1 报文确认 HEAD 零正文。容器退出 0，无 OOM，专有资源已清理。
- 1 GiB 流式生成上传通过：源站精确收到 1,073,741,824 bytes，随后小请求 200；候选 VmHWM 2,118,740 KiB，容器 memory.peak 2,194,599,936 bytes。该检查采用独占 4 GiB 容器和 2 GiB payload 预算，不生成 1 GiB 内存副本或文件；不代表任意并发 RSS 上限。
- 最后 Vary-only 修正和隔离夹具的 v1.3.10 Linux 完整 release 测试 419/419 通过，执行 62.85 秒。Bookworm 构建二进制 SHA256 为 `920de0e8e53d5e4fbe01d9b4a999c18e227f82a6e4c58863d77c81310be6e18f`；该产物面向现有 glibc 2.36 环境，官方平台附件由标签工作流另行打包，以 [Release 实际附件](https://github.com/neko233-com/proxysss/releases/tag/v1.3.10) 为准。打包排队不代表发布成功，官方附件下载与摘要核对属于独立交付验收。

## Windows 实际限制

完整 `cargo test --locked` 已执行：初轮 381/391 通过，10 项失败。9 项初始化失败来自直接 serde 构造配置后共用默认 TLS 路径，已将相关 listener/UDP 测试夹具的证书与私钥隔离，保留全部断言；独立 listener 回归 2/2 通过。隔离后 UDP 的 IPv6 后端 echo 仍失败，专门 hostname/IPv6 用例单独串行也超时。

独立 .NET UDP loopback 诊断不经过 proxysss 或 Tokio：IPv4 往返通过，IPv6 `::1` 发送后接收超时。本次不改系统网络、不跳过或改写专门 IPv6 用例、不放宽证书匹配。隔离后 v1.3.10 完整重跑为 384/391 通过、7 项 UDP 用例失败，执行 21.55 秒；非 IPv6 关键串行回归：新增方法/压缩 wire 5/5、现有 HTTP 策略 10/10、HTTP/H2/SSE/WebSocket e2e 8/8 通过；不能将本机结果称为 Windows 全绿；六平台编译打包也不能替代各平台运行验收。

## 能力边界

脚本路由压缩省略/null 继承全局；显式对象整体覆盖。仅已知 Content-Length、达到最小长度、MIME 允许且≤4 MiB 的普通表示缓冲压缩；未知/大响应继续流式，SSE、206、已有编码不变换。符合协商策略的未知/大表示仍保留 Accept-Encoding Vary，但 HEAD 不假报未执行的压缩；已知低于 min_length 的表示不加该 Vary。HEAD 压缩元数据省略未知压后长度，实际变换及对应 HEAD 将强 ETag 弱化。

授权先于缓存/静态/上游；拒绝、超时、取消和并发预算已用合成离线服务验证，不构成任意外部认证服务验收。真实候选配置差分和线上切换是另一个验收层；完成前保持待验收，不发布私有主机、域名、凭据或运维路径。长期浸泡、独立物理机性能对比、第三方 DNS/ACME 和全部平台安装器未在本记录中宣称通过。
