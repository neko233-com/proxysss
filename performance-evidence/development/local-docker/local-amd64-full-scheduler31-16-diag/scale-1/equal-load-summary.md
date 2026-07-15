# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-scheduler31-16-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.969 < 0.980 (actual=1854.70 target=1914.79)`
| cdn-hot-update | 9911.65 | 10430.10 | 0.950x | -4.97% | 10596.03 | 0.935x | 0.984x | 1.111x | -11.11% | 2.474x | -147.38% | 3.565x | -256.52% | 0 |
| game-long-connection | 1279.65 | 1316.75 | 0.972x | -2.82% | 1323.19 | 0.967x | 0.995x | 1.590x | -59.01% | 2.818x | -181.84% | 3.305x | -230.51% | 0 |
| generic-sse | 228.95 | 237.60 | 0.964x | -3.64% | 240.18 | 0.953x | 0.989x | 1.233x | -23.27% | 2.184x | -118.44% | 2.937x | -193.68% | 0 |
| https-static-small | 1773.95 | 1854.70 | 0.956x | -4.35% | 1914.79 | 0.926x | 0.969x | 1.422x | -42.16% | 2.144x | -114.41% | 2.325x | -132.47% | 0 |
| qcp-transparent | 1783.10 | 1863.90 | 0.957x | -4.33% | 1877.93 | 0.950x | 0.993x | 1.181x | -18.09% | 2.361x | -136.06% | 2.936x | -193.60% | 0 |
| reverse-proxy | 5217.75 | 5406.05 | 0.965x | -3.48% | 5513.44 | 0.946x | 0.981x | 1.221x | -22.10% | 2.480x | -148.01% | 3.726x | -272.65% | 0 |
| static-large | 42.90 | 43.20 | 0.993x | -0.69% | 43.30 | 0.991x | 0.998x | 1.082x | -8.22% | 2.303x | -130.34% | 2.389x | -138.88% | 0 |
| static-small | 10014.55 | 10554.65 | 0.949x | -5.12% | 10713.09 | 0.935x | 0.985x | 1.068x | -6.79% | 2.278x | -127.84% | 3.179x | -217.89% | 0 |
| tcp-stream | 1274.25 | 1310.15 | 0.973x | -2.74% | 1317.31 | 0.967x | 0.995x | 1.594x | -59.43% | 2.973x | -197.28% | 3.477x | -247.66% | 0 |
| udp-stream | 1784.10 | 1870.30 | 0.954x | -4.61% | 1882.80 | 0.948x | 0.993x | 1.251x | -25.05% | 2.434x | -143.36% | 2.875x | -187.47% | 0 |
| websocket-long-connection | 1212.55 | 1243.60 | 0.975x | -2.50% | 1250.39 | 0.970x | 0.995x | 1.613x | -61.33% | 2.859x | -185.93% | 2.975x | -197.47% | 0 |

- Aggregate proxysss ops/s: `34523.40`
- Aggregate nginx ops/s: `36131.00`
- Aggregate proxysss/nginx ratio: `0.956x`
- Aggregate throughput improvement: `-4.45%`
