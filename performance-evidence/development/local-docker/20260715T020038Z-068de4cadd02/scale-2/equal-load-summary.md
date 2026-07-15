# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5662.00 | 5665.00 | 0.999x | -0.05% | 5671.75 | 0.998x | 0.999x | 0.741x | +25.87% | 1.029x | -2.91% | 1.745x | -74.55% | 0 |
| game-long-connection | 944.00 | 944.00 | 1.000x | +0.00% | 950.74 | 0.993x | 0.993x | 0.869x | +13.06% | 0.602x | +39.82% | 2.315x | -131.51% | 0 |
| generic-sse | 107.00 | 107.00 | 1.000x | +0.00% | 107.25 | 0.998x | 0.998x | 0.955x | +4.46% | 1.184x | -18.43% | 2.243x | -124.34% | 0 |
| https-static-small | 853.00 | 852.00 | 1.001x | +0.12% | 853.97 | 0.999x | 0.998x | 0.847x | +15.31% | 0.776x | +22.41% | 1.047x | -4.68% | 0 |
| qcp-transparent | 1024.00 | 1024.00 | 1.000x | +0.00% | 1035.93 | 0.988x | 0.988x | 0.764x | +23.61% | 0.506x | +49.38% | 0.628x | +37.16% | 0 |
| reverse-proxy | 1894.00 | 1893.00 | 1.001x | +0.05% | 1895.73 | 0.999x | 0.999x | 0.919x | +8.06% | 0.799x | +20.07% | 0.600x | +40.00% | 0 |
| static-large | 31.00 | 31.00 | 1.000x | +0.00% | 31.25 | 0.992x | 0.992x | 0.940x | +6.02% | 1.090x | -9.02% | 1.160x | -15.98% | 0 |
| static-small | 4663.00 | 4661.00 | 1.000x | +0.04% | 4669.15 | 0.999x | 0.998x | 0.704x | +29.65% | 0.554x | +44.57% | 0.806x | +19.35% | 0 |
| tcp-stream | 1104.00 | 1104.00 | 1.000x | +0.00% | 1114.75 | 0.990x | 0.990x | 0.840x | +16.04% | 0.441x | +55.89% | 1.182x | -18.23% | 0 |
| udp-stream | 976.00 | 976.00 | 1.000x | +0.00% | 980.21 | 0.996x | 0.996x | 0.749x | +25.06% | 0.443x | +55.70% | 0.934x | +6.57% | 0 |
| websocket-long-connection | 1292.00 | 1296.00 | 0.997x | -0.31% | 1297.65 | 0.996x | 0.999x | 0.810x | +18.99% | 0.721x | +27.91% | 0.655x | +34.54% | 0 |

- Aggregate proxysss ops/s: `18550.00`
- Aggregate nginx ops/s: `18553.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
