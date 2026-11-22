# v1.3.16 独立双容器 2C2G 复测、调度回退与发布资产收缩

2026-11-22 用户否决单机 loopback 和 Windows host client，要求用 Docker 模拟两台独立 Linux 2C2G 机器：一台服务器、一台客户端，经 bridge 网络做真实 HTTP/1.1 keep-alive 对照。v1.3.15 的每核 pinned 四队列在该环境下被推翻；本版改为共享 Tokio runtime + 两个 `SO_REUSEPORT` 队列，并把发布资产从 8 个收缩到 5 个。对应 [HTML 记录](release-1.3.16.html) 同步维护。

## 1. 双容器环境与口径

- 服务器容器：`nginx:1.28`，`--cpus=2 --memory=2g`；后端 nginx 与候选网关都在同一服务器容器内，网关监听 `19080`，后端监听 `19081`，不发布宿主端口。
- 客户端容器：`debian:bookworm-slim`，`--cpus=2 --memory=2g`；Go HTTP/1.1 负载器 `GOMAXPROCS=2`，64 条预连接。
- 网络：Docker bridge `proxysss-bench-two-host`，客户端按服务器容器 DNS 名访问；同一轮只启动一个候选网关。
- cgroup：服务器与客户端实测 `cpu.max=200000 100000`、`memory.max=2147483648`。
- 样本：2 秒预热 + 5 秒测量，3 轮交替候选顺序，报告中位 RPS；每请求校验状态码与正文长度。

这是 Docker bridge 双容器证据，不是物理双机或跨主机生产证据，也不能外推全协议混合负载。原始 JSON、环境说明和汇总在 `performance-evidence/2c2g-http-docker-two-host-two-host-final/`。

## 2. 正式结果

候选二进制 SHA-256 `c28df6663667ad9e41995279e067112d61842382f75e832867a55ba0cdd2e037`，BuildID `8ae11dcfec01f2b9174f90cc3c6d7de98c684826`，23,355,040 B，ELF static-pie stripped。

| 场景 | nginx RPS | proxysss RPS | 比率 | nginx p50 / p95 / p99 ms | proxysss p50 / p95 / p99 ms |
| --- | ---: | ---: | ---: | --- | --- |
| static-1k | 73,955 | 88,555 | **1.197x** | 0.784 / 2.282 / 3.206 | 0.620 / 1.891 / 2.621 |
| static-64k | 44,072 | 54,208 | **1.230x** | 1.397 / 3.774 / 4.773 | 1.071 / 2.960 / 3.909 |
| proxy-1k | 43,933 | 88,490 | **2.014x** | 1.216 / 2.171 / 20.549 | 0.641 / 2.022 / 2.921 |

三项等权合计：nginx `161,960`，proxysss `231,253`，**1.428x**。共 `5,690,455` 次成功响应，0 错误。

## 3. 为什么撤回 pinned 四队列

v1.3.15 的同机 loopback 结论是：双核 `small` 使用每核 pinned 单线程 runtime + 四个 `SO_REUSEPORT` 队列，1 KiB 静态 1.098x。独立双容器复测显示同一配置只有 `0.851x / 1.024x / 1.214x`。根因不是静态 sendfile，而是 64 条长连接在 `SO_REUSEPORT` 上的放置偏斜：pinned shard 无法把 ready 任务迁到空闲核，单核被压满，另一核空闲。

对照过程：

| 配置 | static-1k | static-64k | proxy-1k |
| --- | ---: | ---: | ---: |
| v1.3.15 pinned 四队列 | 0.851x | 1.024x | 1.214x |
| 同二进制 `bulk` 共享 runtime | 1.307x | 1.250x | 2.082x |
| 共享 runtime + 四队列 | 0.985x | 1.174x | 1.415x |
| 共享 runtime + 两队列（采用） | 1.114x | 1.370x | 1.686x |
| 最终正式三轮 | 1.197x | 1.230x | 2.014x |

代码边界：

- 仅 `balanced` 保留每核独立 runtime；`small` 与 `bulk` 在 2 核也使用共享 runtime，允许 work stealing 重平衡 keep-alive 任务。
- `plain_http_accept_worker_count_for` 删除 `small && cores == 2` 固定返回 4 的分支，2 核统一为 2 个 `SO_REUSEPORT` 队列。
- `small` 32 KiB 静态分界、同步 sendfile、fd 池 32/核、版本锚点、EAGAIN 背压不变。
- 单测同步：`Small,2` sharding 为 false，accept worker 为 2。

## 4. 发布资产收缩为 5 个

8 个资产不是 8 个 Linux 发行版，而是 6 个 Rust 目标 + 2 个 Windows MSI。本版收缩为：

