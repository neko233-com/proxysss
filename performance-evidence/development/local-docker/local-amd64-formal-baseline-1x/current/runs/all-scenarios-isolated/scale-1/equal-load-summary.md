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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-baseline-1x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.971 < 0.980 (actual=11225.15 target=11556.52); https-static-small nginx target achievement 0.958 < 0.980 (actual=2548.25 target=2658.69); reverse-proxy nginx target achievement 0.975 < 0.980 (actual=5711.20 target=5859.73); static-small nginx target achievement 0.972 < 0.980 (actual=11183.95 target=11510.79)`
| cdn-hot-update | 10872.60 | 11225.15 | 0.969x | -3.14% | 11556.52 | 0.941x | 0.971x | 0.843x | +15.74% | 1.044x | -4.41% | 1.490x | -48.96% | 0 |
| game-long-connection | 1928.45 | 1960.25 | 0.984x | -1.62% | 1996.01 | 0.966x | 0.982x | 1.129x | -12.89% | 1.007x | -0.73% | 1.514x | -51.39% | 0 |
| generic-sse | 253.35 | 259.85 | 0.975x | -2.50% | 264.62 | 0.957x | 0.982x | 0.939x | +6.14% | 1.006x | -0.64% | 1.825x | -82.47% | 1 |
| https-static-small | 2494.40 | 2548.25 | 0.979x | -2.11% | 2658.69 | 0.938x | 0.958x | 1.093x | -9.32% | 0.992x | +0.75% | 1.309x | -30.90% | 0 |
| qcp-transparent | 1860.95 | 1927.55 | 0.965x | -3.46% | 1960.30 | 0.949x | 0.983x | 0.878x | +12.22% | 0.995x | +0.53% | 1.559x | -55.87% | 0 |
| reverse-proxy | 5555.60 | 5711.20 | 0.973x | -2.72% | 5859.73 | 0.948x | 0.975x | 0.899x | +10.11% | 1.009x | -0.89% | 1.648x | -64.82% | 0 |
| static-large | 50.75 | 51.10 | 0.993x | -0.68% | 51.17 | 0.992x | 0.999x | 0.987x | +1.33% | 1.352x | -35.21% | 1.069x | -6.90% | 0 |
| static-small | 10794.10 | 11183.95 | 0.965x | -3.49% | 11510.79 | 0.938x | 0.972x | 0.819x | +18.14% | 1.024x | -2.45% | 1.421x | -42.08% | 0 |
| tcp-stream | 1928.60 | 1964.65 | 0.982x | -1.83% | 1994.02 | 0.967x | 0.985x | 1.167x | -16.71% | 1.055x | -5.46% | 1.383x | -38.34% | 0 |
| udp-stream | 1922.10 | 1995.15 | 0.963x | -3.66% | 2025.32 | 0.949x | 0.985x | 0.897x | +10.29% | 1.062x | -6.17% | 1.706x | -70.56% | 0 |
| websocket-long-connection | 1799.40 | 1829.45 | 0.984x | -1.64% | 1860.47 | 0.967x | 0.983x | 1.120x | -12.04% | 1.075x | -7.46% | 1.510x | -51.02% | 0 |

- Aggregate proxysss ops/s: `39460.30`
- Aggregate nginx ops/s: `40656.55`
- Aggregate proxysss/nginx ratio: `0.971x`
- Aggregate throughput improvement: `-2.94%`
