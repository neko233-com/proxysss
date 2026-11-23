# 验证与发布流程

2026-10-30 用户要求取消上线前压测门槛：正式发布以功能测试、目标环境验收和发布资产矩阵打包为准，不要求性能胜出或三机性能证据。性能测试保留为可选诊断；未实测的性能或容量不得声称已达标。历史 v1.3.10 的实际验证见 [版本记录](RELEASE-1.3.10.md) / [HTML](release-1.3.10.html)。

2026-11-22 用户要求 Linux release 二进制不再依赖宿主 glibc，并以更少环境依赖的方式发布。v1.3.13 使用完全静态 musl 构建；本次不要求新增性能测试，但静态链接检查、Linux 全量功能、Debian 12 / Rocky/Alma 9 / Ubuntu 24.04 / Alpine 实际启动、Windows 快速测试、发布资产矩阵打包和发布后真实安装必须通过。详见 [v1.3.13 验证说明](RELEASE-1.3.13.md) / [HTML](release-1.3.13.html)。

Linux 发布必须兼容 Ubuntu 与 Debian，分别在 Ubuntu 24.04 和 Debian 12 运行真实二进制功能验证；静态发行包还须覆盖 Rocky Linux 9 / AlmaLinux 9 与 Alpine 最小容器。记录发行版、版本、架构、二进制摘要、`readelf`/`file` 结果及命令输出；某环境的成功不能外推其他版本或架构。生产目标另做实际验收。

## 0.0 Windows 本机完整测试

Windows 本机不再依赖手工打开 Developer Command Prompt：`pwsh -NoProfile -File scripts/test-windows.ps1` 自动定位 Visual Studio Build Tools/Windows SDK 的 `vcvars64.bat`，校验 `cl.exe` 与 MSVC `link.exe`（拒绝 coreutils 的同名 shadow），默认先执行缓存清理，再运行 `cargo test --locked`；可追加 cargo test 过滤参数或使用 `-Release`。CI 的 `test-windows` 与 `test-linux-musl` 同步跑全量测试，发布打包矩阵不替代功能测试。

## 0. 测试前缓存清理（强制）

任何功能测试、Docker 验证、基准测试或打包前，必须先清空项目历史构建与测试缓存：

```powershell
pwsh -NoProfile -File scripts/clean-test-cache.ps1
```

Linux 使用 `bash scripts/clean-test-cache.sh`。脚本只清理项目内的 `target/`、`.benchmark/`、`.cache/`、`.tmp/`、`dist/`、根目录测试产物、benchmark 日志和本项目自建 Docker 验证容器/卷，不会执行全局 `docker system prune`。`verify-local.ps1`、`verify-deep.ps1`、`verify-e2e.ps1`、`verify-docker-scenarios.ps1`、`run-all-verify.ps1`、`benchmark-gateways.ps1` 已内置开始前清理；直接运行 `cargo test` 时必须手动执行。测试结束确认 `target/` 与 `.benchmark/` 合计低于 5 GB，避免项目再次增长到 100 GB 级。

Linux 本机或容器测试可设置 `PROXYSSS_TEST_TEMP_BASE=<绝对路径>`，夹具会落到 `<base>/tests`；用于把临时文件放在项目外原生卷，避免 Windows 共享 v9fs 的瞬时 `ENODATA`。脚本拒绝相对路径。

## 0.1 Debian 12 / Ubuntu 24 超快验收

已有当前候选 Linux 静态二进制后，分钟级双发行版入口：

```powershell
pwsh -NoProfile -File scripts/verify-debian-ubuntu-fast.ps1 -Binary .tmp/perf/v1315/proxysss
```

脚本先检查 `readelf -d` 无 `NEEDED`、`readelf -l` 无 `INTERP`，再分别在 `debian:bookworm-slim` 与 `ubuntu:24.04` 容器启动真实二进制，断言 `--version`、`check-config`、HTTP 200、HEAD 200、Range 206/100 字节、HTTPS 200、管理 `/healthz` 200。默认容器 `1 CPU / 512 MiB`，宿主端口 18081/18443/17977，结束后清理 `bench-verify-*`；不替代完整功能矩阵，也不把某发行版成功外推到其他版本或架构。

v1.3.15 本轮结果：

```text
PASS Debian 12: proxysss 1.3.15; HTTP/HTTPS/Range/HEAD/healthz OK
PASS Ubuntu 24.04 LTS: proxysss 1.3.15; HTTP/HTTPS/Range/HEAD/healthz OK
FAST VERIFY PASS in 3.4s
```

## 0.2 独立双容器 2C2G HTTP 对照