| 资产 | 说明 |
| --- | --- |
| `proxysss-linux-amd64.tar.gz` | `x86_64-unknown-linux-musl`，完全静态，跨 glibc 发行版通用 |
| `proxysss-linux-arm64.tar.gz` | `aarch64-unknown-linux-musl`，完全静态 |
| `proxysss-darwin-arm64.tar.gz` | Apple Silicon macOS |
| `proxysss-windows-amd64.zip` | Windows x86_64 便携包，静态 CRT |
| `proxysss-windows-amd64.msi` | Windows x86_64 当前用户安装包 |

移除项与理由：

- `proxysss-darwin-amd64.tar.gz`：Intel Mac 是存量设备，Apple Silicon 已是新机默认；每次发版单独跑 Intel runner 的成本与收益不匹配。需要 Intel 包时从源码 `cargo build --release --target x86_64-apple-darwin`。
- `proxysss-windows-arm64.zip` / `.msi`：Windows ARM64 仍是小众目标；x64 包可在 Windows 11 ARM 上模拟运行。需要原生 ARM64 时从源码构建。
- Linux 不能合并成一个“通用包”：静态 musl 已解决发行版/glibc 兼容，但 x86_64 与 aarch64 是不同 CPU 架构，必须各有一个二进制。也不需要按 Debian/Rocky/Alpine 分别发版。

## 5. 发布包可移植性

- Linux release 继续使用 `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl`，`mimalloc` 分配器；打包脚本硬检查 `readelf -d` 无 `NEEDED`、`readelf -l` 无 `INTERP`、`file` 为 `statically linked` / `static-pie linked`。
- 同一 x86_64 最终二进制（SHA-256 `0a1e51e7…`）已在 Debian 12、Ubuntu 24.04、Rocky Linux 9、AlmaLinux 9、Alpine 3.21 的官方容器 rootfs（WSL chroot，非 Docker daemon）中启动，五发行版全部通过：`--version`、默认欢迎页 GET 200、真实静态文件 HEAD 200、Range 206 + 100 字节、`/healthz` 200、自签 HTTPS GET 200。默认 `/` 是生成的 welcome 响应，不支持 Range，Range 断言针对注入的静态站点文件。
- Windows release 使用 `-C target-feature=+crt-static`；`package-release.ps1` 用 `dumpbin /dependents` 拒绝 `VCRUNTIME*`、`MSVCP*`、`ucrtbase.dll`、`api-ms-win-crt-*`。CI 和 deploy 与 release 使用同一 `rustflags`。
- Windows ZIP 改用 PowerShell 内置 .NET `System.IO.Compression.ZipArchive`（Optimal 压缩）生成，不再调用系统 `tar.exe`：彻底消除对宿主 bsdtar/GNU tar 版本与 PATH 的强依赖，系统/runner 镜像升级不影响打包。UTF-8 中文文件名与确定性时间戳（2000-01-01）由 .NET 忠实写入，完整文档包本地实测约 77ms。v1.3.16 前两次 CI 预演因 runner 上 bsdtar 静默 exit 1 失败，重构后不再依赖外部归档二进制。
- `scripts/install.ps1` 的 `Test-ServiceEntryExists` 真正查询计划任务，不再引用未定义变量。

## 6. 测试与发布门禁

- Windows 本机 `cargo test --locked`：425 passed / 0 failed（VS2022 BuildTools 14.44 + Windows SDK 10.0.26100；本机 VS2026 的 MSVC 头文件缺失，改用 BuildTools 工具链）。
- Linux `cargo test --locked --target x86_64-unknown-linux-musl`：454 passed / 0 failed（WSL Alpine 3.24，rustc 1.100.0，本版最终代码实测）。
- `cargo fmt --check` 通过。
- 最终 Linux 二进制 SHA-256 `0a1e51e768bda76f315a61218a03bfc02abba71eb1a1846e139df3db4d8612df`，23,494,288 B，`file` 为 static-pie linked，`readelf` 无 `NEEDED` / `INTERP`，`--version` 输出 `proxysss 1.3.16`。
- 发布工作流只打包，不跑性能基准；tag 前必须匹配 `Cargo.toml` 版本与 `CHANGELOG.md` 的 `## v1.3.16`。

## 7. 边界

- 双容器结果只覆盖 static-1k、static-64k、proxy-1k 三个 HTTP/1.1 场景，不是物理双机、真实网卡、TLS/H2、WebSocket、TCP、UDP 或全协议混合负载证据。
- 5 个资产是产品发布矩阵，不表示只支持这些平台源码构建；Intel macOS 与 Windows ARM64 仍可按文档从源码构建。
- 性能结论不能从一个 2C2G 环境外推到其他核数、其他发行版或其他网关功能组合。
