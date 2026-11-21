# v1.3.14 测试缓存治理与干净环境复验

2026-11-22 用户要求在功能全部落地后继续发布线上，并严格避免 proxysss 项目长期占用 100 GB 磁盘。v1.3.14 不改变网关转发行为，重点是建立“每次测试前清空项目构建/测试缓存”的强制规则，并从零缓存环境重新完成 Windows、Linux musl 和静态/反向代理性能复验。对应 [HTML 记录](release-1.3.14.html) 同步维护。

## 1. 100 GB 级缓存来源与清理结果

清理前项目本地占用：

| 目录 | 占用 |
| --- | ---: |
| `target/`（debug 74.22 GB + release 11.38 GB + 其他目标） | 86.63 GB |
| `.benchmark/`（历史 Linux target、runs、多套构建） | 54.21 GB |
| `.tmp/`（验证、发布附件、性能原始日志） | 0.98 GB |
| `.cache/`（CARGO_HOME / Go 构建缓存） | 0.84 GB |
| `dist/`、`logs/` | 约 0.09 GB |

执行 `pwsh -NoProfile -File scripts/clean-test-cache.ps1` 后，`target/`、`.benchmark/`、`.cache/`、`.tmp/`、`dist/`、根目录 `tmp-*` / `.tmp-*` 测试产物、benchmark 日志和本项目 Docker 验证卷全部移除；项目缓存回到 0，D 盘可用空间回到约 1.27 TB。

## 2. 强制规则落地

- 新增 `scripts/clean-test-cache.ps1` 与 `scripts/clean-test-cache.sh`。默认清理项目本地构建/测试状态，只拒绝绝对路径越界和链接穿越；不执行全局 `docker system prune`，不影响其他项目。
- `scripts/verify-local.ps1`、`verify-deep.ps1`、`verify-e2e.ps1`、`verify-docker-scenarios.ps1`、`run-all-verify.ps1`、`benchmark-gateways.ps1` 开始前自动调用清理；`verify-local.ps1` 结束后保留验证报告和本地包，但删除 `target/`、`.benchmark/`、`.cache/`。
- `AGENTS.md` 与 `docs/VALIDATION.md` / `validation.html` 写入：任何 `cargo test`、`cargo build`、`cargo clippy`、Docker 验证、基准或打包前必须清理；测试结束 `target/` 与 `.benchmark/` 合计必须低于 5 GB。
- `KEEP_BENCH_ARTIFACTS=1` 只为一次明确发布证据保留，收集后立即清理，不长期堆积。

## 3. 干净环境功能复验

- Windows 本机：`cargo test --workspace --all-targets --locked`，工作目录 `target` 指向本轮独占的 `.tmp/cargo-target`，`425 passed / 0 failed / 0 ignored`，22.55 秒。
- Linux musl：`x86_64-unknown-linux-musl` 全量测试，构建与测试临时目录使用本轮独占 Docker volume，`454 passed / 0 failed / 0 ignored`，11.68 秒。
- 修复 `filecloud::path` 测试中两个临时目录可能在同一时钟刻度取到相同名称的偶发碰撞；只影响测试夹具命名，不改变运行时路径边界逻辑。

## 4. 干净环境性能复验

环境：Windows 主机 AMD Ryzen 9 7900X3D / Docker Desktop VM，网关容器 Ubuntu 24.04、`linux/amd64`、`--cpus=1 --memory=2g`；HTTP/1.1 keep-alive，16 workers（反向代理 32 workers），3s warmup + 10s 测量，3 轮取中位。

| 场景 | v1.3.13 | v1.3.14 | 变化 | p50 |
| --- | ---: | ---: | ---: | ---: |
| 1KB 静态 | 20,053 RPS | 20,409 RPS | +1.78% | 1.004 → 0.999 ms |
| 64KB 静态 | 15,171 RPS | 15,052 RPS | -0.78% | 1.057 → 1.056 ms |
| 1KB 反向代理 | 9,142 RPS | 9,251 RPS | +1.19% | 3.669 → 3.659 ms |

p99 中位：1KB 静态 2.038 → 2.023 ms，64KB 2.047 → 2.034 ms，反向代理 4.902 → 4.808 ms。64KB 的 -0.78% 落在三轮样本波动内；小文件与反向代理中位吞吐略高。结论是“清缓存、静态链接后性能不降，部分路径略优”，不宣称 nginx 全面胜出或跨主机容量。

## 5. 本地候选二进制

| 项 | 值 |
| --- | --- |
| 目标 | `x86_64-unknown-linux-musl` |
| 文件大小 | 23,358,936 bytes |
| 链接形态 | `ELF 64-bit LSB pie executable`, `static-pie linked`；无 `NEEDED` / `INTERP` |
| BuildID | `6c6367240de3e8f224777f7be41eece044bd658c` |
| SHA256 | `2676D108AE73AA49344ED5D83E139DF5A42810A827FD2B1F8CA5CB32264AF934` |
| 版本输出 | `proxysss 1.3.14` |

以上为本地候选；GitHub Release 正式附件在发布后单独核验。

## 6. 发布门禁状态

- 测试前缓存清理：通过，脚本真实探针验证 `target/`、`.benchmark/`、根目录 `tmp-*` 均被删除。
- Windows 功能测试：通过，425/0。
- Linux musl 功能测试：通过，454/0。
- 静态链接：通过，`static-pie`，无 `NEEDED` / `INTERP`。
- 性能复验：通过，1KB 静态 +1.78%，反向代理 +1.19%，64KB 在噪声内。
- 六平台打包与 GitHub Release：发布后核验。
