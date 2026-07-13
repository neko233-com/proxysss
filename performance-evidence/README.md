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

清单目前是 **schema v2**。不再接受 `strict_superiority: true` 这类只能表达结论的布尔字段：每个场景都必须逐项写入 proxysss/nginx 的 `ops_per_sec`、`p50_ms`、`p95_ms`、`p99_ms`、`errors`，验证器会要求吞吐严格更高、三个百分位严格更低、两边错误均为 0。`role-isolated-all-scenarios` 必须完整列出十个当前 nginx 公平对标场景；`cross-host-wss` 必须列出 `websocket-long-connection`。容量也必须同时记录两边的 opened/failed、open rate 与三项握手百分位，并要求在 **1k-50k** 的可复现包络内双边均完整建连、proxysss 严格胜出。

内存不设固定绝对数字门槛，但不是无限制：每轮必须分别记录 proxysss/nginx 的实际 `current_bytes`、`peak_bytes`、`bytes_per_connection` 与无持续增长结论，且 proxysss 的任一项都不得超过 nginx 的 **2 倍**。若发布包络有明确预算，可另在 benchmark 命令传 `MemoryMax`/Docker `--memory`。默认 4c 参考工作负载是 4096 active WSS + 20k idle WSS，规模复验只把 active WSS 扩为 8192、16384，不用 100k 伪容量替代生产结论。

## JSON 骨架

不要提交此骨架本身作为 tag 清单；六轮、工件 URI 与 SHA-256 都必须替换为实测值。

```json
{
  "schema_version": 1,
  "tag": "vX.Y.Z",
  "commit": "<full-tag-commit-sha>",
  "runs": [
    {
      "kind": "cross-host-wss",
      "scale": 1,
      "role_isolation_proven": true,
      "role_machine_id_hashes": {
        "client": "<sha256>", "gateway": "<sha256>", "backend": "<sha256>"
      },
      "workload": {
        "active_connections": 4096,
        "capacity_connections": 20000,
        "repetitions": 4
      },
      "memory": {
        "proxysss": { "current_bytes": 123456789, "peak_bytes": 234567890, "bytes_per_connection": 11728 },
        "nginx": { "current_bytes": 120000000, "peak_bytes": 220000000, "bytes_per_connection": 11000 },
        "no_runaway_growth": true
      },
      "scenarios": [
        {
          "name": "websocket-long-connection",
          "proxysss": { "ops_per_sec": 2000, "p50_ms": 1.0, "p95_ms": 2.0, "p99_ms": 3.0, "errors": 0 },
          "nginx": { "ops_per_sec": 1900, "p50_ms": 1.1, "p95_ms": 2.2, "p99_ms": 3.3, "errors": 0 }
        }
      ],
      "capacity": {
        "proxysss": { "opened": 20000, "failed": 0, "open_rate_per_sec": 2000, "handshake_p50_ms": 1.0, "handshake_p95_ms": 2.0, "handshake_p99_ms": 3.0 },
        "nginx": { "opened": 20000, "failed": 0, "open_rate_per_sec": 1900, "handshake_p50_ms": 1.1, "handshake_p95_ms": 2.2, "handshake_p99_ms": 3.3 }
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
