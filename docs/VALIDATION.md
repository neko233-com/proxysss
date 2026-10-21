# 验证与发布流程

当前 v1.3.10 按 2026-10-21 用户“没问题就全部发布”的明确授权，以生产 Linux nginx 迁移的功能与稳定性验收发布；仅本标签豁免严格性能胜出及跨主机性能证据。Linux 完整协议/容量与六平台打包保留，Windows 完整执行并披露本机 IPv6 UDP loopback 不可达的环境失败，不改写/跳过专门 IPv6 用例，不宣称全平台运行全绿。详见 [v1.3.10 验证记录](RELEASE-1.3.10.md) / [HTML](release-1.3.10.html)。

本页定义验证顺序与证据边界。2026-09-21 用户授权 v1.3.9 优先发布稳定功能版本：Linux 功能与混合负载稳定性、本机 Windows 测试、六平台打包通过即可，不要求超过 nginx 或提供跨主机性能胜出证据。详见 [本次版本验证](RELEASE-1.3.9.md) / [HTML](release-1.3.9.html)。CDN 后续优化优先小文件，大、小文件择一胜出即可，不再以同时胜出阻塞当前发布。产品能力是否存在，以代码、配置和功能测试为准；性能是否胜出，以相同提交、相同资源的实测为准。不能承诺在任意硬件与负载下始终最优。

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

## 2. Linux 混合性能

在干净提交上执行；Windows 必须使用 Git Bash。保存原始工件，不能让默认清理删除证据或共享构建缓存：

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

现有严格实验入口仍输出逐项吞吐与 p50/p95/p99 的历史比较门槛；它用于诊断，不是本次 v1.3.9 稳定版本的发布否决条件。相对速度落后与功能/运行错误必须分别报告。CDN 优化优先小文件，不要求大、小文件同时胜出。旧共享角色诊断 `benchmark-all-scenarios.sh` 的 `CRITICAL_RATIO=0.97` 和 `MIN_RATIO=0.50` 不等于严格发布通过。内存报告双方 cgroup current、peak、每连接成本，各项不得超过 nginx 的 2 倍；只在声明实际生产预算时设置 MemoryMax。4c/4096 active WSS/20k idle WSS 是参考负载，100k 容量为额外诊断。

每轮原始结果保存在带 `-r1`、`-r2` 等序号的目录，包含逐场景输出和 results.jsonl；后续轮次不得覆盖早期失败详情。无序号场景文件仅为最后一轮的便捷副本。网关、后端与客户端的逐轮容器日志和测量前后网络计数器在活跃窗口之外采集，便于区分 UDP 内核丢包、超时及连接错误。UDP/QCP echo 检查回复长度和内容，失败输出阶段、错误种类和系统错误码，每进程最多打印 8 条详情，额外错误仍计数。计数器或错误详情缺失时，不得从汇总的 errors 数字猜测根因。

ARM Docker 上的 amd64 模拟必须记为 `emulated-amd64`；通过原生 Zig 交叉构建、在目标容器执行后测量。Docker Desktop 的 native-amd64 代表 Linux VM 中原生指令执行，不代表独立物理机。

WSS 容量必须证明连接在整个空闲持有期后仍可用。bench websocket --hold-connections 输出 hold ready 后，控制器才开始等待并采集连接存活期间的 cgroup current/peak；结束时逐条真实 echo，要求 retained 等于 requested 且 hold failures 为 0。只统计曾经 opened、只持有 socket 句柄或在客户端退出后采样，均不能证明容量。nginx 测试 idle timeout 必须长于持有期；活跃 WSS 的共同开始时间只能在所有连接完成 warm-up echo 后发布。Docker 客户端和后端显式按声明 CPU 配额设置 TOKIO_WORKER_THREADS；汇总拒绝缺少存活/内存字段的旧报告，且逐轮限制 current、peak、每连接成本不超过 nginx 的 2 倍。WSS 等负载目标只统计测量窗口内、距结束至少 5 ms 的完整 tick；例如 4096 连接、5 秒、300000 us 间隔的目标是 65536 次，而非无限时长速率乘窗口。CLI 显示可执行 target ops/sec，汇总要求双方目标相同且每轮完成至少 98%；没有完整 tick 的计划不能作为延迟证据。等待 tick、收发及重连都在测量截止时间停止，不把超时后的完成计入吞吐。容量矩阵默认在持有期一半时采在线内存。

