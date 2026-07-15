# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15889.00 | 11741.00 | 1.353x | +35.33% | - | - | - | 0.640x | +36.02% | 0.905x | +9.54% | 0.849x | +15.13% | 0 |
| game-long-connection | 3020.00 | 3554.00 | 0.850x | -15.03% | - | - | - | 1.666x | -66.61% | 0.952x | +4.85% | 1.007x | -0.74% | 0 |
| generic-sse | 343.00 | 323.00 | 1.062x | +6.19% | - | - | - | 0.865x | +13.50% | 1.165x | -16.51% | 0.873x | +12.73% | 0 |
| https-static-small | 1981.00 | 1756.00 | 1.128x | +12.81% | - | - | - | 0.957x | +4.27% | 1.069x | -6.95% | 0.945x | +5.46% | 0 |
| qcp-transparent | 3832.00 | 2193.00 | 1.747x | +74.74% | - | - | - | 0.252x | +74.83% | 0.865x | +13.52% | 0.863x | +13.70% | 0 |
| reverse-proxy | 6855.00 | 5298.00 | 1.294x | +29.39% | - | - | - | 0.647x | +35.34% | 0.973x | +2.71% | 0.569x | +43.09% | 0 |
| static-large | 75.00 | 73.00 | 1.027x | +2.74% | - | - | - | 1.008x | -0.78% | 1.468x | -46.80% | 0.861x | +13.93% | 0 |
| static-small | 15867.00 | 11559.00 | 1.373x | +37.27% | - | - | - | 0.687x | +31.26% | 0.877x | +12.31% | 0.964x | +3.58% | 0 |
| tcp-stream | 3359.00 | 2187.00 | 1.536x | +53.59% | - | - | - | 0.629x | +37.15% | 0.772x | +22.77% | 0.798x | +20.21% | 0 |
| udp-stream | 2331.00 | 2333.00 | 0.999x | -0.09% | - | - | - | 1.007x | -0.72% | 1.089x | -8.87% | 1.131x | -13.10% | 0 |
| websocket-long-connection | 2919.00 | 2971.00 | 0.982x | -1.75% | - | - | - | 1.303x | -30.30% | 0.859x | +14.15% | 0.947x | +5.32% | 0 |

- Aggregate proxysss ops/s: `56471.00`
- Aggregate nginx ops/s: `43988.00`
- Aggregate proxysss/nginx ratio: `1.284x`
- Aggregate throughput improvement: `+28.38%`
