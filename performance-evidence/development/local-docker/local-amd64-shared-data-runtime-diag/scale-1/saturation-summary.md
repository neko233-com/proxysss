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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-shared-data-runtime-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 23397.00 | 19830.75 | 1.180x | +17.98% | - | - | - | 0.754x | +24.56% | 0.995x | +0.50% | 1.076x | -7.59% | 0 |
| game-long-connection | 2366.00 | 3918.00 | 0.604x | -39.61% | - | - | - | 1.439x | -43.87% | 2.166x | -116.60% | 2.260x | -125.96% | 0 |
| generic-sse | 555.75 | 489.00 | 1.137x | +13.65% | - | - | - | 0.839x | +16.10% | 0.975x | +2.47% | 1.131x | -13.08% | 1 |
| https-static-small | 3531.00 | 6180.50 | 0.571x | -42.87% | - | - | - | 1.880x | -87.96% | 1.740x | -73.96% | 1.513x | -51.30% | 0 |
| qcp-transparent | 4226.75 | 4102.50 | 1.030x | +3.03% | - | - | - | 0.845x | +15.55% | 1.130x | -13.05% | 1.262x | -26.24% | 0 |
| reverse-proxy | 12193.50 | 11435.50 | 1.066x | +6.63% | - | - | - | 0.887x | +11.35% | 1.077x | -7.70% | 1.278x | -27.82% | 0 |
| static-large | 94.00 | 84.25 | 1.116x | +11.57% | - | - | - | 0.895x | +10.45% | 0.907x | +9.31% | 0.569x | +43.11% | 0 |
| static-small | 23922.50 | 19875.00 | 1.204x | +20.36% | - | - | - | 0.705x | +29.53% | 0.998x | +0.23% | 1.029x | -2.93% | 0 |
| tcp-stream | 2490.75 | 4161.25 | 0.599x | -40.14% | - | - | - | 1.411x | -41.06% | 2.169x | -116.86% | 2.406x | -140.65% | 0 |
| udp-stream | 4286.75 | 4003.75 | 1.071x | +7.07% | - | - | - | 0.777x | +22.27% | 1.105x | -10.48% | 1.240x | -23.97% | 0 |
| websocket-long-connection | 2286.25 | 3973.75 | 0.575x | -42.47% | - | - | - | 1.552x | -55.21% | 2.198x | -119.85% | 2.392x | -139.15% | 0 |

- Aggregate proxysss ops/s: `79350.25`
- Aggregate nginx ops/s: `78054.25`
- Aggregate proxysss/nginx ratio: `1.017x`
- Aggregate throughput improvement: `+1.66%`
