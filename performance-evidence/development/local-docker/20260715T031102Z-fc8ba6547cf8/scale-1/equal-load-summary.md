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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031102Z-fc8ba6547cf8/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5666.50 | 5666.00 | 1.000x | +0.01% | 5664.00 | 1.000x | 1.000x | 0.865x | +13.50% | 1.386x | -38.63% | 2.500x | -149.96% | 0 |
| game-long-connection | 884.00 | 884.00 | 1.000x | +0.00% | 884.00 | 1.000x | 1.000x | 1.161x | -16.10% | 1.099x | -9.89% | 2.322x | -132.24% | 0 |
| generic-sse | 89.50 | 89.50 | 1.000x | +0.00% | 89.00 | 1.006x | 1.006x | 1.015x | -1.46% | 1.268x | -26.84% | 1.091x | -9.09% | 0 |
| https-static-small | 426.00 | 426.50 | 0.999x | -0.12% | 424.00 | 1.005x | 1.006x | 0.937x | +6.32% | 1.022x | -2.20% | 2.122x | -112.18% | 0 |
| qcp-transparent | 620.00 | 620.00 | 1.000x | +0.00% | 620.00 | 1.000x | 1.000x | 0.968x | +3.25% | 1.573x | -57.33% | 3.249x | -224.92% | 0 |
| reverse-proxy | 2563.00 | 2562.00 | 1.000x | +0.04% | 2560.00 | 1.001x | 1.001x | 1.056x | -5.61% | 2.459x | -145.95% | 1.370x | -37.02% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 20.00 | 1.050x | 1.050x | 1.013x | -1.34% | 1.185x | -18.50% | 1.047x | -4.66% | 0 |
| static-small | 6049.50 | 6053.00 | 0.999x | -0.06% | 6048.00 | 1.000x | 1.001x | 0.894x | +10.56% | 1.408x | -40.80% | 1.748x | -74.76% | 0 |
| tcp-stream | 868.00 | 868.00 | 1.000x | +0.00% | 868.00 | 1.000x | 1.000x | 1.132x | -13.22% | 1.731x | -73.07% | 0.704x | +29.63% | 0 |
| udp-stream | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.958x | +4.17% | 1.334x | -33.45% | 4.122x | -312.22% | 0 |
| websocket-long-connection | 924.00 | 920.00 | 1.004x | +0.43% | 924.00 | 1.000x | 0.996x | 1.058x | -5.76% | 1.383x | -38.34% | 2.130x | -112.99% | 0 |

- Aggregate proxysss ops/s: `19007.50`
- Aggregate nginx ops/s: `19006.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
