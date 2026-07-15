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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7033.33 | 7034.00 | 1.000x | -0.01% | 7029.33 | 1.001x | 1.001x | 0.821x | +17.92% | 0.688x | +31.23% | 0.901x | +9.94% | 0 |
| game-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.083x | -8.30% | 1.188x | -18.83% | 0.980x | +1.97% | 0 |
| generic-sse | 112.33 | 112.33 | 1.000x | +0.00% | 112.00 | 1.003x | 1.003x | 0.978x | +2.21% | 0.873x | +12.71% | 0.896x | +10.41% | 0 |
| https-static-small | 1375.33 | 1376.00 | 1.000x | -0.05% | 1376.00 | 1.000x | 1.000x | 0.951x | +4.94% | 0.876x | +12.42% | 0.743x | +25.65% | 0 |
| qcp-transparent | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.873x | +12.73% | 1.127x | -12.72% | 1.839x | -83.95% | 0 |
| reverse-proxy | 2813.00 | 2812.67 | 1.000x | +0.01% | 2805.33 | 1.003x | 1.003x | 0.959x | +4.13% | 0.910x | +8.98% | 0.935x | +6.47% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 21.33 | 1.031x | 1.031x | 1.038x | -3.78% | 1.055x | -5.47% | 2.166x | -116.61% | 0 |
| static-small | 7037.33 | 7037.67 | 1.000x | -0.00% | 7029.33 | 1.001x | 1.001x | 0.862x | +13.79% | 0.721x | +27.89% | 1.080x | -7.98% | 0 |
| tcp-stream | 898.67 | 898.67 | 1.000x | +0.00% | 898.67 | 1.000x | 1.000x | 1.126x | -12.61% | 1.442x | -44.23% | 2.174x | -117.36% | 0 |
| udp-stream | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.887x | +11.33% | 0.898x | +10.16% | 1.362x | -36.22% | 0 |
| websocket-long-connection | 858.67 | 858.67 | 1.000x | +0.00% | 858.67 | 1.000x | 1.000x | 1.021x | -2.08% | 1.067x | -6.68% | 0.909x | +9.09% | 0 |

- Aggregate proxysss ops/s: `22838.66`
- Aggregate nginx ops/s: `22840.01`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
