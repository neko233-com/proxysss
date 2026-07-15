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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-realtime-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.875 < 0.980 (actual=21184.00 target=24205.75); game-long-connection nginx target achievement 0.948 < 0.980 (actual=2403.75 target=2535.66); generic-sse nginx target achievement 0.919 < 0.980 (actual=430.25 target=468.06); reverse-proxy nginx target achievement 0.888 < 0.980 (actual=10653.25 target=11994.00); static-small nginx target achievement 0.881 < 0.980 (actual=21461.25 target=24371.67); tcp-stream nginx target achievement 0.948 < 0.980 (actual=2425.50 target=2557.54); websocket-long-connection nginx target achievement 0.941 < 0.980 (actual=2250.75 target=2391.63)`
| cdn-hot-update | 6192.25 | 21184.00 | 0.292x | -70.77% | 24205.75 | 0.256x | 0.875x | 2.943x | -194.35% | 8.619x | -761.87% | 11.279x | -1027.87% | 0 |
| game-long-connection | 1220.25 | 2403.75 | 0.508x | -49.24% | 2535.66 | 0.481x | 0.948x | 5.427x | -442.73% | 4.277x | -327.68% | 3.712x | -271.16% | 0 |
| generic-sse | 171.00 | 430.25 | 0.397x | -60.26% | 468.06 | 0.365x | 0.919x | 3.562x | -256.21% | 6.376x | -537.58% | 7.151x | -615.14% | 1 |
| reverse-proxy | 3418.50 | 10653.25 | 0.321x | -67.91% | 11994.00 | 0.285x | 0.888x | 3.947x | -294.70% | 7.064x | -606.36% | 10.572x | -957.21% | 0 |
| static-small | 5923.25 | 21461.25 | 0.276x | -72.40% | 24371.67 | 0.243x | 0.881x | 2.955x | -195.51% | 8.021x | -702.07% | 11.543x | -1054.29% | 0 |
| tcp-stream | 1226.75 | 2425.50 | 0.506x | -49.42% | 2557.54 | 0.480x | 0.948x | 5.495x | -449.54% | 4.496x | -349.61% | 3.829x | -282.86% | 0 |
| websocket-long-connection | 1145.25 | 2250.75 | 0.509x | -49.12% | 2391.63 | 0.479x | 0.941x | 5.129x | -412.95% | 4.089x | -308.86% | 3.292x | -229.15% | 0 |

- Aggregate proxysss ops/s: `19297.25`
- Aggregate nginx ops/s: `60808.75`
- Aggregate proxysss/nginx ratio: `0.317x`
- Aggregate throughput improvement: `-68.27%`
