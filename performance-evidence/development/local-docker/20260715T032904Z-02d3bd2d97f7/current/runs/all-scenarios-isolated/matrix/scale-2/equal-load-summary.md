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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6447.00 | 6447.00 | 1.000x | +0.00% | 6432.00 | 1.002x | 1.002x | 0.732x | +26.80% | 0.927x | +7.34% | 1.497x | -49.70% | 0 |
| game-long-connection | 880.00 | 880.00 | 1.000x | +0.00% | 880.00 | 1.000x | 1.000x | 1.397x | -39.71% | 1.622x | -62.23% | 0.924x | +7.57% | 0 |
| generic-sse | 113.00 | 113.00 | 1.000x | +0.00% | 112.00 | 1.009x | 1.009x | 0.980x | +2.05% | 1.418x | -41.80% | 1.483x | -48.33% | 0 |
| https-static-small | 1376.00 | 1376.00 | 1.000x | +0.00% | 1376.00 | 1.000x | 1.000x | 0.925x | +7.45% | 1.433x | -43.30% | 1.127x | -12.66% | 0 |
| qcp-transparent | 872.00 | 872.00 | 1.000x | +0.00% | 872.00 | 1.000x | 1.000x | 0.831x | +16.85% | 0.649x | +35.14% | 1.685x | -68.48% | 0 |
| reverse-proxy | 2988.00 | 2988.50 | 1.000x | -0.02% | 2976.00 | 1.004x | 1.004x | 1.006x | -0.65% | 2.447x | -144.70% | 2.146x | -114.63% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 20.00 | 1.100x | 1.100x | 1.007x | -0.67% | 0.664x | +33.63% | 0.943x | +5.72% | 0 |
| static-small | 6413.00 | 6409.50 | 1.001x | +0.05% | 6400.00 | 1.002x | 1.001x | 0.737x | +26.32% | 1.519x | -51.94% | 1.301x | -30.13% | 0 |
| tcp-stream | 888.00 | 888.00 | 1.000x | +0.00% | 888.00 | 1.000x | 1.000x | 1.196x | -19.63% | 1.500x | -49.95% | 1.613x | -61.30% | 0 |
| udp-stream | 864.00 | 864.00 | 1.000x | +0.00% | 864.00 | 1.000x | 1.000x | 0.858x | +14.24% | 0.696x | +30.38% | 1.436x | -43.63% | 0 |
| websocket-long-connection | 856.00 | 856.00 | 1.000x | +0.00% | 856.00 | 1.000x | 1.000x | 1.109x | -10.85% | 0.865x | +13.48% | 0.832x | +16.83% | 0 |

- Aggregate proxysss ops/s: `21719.00`
- Aggregate nginx ops/s: `21716.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
