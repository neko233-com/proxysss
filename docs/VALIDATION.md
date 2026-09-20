# 验证与发布流程

本页定义验证顺序与证据边界。产品能力是否存在，以代码、配置和功能测试为准；性能是否胜出，以相同提交、相同资源的实测为准。不能承诺在任意硬件与负载下始终最优。

## 1. Ubuntu 24 LTS 功能检查

Windows Docker Desktop 在 PowerShell 运行：

```powershell
.\scripts\verify-docker-scenarios.ps1
```

Linux/macOS 使用 `scripts/verify-docker-scenarios.sh`。脚本探测 Ubuntu 24.04 x86_64，检查格式、Clippy、两轮测试、全场景配置、CLI 能力和 nginx 对照声明。固定容器名 `proxysss-verify` 带项目所有者标记；不得清理其他任务的容器。最新报告位于 `.tmp/docker-scenarios/`，测试目录在结束或失败后清理。

CPU 功能探针 `scripts/verify-linux-cpu/main.go` 随 Docker 功能检查运行：用 taskset 收窄真实网关的 CPU 范围，验证 HTTP/TCP/UDP 正文、所有线程 affinity，以及 HTTP/native relay/sendfile owner 数量。单独在带 CPU quota 的容器中可使用 `go run scripts/verify-linux-cpu/main.go --binary <Linux二进制> --expected-workers <预算>` 断言 quota 生效；这是功能检查，不是性能胜出证据。检查结果保存为 cpu-confinement.txt。

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

预连接、两次 GET/SSE 资源请求、每条实时连接的一次 echo、1000 ms 解暂停稳定期和 2000 ms future-start lead 均在活跃测量之外。构建、容器准备、预热和结果处理另记 wall time。客户端与后端进程遵守 `TOKIO_WORKER_THREADS`，等负载 HTTP/SSE 和大文件各使用两个 I/O worker，实时协议一个；混合负载按场景划分互不重叠的 client CPU，饱和时 worker 数等于该场景 CPU 数；只有串行 isolated-saturation 诊断使用全部 client cpuset。单场景 CPU 满载时只能说明当前发生器资源下的比较，不能断言网关容量上限。官方 fixture、解析和验收工具使用 Go 或原生编译工具。

HTTP/1 每个并发 worker 使用独立、最多一个空闲连接的 client，并先完成 HEAD，确保声明的连接数真实建立且随后复用；HTTPS/H2 仍共享一个多路复用 client。HTTP 下载逐块消费并计数，不把完整大文件收进发生器内存。`bench http` 默认仅接受 2xx；`--expected-status` 可指定精确状态，`--expected-body-bytes` 校验正文长度。正式 HTTP 夹具要求 200，静态夹具还要求实际文件长度；错误页、短正文和意外长正文不能算吞吐成功。

预热错过 `--start-at-unix-ms` 时整波失败，不能自行延后开始而把异步运行当作同时混合负载。双方每轮都必须完成至少 98% 的可执行目标，参考 nginx 不达标同样使比较无效。CDN 预检在测量前依次确认 v1 正文、原子发布 v2、确认双方确实返回 v2；其后测量的是更新生效后的读取吞吐，不声称是一秒内连续发布的性能。

严格入口要求零错误、每项吞吐与 p50/p95/p99 都胜出；单项通过不能覆盖混合回归。旧共享角色诊断 `benchmark-all-scenarios.sh` 的 `CRITICAL_RATIO=0.97` 和 `MIN_RATIO=0.50` 不等于严格发布通过。内存报告双方 cgroup current、peak、每连接成本，各项不得超过 nginx 的 2 倍；只在声明实际生产预算时设置 MemoryMax。4c/4096 active WSS/20k idle WSS 是参考负载，100k 容量为额外诊断。

每轮原始结果保存在带 `-r1`、`-r2` 等序号的目录，包含逐场景输出和 results.jsonl；后续轮次不得覆盖早期失败详情。无序号场景文件仅为最后一轮的便捷副本。网关、后端与客户端的逐轮容器日志和测量前后网络计数器在活跃窗口之外采集，便于区分 UDP 内核丢包、超时及连接错误。UDP/QCP echo 检查回复长度和内容，失败输出阶段、错误种类和系统错误码，每进程最多打印 8 条详情，额外错误仍计数。计数器或错误详情缺失时，不得从汇总的 errors 数字猜测根因。

