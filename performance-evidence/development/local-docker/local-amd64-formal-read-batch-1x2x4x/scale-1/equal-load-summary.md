# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for `game-long-connection, qcp-transparent, tcp-stream, udp-stream, websocket-long-connection`
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=true, strict=true)
- Saturation ops gate: `false`
- Equal-load latency gate: `true`
- Minimum fixed-load completion: `0.980`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-read-batch-1x2x4x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.973 < 0.980 (actual=1695.35 target=1742.16)`
| cdn-hot-update | 6388.90 | 6361.05 | 1.004x | +0.44% | 6451.61 | 0.990x | 0.986x | 0.737x | +26.26% | 0.634x | +36.56% | 0.502x | +49.77% | 0 |
| game-long-connection | 1162.40 | 1161.20 | 1.001x | +0.10% | 1163.97 | 0.999x | 0.998x | 0.970x | +3.05% | 0.746x | +25.42% | 0.589x | +41.06% | 0 |
| generic-sse | 146.45 | 146.30 | 1.001x | +0.10% | 147.45 | 0.993x | 0.992x | 0.919x | +8.11% | 0.628x | +37.16% | 0.479x | +52.08% | 0 |
| https-static-small | 1696.80 | 1695.35 | 1.001x | +0.09% | 1742.16 | 0.974x | 0.973x | 0.983x | +1.71% | 0.802x | +19.82% | 0.534x | +46.59% | 0 |
| qcp-transparent | 1192.65 | 1193.75 | 0.999x | -0.09% | 1197.25 | 0.996x | 0.997x | 0.826x | +17.43% | 0.640x | +36.04% | 0.701x | +29.94% | 0 |
| reverse-proxy | 3541.70 | 3542.25 | 1.000x | -0.02% | 3583.83 | 0.988x | 0.988x | 0.849x | +15.07% | 0.622x | +37.79% | 0.479x | +52.12% | 0 |
| static-large | 25.30 | 25.30 | 1.000x | +0.00% | 25.36 | 0.998x | 0.998x | 0.990x | +0.96% | 0.865x | +13.51% | 0.727x | +27.33% | 0 |
| static-small | 6345.70 | 6313.50 | 1.005x | +0.51% | 6388.50 | 0.993x | 0.988x | 0.747x | +25.32% | 0.623x | +37.71% | 0.541x | +45.90% | 0 |
| tcp-stream | 1198.80 | 1197.40 | 1.001x | +0.12% | 1199.76 | 0.999x | 0.998x | 0.993x | +0.71% | 0.743x | +25.71% | 0.747x | +25.28% | 0 |
| udp-stream | 1196.00 | 1197.80 | 0.998x | -0.15% | 1200.30 | 0.996x | 0.998x | 0.844x | +15.56% | 0.630x | +36.98% | 0.742x | +25.78% | 0 |
| websocket-long-connection | 1153.60 | 1152.80 | 1.001x | +0.07% | 1155.57 | 0.998x | 0.998x | 0.991x | +0.87% | 0.760x | +24.00% | 0.591x | +40.91% | 0 |

- Aggregate proxysss ops/s: `24048.30`
- Aggregate nginx ops/s: `23986.70`
- Aggregate proxysss/nginx ratio: `1.003x`
- Aggregate throughput improvement: `+0.26%`
