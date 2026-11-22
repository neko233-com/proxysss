# v1.3.15 双核调度、静态 sendfile 分界与 Linux 2C2G 对照

2026-11-22 用户要求继续优化 proxysss、修复不合理点，在 Debian 12 与 Ubuntu 24.04 LTS 上快速验证，并希望在 2C2G 限制下总体性能高于 nginx；全部落地后按既有授权直接发布。v1.3.15 只调整 Linux 双核明文 HTTP 调度、`small` 静态对象分界和对应测试夹具，不改变协议语义。对应 [HTML 记录](release-1.3.15.html) 同步维护。

## 1. 双核调度修正

`small` 原先在双核 Linux 上共用按数据核预算扩展 worker 的 Tokio runtime。较早的同机对照中，长连接在共享 runtime 上跨核 work stealing，1 KiB 静态明显低于每核 pinned 的 `balanced`；根因是 keep-alive 连接反复迁移/唤醒，不是 worker 数不足。

v1.3.15 的边界：

- 仅 Linux、`runtime.performance.enabled=true`、数据核预算为 2、`traffic_profile: small` 时，明文 HTTP 使用每核 pinned 单线程 runtime。
- 每个 runtime 上建立两个 `SO_REUSEPORT` 接受队列，总计四个；四队列只降低长连接 hash 放置方差，不增加 runtime 数。实测 2 队列和 8 队列都更差，未采用。
- TLS accept worker 数与明文 accept 数解耦，TLS 仍按数据核预算建立。
- 1 核继续使用共享 runtime；3 核及以上保留共享 runtime 吸收稀疏 placement；`balanced`/`bulk` 既有规则不变。

## 2. `small` 静态 32 KiB sendfile 分界

原 `small` 对 32 MiB 以下完整静态对象缓存正文，64 KiB 响应每请求走用户态写。v1.3.15 将分界改为 32 KiB：

- 小于 32 KiB：有界正文缓存 + 预构建 HTTP/1 keep-alive 完整响应。
- 大于等于 32 KiB：完整 GET 在 HTTP owner 内同步 sendfile，使用既有版本锚点、fd 池、EAGAIN 背压和 `TCP_CORK`。
- 大文件 sendfile reactor 仍只属于 `balanced`/`bulk`；`small` 不启用 reactor。
- sendfile 空闲 fd 池按每个数据核 32 个、最高 256 个预留；双核容量 64，覆盖 64 条并发 keep-alive 的 64 KiB 静态请求，池耗尽仍保持版本锚点语义。

## 3. Linux 2C2G 最终对照

环境：Ubuntu 22.04.5 LTS，Intel Xeon 6986P-C，`nproc=2`，cgroup `cpu.max=200000 100000`（硬限制 2 CPU）；物理/ cgroup 内存 4 GiB，单个网关进程用 `prlimit --as=2147483648` 限制 2 GiB 地址空间。proxysss RSS 约 17 MiB，未出现 OOM。**这是 2 vCPU + 2 GiB 地址空间限制，不是严格 2 GiB cgroup 内存限制。**

对照为 nginx 1.28.0 源码构建：2 worker、`reuseport`、`sendfile`/`tcp_nopush`/`tcp_nodelay`、`open_file_cache`、upstream `keepalive 128`、默认 `proxy_buffering`。两侧同一后端、fixture、Go 负载器、64 条预连接、2 秒预热 + 5 秒测量、3 轮交替顺序；每请求校验状态与正文长度，共 2,888,924 次成功响应、0 错误。

| 场景 | nginx 1.28 RPS | proxysss RPS | 比率 | nginx p50 / p95 / p99 ms | proxysss p50 / p95 / p99 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 KiB 静态 | 47,272.4 | 51,894.0 | **1.098x** | 0.979 / 3.865 / 5.666 | 0.891 / 3.464 / 5.018 |
| 64 KiB 静态 | 32,375.4 | 37,247.4 | **1.150x** | 1.359 / 5.034 / 8.023 | 1.375 / 4.493 / 6.373 |
| 1 KiB 反向代理 | 28,334.4 | 51,399.2 | **1.814x** | 2.002 / 4.483 / 6.074 | 0.894 / 3.531 / 5.153 |
| **三项等权合计** | **107,982.2** | **140,540.6** | **1.302x** | — | — |

三个场景的吞吐中位数均高于 nginx 1.28；静态 1 KiB 高约 9.8%，静态 64 KiB 高约 15.0%，反向代理高约 81.4%。静态 1 KiB 三项延迟均低于 nginx；64 KiB 的 p50 略高 0.016 ms，p95/p99 更低；反代三项延迟均明显更低。

原始 JSON、配置、脚本与环境说明：

- `performance-evidence/2c2g-http-linux-2c2g-final/`
- `raw/final-r{1,2,3}-{nginx128,proxysss}-{static-1k,static-64k,proxy-1k}.json`

单机 loopback 结果只适用于声明环境，不是跨主机、真实网卡或严格全协议混合负载证据；不能据此宣称所有生产路径必胜。

## 4. Debian 12 / Ubuntu 24.04 快速验收

`scripts/verify-debian-ubuntu-fast.ps1` 对当前静态二进制执行：

1. `file`、`readelf -d` 无 `NEEDED`、`readelf -l` 无 `INTERP`。
2. `debian:bookworm-slim` 与 `ubuntu:24.04` 容器各 `1 CPU / 512 MiB` 启动真实二进制。
3. `--version`、`check-config`、HTTP 200、HEAD 200、Range 206/100 字节、HTTPS 200、管理 `/healthz` 200。

本轮结果：

```text
PASS Debian 12: proxysss 1.3.15; HTTP/HTTPS/Range/HEAD/healthz OK
PASS Ubuntu 24.04 LTS: proxysss 1.3.15; HTTP/HTTPS/Range/HEAD/healthz OK
FAST VERIFY PASS in 3.4s
```

候选二进制 `proxysss-v1315`：`ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped`，BuildID `d060eab94394714abe57cc552769b671581ea9b8`，SHA-256 `570e5188b8adfe1d689fe731c120a86371f930647ba5b2ec18a1496dca6be539`，`readelf -d` 无 `NEEDED`、`readelf -l` 无 `INTERP`。该脚本是秒级冒烟入口，不替代 Rocky/Alma/Alpine 跨发行版启动、完整功能矩阵或生产目标验收。

## 5. 测试

- Windows 本机 `cargo test --locked`：`425 passed / 0 failed / 0 ignored`。
- Linux 本机 `cargo test --locked`：`454 passed / 0 failed / 0 ignored`，32.57 秒。
- 测试夹具支持 `PROXYSSS_TEST_TEMP_BASE`（仅接受绝对路径，落到 `<base>/tests`），使 Linux 容器/本机夹具可放在项目外 overlayfs，避免 Windows v9fs 的并发快照 `ENODATA`。
- 修复 `filesystem_boundary` 的相对根路径测试：当夹具位于 cwd 之外时，按路径组件生成真实相对路径，不再假设 `strip_prefix(cwd)` 一定成功；产品路径边界逻辑未改。

## 6. 发布资产与限制

发布前重新构建 `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl` 完全静态产物，执行静态链接检查、Debian 12 / Ubuntu 24.04 冒烟、六平台打包、GitHub Release 和资产验收。本文档不以同机 loopback 数据替代生产跨主机证据；2 GiB 口径明确为地址空间限制，不包装成严格 cgroup 内存测试。
