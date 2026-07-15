# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 20320.00 | 23094.00 | 0.880x | -12.01% | - | - | - | 0.707x | +29.32% | 1.929x | -92.95% | 2.548x | -154.83% | 0 |
| game-long-connection | 5377.50 | 3816.50 | 1.409x | +40.90% | - | - | - | 0.573x | +42.67% | 0.925x | +7.49% | 1.055x | -5.46% | 0 |
| generic-sse | 689.50 | 523.50 | 1.317x | +31.71% | - | - | - | 0.476x | +52.43% | 1.487x | -48.68% | 2.369x | -136.88% | 0 |
| https-static-small | 4988.00 | 4488.50 | 1.111x | +11.13% | - | - | - | 0.825x | +17.54% | 1.056x | -5.58% | 1.104x | -10.44% | 0 |
| qcp-transparent | 5253.00 | 3234.50 | 1.624x | +62.41% | - | - | - | 0.394x | +60.65% | 1.246x | -24.57% | 1.357x | -35.72% | 0 |
| reverse-proxy | 10641.50 | 9984.50 | 1.066x | +6.58% | - | - | - | 0.723x | +27.67% | 1.558x | -55.75% | 2.483x | -148.31% | 0 |
| static-large | 123.50 | 132.50 | 0.932x | -6.79% | - | - | - | 1.185x | -18.52% | 1.531x | -53.14% | 0.546x | +45.39% | 0 |
| static-small | 20265.50 | 21297.00 | 0.952x | -4.84% | - | - | - | 0.750x | +24.96% | 2.032x | -103.20% | 2.406x | -140.61% | 0 |
| tcp-stream | 5658.00 | 4598.00 | 1.231x | +23.05% | - | - | - | 0.634x | +36.56% | 0.804x | +19.60% | 1.029x | -2.87% | 0 |
| udp-stream | 5538.50 | 3532.00 | 1.568x | +56.81% | - | - | - | 0.389x | +61.09% | 1.160x | -15.98% | 1.406x | -40.63% | 0 |
| websocket-long-connection | 5307.50 | 3734.00 | 1.421x | +42.14% | - | - | - | 0.541x | +45.91% | 1.021x | -2.08% | 0.940x | +6.00% | 0 |

- Aggregate proxysss ops/s: `84162.50`
- Aggregate nginx ops/s: `78435.00`
- Aggregate proxysss/nginx ratio: `1.073x`
- Aggregate throughput improvement: `+7.30%`
