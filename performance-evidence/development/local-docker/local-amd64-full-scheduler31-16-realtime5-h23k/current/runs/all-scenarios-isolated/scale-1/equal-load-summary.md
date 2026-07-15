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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-scheduler31-16-realtime5-h23k/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.974 < 0.980 (actual=2546.70 target=2613.52)`
| cdn-hot-update | 10162.15 | 10314.85 | 0.985x | -1.48% | 10464.36 | 0.971x | 0.986x | 0.786x | +21.43% | 0.971x | +2.91% | 2.074x | -107.40% | 0 |
| game-long-connection | 1804.70 | 1826.05 | 0.988x | -1.17% | 1837.81 | 0.982x | 0.994x | 1.039x | -3.91% | 1.218x | -21.84% | 2.087x | -108.75% | 0 |
| generic-sse | 226.85 | 228.60 | 0.992x | -0.77% | 231.27 | 0.981x | 0.988x | 0.918x | +8.18% | 1.247x | -24.74% | 1.706x | -70.64% | 0 |
| https-static-small | 2464.15 | 2546.70 | 0.968x | -3.24% | 2613.52 | 0.943x | 0.974x | 1.041x | -4.06% | 1.083x | -8.30% | 1.724x | -72.37% | 0 |
| qcp-transparent | 1747.70 | 1777.20 | 0.983x | -1.66% | 1788.91 | 0.977x | 0.993x | 0.864x | +13.57% | 1.191x | -19.06% | 2.069x | -106.86% | 0 |
| reverse-proxy | 5263.35 | 5333.30 | 0.987x | -1.31% | 5416.38 | 0.972x | 0.985x | 0.881x | +11.94% | 1.091x | -9.13% | 2.345x | -134.49% | 0 |
| static-large | 41.30 | 41.30 | 1.000x | +0.00% | 41.40 | 0.998x | 0.998x | 0.986x | +1.39% | 1.555x | -55.50% | 1.456x | -45.65% | 0 |
| static-small | 10249.40 | 10396.30 | 0.986x | -1.41% | 10564.54 | 0.970x | 0.984x | 0.810x | +19.01% | 1.018x | -1.81% | 2.129x | -112.91% | 0 |
| tcp-stream | 1829.40 | 1853.15 | 0.987x | -1.28% | 1863.93 | 0.981x | 0.994x | 1.005x | -0.48% | 1.197x | -19.70% | 1.958x | -95.77% | 0 |
| udp-stream | 1739.65 | 1767.10 | 0.984x | -1.55% | 1779.36 | 0.978x | 0.993x | 0.821x | +17.87% | 1.217x | -21.71% | 2.138x | -113.77% | 0 |
| websocket-long-connection | 1707.35 | 1729.90 | 0.987x | -1.30% | 1740.64 | 0.981x | 0.994x | 1.020x | -2.02% | 1.218x | -21.82% | 2.057x | -105.75% | 0 |

- Aggregate proxysss ops/s: `37236.00`
- Aggregate nginx ops/s: `37814.45`
- Aggregate proxysss/nginx ratio: `0.985x`
- Aggregate throughput improvement: `-1.53%`
