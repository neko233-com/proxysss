# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.978 < 0.980 (actual=22.00 target=22.50); udp-stream nginx target achievement 0.972 < 0.980 (actual=1152.00 target=1185.23); websocket-long-connection nginx target achievement 0.970 < 0.980 (actual=960.00 target=989.49)`
| cdn-hot-update | 5191.00 | 5193.00 | 1.000x | -0.04% | 5199.66 | 0.998x | 0.999x | 0.845x | +15.47% | 1.277x | -27.74% | 1.266x | -26.60% | 0 |
| game-long-connection | 1024.00 | 1024.00 | 1.000x | +0.00% | 1036.74 | 0.988x | 0.988x | 1.205x | -20.48% | 0.953x | +4.66% | 0.875x | +12.55% | 0 |
| generic-sse | 127.00 | 127.00 | 1.000x | +0.00% | 127.25 | 0.998x | 0.998x | 0.966x | +3.39% | 1.051x | -5.14% | 1.378x | -37.76% | 0 |
| https-static-small | 925.00 | 924.00 | 1.001x | +0.11% | 926.25 | 0.999x | 0.998x | 1.056x | -5.64% | 1.435x | -43.46% | 0.985x | +1.52% | 0 |
| qcp-transparent | 1536.00 | 1536.00 | 1.000x | +0.00% | 1539.72 | 0.998x | 0.998x | 0.820x | +17.98% | 2.115x | -111.51% | 2.002x | -100.19% | 0 |
| reverse-proxy | 2373.00 | 2377.00 | 0.998x | -0.17% | 2378.74 | 0.998x | 0.999x | 1.026x | -2.56% | 2.207x | -120.70% | 1.824x | -82.44% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.50 | 0.978x | 0.978x | 0.958x | +4.16% | 0.824x | +17.62% | 0.941x | +5.88% | 0 |
| static-small | 5209.00 | 5212.00 | 0.999x | -0.06% | 5215.12 | 0.999x | 0.999x | 0.789x | +21.08% | 1.558x | -55.83% | 1.100x | -10.03% | 0 |
| tcp-stream | 1376.00 | 1370.00 | 1.004x | +0.44% | 1378.72 | 0.998x | 0.994x | 1.136x | -13.64% | 0.877x | +12.27% | 0.402x | +59.84% | 0 |
| udp-stream | 1152.00 | 1152.00 | 1.000x | +0.00% | 1185.23 | 0.972x | 0.972x | 0.865x | +13.53% | 1.979x | -97.87% | 3.081x | -208.06% | 0 |
| websocket-long-connection | 960.00 | 960.00 | 1.000x | +0.00% | 989.49 | 0.970x | 0.970x | 1.017x | -1.69% | 1.393x | -39.31% | 1.474x | -47.38% | 0 |

- Aggregate proxysss ops/s: `19895.00`
- Aggregate nginx ops/s: `19897.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
