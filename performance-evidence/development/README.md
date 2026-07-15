# 开发期性能证据

`local-docker/` 保存从本机 `.benchmark/direct-ubuntu24-amd64/` 筛选出的历史报告，用于跨电脑继续性能分析。当前快照覆盖 89 个 run；归档包含 host fingerprint、validation timing、matrix 日志、run metadata、equal-load plan、Markdown/HTML summary、JSON/JSONL 结果和 cgroup 内存 current/peak。`local-docker/INDEX.tsv` 提供 run、commit、执行模式、验证耗时、已生成尺度和状态的机器可读索引。

这里不保存交叉编译 target、Docker image context、临时二进制、client 容器文件或完整原始 payload；这些本地产物超过 9 GiB，并不增加报告可审计性。

更新归档：

```bash
scripts/archive-local-benchmark-reports.sh
```

查看最近一次报告：

```bash
latest=$(find performance-evidence/development/local-docker -mindepth 1 -maxdepth 1 -type d | sort | tail -1)
column -t -s $'\t' performance-evidence/development/local-docker/INDEX.tsv | tail -20
cat "$latest/host-fingerprint.txt"
find "$latest" -name '*-summary.md' -print -exec cat {} \;
```

这些是开发期诊断证据，包含通过与失败的实验。只有 `performance-evidence/vX.Y.Z.json` 严格 manifest 才能作为 release tag 的生产证据；不要把本目录中的 emulated-amd64 报告描述为物理 x86 证据。
