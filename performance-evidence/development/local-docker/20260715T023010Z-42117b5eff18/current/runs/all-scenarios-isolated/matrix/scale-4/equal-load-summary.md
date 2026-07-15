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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.962 < 0.980 (actual=22.00 target=22.87); udp-stream nginx target achievement 0.969 < 0.980 (actual=960.00 target=990.37); websocket-long-connection nginx target achievement 0.979 < 0.980 (actual=1088.00 target=1111.50)`
| cdn-hot-update | 4990.50 | 4990.00 | 1.000x | +0.01% | 4995.90 | 0.999x | 0.999x | 0.838x | +16.25% | 1.523x | -52.34% | 0.996x | +0.42% | 0 |
| game-long-connection | 1792.00 | 1792.00 | 1.000x | +0.00% | 1808.93 | 0.991x | 0.991x | 1.403x | -40.30% | 1.646x | -64.61% | 1.273x | -27.31% | 0 |
| generic-sse | 137.00 | 137.00 | 1.000x | +0.00% | 137.25 | 0.998x | 0.998x | 1.020x | -1.99% | 1.514x | -51.37% | 0.917x | +8.29% | 0 |
| https-static-small | 1196.00 | 1196.00 | 1.000x | +0.00% | 1198.37 | 0.998x | 0.998x | 1.117x | -11.68% | 4.349x | -334.86% | 0.961x | +3.95% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 978.98 | 0.981x | 0.981x | 0.834x | +16.55% | 1.412x | -41.20% | 4.500x | -350.03% | 0 |
| reverse-proxy | 2266.50 | 2264.50 | 1.001x | +0.09% | 2268.62 | 0.999x | 0.998x | 1.023x | -2.35% | 1.245x | -24.49% | 0.795x | +20.54% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.87 | 0.962x | 0.962x | 0.946x | +5.43% | 1.569x | -56.95% | 0.886x | +11.35% | 0 |
| static-small | 4929.00 | 4922.50 | 1.001x | +0.13% | 4934.84 | 0.999x | 0.997x | 0.833x | +16.67% | 1.191x | -19.08% | 1.011x | -1.06% | 0 |
| tcp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 963.86 | 0.996x | 0.996x | 1.497x | -49.72% | 3.129x | -212.86% | 3.058x | -205.78% | 0 |
| udp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 990.37 | 0.969x | 0.969x | 0.805x | +19.52% | 1.586x | -58.60% | 2.468x | -146.84% | 0 |
| websocket-long-connection | 1088.00 | 1088.00 | 1.000x | +0.00% | 1111.50 | 0.979x | 0.979x | 1.160x | -16.01% | 1.180x | -17.99% | 0.990x | +1.05% | 0 |

- Aggregate proxysss ops/s: `19301.00`
- Aggregate nginx ops/s: `19292.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.05%`
