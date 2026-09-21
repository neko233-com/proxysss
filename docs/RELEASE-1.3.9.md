# v1.3.9 稳定版本验证

2026-09-21 用户明确要求尽快发布可用的稳定版本，性能不必超过 nginx。本次 v1.3.9 豁免严格性能胜出和跨主机性能证据，功能、稳定性与打包检查保留。CDN 后续优化优先小文件，大、小文件择一胜出即可，所有下载功能继续覆盖。面向用户的 [HTML 说明](release-1.3.9.html) 与本页同步维护。

## Linux 已完成

运行代码提交为 a58e2ffd2b7fec545b42c259bb315c38282ade81。Ubuntu 24.04 x86_64 Docker 中连续两轮各 388 项 Rust 测试通过，Clippy、格式、Go/文档检查、真实 CPU 约束及 CLI 全场景验证通过。测试覆盖 HTTP/1、HTTP/2、TLS、HTTP/3、代理、WebSocket、TCP/UDP、静态下载、条件/多段 Range、鉴权、重载、资源回收及内嵌脚本等现有套件。

黑名单新增验证覆盖重启期限、并发快照、写入失败、取消责任和真实管理封禁；同一 HTTP/1 连接及 HTTP/2、AI 路由、新 WS 握手执行更新后的策略，既有 WS 继续收发。Linux 文件夹具使用独占原生 Docker volume，源码仍为 Windows 共享目录；共享 v9fs 的 rename 可见性不能作为 Linux 本地磁盘语义。

无插桩 1×/2×/4× 混合矩阵同时运行 11 类负载；18 个测量窗口、198 个原始样本零错误，132 个等负载样本最低完成率 98%。活跃测量 18 秒，矩阵总编排 172 秒；这不是长期浸泡或独立物理机验证。4× cgroup current/peak：proxysss 89264128/296955904 bytes，nginx 208248832/409784320 bytes；没有把测量后进程内存冒充活跃连接持有期的每连接成本。

小文件吞吐比为 nginx 的 1.863/1.997/1.730 倍，大文件为 0.714/0.640/0.679 倍；小文件 p99 与部分 HTTP/长连接延迟仍落后。保留这些差距，不能宣传所有场景性能最优。

功能与矩阵摘要保存在本地 `.benchmark/blacklist-persistence-functional/`，完整原始矩阵在 `.benchmark/direct-ubuntu24-amd64/goal-blacklist-persistence/`。固定 Linux release SHA256：`5e11677be0544f0bf5b4f1f22ec3e0a24c0fc1a1136ac04db9c9ebed85f22711`。这是功能代码的本地验证二进制，平台发布包会独立构建并提供自身摘要。

## Windows 与发布检查

本机 Windows `cargo test --locked` 已通过：361 项测试，零失败，测试执行 22.09 秒；没有运行 Windows 性能矩阵。日志保存在本地 `.tmp/goal-v1.3.9-windows-tests.log` 并归档至 `.benchmark/v1.3.9-release/`。GitHub Release 的发布步骤依赖同一提交的六平台打包全部成功，最终附件以 [v1.3.9 Release](https://github.com/neko233-com/proxysss/releases/tag/v1.3.9) 为准。没有把未执行的安装器、第三方云服务或长期线上运行写成已验收。
