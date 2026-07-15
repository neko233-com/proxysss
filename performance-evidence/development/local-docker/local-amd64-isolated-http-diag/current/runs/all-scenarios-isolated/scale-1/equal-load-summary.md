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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-http-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.962 < 0.980 (actual=25840.75 target=26868.18); generic-sse nginx target achievement 0.974 < 0.980 (actual=526.50 target=540.69); reverse-proxy nginx target achievement 0.959 < 0.980 (actual=13832.75 target=14427.41); static-small nginx target achievement 0.956 < 0.980 (actual=24910.75 target=26058.63)`
| cdn-hot-update | 24558.00 | 25840.75 | 0.950x | -4.96% | 26868.18 | 0.914x | 0.962x | 0.967x | +3.27% | 1.667x | -66.70% | 1.818x | -81.85% | 0 |
| generic-sse | 514.50 | 526.50 | 0.977x | -2.28% | 540.69 | 0.952x | 0.974x | 1.065x | -6.55% | 1.520x | -52.02% | 1.484x | -48.41% | 1 |
| reverse-proxy | 13224.25 | 13832.75 | 0.956x | -4.40% | 14427.41 | 0.917x | 0.959x | 1.056x | -5.65% | 1.487x | -48.71% | 1.579x | -57.92% | 0 |
| static-small | 23928.00 | 24910.75 | 0.961x | -3.95% | 26058.63 | 0.918x | 0.956x | 0.963x | +3.74% | 1.635x | -63.53% | 1.649x | -64.95% | 0 |

- Aggregate proxysss ops/s: `62224.75`
- Aggregate nginx ops/s: `65110.75`
- Aggregate proxysss/nginx ratio: `0.956x`
- Aggregate throughput improvement: `-4.43%`
