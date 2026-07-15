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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-large-offload-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.969 < 0.980 (actual=7002.00 target=7223.48); https-static-small nginx target achievement 0.947 < 0.980 (actual=2340.75 target=2472.95); reverse-proxy nginx target achievement 0.967 < 0.980 (actual=3800.00 target=3931.69); static-small nginx target achievement 0.969 < 0.980 (actual=6731.50 target=6944.44)`
| cdn-hot-update | 6509.75 | 7002.00 | 0.930x | -7.03% | 7223.48 | 0.901x | 0.969x | 1.242x | -24.21% | 7.886x | -688.58% | 10.105x | -910.52% | 0 |
| game-long-connection | 2099.00 | 2183.00 | 0.962x | -3.85% | 2202.04 | 0.953x | 0.991x | 2.100x | -110.00% | 4.248x | -324.79% | 4.103x | -310.26% | 0 |
| generic-sse | 156.50 | 166.75 | 0.939x | -6.15% | 169.56 | 0.923x | 0.983x | 1.386x | -38.61% | 6.966x | -596.57% | 9.467x | -846.69% | 1 |
| https-static-small | 2089.00 | 2340.75 | 0.892x | -10.76% | 2472.95 | 0.845x | 0.947x | 1.489x | -48.86% | 2.901x | -190.06% | 2.868x | -186.79% | 0 |
| qcp-transparent | 1204.25 | 1252.00 | 0.962x | -3.81% | 1258.46 | 0.957x | 0.995x | 2.084x | -108.41% | 5.440x | -444.00% | 4.346x | -334.55% | 0 |
| reverse-proxy | 3626.00 | 3800.00 | 0.954x | -4.58% | 3931.69 | 0.922x | 0.967x | 1.470x | -47.02% | 8.277x | -727.72% | 9.976x | -897.61% | 0 |
| static-large | 49.50 | 49.50 | 1.000x | +0.00% | 49.70 | 0.996x | 0.996x | 1.098x | -9.84% | 2.739x | -173.85% | 1.262x | -26.22% | 0 |
| static-small | 6297.75 | 6731.50 | 0.936x | -6.44% | 6944.44 | 0.907x | 0.969x | 1.308x | -30.80% | 8.170x | -717.04% | 10.161x | -916.06% | 0 |
| tcp-stream | 2029.50 | 2101.75 | 0.966x | -3.44% | 2113.05 | 0.960x | 0.995x | 1.994x | -99.45% | 4.264x | -326.41% | 4.759x | -375.85% | 0 |
| udp-stream | 1213.00 | 1262.00 | 0.961x | -3.88% | 1266.62 | 0.958x | 0.996x | 2.031x | -103.11% | 5.393x | -439.26% | 4.237x | -323.72% | 0 |
| websocket-long-connection | 2022.50 | 2106.00 | 0.960x | -3.96% | 2122.58 | 0.953x | 0.992x | 1.897x | -89.68% | 4.073x | -307.31% | 4.339x | -333.94% | 0 |

- Aggregate proxysss ops/s: `27296.75`
- Aggregate nginx ops/s: `28995.25`
- Aggregate proxysss/nginx ratio: `0.941x`
- Aggregate throughput improvement: `-5.86%`
