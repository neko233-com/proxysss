# v1.3.13 全静态 Linux 发布验证

2026-11-22 用户要求 Linux release 二进制不要依赖 glibc，并减少环境依赖、保持高性能。v1.3.13 将 Linux 资产切换为完全静态的 musl 构建，修复 Debian 12（glibc 2.36）、Rocky Linux 9 / AlmaLinux 9（glibc 2.34）等环境无法启动旧二进制的问题。对应 [HTML 记录](release-1.3.13.html) 同步维护。

本次用户明确授权发布门禁不要求新增性能测试与 nginx 对比，因此不宣称性能胜出、跨主机性能或容量达标。静态链接、Linux 全量功能、跨发行版真实启动、Windows 快速测试、六平台打包和发布后真实安装仍是发布门槛。发布完成后补充了 v1.3.12 → v1.3.13 同机对照，作为“静态化是否牺牲性能”的可复现说明，结果见[发布后性能对照](#发布后性能对照补充说明不构成发布门禁)。

## 发布形态

- Linux 目标固定为 `x86_64-unknown-linux-musl` 与 `aarch64-unknown-linux-musl`，资产名保持 `proxysss-linux-amd64.tar.gz` / `proxysss-linux-arm64.tar.gz`。
- musl 构建启用 `mimalloc` 全局分配器；glibc/macOS 构建继续使用 `tikv-jemallocator`，避免为了可移植性牺牲分配性能。
- 构建产物不链接宿主 glibc、`ld-linux`、OpenSSL、zlib 或其他共享库。`scripts/package-release.ps1` 对每个 Linux 归档硬断言 `readelf -d` 无 `NEEDED`、`readelf -l` 无 `INTERP`、`file` 报告 `statically linked` 或 `static-pie linked`。
- CI、release 与 deploy 的 Linux 矩阵同步使用 musl；Linux runner 安装 `musl-tools`，aarch64 将 `musl-gcc` 暴露为 `aarch64-linux-musl-gcc`。Windows、macOS、MSI 和资产命名流程不变。

## 源码与本地验证

- Linux musl 全量测试：`454 passed / 0 failed`，使用容器独占的原生 ext4 测试卷，未把 Windows 共享目录的替换语义当作 Linux 本地文件系统。
- Windows 本机完整测试：`425 passed / 0 failed`，执行 22.52 秒；使用 VS2022 BuildTools MSVC 环境，覆盖 HTTP、HTTP/2、HTTP/3 校验、SSE、WebSocket、TCP/UDP、WebDAV、静态/Range、安全与脚本路径。
- `cargo clippy --locked --workspace --all-targets -- -D warnings` 通过；`cargo fmt --all -- --check` 通过。
- 文档链接校验与 Go 文档/基准辅助测试通过。

## 本地候选二进制身份

| 平台 | 文件大小 | 链接形态 | SHA256 |
| --- | ---: | --- | --- |
| Linux x86_64 musl | 23,358,936 bytes | `ELF 64-bit LSB pie executable`, `static-pie linked`，无 `NEEDED` / `INTERP` | `9fad3d63cb982369cd06b6b572cdfc07f0bfc4c50a978ebeeca60f83e82d6400` |
| Linux aarch64 musl | 18,926,016 bytes | `ELF 64-bit LSB executable`, `statically linked`，无动态段 | `ea8e9c48843a41ff20ddc4e6c9feb547fb5347e240a6f03bd4cc14fe35ad9a41` |

x86_64 产物 BuildID 为 `2a646436b62f0c2f13be1ce8ccabfbcc4cea17b7`；`proxysss --version` 输出 `proxysss 1.3.13`。以上摘要属于本地最终构建候选，不能代替 GitHub Release 正式附件。

## GitHub Release 实际附件验收（已完成）

2026-11-22 03:58 CST 发布 `v1.3.13`，页面为 <https://github.com/neko233-com/proxysss/releases/tag/v1.3.13>，`gh release list` 显示为 Latest。tag 指向 `9db4f53`；Release workflow `44942784323` 成功，PR CI `44938896247` 与 main CI `44940521644` 的六平台构建均成功。

正式附件共 8 个，全部下载回本地后与 GitHub API `digest` 逐项比对一致：

| 附件 | 大小 | SHA256 |
| --- | ---: | --- |
| `proxysss-linux-amd64.tar.gz` | 10,328,184 bytes | `A8B7CDA3999BE78867DABA9F95F37EA3CEADE5D5513E3534D03F9CB4AD2E5A75` |
| `proxysss-linux-arm64.tar.gz` | 9,274,850 bytes | `43270CEA7765D8413739D53982E66579E11CC0A7D77136E41DFD8035FA1AE302` |
| `proxysss-windows-amd64.zip` | 9,427,226 bytes | `1E0090605092ABDA57E77E9AEC870A37066A2633EC1B56084D034053015F2EAA` |
| `proxysss-windows-arm64.zip` | 9,049,307 bytes | `830B1047FF84A8DCB96AF55A5E22A9AA08049AEFDED6BE2BE4DFB2D314D98013` |
| `proxysss-darwin-amd64.tar.gz` | 9,604,123 bytes | `5CF3A6CE3A2463BD493EB03794AC958B8DAA2091B1B0DD4EC2AA21310A916137` |
| `proxysss-darwin-arm64.tar.gz` | 8,735,943 bytes | `6EB5882B483A2FCE18C08FF3CB4E6377D97876C446E9CBA850146B2E6072F480` |
| `proxysss-windows-amd64.msi` | 7,364,608 bytes | `F055DADC45B939550AAA91452F818BE330B0A655DAB714A13BCB100DB61D5A7F` |
| `proxysss-windows-arm64.msi` | 6,860,800 bytes | `CAA3AA008194F47110F02E11F8922CE884F91B43BEFA89F1B23D50EB38100DB5` |

正式 Linux 归档解包后，`proxysss` 与 `scripts/install.sh` 均为 `-rwxr-xr-x`，不依赖 Windows 侧权限修补。归档内二进制身份如下：

| 平台 | 文件大小 | 链接形态 | BuildID | SHA256 |
| --- | ---: | --- | --- | --- |
| Linux x86_64 musl | 23,965,336 bytes | `ELF 64-bit LSB pie executable`, `static-pie linked` | `2947d03abdaf17d138ad7ad34d4cb1f662453784` | `7c947768d65e1657963d990115adba2282e7bff9d0e2de9fae8d55931379f63c` |
| Linux aarch64 musl | 18,979,440 bytes | `ELF 64-bit LSB executable`, `statically linked` | `d9f1dea88b593f7eb18e895a62cb8b4e8ad5d273` | `a8e2610de2c79894280ac7bdfce99b88f9ad56ea5d5ffff6311084c2ce342d94` |

## 正式附件运行验收

- `readelf -d` 对 x86_64 与 aarch64 均无 `NEEDED`，`readelf -l` 均无 `INTERP`；`file` 分别报告 `static-pie linked` 与 `statically linked`；`ldd` 分别报告 `statically linked` 与 `not a dynamic executable`。
- 正式 x86_64 归档在 Debian 12、Rocky Linux 9、AlmaLinux 9、Ubuntu 24.04、Alpine 3.20 五个最小容器中真实启动，均返回 `PASS http=200 https=200 h2=2 range=206 head=200 metrics=200 admin_healthz=200`。
- 正式 aarch64 归档通过 `qemu-aarch64-static` 执行同等启动与协议冒烟，返回同一串 PASS 指标；这不等于真实 arm64 物理机性能或全部硬件兼容性。
- Debian 12 干净容器用归档内官方 `scripts/install.sh --version v1.3.13 --skip-init --no-service-restart` 完成真实下载与安装：公开下载地址可用，`/usr/local/bin/proxysss --version` 为 `proxysss 1.3.13`，安装后的文件为 `static-pie linked`。容器无 systemd，因此 `service install` 打印了预期的 user daemon 重载提示，但安装步骤正常完成。
- 同一正式 x86_64 二进制在 Debian 12 执行 `proxysss init` 与 `check-config`：自动生成证书和 `~/.config/proxysss/proxysss.yaml`，配置检查通过。
- 正式 `proxysss-windows-amd64.zip` 解包后 `proxysss.exe --version` 输出 `proxysss 1.3.13`，PE 架构为 AMD64；`proxysss-windows-arm64.zip` 内 `proxysss.exe` PE 架构为 ARM64。
- 两个正式 MSI 均以 `msiexec /a` 管理安装模式成功解包，产品版本 1.3.13；x64 MSI 解包出的 `proxysss.exe` 可直接运行并输出 `proxysss 1.3.13`，arm64 MSI 内 `proxysss.exe` PE 架构为 ARM64。
- 两个 macOS 归档内 `proxysss` 与 `scripts/install.sh` 权限均为 `-rwxr-xr-x`，二进制大小分别为 21,359,584 / 18,078,736 bytes；macOS 构建在 CI 中通过。当前验收机为 Windows，未声称 macOS 二进制已在本机运行。

## 本地候选跨发行版启动结果

同一 x86_64 静态二进制在以下最小环境真实启动，均返回：

```text
PASS http=200 https=200 h2=2 range=206 head=200 metrics=200 admin_healthz=200
```

- Debian 12 `debian:bookworm-slim`
- Rocky Linux 9 `rockylinux:9`
- AlmaLinux 9 `almalinux:9`
- Ubuntu 24.04 `ubuntu:24.04`
- Alpine 3.20 `alpine:3.20`

aarch64 产物通过 `qemu-aarch64-static` 执行同等启动与协议冒烟，也返回同一串 PASS 指标。QEMU 结果证明静态加载与运行时路径，不等同于真实 arm64 物理机性能或全部硬件兼容性。

## 发布后性能对照（补充说明，不构成发布门禁）

用户在 v1.3.13 发布时授权不做新增性能测试。发布后为回答“零 glibc 静态化是否牺牲性能”，补做了 v1.3.12（官方 glibc 动态二进制）与 v1.3.13（官方 static-pie musl + mimalloc 二进制）的同机对照。原始 JSON 保存在本地 `.tmp/perf-results/`，负载器为 Go 原生 HTTP/1.1 keep-alive 客户端；结果只说明本机同配额差异，不构成 nginx 对比或跨主机证据。

### 测试条件

| 项 | 值 |
| --- | --- |
| 主机 | AMD Ryzen 9 7900X3D，24 logical CPUs，64GB RAM |
| 虚拟化 | Docker Desktop VM，2 CPUs |
| 网关容器 | Ubuntu 24.04，`linux/amd64`，`--cpus=1` 或 `--cpus=2`，`--memory=2g` |
| 版本 | v1.3.12 官方 glibc 动态；v1.3.13 官方 static-pie musl + mimalloc |
| 配置 | `runtime.performance.enabled=false`，admin 关闭，日志 warn/error，访问日志关闭 |
| 负载 | 1KB `small.bin`、64KB `large.bin`；3s warmup + 10s 测量；HTTP/1.1 keep-alive，16 workers |

### 单核静态（小文件 5 轮、大文件 3 轮，取中位）

| 场景 | v1.3.12 | v1.3.13 | 吞吐变化 | p50 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1KB 静态 | 20,001 RPS | 20,071 RPS | +0.35% | 1.01 → 1.01 ms | 2.04 → 2.03 ms |
| 64KB 静态 | 15,012 RPS | 15,100 RPS | +0.59% | 1.06 → 1.06 ms | 2.04 → 2.04 ms |

### 双核静态与反向代理（取中位）

| 场景 | v1.3.12 | v1.3.13 | 吞吐变化 |
| --- | ---: | ---: | ---: |
| 1KB 静态 | 20,079 RPS | 20,137 RPS | +0.29% |
| 64KB 静态 | 15,121 RPS | 14,978 RPS | -0.95% |
| 1KB 反向代理 | 9,220 RPS | 9,110 RPS | -1.19% |

双核 64KB 样本分布在 14,575–15,233 RPS，反向代理样本分布在 8,789–9,417 RPS，上述差异未超出重复样本波动；反向代理 p50 保持 3.66–3.70 ms。

### 启动与内存

| 指标 | v1.3.12 | v1.3.13 |
| --- | ---: | ---: |
| 容器启动到首个 HTTP 200（5 次中位） | 35 ms | 36 ms |
| 空闲 cgroup `memory.current`（3 次中位） | 8.07 MB | 19.81 MB |
| 设置 `MIMALLOC_ARENA_EAGER_COMMIT=0` 后 | — | 8.00 MB，吞吐与默认持平 |

mimalloc 默认 arena 采用 eager commit。对内存敏感的小型 VPS，可在服务环境设置 `MIMALLOC_ARENA_EAGER_COMMIT=0`；实测静态与反向代理吞吐不变，cgroup 常驻回到 v1.3.12 水平。该选项属于运行时调优，默认安装不强制改写环境。

### 结论

- 零 glibc 静态化没有造成吞吐或 p50 回退；单核受限下 1KB/64KB 静态中位分别 +0.35%/+0.59%，双核与反向代理差异落在重复样本噪声带内。
- 这是边际收益和“没有为兼容性牺牲性能”的证据，不是全面超越 nginx 或跨主机容量证明，不改变本次发布不以性能胜出为门禁的授权口径。

## 发布门禁状态

- 静态链接门禁：通过。x86_64/aarch64 正式二进制均无 `NEEDED` 与 `INTERP`。
- Linux 全量功能：通过，`454 passed / 0 failed`。
- 五发行版启动：通过，正式 x86_64 附件全部返回同一串 PASS 指标。
- arm64 运行路径：通过 QEMU 同等冒烟。
- Windows 快速测试：通过，`425 passed / 0 failed`；正式 zip 与 MSI 均完成架构/版本验收。
- 六平台打包：通过，8 个正式附件全部可下载且 SHA256 一致。
- 发布后真实安装：通过，Debian 12 使用官方 `install.sh` 安装并运行 `proxysss 1.3.13`。
- 新增性能测试与 nginx 对比：发布门禁未要求，发布后已补充 v1.3.12 → v1.3.13 同机对照，结果见上文；不宣称 nginx 胜出或跨主机性能。

v1.3.13 已满足本次发布要求。