2026-11-22 用户否决单机 loopback、Windows host client 和 Docker published-port 数据，要求两台独立 Linux 容器各 2 CPU / 2 GiB，一台服务器、一台客户端，经 bridge 网络压测。正式入口：

```powershell
pwsh -NoProfile -File scripts/bench-2c2g-docker-two-host.ps1 -Binary <Linux二进制>
```

服务器容器使用 `nginx:1.28`，后端 nginx 与候选网关同容器（19080/19081），不发布宿主端口；客户端容器为 `debian:bookworm-slim`，Go HTTP/1.1 负载器 64 条预连接，`GOMAXPROCS=2`。每轮 2 秒预热 + 5 秒测量，3 轮交替候选顺序，每请求校验状态与正文长度。两侧实测 `cpu.max=200000 100000`、`memory.max=2147483648`。

| 场景 | nginx RPS | proxysss RPS | 比率 | nginx p50 / p95 / p99 ms | proxysss p50 / p95 / p99 ms |
| --- | ---: | ---: | ---: | --- | --- |
| static-1k | 73,955 | 88,555 | 1.197x | 0.784 / 2.282 / 3.206 | 0.620 / 1.891 / 2.621 |
| static-64k | 44,072 | 54,208 | 1.230x | 1.397 / 3.774 / 4.773 | 1.071 / 2.960 / 3.909 |
| proxy-1k | 43,933 | 88,490 | 2.014x | 1.216 / 2.171 / 20.549 | 0.641 / 2.022 / 2.921 |

三项等权合计 nginx `161,960`、proxysss `231,253`，**1.428x**；5,690,455 次成功响应、0 错误。原始 JSON、环境与汇总在 `performance-evidence/2c2g-http-docker-two-host-two-host-final/`。

该结果推翻了 v1.3.15 的每核 pinned 四队列：同二进制双容器只有 0.851x / 1.024x / 1.214x，放置偏斜把单核打满而另一核空闲。当前实现为共享 Tokio runtime + 两个 `SO_REUSEPORT` 队列，仅 `balanced` 保留每核独立 runtime。

这是 Docker bridge 双容器证据，不是物理双机或跨主机生产证据，不能外推 TLS/H2、WebSocket、TCP、UDP 或全协议混合负载。

## 0.2.1 单机 loopback 历史对照

Linux 真机最终证据：Ubuntu 22.04.5 LTS，2 vCPU（cgroup `cpu.max=200000 100000`），单个网关进程 `prlimit --as=2147483648`；nginx 1.28.0 源码构建，2 worker、`reuseport`、`sendfile`、`open_file_cache`、upstream `keepalive 128`、默认 `proxy_buffering`。两侧同一后端与 Go 负载器，64 条预连接、2 秒预热 + 5 秒测量、3 轮交替顺序、每请求校验状态与正文长度。

| 场景 | nginx 1.28 RPS | proxysss RPS | 比率 |
| --- | ---: | ---: | ---: |
| 1 KiB 静态 | 47,272.4 | 51,894.0 | 1.098x |
| 64 KiB 静态 | 32,375.4 | 37,247.4 | 1.150x |
| 1 KiB 反代 | 28,334.4 | 51,399.2 | 1.814x |
| 三项等权合计 | 107,982.2 | 140,540.6 | 1.302x |

2,888,924 次响应零错误；三个场景的吞吐中位数均高于 nginx。原始 JSON、配置、`run-final.sh` 与 `env.txt` 位于 `performance-evidence/2c2g-http-linux-2c2g-final/`。

2 GiB 是每进程虚拟地址空间上限，不是严格 2 GiB cgroup 内存限制：沙箱 `memory.max` 只读为 4 GiB，proxysss RSS 约 17 MiB。结果是单机 loopback、声明配置下的证据，不是跨主机、真实网卡或严格全协议混合负载结论。

Windows Docker 的 `scripts/bench-2c2g-http.ps1 -Binary <Linux二进制> -Tag <标签>` 保留为同机诊断入口：默认网关 2 CPU / 2 GiB、后端 1 CPU / 512 MiB、64 条预连接、3 轮中位；Docker Desktop 端口转发和共享文件系统会改变数值，不能作为跨主机或生产性能证据。

## 1. Ubuntu 24 LTS 功能检查

Windows Docker Desktop 在 PowerShell 运行：

```powershell
.\scripts\verify-docker-scenarios.ps1
```

Linux/macOS 使用 `scripts/verify-docker-scenarios.sh`。脚本探测 Ubuntu 24.04 x86_64，检查格式、Clippy、两轮测试、全场景配置、CLI 能力和 nginx 对照声明。固定容器名 `proxysss-verify` 带项目所有者标记；不得清理其他任务的容器。最新报告位于 `.tmp/docker-scenarios/`，测试目录在结束或失败后清理。

