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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-1x2x4x-final/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.966 < 0.980 (actual=1611.35 target=1668.06)`
| cdn-hot-update | 6408.30 | 6391.75 | 1.003x | +0.26% | 6486.92 | 0.988x | 0.985x | 0.754x | +24.59% | 0.803x | +19.73% | 0.782x | +21.84% | 0 |
| game-long-connection | 1151.20 | 1148.90 | 1.002x | +0.20% | 1154.23 | 0.997x | 0.995x | 1.021x | -2.07% | 0.976x | +2.43% | 0.955x | +4.45% | 0 |
| generic-sse | 143.90 | 143.95 | 1.000x | -0.03% | 145.09 | 0.992x | 0.992x | 0.972x | +2.78% | 0.887x | +11.26% | 0.946x | +5.39% | 0 |
| https-static-small | 1624.55 | 1611.35 | 1.008x | +0.82% | 1668.06 | 0.974x | 0.966x | 0.994x | +0.59% | 0.984x | +1.62% | 0.870x | +13.04% | 0 |
| qcp-transparent | 1206.10 | 1204.40 | 1.001x | +0.14% | 1210.65 | 0.996x | 0.995x | 0.879x | +12.08% | 0.872x | +12.84% | 0.860x | +14.03% | 0 |
| reverse-proxy | 3519.25 | 3510.25 | 1.003x | +0.26% | 3551.61 | 0.991x | 0.988x | 0.902x | +9.78% | 0.836x | +16.45% | 0.746x | +25.37% | 0 |
| static-large | 25.60 | 25.60 | 1.000x | +0.00% | 25.60 | 1.000x | 1.000x | 0.989x | +1.15% | 0.951x | +4.93% | 0.704x | +29.63% | 0 |
| static-small | 6223.25 | 6201.95 | 1.003x | +0.34% | 6289.31 | 0.989x | 0.986x | 0.769x | +23.15% | 0.826x | +17.41% | 0.818x | +18.17% | 0 |
| tcp-stream | 1163.90 | 1160.70 | 1.003x | +0.28% | 1166.52 | 0.998x | 0.995x | 0.991x | +0.94% | 0.981x | +1.87% | 1.157x | -15.70% | 0 |
| udp-stream | 1186.65 | 1185.05 | 1.001x | +0.14% | 1190.65 | 0.997x | 0.995x | 0.869x | +13.13% | 0.850x | +15.05% | 0.912x | +8.85% | 0 |
| websocket-long-connection | 1135.60 | 1133.30 | 1.002x | +0.20% | 1138.95 | 0.997x | 0.995x | 1.007x | -0.70% | 0.994x | +0.61% | 1.190x | -19.00% | 0 |

- Aggregate proxysss ops/s: `23788.30`
- Aggregate nginx ops/s: `23717.20`
- Aggregate proxysss/nginx ratio: `1.003x`
- Aggregate throughput improvement: `+0.30%`
