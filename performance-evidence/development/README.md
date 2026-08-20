# 开发期性能证据

`local-docker/` 是本机 benchmark 归档目录，不再随仓库提交。默认 benchmark 结束会清理 `.benchmark/`；需要留存时显式设置 `KEEP_BENCH_ARTIFACTS=1` 或 `BENCH_ROOT`，再按需将筛选结果存入本目录。

这里不保存交叉编译 target、Docker image context、临时二进制、client 容器文件或完整原始 payload；这些本地产物超过 9 GiB，并不增加报告可审计性。benchmark 默认把 Rust target 放在可清理的 `.benchmark/target`，并在运行结束移除本次创建的 proxysss benchmark 镜像；只有显式设置 `BENCH_ROOT`、`KEEP_BENCH_ARTIFACTS=1` 或 `CARGO_TARGET_DIR` 才保留这些产物。

更新归档：

```bash
scripts/archive-local-benchmark-reports.sh
```

查看本机最近一次报告（目录存在时）：

```bash
latest=$(find performance-evidence/development/local-docker -mindepth 1 -maxdepth 1 -type d | sort | tail -1)
column -t -s $'\t' performance-evidence/development/local-docker/INDEX.tsv | tail -20
cat "$latest/host-fingerprint.txt"
find "$latest" -name '*-summary.md' -print -exec cat {} \;
```

这些是开发期诊断证据，包含通过与失败的实验。只有 `performance-evidence/vX.Y.Z.json` 严格 manifest 才能作为 release tag 的生产证据；不要把本目录中的 emulated-amd64 报告描述为物理 x86 证据。
