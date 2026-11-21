# v1.3.13 全静态 Linux 发布验证

2026-11-22 用户要求 Linux release 二进制不要依赖 glibc，并减少环境依赖、保持高性能。v1.3.13 将 Linux 资产切换为完全静态的 musl 构建，修复 Debian 12（glibc 2.36）、Rocky Linux 9 / AlmaLinux 9（glibc 2.34）等环境无法启动旧二进制的问题。对应 [HTML 记录](release-1.3.13.html) 同步维护。

本次用户明确授权不新增性能测试与 nginx 对比，因此不宣称性能胜出、跨主机性能或容量达标。静态链接、Linux 全量功能、跨发行版真实启动、Windows 快速测试、六平台打包和发布后真实安装仍是发布门槛。

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

## 二进制身份

| 平台 | 文件大小 | 链接形态 | SHA256 |
| --- | ---: | --- | --- |
| Linux x86_64 musl | 23,358,936 bytes | `ELF 64-bit LSB pie executable`, `static-pie linked`，无 `NEEDED` / `INTERP` | `9fad3d63cb982369cd06b6b572cdfc07f0bfc4c50a978ebeeca60f83e82d6400` |
| Linux aarch64 musl | 18,926,016 bytes | `ELF 64-bit LSB executable`, `statically linked`，无动态段 | `ea8e9c48843a41ff20ddc4e6c9feb547fb5347e240a6f03bd4cc14fe35ad9a41` |

x86_64 产物 BuildID 为 `2a646436b62f0c2f13be1ce8ccabfbcc4cea17b7`；`proxysss --version` 输出 `proxysss 1.3.13`。以上摘要属于本地最终构建候选；GitHub Release 实际附件须在发布后重新下载、核对摘要并安装验证。

## 跨发行版启动结果

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

## 发布后必须完成

tag 工作流生成的资产下载后，必须再次执行：

```bash
readelf -d proxysss   # 不得出现 NEEDED
readelf -l proxysss   # 不得出现 INTERP
file proxysss         # statically linked 或 static-pie linked
sha256sum proxysss
```

然后在 Debian 12 干净容器中用官方归档完成安装、启动和 HTTP/HTTPS/HTTP2/Range/HEAD/metrics 冒烟。只有下载后的官方附件通过，才可宣称发布成功；排队、构建中或仅有 tag 不等于交付完成。
