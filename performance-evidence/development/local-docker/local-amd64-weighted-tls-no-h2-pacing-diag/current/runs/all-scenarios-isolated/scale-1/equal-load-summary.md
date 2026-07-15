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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-weighted-tls-no-h2-pacing-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.940 < 0.980 (actual=6870.75 target=7310.94); game-long-connection nginx target achievement 0.955 < 0.980 (actual=1512.75 target=1584.79); generic-sse nginx target achievement 0.958 < 0.980 (actual=149.50 target=155.99); https-static-small nginx target achievement 0.903 < 0.980 (actual=2115.75 target=2343.98); qcp-transparent nginx target achievement 0.963 < 0.980 (actual=1181.25 target=1226.24); reverse-proxy nginx target achievement 0.938 < 0.980 (actual=3510.75 target=3740.94); static-small nginx target achievement 0.928 < 0.980 (actual=6452.75 target=6956.52); tcp-stream nginx target achievement 0.954 < 0.980 (actual=1498.25 target=1570.17); udp-stream nginx target achievement 0.962 < 0.980 (actual=1135.00 target=1180.12); websocket-long-connection nginx target achievement 0.949 < 0.980 (actual=1423.25 target=1499.81)`
| cdn-hot-update | 6977.75 | 6870.75 | 1.016x | +1.56% | 7310.94 | 0.954x | 0.940x | 0.677x | +32.28% | 0.790x | +21.04% | 1.204x | -20.38% | 0 |
| game-long-connection | 1566.25 | 1512.75 | 1.035x | +3.54% | 1584.79 | 0.988x | 0.955x | 0.992x | +0.78% | 1.101x | -10.14% | 0.732x | +26.75% | 0 |
| generic-sse | 151.25 | 149.50 | 1.012x | +1.17% | 155.99 | 0.970x | 0.958x | 0.884x | +11.55% | 0.675x | +32.49% | 0.948x | +5.17% | 0 |
| https-static-small | 2170.25 | 2115.75 | 1.026x | +2.58% | 2343.98 | 0.926x | 0.903x | 1.004x | -0.40% | 0.920x | +7.96% | 0.851x | +14.87% | 0 |
| qcp-transparent | 1194.00 | 1181.25 | 1.011x | +1.08% | 1226.24 | 0.974x | 0.963x | 0.771x | +22.87% | 1.074x | -7.45% | 0.948x | +5.23% | 0 |
| reverse-proxy | 3599.00 | 3510.75 | 1.025x | +2.51% | 3740.94 | 0.962x | 0.938x | 0.775x | +22.49% | 0.845x | +15.50% | 0.882x | +11.76% | 0 |
| static-large | 47.25 | 46.75 | 1.011x | +1.07% | 47.50 | 0.995x | 0.984x | 0.935x | +6.49% | 0.758x | +24.22% | 0.779x | +22.13% | 0 |
| static-small | 6631.00 | 6452.75 | 1.028x | +2.76% | 6956.52 | 0.953x | 0.928x | 0.746x | +25.38% | 0.870x | +12.97% | 1.022x | -2.15% | 0 |
| tcp-stream | 1551.75 | 1498.25 | 1.036x | +3.57% | 1570.17 | 0.988x | 0.954x | 0.996x | +0.39% | 0.978x | +2.23% | 0.738x | +26.22% | 0 |
| udp-stream | 1151.00 | 1135.00 | 1.014x | +1.41% | 1180.12 | 0.975x | 0.962x | 0.766x | +23.45% | 0.844x | +15.57% | 0.847x | +15.27% | 0 |
| websocket-long-connection | 1485.00 | 1423.25 | 1.043x | +4.34% | 1499.81 | 0.990x | 0.949x | 0.912x | +8.77% | 1.193x | -19.27% | 0.582x | +41.84% | 0 |

- Aggregate proxysss ops/s: `26524.50`
- Aggregate nginx ops/s: `25896.75`
- Aggregate proxysss/nginx ratio: `1.024x`
- Aggregate throughput improvement: `+2.42%`
