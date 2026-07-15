# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-volume-gate-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.972 < 0.980 (actual=6679.00 target=6869.90); generic-sse nginx target achievement 0.979 < 0.980 (actual=158.25 target=161.69); https-static-small nginx target achievement 0.948 < 0.980 (actual=2078.75 target=2191.78); reverse-proxy nginx target achievement 0.971 < 0.980 (actual=3537.75 target=3641.74); static-small nginx target achievement 0.974 < 0.980 (actual=6889.50 target=7071.82)`
| cdn-hot-update | 6641.25 | 6679.00 | 0.994x | -0.57% | 6869.90 | 0.967x | 0.972x | 0.850x | +15.04% | 1.158x | -15.82% | 1.988x | -98.76% | 0 |
| game-long-connection | 1494.00 | 1486.50 | 1.005x | +0.50% | 1496.73 | 0.998x | 0.993x | 1.105x | -10.51% | 1.436x | -43.65% | 1.160x | -15.96% | 0 |
| generic-sse | 159.50 | 158.25 | 1.008x | +0.79% | 161.69 | 0.986x | 0.979x | 1.005x | -0.46% | 1.183x | -18.29% | 1.618x | -61.76% | 1 |
| https-static-small | 2016.00 | 2078.75 | 0.970x | -3.02% | 2191.78 | 0.920x | 0.948x | 1.024x | -2.42% | 1.100x | -10.03% | 1.246x | -24.64% | 0 |
| qcp-transparent | 1128.00 | 1128.00 | 1.000x | +0.00% | 1133.95 | 0.995x | 0.995x | 1.120x | -12.03% | 1.493x | -49.27% | 1.590x | -58.97% | 0 |
| reverse-proxy | 3535.00 | 3537.75 | 0.999x | -0.08% | 3641.74 | 0.971x | 0.971x | 0.930x | +7.01% | 1.190x | -18.97% | 1.548x | -54.81% | 0 |
| static-large | 44.75 | 44.75 | 1.000x | +0.00% | 44.80 | 0.999x | 0.999x | 1.008x | -0.84% | 1.089x | -8.93% | 1.265x | -26.49% | 0 |
| static-small | 6862.50 | 6889.50 | 0.996x | -0.39% | 7071.82 | 0.970x | 0.974x | 0.867x | +13.30% | 1.155x | -15.53% | 1.880x | -88.01% | 0 |
| tcp-stream | 1476.00 | 1470.00 | 1.004x | +0.41% | 1479.56 | 0.998x | 0.994x | 1.099x | -9.94% | 1.330x | -33.00% | 1.079x | -7.93% | 0 |
| udp-stream | 1154.25 | 1154.00 | 1.000x | +0.02% | 1158.41 | 0.996x | 0.996x | 1.059x | -5.95% | 1.348x | -34.84% | 1.524x | -52.36% | 0 |
| websocket-long-connection | 1431.50 | 1428.00 | 1.002x | +0.25% | 1440.14 | 0.994x | 0.992x | 1.102x | -10.20% | 1.232x | -23.17% | 1.161x | -16.15% | 0 |

- Aggregate proxysss ops/s: `25942.75`
- Aggregate nginx ops/s: `26054.50`
- Aggregate proxysss/nginx ratio: `0.996x`
- Aggregate throughput improvement: `-0.43%`
