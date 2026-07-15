# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `unspecified`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.888 < 0.980 (actual=10766.50 target=12130.40); game-long-connection nginx target achievement 0.863 < 0.980 (actual=5592.75 target=6482.98); generic-sse nginx target achievement 0.906 < 0.980 (actual=331.00 target=365.16); https-static-small nginx target achievement 0.861 < 0.980 (actual=2450.00 target=2844.95); qcp-transparent nginx target achievement 0.936 < 0.980 (actual=10573.00 target=11299.44); reverse-proxy nginx target achievement 0.900 < 0.980 (actual=5990.50 target=6655.57); static-small nginx target achievement 0.866 < 0.980 (actual=12595.50 target=14538.85); tcp-stream nginx target achievement 0.886 < 0.980 (actual=7490.00 target=8456.66); udp-stream nginx target achievement 0.917 < 0.980 (actual=7908.00 target=8620.69); websocket-long-connection nginx target achievement 0.895 < 0.980 (actual=4121.50 target=4605.64)`
| cdn-hot-update | 9811.00 | 10766.50 | 0.911x | -8.87% | 12130.40 | 0.809x | 0.888x | 0.927x | +7.30% | 1.910x | -91.05% | 1.567x | -56.66% | 0 |
| game-long-connection | 5486.75 | 5592.75 | 0.981x | -1.90% | 6482.98 | 0.846x | 0.863x | 1.834x | -83.42% | 1.198x | -19.76% | 0.929x | +7.06% | 0 |
| generic-sse | 292.75 | 331.00 | 0.884x | -11.56% | 365.16 | 0.802x | 0.906x | 1.211x | -21.07% | 1.871x | -87.10% | 1.737x | -73.73% | 0 |
| https-static-small | 2617.00 | 2450.00 | 1.068x | +6.82% | 2844.95 | 0.920x | 0.861x | 1.077x | -7.68% | 0.803x | +19.75% | 0.732x | +26.77% | 0 |
| qcp-transparent | 8411.75 | 10573.00 | 0.796x | -20.44% | 11299.44 | 0.744x | 0.936x | 0.938x | +6.23% | 2.716x | -171.64% | 2.080x | -108.03% | 0 |
| reverse-proxy | 5593.50 | 5990.50 | 0.934x | -6.63% | 6655.57 | 0.840x | 0.900x | 1.249x | -24.86% | 1.718x | -71.80% | 1.565x | -56.53% | 0 |
| static-large | 68.25 | 68.00 | 1.004x | +0.37% | 68.42 | 0.998x | 0.994x | 1.000x | +0.03% | 0.859x | +14.15% | 0.709x | +29.06% | 0 |
| static-small | 11764.00 | 12595.50 | 0.934x | -6.60% | 14538.85 | 0.809x | 0.866x | 0.770x | +23.05% | 1.505x | -50.54% | 1.402x | -40.21% | 0 |
| tcp-stream | 6802.75 | 7490.00 | 0.908x | -9.18% | 8456.66 | 0.804x | 0.886x | 1.293x | -29.31% | 1.420x | -42.04% | 1.093x | -9.26% | 0 |
| udp-stream | 6118.50 | 7908.00 | 0.774x | -22.63% | 8620.69 | 0.710x | 0.917x | 0.958x | +4.23% | 2.396x | -139.58% | 1.819x | -81.90% | 0 |
| websocket-long-connection | 4152.00 | 4121.50 | 1.007x | +0.74% | 4605.64 | 0.902x | 0.895x | 1.853x | -85.26% | 1.065x | -6.49% | 1.030x | -3.02% | 0 |

- Aggregate proxysss ops/s: `61118.25`
- Aggregate nginx ops/s: `67886.75`
- Aggregate proxysss/nginx ratio: `0.900x`
- Aggregate throughput improvement: `-9.97%`
