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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-udp-shared-tls-isolated-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.970 < 0.980 (actual=10913.75 target=11247.80); generic-sse nginx target achievement 0.976 < 0.980 (actual=258.00 target=264.41); https-static-small nginx target achievement 0.941 < 0.980 (actual=2388.00 target=2536.46); reverse-proxy nginx target achievement 0.965 < 0.980 (actual=5574.75 target=5776.17); static-small nginx target achievement 0.958 < 0.980 (actual=10675.25 target=11145.94); tcp-stream nginx target achievement 0.978 < 0.980 (actual=2419.00 target=2472.19)`
| cdn-hot-update | 10869.25 | 10913.75 | 0.996x | -0.41% | 11247.80 | 0.966x | 0.970x | 0.859x | +14.11% | 0.959x | +4.10% | 0.840x | +15.97% | 0 |
| game-long-connection | 2439.25 | 2427.50 | 1.005x | +0.48% | 2473.72 | 0.986x | 0.981x | 1.246x | -24.59% | 1.052x | -5.22% | 0.952x | +4.82% | 0 |
| generic-sse | 259.25 | 258.00 | 1.005x | +0.48% | 264.41 | 0.980x | 0.976x | 0.996x | +0.45% | 0.764x | +23.60% | 0.910x | +9.01% | 1 |
| https-static-small | 2331.50 | 2388.00 | 0.976x | -2.37% | 2536.46 | 0.919x | 0.941x | 1.118x | -11.79% | 0.960x | +3.99% | 0.829x | +17.06% | 0 |
| qcp-transparent | 1933.25 | 1943.25 | 0.995x | -0.51% | 1968.50 | 0.982x | 0.987x | 0.920x | +7.97% | 0.968x | +3.19% | 1.001x | -0.09% | 0 |
| reverse-proxy | 5578.00 | 5574.75 | 1.001x | +0.06% | 5776.17 | 0.966x | 0.965x | 0.931x | +6.92% | 0.746x | +25.40% | 0.742x | +25.78% | 0 |
| static-large | 60.00 | 60.00 | 1.000x | +0.00% | 60.37 | 0.994x | 0.994x | 0.976x | +2.41% | 0.619x | +38.07% | 1.196x | -19.59% | 0 |
| static-small | 10813.50 | 10675.25 | 1.013x | +1.30% | 11145.94 | 0.970x | 0.958x | 0.866x | +13.35% | 0.934x | +6.60% | 0.826x | +17.44% | 0 |
| tcp-stream | 2419.50 | 2419.00 | 1.000x | +0.02% | 2472.19 | 0.979x | 0.978x | 1.288x | -28.81% | 0.907x | +9.29% | 1.059x | -5.94% | 0 |
| udp-stream | 1928.50 | 1945.50 | 0.991x | -0.87% | 1964.64 | 0.982x | 0.990x | 0.943x | +5.71% | 0.829x | +17.10% | 1.004x | -0.37% | 0 |
| websocket-long-connection | 2240.50 | 2246.25 | 0.997x | -0.26% | 2285.06 | 0.980x | 0.983x | 1.189x | -18.88% | 0.979x | +2.06% | 0.938x | +6.22% | 0 |

- Aggregate proxysss ops/s: `40872.50`
- Aggregate nginx ops/s: `40851.25`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.05%`
