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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-pool-fair32-1x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.969 < 0.980 (actual=1824.90 target=1883.68)`
| cdn-hot-update | 7530.55 | 7622.65 | 0.988x | -1.21% | 7751.94 | 0.971x | 0.983x | 0.847x | +15.32% | 1.114x | -11.35% | 1.529x | -52.94% | 0 |
| game-long-connection | 1651.35 | 1668.45 | 0.990x | -1.02% | 1681.38 | 0.982x | 0.992x | 1.139x | -13.88% | 1.403x | -40.31% | 1.643x | -64.34% | 0 |
| generic-sse | 171.40 | 172.95 | 0.991x | -0.90% | 175.09 | 0.979x | 0.988x | 0.995x | +0.51% | 1.203x | -20.27% | 1.625x | -62.50% | 0 |
| https-static-small | 1787.40 | 1824.90 | 0.979x | -2.05% | 1883.68 | 0.949x | 0.969x | 1.088x | -8.79% | 1.212x | -21.16% | 1.288x | -28.78% | 0 |
| qcp-transparent | 1280.25 | 1297.55 | 0.987x | -1.33% | 1308.69 | 0.978x | 0.991x | 0.965x | +3.47% | 1.172x | -17.18% | 1.620x | -62.05% | 0 |
| reverse-proxy | 3901.85 | 3934.50 | 0.992x | -0.83% | 4003.50 | 0.975x | 0.983x | 0.935x | +6.47% | 1.175x | -17.51% | 1.619x | -61.87% | 0 |
| static-large | 42.90 | 42.90 | 1.000x | +0.00% | 42.97 | 0.998x | 0.998x | 1.001x | -0.11% | 1.101x | -10.07% | 1.759x | -75.85% | 0 |
| static-small | 7425.90 | 7481.75 | 0.993x | -0.75% | 7633.59 | 0.973x | 0.980x | 0.822x | +17.81% | 1.113x | -11.31% | 1.524x | -52.45% | 0 |
| tcp-stream | 1645.70 | 1664.25 | 0.989x | -1.11% | 1678.20 | 0.981x | 0.992x | 1.135x | -13.54% | 1.401x | -40.08% | 1.536x | -53.57% | 0 |
| udp-stream | 1289.65 | 1307.90 | 0.986x | -1.40% | 1317.31 | 0.979x | 0.993x | 0.952x | +4.84% | 1.243x | -24.33% | 1.571x | -57.08% | 0 |
| websocket-long-connection | 1561.90 | 1575.60 | 0.991x | -0.87% | 1590.46 | 0.982x | 0.991x | 1.157x | -15.75% | 1.299x | -29.91% | 1.701x | -70.08% | 0 |

- Aggregate proxysss ops/s: `28288.85`
- Aggregate nginx ops/s: `28593.40`
- Aggregate proxysss/nginx ratio: `0.989x`
- Aggregate throughput improvement: `-1.07%`
