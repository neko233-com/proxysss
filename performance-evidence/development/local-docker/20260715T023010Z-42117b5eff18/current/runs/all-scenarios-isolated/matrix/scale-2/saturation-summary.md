# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 23648.00 | 24676.00 | 0.958x | -4.17% | - | - | - | 0.823x | +17.70% | 1.470x | -47.03% | 1.546x | -54.56% | 0 |
| game-long-connection | 6181.00 | 4824.50 | 1.281x | +28.12% | - | - | - | 0.529x | +47.14% | 1.078x | -7.83% | 1.076x | -7.59% | 0 |
| generic-sse | 753.50 | 485.00 | 1.554x | +55.36% | - | - | - | 0.446x | +55.37% | 1.373x | -37.34% | 1.274x | -27.39% | 0 |
| https-static-small | 8707.50 | 5352.00 | 1.627x | +62.70% | - | - | - | 0.474x | +52.59% | 0.973x | +2.69% | 0.642x | +35.79% | 0 |
| qcp-transparent | 5545.50 | 4161.00 | 1.333x | +33.27% | - | - | - | 0.478x | +52.16% | 1.333x | -33.26% | 1.348x | -34.79% | 0 |
| reverse-proxy | 11346.50 | 12352.00 | 0.919x | -8.14% | - | - | - | 0.913x | +8.66% | 1.749x | -74.90% | 2.250x | -124.99% | 0 |
| static-large | 90.50 | 89.00 | 1.017x | +1.69% | - | - | - | 1.047x | -4.69% | 0.866x | +13.35% | 0.449x | +55.07% | 0 |
| static-small | 23939.00 | 23755.00 | 1.008x | +0.77% | - | - | - | 0.782x | +21.77% | 1.435x | -43.54% | 1.511x | -51.10% | 0 |
| tcp-stream | 6697.50 | 4529.50 | 1.479x | +47.86% | - | - | - | 0.476x | +52.43% | 0.933x | +6.71% | 1.067x | -6.69% | 0 |
| udp-stream | 5512.00 | 4877.50 | 1.130x | +13.01% | - | - | - | 0.529x | +47.15% | 1.406x | -40.57% | 1.402x | -40.16% | 0 |
| websocket-long-connection | 5978.50 | 3851.50 | 1.552x | +55.23% | - | - | - | 0.490x | +50.95% | 0.947x | +5.30% | 0.932x | +6.76% | 0 |

- Aggregate proxysss ops/s: `98399.50`
- Aggregate nginx ops/s: `88953.00`
- Aggregate proxysss/nginx ratio: `1.106x`
- Aggregate throughput improvement: `+10.62%`