文件夹具 `.tmp/tests` 挂载本次容器独占的匿名 Docker volume，验证 Linux 原生文件系统的重命名、权限和链接语义；退出后由 `--rm` 回收该卷。源码、构建缓存与报告仍在工作区，报告记录两种文件系统。不能把 Windows 共享目录的 v9fs 行为当作 Linux 本地磁盘：本机独立诊断中同目录替换会短暂返回 ENOENT，原生 Linux 卷未出现；并发快照测试保持严格断言，不忽略或重试该错误。这不是断电持久性测试，也不代表网络文件系统全部可用。

CPU 功能探针 `scripts/verify-linux-cpu/main.go` 随 Docker 功能检查运行：用 taskset 收窄真实网关的 CPU 范围，验证 HTTP/TCP/UDP 正文、所有线程 affinity，以及 HTTP/native relay/sendfile owner 数量。单独在带 CPU quota 的容器中可使用 `go run scripts/verify-linux-cpu/main.go --binary <Linux二进制> --expected-workers <预算>` 断言 quota 生效；这是功能检查，不是性能胜出证据。检查结果保存为 cpu-confinement.txt。

同一入口也运行 `scripts/benchmark-helper_test.go`，保护性能报告、原始样本汇总和门槛判定；这些 Go 验证只在本地执行，不加入 GitHub Actions。

下载验收包括 GET/HEAD、单段/多段 Range、后缀与开放区间、重叠合并、越界、条件响应、鉴权顺序、超大文件局部读取和截断错误。协议测试应穿过真实监听器，覆盖 HTTP/1.1、HTTP/2 与 TLS。长连接检查双向背压、半关闭、断连清理、超时、重载和 UDP association 上限。现有套件不等于任意第三方客户端、真实云 DNS/ACME 或所有故障组合已验收。

### 1.1 静态 Linux 发布二进制

Linux 发布资产固定使用 `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl`，目标文件不得包含 glibc、`ld-linux`、OpenSSL 或其他宿主共享库。`scripts/package-release.ps1` 对每个 Linux 归档执行以下断言，任一失败即拒绝打包：

```bash
readelf -d proxysss   # 不得出现 NEEDED
readelf -l proxysss   # 不得出现 INTERP
file proxysss         # 必须包含 statically linked 或 static-pie linked
```

tag 前还必须把同一构建产物放进 Debian 12、Rocky Linux 9 / AlmaLinux 9、Ubuntu 24.04 与 Alpine 容器，实际启动网关并完成 HTTP 静态响应、HTTPS、HTTP/2、Range、HEAD、metrics 与管理 healthz；只做 `--version` 不足以证明运行正常。Linux 资源名保持 `proxysss-linux-amd64.tar.gz` / `proxysss-linux-arm64.tar.gz`，升级安装器无需改下载地址。musl 构建启用 `mimalloc`，glibc/macOS 构建继续使用 `jemalloc`。本次静态发布不包含新增性能对比，不能用便携性替代性能证据。

## 2. Linux 混合性能（可选诊断）

在干净提交上执行；Windows 必须使用 Git Bash。仅在明确需要发布证据时设置 `KEEP_BENCH_ARTIFACTS=1` 保存原始工件，证据收集后立即清理 `.benchmark/`；默认测试前清理会删除上一轮缓存：

```bash
KEEP_BENCH_ARTIFACTS=1 REUSE_BENCH_IMAGE=1 \
  bash scripts/benchmark-ubuntu24-amd64-docker.sh
```

角色隔离入口将 gateway、backend、client 放入互不重叠的 cpuset/cgroup，检查 Ubuntu 24.04 x86_64。默认 `balanced`，静态大文件并发从每数据核两条开始；所有场景同时按 1×/2×/4× 放大。nginx 对照为 1.31.2、`-O3 -fno-plt`，双方使用相同 CPU、nofile、sysctl 和等价协议配置。

测量包含小静态、大文件、CDN 热更新、HTTPS 静态、HTTP 反代、SSE、WebSocket、游戏 TCP、通用 TCP、UDP；本机严格入口还包含透明 QCP。QCP 只证明 UDP 转发，不证明解析或终止 QCP 帧。New API 供应商路由与 KCP 特殊封装不参与默认 nginx 对标。

