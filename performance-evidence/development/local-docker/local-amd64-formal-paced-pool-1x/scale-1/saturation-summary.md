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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-paced-pool-1x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15414.10 | 21541.45 | 0.716x | -28.44% | - | - | - | 0.854x | +14.56% | 2.127x | -112.70% | 2.253x | -125.26% | 0 |
| game-long-connection | 3723.20 | 3705.95 | 1.005x | +0.47% | - | - | - | 0.702x | +29.77% | 1.381x | -38.09% | 1.376x | -37.64% | 0 |
| generic-sse | 368.10 | 469.05 | 0.785x | -21.52% | - | - | - | 0.925x | +7.51% | 2.078x | -107.79% | 2.315x | -131.47% | 0 |
| https-static-small | 3767.80 | 5585.45 | 0.675x | -32.54% | - | - | - | 1.518x | -51.79% | 1.541x | -54.14% | 1.319x | -31.85% | 0 |
| qcp-transparent | 2778.45 | 3557.80 | 0.781x | -21.91% | - | - | - | 0.790x | +21.02% | 2.160x | -115.97% | 2.566x | -156.56% | 0 |
| reverse-proxy | 8081.15 | 10959.70 | 0.737x | -26.26% | - | - | - | 1.022x | -2.16% | 2.134x | -113.39% | 2.333x | -133.30% | 0 |
| static-large | 115.70 | 85.20 | 1.358x | +35.80% | - | - | - | 0.733x | +26.67% | 0.734x | +26.64% | 0.638x | +36.16% | 0 |
| static-small | 15687.55 | 21273.65 | 0.737x | -26.26% | - | - | - | 0.859x | +14.14% | 2.006x | -100.59% | 2.202x | -120.17% | 0 |
| tcp-stream | 3684.65 | 3692.00 | 0.998x | -0.20% | - | - | - | 0.693x | +30.68% | 1.418x | -41.80% | 1.408x | -40.79% | 0 |
| udp-stream | 2776.50 | 3534.85 | 0.785x | -21.45% | - | - | - | 0.780x | +22.03% | 2.142x | -114.23% | 2.563x | -156.26% | 0 |
| websocket-long-connection | 3494.25 | 3599.90 | 0.971x | -2.93% | - | - | - | 0.727x | +27.34% | 1.421x | -42.09% | 1.442x | -44.19% | 0 |

- Aggregate proxysss ops/s: `59891.45`
- Aggregate nginx ops/s: `78005.00`
- Aggregate proxysss/nginx ratio: `0.768x`
- Aggregate throughput improvement: `-23.22%`
