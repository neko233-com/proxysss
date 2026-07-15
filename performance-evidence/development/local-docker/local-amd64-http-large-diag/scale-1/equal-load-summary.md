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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-large-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.967 < 0.980 (actual=22654.25 target=23426.06); generic-sse nginx target achievement 0.971 < 0.980 (actual=477.50 target=491.52); reverse-proxy nginx target achievement 0.969 < 0.980 (actual=10618.25 target=10962.66); static-small nginx target achievement 0.957 < 0.980 (actual=22607.25 target=23616.24)`
| cdn-hot-update | 21883.00 | 22654.25 | 0.966x | -3.40% | 23426.06 | 0.934x | 0.967x | 0.984x | +1.63% | 1.407x | -40.65% | 1.519x | -51.94% | 0 |
| generic-sse | 470.50 | 477.50 | 0.985x | -1.47% | 491.52 | 0.957x | 0.971x | 1.082x | -8.16% | 1.371x | -37.07% | 1.269x | -26.86% | 1 |
| reverse-proxy | 10340.75 | 10618.25 | 0.974x | -2.61% | 10962.66 | 0.943x | 0.969x | 1.087x | -8.73% | 1.387x | -38.73% | 1.201x | -20.11% | 0 |
| static-large | 71.25 | 71.25 | 1.000x | +0.00% | 71.40 | 0.998x | 0.998x | 1.157x | -15.66% | 1.458x | -45.78% | 1.305x | -30.46% | 0 |
| static-small | 22230.75 | 22607.25 | 0.983x | -1.67% | 23616.24 | 0.941x | 0.957x | 0.993x | +0.71% | 1.394x | -39.36% | 1.435x | -43.53% | 0 |

- Aggregate proxysss ops/s: `54996.25`
- Aggregate nginx ops/s: `56428.50`
- Aggregate proxysss/nginx ratio: `0.975x`
- Aggregate throughput improvement: `-2.54%`