默认每尺度 1 次饱和、2 次顺序相反的等负载采样，共 18 秒活跃测量，上限 20 秒。等负载取较慢网关饱和速率的 25%，延迟使用两次样本中位数；双方完成至少 98% 可执行目标。HTTP/SSE 均匀错峰计划只计严格早于截止点的请求；半整数总量不能把恰在截止点的请求向上计入，没有可执行请求的计划直接报错。HTTP/SSE 使用均匀错峰，UDP/QCP、TCP、WebSocket 分别预留 1/2/5 ms 完成时间。每波提前预留测量预算，进程宽限期不能无限延长。

预连接、每条 HTTP/1 连接的两次完整 GET/SSE 资源请求（H2 为共享会话两次）、每条实时连接的一次 echo、1000 ms 解暂停稳定期和 2000 ms future-start lead 均在活跃测量之外。构建、容器准备、预热和结果处理另记 wall time。客户端与后端进程遵守 `TOKIO_WORKER_THREADS`，等负载 HTTP/SSE 和大文件各使用两个 I/O worker，实时协议一个；混合负载按场景划分互不重叠的 client CPU，饱和时 worker 数等于该场景 CPU 数；只有串行 isolated-saturation 诊断使用全部 client cpuset。单场景 CPU 满载时只能说明当前发生器资源下的比较，不能断言网关容量上限。官方 fixture、解析和验收工具使用 Go 或原生编译工具。

每波结果目录独立保留 `clients.meta`（最后两列为计划 worker 数与 CPU 列表）和各场景的 `*-resources.txt`。发生器在 taskset 后、exec/预热前读取真实 `/proc/self/status` affinity 与 worker 环境值；汇总还核对实际 `bench` 输出的 runtime worker 数。缺失、重复或不一致会使该波失败。`run-metadata.txt` 区分混合场景分区与串行诊断的全 client CPU，不能把容器的 12 核配额写成每个混合发生器的 12 个 worker。此记录不等于整个采样窗口的线程调度追踪。

HTTP/1 每个并发 worker 使用独立、最多一个空闲连接的 client，并先完成 HEAD，再在每条连接上完整读取两次 GET，确保所有声明连接的真实资源路径与 TCP 接收窗口都参与预热；HTTPS/H2 在共享多路复用 client 上预热两次。HTTP 下载逐块消费并计数，不把完整大文件收进发生器内存。`bench http` 默认仅接受 2xx；`--expected-status` 可指定精确状态，`--expected-body-bytes` 校验正文长度。正式 HTTP 夹具要求 200，静态夹具还要求实际文件长度；错误页、短正文和意外长正文不能算吞吐成功。

有限 SSE 使用 --drain-response，逐块读到 HTTP 正文结束才记录成功并复用连接；仍记录首块延迟，字节数包含完整响应，空正文、截断和非成功状态均失败。Go 夹具允许 keep-alive，2 ms 间隔只在事件之间，不在最后一块后额外等待。SSE 的 HTTP/1 每个 worker 同样持有独立 client，预先完成两次响应；HTTP/SSE 测量截止均取消未完成的响应头/正文等待，不延长窗口或计算迟到成功。不带该选项时仍按 --max-chunks 提前结束持续 SSE，这种模式可能关闭 HTTP/1 连接，不能用来声称稳态 keep-alive 容量。

预热错过 `--start-at-unix-ms` 时整波失败，不能自行延后开始而把异步运行当作同时混合负载。双方每轮都必须完成至少 98% 的可执行目标，参考 nginx 不达标同样使比较无效。CDN 预检在测量前依次确认 v1 正文、原子发布 v2、确认双方确实返回 v2；其后测量的是更新生效后的读取吞吐，不声称是一秒内连续发布的性能。

现有严格实验入口仍输出逐项吞吐与 p50/p95/p99 的历史比较门槛；它用于可选诊断，不是发布前提。相对速度落后与功能/运行错误必须分别报告。CDN 优化优先小文件，不要求大、小文件同时胜出。旧共享角色诊断 `benchmark-all-scenarios.sh` 的 `CRITICAL_RATIO=0.97` 和 `MIN_RATIO=0.50` 不等于严格发布通过。内存报告双方 cgroup current、peak、每连接成本，各项不得超过 nginx 的 2 倍；只在声明实际生产预算时设置 MemoryMax。4c/4096 active WSS/20k idle WSS 是参考负载，100k 容量为额外诊断。

每轮原始结果保存在带 `-r1`、`-r2` 等序号的目录，包含逐场景输出和 results.jsonl；后续轮次不得覆盖早期失败详情。无序号场景文件仅为最后一轮的便捷副本。网关、后端与客户端的逐轮容器日志和测量前后网络计数器在活跃窗口之外采集，便于区分 UDP 内核丢包、超时及连接错误。UDP/QCP echo 检查回复长度和内容，失败输出阶段、错误种类和系统错误码，每进程最多打印 8 条详情，额外错误仍计数。计数器或错误详情缺失时，不得从汇总的 errors 数字猜测根因。