重复样本的吞吐和延迟取中位数，错误取最大值，**完成率取最差单轮**。汇总 JSON 保留 `sample_count`、`worst_target_achievement` 与 `worst_target_sample`；Markdown/HTML 显示最低完成率，错误指出最差轮次。完成率由该轮成功次数、测量时长和可执行目标计算，不用四舍五入后的中位吞吐替代。样本数量须符合声明，重复中的协议/并发/时长/目标一致，两边目标也须一致；缺失轮次、缺失最低完成率或已汇总数据再次聚合均不能作为通过证据。例：1032/1056 与 1056/1056 必须按 97.73% 失败，不能按两轮平均的 98.86% 通过。修复报告后重算保留的原始样本不等于重新运行性能测试。

## 3. Windows 功能验收

本次稳定发布在 Linux 功能与混合负载稳定性通过后，在本机 Windows 快速运行 `cargo test --locked`，确认现有单元与集成用例通过；不执行 Windows 性能矩阵。用户当前验收要求以这轮本机测试作为 Windows 快速功能验证；不能把未覆盖的安装器、第三方服务或外部系统场景写成已实测。Windows 桌面成绩不代替 Linux 性能证据。

## 4. Push 与发布

使用 [gh-cli 技能](../skills/gh-cli/SKILL.md)，检查实际 Actions 状态。CI 只做六平台打包，禁止云端功能测试和性能 benchmark。Node.js action 使用 24 LTS 或更高，artifact action 至少 v6。

正式版本必须匹配 Cargo.toml、CHANGELOG.md 与 [性能证据清单](../performance-evidence/README.md)。schema v3 要求同一 tag 提交上的角色隔离矩阵和三台不同 machine-id Linux 主机的 WSS 1×/2×/4× 复验、真实工件 URI/摘要、吞吐/延迟/容量/内存数据。运行 `go run scripts/verify-production-evidence.go --manifest performance-evidence/vX.Y.Z.json --tag vX.Y.Z --commit <tag-commit>` 后才能推 tag。v1.3.8 的一次性豁免仍仅限旧标签；v1.3.9 另有 2026-09-21 用户明确授权的稳定功能发布例外，使用本次版本验证记录，免于严格性能胜出与跨主机证据检查。

v1.3.10 的一次性例外仅按页首本次授权执行，使用其版本记录披露 Linux 真实产物验收和 Windows 环境限制；不能把该例外扩展到其它标签、跨平台运行全绿或性能胜出声明。最终附件来自标签构建，早期功能候选的 SHA 不能冒充平台正式附件。

报告明确列出通过项、失败项、未执行项、提交和工件位置。性能未胜出时保留真实结果，不生成虚假胜出清单；本次 v1.3.9 依明确授权按稳定性发布。只有 tag、构建、六个附件与 GitHub Release 实际核验完成后，才能报告发布完成。

## 诊断发生器瓶颈

先区分网关与客户端资源，再解释接近 1.0 的比值。3efde31 的一次独立 CPU 观察器诊断固定网关/后端/客户端为 4/6/12 核，四个观察器另占 CPU 22–23。18 波全混合、54 个角色窗口的 25 ms 线程采样中，大文件客户端饱和运行时间约 964–993 ms/秒，接近其单核上限；CDN 约 780–941 ms/秒，仍不足以断定每种负载都受客户端限制。等负载 CDN 双 worker 累计调度等待约 212–245 ms/秒，网关 native realtime owner 约 21–28 ms/秒；多个线程等待可重叠，不能当作某次请求的延迟。

该诊断使用固定旧发生器与二进制，窗口边界滞后最多 27.3 ms；退出线程保留最后可见样本，计数可能低估。完整计数、覆盖时间、源码和角色隔离信息保存在本地 `.benchmark/goal-rolecpu-3ef/`，不作为发布通过证据。发生器改为流式消费和独立连接池后必须重建双方数据，不能把发生器变化归为网关优化收益。
