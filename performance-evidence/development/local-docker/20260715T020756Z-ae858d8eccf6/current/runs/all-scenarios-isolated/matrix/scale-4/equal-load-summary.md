# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4960.50 | 4961.00 | 1.000x | -0.01% | 4963.93 | 0.999x | 0.999x | 0.817x | +18.34% | 2.035x | -103.53% | 1.047x | -4.75% | 0 |
| game-long-connection | 1120.00 | 1120.00 | 1.000x | +0.00% | 1123.12 | 0.997x | 0.997x | 1.116x | -11.59% | 1.216x | -21.64% | 0.513x | +48.67% | 0 |
| generic-sse | 144.00 | 144.00 | 1.000x | +0.00% | 144.37 | 0.997x | 0.997x | 0.989x | +1.14% | 1.708x | -70.81% | 1.672x | -67.20% | 0 |
| https-static-small | 1003.00 | 1003.00 | 1.000x | +0.00% | 1003.61 | 0.999x | 0.999x | 1.075x | -7.46% | 5.696x | -469.58% | 5.015x | -401.49% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 966.48 | 0.993x | 0.993x | 0.835x | +16.48% | 2.685x | -168.46% | 5.163x | -416.33% | 0 |
| reverse-proxy | 2518.00 | 2518.00 | 1.000x | +0.00% | 2519.83 | 0.999x | 0.999x | 1.027x | -2.74% | 1.721x | -72.09% | 1.005x | -0.53% | 0 |
| static-large | 31.50 | 31.50 | 1.000x | +0.00% | 31.62 | 0.996x | 0.996x | 0.943x | +5.73% | 1.110x | -10.98% | 0.806x | +19.39% | 0 |
| static-small | 5260.00 | 5260.50 | 1.000x | -0.01% | 5263.37 | 0.999x | 0.999x | 0.805x | +19.54% | 1.548x | -54.76% | 0.736x | +26.42% | 0 |
| tcp-stream | 1392.00 | 1392.00 | 1.000x | +0.00% | 1403.08 | 0.992x | 0.992x | 1.111x | -11.11% | 0.776x | +22.37% | 0.237x | +76.27% | 0 |
| udp-stream | 944.00 | 944.00 | 1.000x | +0.00% | 947.22 | 0.997x | 0.997x | 0.860x | +14.02% | 2.538x | -153.78% | 1.097x | -9.65% | 0 |
| websocket-long-connection | 1200.00 | 1200.00 | 1.000x | +0.00% | 1207.23 | 0.994x | 0.994x | 0.954x | +4.56% | 1.371x | -37.10% | 4.549x | -354.90% | 0 |

- Aggregate proxysss ops/s: `19533.00`
- Aggregate nginx ops/s: `19534.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
