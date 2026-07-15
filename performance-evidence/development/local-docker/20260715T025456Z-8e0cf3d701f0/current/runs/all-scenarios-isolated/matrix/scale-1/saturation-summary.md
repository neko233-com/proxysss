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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30092.00 | 25563.00 | 1.177x | +17.72% | - | - | - | 0.703x | +29.73% | 1.004x | -0.41% | 0.977x | +2.33% | 0 |
| game-long-connection | 6491.50 | 3883.50 | 1.672x | +67.16% | - | - | - | 0.301x | +69.86% | 1.016x | -1.59% | 0.831x | +16.88% | 0 |
| generic-sse | 614.50 | 440.50 | 1.395x | +39.50% | - | - | - | 0.626x | +37.38% | 0.889x | +11.07% | 0.844x | +15.62% | 0 |
| https-static-small | 6261.50 | 4845.00 | 1.292x | +29.24% | - | - | - | 0.571x | +42.86% | 0.965x | +3.54% | 0.760x | +24.01% | 0 |
| qcp-transparent | 5189.00 | 3980.50 | 1.304x | +30.36% | - | - | - | 0.557x | +44.31% | 0.891x | +10.89% | 0.905x | +9.46% | 0 |
| reverse-proxy | 11863.00 | 10518.00 | 1.128x | +12.79% | - | - | - | 0.855x | +14.54% | 0.927x | +7.26% | 0.914x | +8.60% | 0 |
| static-large | 86.00 | 87.50 | 0.983x | -1.71% | - | - | - | 0.964x | +3.59% | 0.978x | +2.17% | 0.772x | +22.77% | 0 |
| static-small | 31792.00 | 24895.00 | 1.277x | +27.70% | - | - | - | 0.668x | +33.25% | 0.930x | +6.98% | 0.906x | +9.39% | 0 |
| tcp-stream | 5142.00 | 3741.00 | 1.374x | +37.45% | - | - | - | 0.487x | +51.31% | 1.042x | -4.22% | 0.895x | +10.54% | 0 |
| udp-stream | 4867.50 | 3813.00 | 1.277x | +27.66% | - | - | - | 0.606x | +39.44% | 0.898x | +10.17% | 0.879x | +12.12% | 0 |
| websocket-long-connection | 4938.50 | 3682.50 | 1.341x | +34.11% | - | - | - | 0.520x | +48.01% | 1.014x | -1.38% | 0.879x | +12.08% | 0 |

- Aggregate proxysss ops/s: `107337.50`
- Aggregate nginx ops/s: `85449.50`
- Aggregate proxysss/nginx ratio: `1.256x`
- Aggregate throughput improvement: `+25.62%`