ARM Docker 上的 amd64 模拟必须记为 `emulated-amd64`；通过原生 Zig 交叉构建、在目标容器执行后测量。Docker Desktop 的 native-amd64 代表 Linux VM 中原生指令执行，不代表独立物理机。

WSS 容量必须证明连接在整个空闲持有期后仍可用。bench websocket --hold-connections 输出 hold ready 后，控制器才开始等待并采集连接存活期间的 cgroup current/peak；结束时逐条真实 echo，要求 retained 等于 requested 且 hold failures 为 0。只统计曾经 opened、只持有 socket 句柄或在客户端退出后采样，均不能证明容量。nginx 测试 idle timeout 必须长于持有期；活跃 WSS 的共同开始时间只能在所有连接完成 warm-up echo 后发布。Docker 客户端和后端显式按声明 CPU 配额设置 TOKIO_WORKER_THREADS；汇总拒绝缺少存活/内存字段的旧报告，且逐轮限制 current、peak、每连接成本不超过 nginx 的 2 倍。WSS 等负载目标只统计测量窗口内、距结束至少 5 ms 的完整 tick；例如 4096 连接、5 秒、300000 us 间隔的目标是 65536 次，而非无限时长速率乘窗口。CLI 显示可执行 target ops/sec，汇总要求双方目标相同且每轮完成至少 98%；没有完整 tick 的计划不能作为延迟证据。等待 tick、收发及重连都在测量截止时间停止，不把超时后的完成计入吞吐。容量矩阵默认在持有期一半时采在线内存。

重复样本的吞吐和延迟取中位数，错误取最大值，**完成率取最差单轮**。汇总 JSON 保留 `sample_count`、`worst_target_achievement` 与 `worst_target_sample`；Markdown/HTML 显示最低完成率，错误指出最差轮次。完成率由该轮成功次数、测量时长和可执行目标计算，不用四舍五入后的中位吞吐替代。样本数量须符合声明，重复中的协议/并发/时长/目标一致，两边目标也须一致；缺失轮次、缺失最低完成率或已汇总数据再次聚合均不能作为通过证据。例：1032/1056 与 1056/1056 必须按 97.73% 失败，不能按两轮平均的 98.86% 通过。修复报告后重算保留的原始样本不等于重新运行性能测试。

## 3. Windows 功能验收

在本机 Windows 运行 `cargo test --locked`，确认现有单元与集成用例通过；不执行 Windows 性能矩阵。用户当前验收要求以这轮本机测试作为 Windows 快速功能验证；不能把未覆盖的安装器、第三方服务或外部系统场景写成已实测。Windows 桌面成绩不代替 Linux 性能证据。

## 4. Push 与发布

使用 [gh-cli 技能](../skills/gh-cli/SKILL.md)，检查实际 Actions 状态。CI 只做发布资产矩阵打包，禁止云端功能测试和性能 benchmark。Node.js action 使用 24 LTS 或更高，artifact action 至少 v6。

正式版本的 `Cargo.toml` 版本与 `CHANGELOG.md` 标签章节须匹配。先完成 Linux 和 Windows 功能验证，按部署要求打包上传并核验目标环境，再推送 tag；发布资产矩阵打包和 GitHub Release 附件均实际核验后才能报告发布完成。性能证据不再是发布门槛；如作性能比较，应保留可审计原始工件并如实说明环境与限制。

## 诊断发生器瓶颈

先区分网关与客户端资源，再解释接近 1.0 的比值。3efde31 的一次独立 CPU 观察器诊断固定网关/后端/客户端为 4/6/12 核，四个观察器另占 CPU 22–23。18 波全混合、54 个角色窗口的 25 ms 线程采样中，大文件客户端饱和运行时间约 964–993 ms/秒，接近其单核上限；CDN 约 780–941 ms/秒，仍不足以断定每种负载都受客户端限制。等负载 CDN 双 worker 累计调度等待约 212–245 ms/秒，网关 native realtime owner 约 21–28 ms/秒；多个线程等待可重叠，不能当作某次请求的延迟。

该诊断使用固定旧发生器与二进制，窗口边界滞后最多 27.3 ms；退出线程保留最后可见样本，计数可能低估。完整计数、覆盖时间、源码和角色隔离信息保存在本地 `.benchmark/goal-rolecpu-3ef/`，不作为发布通过证据。发生器改为流式消费和独立连接池后必须重建双方数据，不能把发生器变化归为网关优化收益。


