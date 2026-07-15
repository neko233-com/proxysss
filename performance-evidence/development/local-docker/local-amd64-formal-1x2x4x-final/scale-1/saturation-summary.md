# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-1x2x4x-final/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22083.35 | 21626.50 | 1.021x | +2.11% | - | - | - | 0.765x | +23.51% | 1.148x | -14.85% | 1.161x | -16.09% | 0 |
| game-long-connection | 4134.90 | 3847.65 | 1.075x | +7.47% | - | - | - | 0.816x | +18.37% | 1.242x | -24.17% | 1.289x | -28.88% | 0 |
| generic-sse | 628.05 | 483.65 | 1.299x | +29.86% | - | - | - | 0.604x | +39.60% | 1.152x | -15.20% | 1.253x | -25.27% | 0 |
| https-static-small | 5627.30 | 5560.45 | 1.012x | +1.20% | - | - | - | 0.851x | +14.85% | 1.255x | -25.45% | 1.062x | -6.19% | 0 |
| qcp-transparent | 5212.20 | 4035.55 | 1.292x | +29.16% | - | - | - | 0.525x | +47.54% | 1.156x | -15.61% | 1.231x | -23.11% | 0 |
| reverse-proxy | 13313.55 | 11838.95 | 1.125x | +12.46% | - | - | - | 0.777x | +22.32% | 1.171x | -17.10% | 1.185x | -18.49% | 0 |
| static-large | 89.15 | 85.35 | 1.045x | +4.45% | - | - | - | 0.940x | +6.02% | 0.991x | +0.93% | 1.003x | -0.25% | 0 |
| static-small | 22007.85 | 20966.80 | 1.050x | +4.97% | - | - | - | 0.728x | +27.18% | 1.132x | -13.24% | 1.135x | -13.54% | 0 |
| tcp-stream | 4123.35 | 3888.50 | 1.060x | +6.04% | - | - | - | 0.823x | +17.71% | 1.244x | -24.41% | 1.294x | -29.37% | 0 |
| udp-stream | 5179.75 | 3969.20 | 1.305x | +30.50% | - | - | - | 0.538x | +46.20% | 1.154x | -15.36% | 1.213x | -21.34% | 0 |
| websocket-long-connection | 3796.70 | 3812.90 | 0.996x | -0.42% | - | - | - | 0.897x | +10.26% | 1.285x | -28.54% | 1.350x | -35.03% | 0 |

- Aggregate proxysss ops/s: `86196.15`
- Aggregate nginx ops/s: `80115.50`
- Aggregate proxysss/nginx ratio: `1.076x`
- Aggregate throughput improvement: `+7.59%`
