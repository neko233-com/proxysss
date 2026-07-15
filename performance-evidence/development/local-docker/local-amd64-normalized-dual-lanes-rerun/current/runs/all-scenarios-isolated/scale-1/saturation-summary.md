# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-normalized-dual-lanes-rerun/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 16267.45 | 23213.05 | 0.701x | -29.92% | - | - | - | 1.304x | -30.35% | 1.575x | -57.48% | 1.547x | -54.65% | 0 |
| game-long-connection | 3776.60 | 4170.90 | 0.905x | -9.45% | - | - | - | 1.025x | -2.52% | 1.296x | -29.56% | 1.306x | -30.55% | 0 |
| generic-sse | 403.20 | 520.00 | 0.775x | -22.46% | - | - | - | 0.905x | +9.51% | 2.114x | -111.38% | 2.150x | -114.98% | 0 |
| https-static-small | 5159.40 | 6018.35 | 0.857x | -14.27% | - | - | - | 1.117x | -11.72% | 1.255x | -25.48% | 1.141x | -14.08% | 0 |
| qcp-transparent | 3310.85 | 4205.90 | 0.787x | -21.28% | - | - | - | 0.856x | +14.41% | 1.912x | -91.21% | 1.980x | -98.01% | 0 |
| reverse-proxy | 9163.15 | 12641.50 | 0.725x | -27.52% | - | - | - | 1.236x | -23.58% | 1.830x | -83.02% | 1.815x | -81.49% | 0 |
| static-large | 118.15 | 92.85 | 1.272x | +27.25% | - | - | - | 0.771x | +22.86% | 0.789x | +21.08% | 0.753x | +24.67% | 0 |
| static-small | 15844.75 | 23001.90 | 0.689x | -31.12% | - | - | - | 1.367x | -36.65% | 1.567x | -56.73% | 1.546x | -54.63% | 0 |
| tcp-stream | 3782.80 | 4147.50 | 0.912x | -8.79% | - | - | - | 1.021x | -2.09% | 1.282x | -28.18% | 1.296x | -29.56% | 0 |
| udp-stream | 3298.75 | 4228.95 | 0.780x | -22.00% | - | - | - | 0.868x | +13.18% | 1.888x | -88.84% | 1.904x | -90.40% | 0 |
| websocket-long-connection | 3595.65 | 4011.05 | 0.896x | -10.36% | - | - | - | 1.028x | -2.81% | 1.322x | -32.24% | 1.318x | -31.77% | 0 |

- Aggregate proxysss ops/s: `64720.75`
- Aggregate nginx ops/s: `86251.95`
- Aggregate proxysss/nginx ratio: `0.750x`
- Aggregate throughput improvement: `-24.96%`