ARM Docker 上的 amd64 模拟必须记为 `emulated-amd64`；通过原生 Zig 交叉构建、在目标容器执行后测量。Docker Desktop 的 native-amd64 代表 Linux VM 中原生指令执行，不代表独立物理机。

WSS 容量必须证明连接在整个空闲持有期后仍可用。bench websocket --hold-connections 输出 hold ready 后，控制器才开始等待并采集连接存活期间的 cgroup current/peak；结束时逐条真实 echo，要求 retained 等于 requested 且 hold failures 为 0。只统计曾经 opened、只持有 socket 句柄或在客户端退出后采样，均不能证明容量。nginx 测试 idle timeout 必须长于持有期；活跃 WSS 的共同开始时间只能在所有连接完成 warm-up echo 后发布。Docker 客户端和后端显式按声明 CPU 配额设置 TOKIO_WORKER_THREADS；汇总拒绝缺少存活/内存字段的旧报告，且逐轮限制 current、peak、每连接成本不超过 nginx 的 2 倍。WSS 等负载目标只统计测量窗口内、距结束至少 5 ms 的完整 tick；例如 4096 连接、5 秒、300000 us 间隔的目标是 65536 次，而非无限时长速率乘窗口。CLI 显示可执行 target ops/sec，汇总要求双方目标相同且每轮完成至少 98%；没有完整 tick 的计划不能作为延迟证据。等待 tick、收发及重连都在测量截止时间停止，不把超时后的完成计入吞吐。容量矩阵默认在持有期一半时采在线内存。

## 3. Windows 功能验收

Linux 全部功能与性能门槛通过后，在本机 Windows 快速运行 `cargo test --locked`，确认现有单元与集成用例通过；不执行 Windows 性能矩阵。用户当前验收要求以这轮本机测试作为 Windows 快速功能验证；不能把未覆盖的安装器、第三方服务或外部系统场景写成已实测。Windows 桌面成绩不代替 Linux 性能证据。

## 4. Push 与发布

使用 [gh-cli 技能](../skills/gh-cli/SKILL.md)，检查实际 Actions 状态。CI 只做六平台打包，禁止云端功能测试和性能 benchmark。Node.js action 使用 24 LTS 或更高，artifact action 至少 v6。

正式版本必须匹配 Cargo.toml、CHANGELOG.md 与 [性能证据清单](../performance-evidence/README.md)。schema v3 要求同一 tag 提交上的角色隔离矩阵和三台不同 machine-id Linux 主机的 WSS 1×/2×/4× 复验、真实工件 URI/摘要、吞吐/延迟/容量/内存数据。运行 `go run scripts/verify-production-evidence.go --manifest performance-evidence/vX.Y.Z.json --tag vX.Y.Z --commit <tag-commit>` 后才能推 tag。v1.3.8 的一次性功能发布豁免不适用于其他版本。

报告明确列出通过项、失败项、未执行项、提交和工件位置。缺少跨主机环境或严格性能未过时，保留修复和证据，不能生成虚假清单或宣称发布完成。

## 诊断发生器瓶颈

先区分网关与客户端资源，再解释接近 1.0 的比值。3efde31 的一次独立 CPU 观察器诊断固定网关/后端/客户端为 4/6/12 核，四个观察器另占 CPU 22–23。18 波全混合、54 个角色窗口的 25 ms 线程采样中，大文件客户端饱和运行时间约 964–993 ms/秒，接近其单核上限；CDN 约 780–941 ms/秒，仍不足以断定每种负载都受客户端限制。等负载 CDN 双 worker 累计调度等待约 212–245 ms/秒，网关 native realtime owner 约 21–28 ms/秒；多个线程等待可重叠，不能当作某次请求的延迟。

该诊断使用固定旧发生器与二进制，窗口边界滞后最多 27.3 ms；退出线程保留最后可见样本，计数可能低估。完整计数、覆盖时间、源码和角色隔离信息保存在本地 `.benchmark/goal-rolecpu-3ef/`，不作为发布通过证据。发生器改为流式消费和独立连接池后必须重建双方数据，不能把发生器变化归为网关优化收益。
