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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6647.50 | 6651.50 | 0.999x | -0.06% | 6624.00 | 1.004x | 1.004x | 0.831x | +16.85% | 0.869x | +13.05% | 2.022x | -102.23% | 0 |
| game-long-connection | 1000.00 | 1000.00 | 1.000x | +0.00% | 1000.00 | 1.000x | 1.000x | 1.018x | -1.84% | 0.790x | +21.03% | 1.052x | -5.22% | 0 |
| generic-sse | 119.50 | 119.50 | 1.000x | +0.00% | 118.00 | 1.013x | 1.013x | 0.988x | +1.18% | 1.697x | -69.70% | 1.492x | -49.19% | 0 |
| https-static-small | 1313.00 | 1313.00 | 1.000x | +0.00% | 1312.00 | 1.001x | 1.001x | 0.976x | +2.35% | 1.160x | -15.96% | 2.015x | -101.51% | 0 |
| qcp-transparent | 1024.00 | 1024.00 | 1.000x | +0.00% | 1024.00 | 1.000x | 1.000x | 0.875x | +12.46% | 1.539x | -53.94% | 2.109x | -110.92% | 0 |
| reverse-proxy | 3210.00 | 3210.50 | 1.000x | -0.02% | 3200.00 | 1.003x | 1.003x | 1.000x | +0.00% | 1.604x | -60.42% | 2.241x | -124.13% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 20.00 | 1.125x | 1.125x | 0.998x | +0.16% | 1.025x | -2.51% | 1.154x | -15.45% | 0 |
| static-small | 6611.00 | 6609.50 | 1.000x | +0.02% | 6592.00 | 1.003x | 1.003x | 0.780x | +21.99% | 0.660x | +34.02% | 1.305x | -30.54% | 0 |
| tcp-stream | 1032.00 | 1032.00 | 1.000x | +0.00% | 1032.00 | 1.000x | 1.000x | 1.068x | -6.79% | 1.054x | -5.44% | 1.475x | -47.51% | 0 |
| udp-stream | 1016.00 | 1008.00 | 1.008x | +0.79% | 1016.00 | 1.000x | 0.992x | 0.898x | +10.18% | 1.258x | -25.83% | 1.371x | -37.14% | 0 |
| websocket-long-connection | 1048.00 | 1048.00 | 1.000x | +0.00% | 1048.00 | 1.000x | 1.000x | 1.012x | -1.24% | 1.268x | -26.83% | 1.553x | -55.27% | 0 |

- Aggregate proxysss ops/s: `23043.50`
- Aggregate nginx ops/s: `23038.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
