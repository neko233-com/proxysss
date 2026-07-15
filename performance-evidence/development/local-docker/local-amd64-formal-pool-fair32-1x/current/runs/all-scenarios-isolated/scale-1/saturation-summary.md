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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-pool-fair32-1x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15505.70 | 21558.70 | 0.719x | -28.08% | - | - | - | 0.839x | +16.08% | 2.109x | -110.93% | 2.264x | -126.35% | 0 |
| game-long-connection | 3363.30 | 3751.05 | 0.897x | -10.34% | - | - | - | 0.733x | +26.70% | 1.602x | -60.17% | 1.663x | -66.29% | 0 |
| generic-sse | 350.20 | 477.45 | 0.733x | -26.65% | - | - | - | 0.984x | +1.59% | 2.221x | -122.07% | 2.512x | -151.22% | 0 |
| https-static-small | 3767.70 | 5481.55 | 0.687x | -31.27% | - | - | - | 1.479x | -47.86% | 1.509x | -50.90% | 1.311x | -31.13% | 0 |
| qcp-transparent | 2617.65 | 3607.40 | 0.726x | -27.44% | - | - | - | 0.866x | +13.36% | 2.302x | -130.22% | 2.649x | -164.91% | 0 |
| reverse-proxy | 8007.70 | 11148.00 | 0.718x | -28.17% | - | - | - | 1.046x | -4.57% | 2.169x | -116.91% | 2.370x | -137.02% | 0 |
| static-large | 112.65 | 85.95 | 1.311x | +31.06% | - | - | - | 0.763x | +23.72% | 0.755x | +24.51% | 0.800x | +20.03% | 0 |
| static-small | 15268.30 | 21354.60 | 0.715x | -28.50% | - | - | - | 0.853x | +14.68% | 2.085x | -108.47% | 2.263x | -126.29% | 0 |
| tcp-stream | 3356.90 | 3730.05 | 0.900x | -10.00% | - | - | - | 0.738x | +26.17% | 1.583x | -58.31% | 1.728x | -72.79% | 0 |
| udp-stream | 2634.70 | 3669.05 | 0.718x | -28.19% | - | - | - | 0.889x | +11.07% | 2.329x | -132.93% | 2.623x | -162.25% | 0 |
| websocket-long-connection | 3181.30 | 3602.65 | 0.883x | -11.70% | - | - | - | 0.745x | +25.45% | 1.613x | -61.32% | 1.686x | -68.59% | 0 |

- Aggregate proxysss ops/s: `58166.10`
- Aggregate nginx ops/s: `78466.45`
- Aggregate proxysss/nginx ratio: `0.741x`
- Aggregate throughput improvement: `-25.87%`
