# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30256.00 | 25448.67 | 1.189x | +18.89% | - | - | - | 0.595x | +40.46% | 1.447x | -44.72% | 1.636x | -63.59% | 0 |
| game-long-connection | 7638.33 | 3576.33 | 2.136x | +113.58% | - | - | - | 0.423x | +57.72% | 0.684x | +31.57% | 0.643x | +35.69% | 0 |
| generic-sse | 701.00 | 560.67 | 1.250x | +25.03% | - | - | - | 0.650x | +35.01% | 1.334x | -33.39% | 2.019x | -101.86% | 0 |
| https-static-small | 3612.67 | 5943.67 | 0.608x | -39.22% | - | - | - | 1.409x | -40.86% | 2.508x | -150.75% | 1.683x | -68.32% | 0 |
| qcp-transparent | 6231.00 | 3695.00 | 1.686x | +68.63% | - | - | - | 0.415x | +58.47% | 1.176x | -17.58% | 1.515x | -51.51% | 0 |
| reverse-proxy | 13793.67 | 12441.33 | 1.109x | +10.87% | - | - | - | 0.759x | +24.07% | 1.332x | -33.18% | 1.828x | -82.77% | 0 |
| static-large | 93.00 | 98.67 | 0.943x | -5.75% | - | - | - | 1.019x | -1.94% | 1.211x | -21.11% | 1.358x | -35.83% | 0 |
| static-small | 31601.00 | 25310.33 | 1.249x | +24.85% | - | - | - | 0.578x | +42.16% | 1.334x | -33.39% | 1.601x | -60.09% | 0 |
| tcp-stream | 7907.33 | 3587.33 | 2.204x | +120.42% | - | - | - | 0.414x | +58.64% | 0.636x | +36.37% | 0.676x | +32.38% | 0 |
| udp-stream | 5930.67 | 3605.00 | 1.645x | +64.51% | - | - | - | 0.429x | +57.12% | 1.183x | -18.30% | 1.442x | -44.24% | 0 |
| websocket-long-connection | 6974.33 | 3479.00 | 2.005x | +100.47% | - | - | - | 0.459x | +54.11% | 0.730x | +27.00% | 0.745x | +25.49% | 0 |

- Aggregate proxysss ops/s: `114739.00`
- Aggregate nginx ops/s: `87746.00`
- Aggregate proxysss/nginx ratio: `1.308x`
- Aggregate throughput improvement: `+30.76%`
