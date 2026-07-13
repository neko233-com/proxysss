# 正式 Linux 性能证据清单

正式 tag `vX.Y.Z` 必须在同一提交加入 `performance-evidence/vX.Y.Z.json`。`release.yml` 会运行：

```bash
go run scripts/verify-production-evidence.go \
  --manifest performance-evidence/vX.Y.Z.json \
  --tag vX.Y.Z --commit <tag-commit>
```

它不是可填空的性能声明：每个 `uri` 必须指向可审计的原始 benchmark 工件，`sha256` 必须是该工件的真实摘要。`.benchmark/` 保持本地忽略；把原始工件上传到受控的 CI artifact、对象存储或归档系统，再在清单中记录不可变 URI。

## 必需运行

每种 `kind` 都要有 `scale` 为 `1`、`2`、`4` 的一轮，共六轮：

- `role-isolated-all-scenarios`：Docker/cgroup/cpuset/network namespace 角色隔离的全场景 mixed、isolated saturation、equal-load 和容量原始结果。
- `cross-host-wss`：独立 client、gateway、backend Linux 主机的 WSS 吞吐、等负载 p50/p95/p99 与容量原始结果。

每轮都必须是严格胜出与零错误；`cross-host-wss` 的三个 machine-id hash 必须彼此不同。单机 `role-isolated-all-scenarios` 可以记录同一宿主机 hash，但 `role_isolation_proven` 必须为 `true`，表示 cpuset、cgroup 与网络命名空间预检通过。

内存不是固定数字门槛。每轮必须保存 current/peak、每连接成本与无持续增长结论；若发布包络有明确预算，可另在 benchmark 命令传 `MemoryMax`/Docker `--memory`。

## JSON 骨架

不要提交此骨架本身作为 tag 清单；六轮、工件 URI 与 SHA-256 都必须替换为实测值。

```json
{
  "schema_version": 1,
  "tag": "vX.Y.Z",
  "commit": "<full-tag-commit-sha>",
  "runs": [
    {
      "kind": "role-isolated-all-scenarios",
      "scale": 1,
      "strict_superiority": true,
      "zero_errors": true,
      "saturation_ops_strictly_won": true,
      "equal_load_percentiles_strictly_won": true,
      "capacity_strictly_won": true,
      "role_isolation_proven": true,
      "role_machine_id_hashes": {
        "client": "<sha256>", "gateway": "<sha256>", "backend": "<sha256>"
      },
      "memory": {
        "current_and_peak_recorded": true,
        "per_connection_recorded": true,
        "no_runaway_growth": true
      },
      "artifacts": [
        { "name": "saturation", "sha256": "<sha256>", "uri": "artifact://..." },
        { "name": "equal-load", "sha256": "<sha256>", "uri": "artifact://..." },
        { "name": "capacity", "sha256": "<sha256>", "uri": "artifact://..." }
      ]
    }
  ]
}
```
