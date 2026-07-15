# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.956 < 0.980 (actual=19.00 target=19.87); udp-stream nginx target achievement 0.979 < 0.980 (actual=1216.00 target=1241.85)`
| cdn-hot-update | 5470.00 | 5472.00 | 1.000x | -0.04% | 5480.39 | 0.998x | 0.998x | 0.764x | +23.64% | 0.899x | +10.05% | 0.616x | +38.43% | 0 |
| game-long-connection | 1184.00 | 1184.00 | 1.000x | +0.00% | 1198.37 | 0.988x | 0.988x | 1.096x | -9.56% | 0.504x | +49.59% | 0.773x | +22.71% | 0 |
| generic-sse | 143.00 | 143.00 | 1.000x | +0.00% | 144.00 | 0.993x | 0.993x | 0.961x | +3.93% | 0.402x | +59.80% | 0.705x | +29.53% | 0 |
| https-static-small | 1092.50 | 1092.50 | 1.000x | +0.00% | 1093.49 | 0.999x | 0.999x | 1.055x | -5.46% | 0.932x | +6.75% | 1.053x | -5.34% | 0 |
| qcp-transparent | 1568.00 | 1568.00 | 1.000x | +0.00% | 1585.49 | 0.989x | 0.989x | 0.899x | +10.06% | 0.323x | +67.65% | 0.401x | +59.94% | 0 |
| reverse-proxy | 2720.00 | 2720.00 | 1.000x | +0.00% | 2723.69 | 0.999x | 0.999x | 0.975x | +2.55% | 0.444x | +55.59% | 0.451x | +54.94% | 0 |
| static-large | 19.00 | 19.00 | 1.000x | +0.00% | 19.87 | 0.956x | 0.956x | 1.018x | -1.76% | 0.505x | +49.54% | 0.726x | +27.35% | 0 |
| static-small | 4905.50 | 4911.00 | 0.999x | -0.11% | 4912.50 | 0.999x | 1.000x | 0.797x | +20.32% | 0.576x | +42.38% | 0.633x | +36.68% | 0 |
| tcp-stream | 1088.00 | 1088.00 | 1.000x | +0.00% | 1093.72 | 0.995x | 0.995x | 1.140x | -13.99% | 0.800x | +19.97% | 0.576x | +42.43% | 0 |
| udp-stream | 1216.00 | 1216.00 | 1.000x | +0.00% | 1241.85 | 0.979x | 0.979x | 0.870x | +13.02% | 0.460x | +54.01% | 0.550x | +45.02% | 0 |
| websocket-long-connection | 992.00 | 992.00 | 1.000x | +0.00% | 1004.36 | 0.988x | 0.988x | 0.948x | +5.20% | 0.505x | +49.55% | 0.788x | +21.23% | 0 |

- Aggregate proxysss ops/s: `20398.00`
- Aggregate nginx ops/s: `20405.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.04%`
