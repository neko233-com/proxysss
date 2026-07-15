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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.952 < 0.980 (actual=20.00 target=21.00)`
| cdn-hot-update | 6603.50 | 6599.50 | 1.001x | +0.06% | 6612.94 | 0.999x | 0.998x | 0.839x | +16.09% | 1.314x | -31.39% | 1.602x | -60.17% | 0 |
| game-long-connection | 1064.00 | 1064.00 | 1.000x | +0.00% | 1069.38 | 0.995x | 0.995x | 1.068x | -6.84% | 1.628x | -62.80% | 1.595x | -59.47% | 0 |
| generic-sse | 112.00 | 112.00 | 1.000x | +0.00% | 112.75 | 0.993x | 0.993x | 0.964x | +3.57% | 1.636x | -63.64% | 1.196x | -19.58% | 0 |
| https-static-small | 1195.50 | 1194.50 | 1.001x | +0.08% | 1197.07 | 0.999x | 0.998x | 0.935x | +6.48% | 1.256x | -25.63% | 1.250x | -25.01% | 0 |
| qcp-transparent | 920.00 | 920.00 | 1.000x | +0.00% | 922.40 | 0.997x | 0.997x | 0.936x | +6.39% | 2.154x | -115.36% | 2.078x | -107.85% | 0 |
| reverse-proxy | 2775.50 | 2772.00 | 1.001x | +0.13% | 2778.26 | 0.999x | 0.998x | 0.991x | +0.91% | 1.939x | -93.86% | 0.920x | +8.00% | 0 |
| static-large | 20.00 | 20.00 | 1.000x | +0.00% | 21.00 | 0.952x | 0.952x | 1.024x | -2.41% | 1.223x | -22.29% | 1.172x | -17.20% | 0 |
| static-small | 6239.50 | 6246.50 | 0.999x | -0.11% | 6250.00 | 0.998x | 0.999x | 0.842x | +15.78% | 1.665x | -66.51% | 2.261x | -126.08% | 0 |
| tcp-stream | 1080.00 | 1076.00 | 1.004x | +0.37% | 1081.37 | 0.999x | 0.995x | 1.031x | -3.11% | 1.498x | -49.77% | 2.424x | -142.35% | 0 |
| udp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 965.37 | 0.994x | 0.994x | 0.921x | +7.91% | 2.023x | -102.35% | 1.821x | -82.05% | 0 |
| websocket-long-connection | 904.00 | 904.00 | 1.000x | +0.00% | 906.93 | 0.997x | 0.997x | 0.998x | +0.16% | 1.990x | -98.96% | 1.048x | -4.85% | 0 |

- Aggregate proxysss ops/s: `21874.00`
- Aggregate nginx ops/s: `21868.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.03%`
