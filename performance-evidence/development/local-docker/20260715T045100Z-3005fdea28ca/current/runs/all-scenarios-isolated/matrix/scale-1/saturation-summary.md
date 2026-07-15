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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T045100Z-3005fdea28ca/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 26854.00 | 23382.33 | 1.148x | +14.85% | - | - | - | 0.703x | +29.68% | 1.116x | -11.60% | 1.211x | -21.14% | 0 |
| game-long-connection | 4044.00 | 3480.00 | 1.162x | +16.21% | - | - | - | 0.747x | +25.27% | 1.133x | -13.29% | 1.409x | -40.90% | 0 |
| generic-sse | 473.00 | 439.33 | 1.077x | +7.66% | - | - | - | 0.825x | +17.50% | 1.316x | -31.57% | 1.458x | -45.81% | 0 |
| https-static-small | 2778.33 | 4875.00 | 0.570x | -43.01% | - | - | - | 1.969x | -96.90% | 1.755x | -75.52% | 1.879x | -87.87% | 0 |
| qcp-transparent | 3448.00 | 3207.33 | 1.075x | +7.50% | - | - | - | 0.809x | +19.13% | 1.206x | -20.60% | 1.478x | -47.82% | 0 |
| reverse-proxy | 10784.33 | 10149.67 | 1.063x | +6.25% | - | - | - | 0.869x | +13.15% | 1.181x | -18.10% | 1.345x | -34.51% | 0 |
| static-large | 86.00 | 85.33 | 1.008x | +0.79% | - | - | - | 0.981x | +1.94% | 0.978x | +2.19% | 1.064x | -6.38% | 0 |
| static-small | 25615.67 | 25054.67 | 1.022x | +2.24% | - | - | - | 0.802x | +19.84% | 1.259x | -25.89% | 1.391x | -39.05% | 0 |
| tcp-stream | 3556.33 | 3425.00 | 1.038x | +3.83% | - | - | - | 0.807x | +19.35% | 1.317x | -31.67% | 1.512x | -51.19% | 0 |
| udp-stream | 3427.00 | 3323.00 | 1.031x | +3.13% | - | - | - | 0.821x | +17.90% | 1.252x | -25.22% | 1.555x | -55.52% | 0 |
| websocket-long-connection | 3785.67 | 3343.67 | 1.132x | +13.22% | - | - | - | 0.752x | +24.81% | 1.143x | -14.34% | 1.301x | -30.12% | 0 |

- Aggregate proxysss ops/s: `84852.33`
- Aggregate nginx ops/s: `80765.33`
- Aggregate proxysss/nginx ratio: `1.051x`
- Aggregate throughput improvement: `+5.06%`
