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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 19856.50 | 23414.00 | 0.848x | -15.19% | - | - | - | 0.909x | +9.06% | 2.042x | -104.24% | 2.673x | -167.30% | 0 |
| game-long-connection | 7047.50 | 4492.50 | 1.569x | +56.87% | - | - | - | 0.482x | +51.83% | 0.759x | +24.08% | 0.906x | +9.36% | 0 |
| generic-sse | 621.00 | 577.50 | 1.075x | +7.53% | - | - | - | 0.673x | +32.71% | 1.599x | -59.86% | 2.291x | -129.07% | 0 |
| https-static-small | 5155.00 | 4014.50 | 1.284x | +28.41% | - | - | - | 0.931x | +6.91% | 1.127x | -12.75% | 0.690x | +31.01% | 0 |
| qcp-transparent | 3869.50 | 3866.00 | 1.001x | +0.09% | - | - | - | 0.760x | +23.96% | 1.625x | -62.45% | 2.013x | -101.32% | 0 |
| reverse-proxy | 10079.50 | 10105.00 | 0.997x | -0.25% | - | - | - | 0.805x | +19.48% | 1.762x | -76.21% | 1.909x | -90.88% | 0 |
| static-large | 126.50 | 136.00 | 0.930x | -6.99% | - | - | - | 1.256x | -25.63% | 0.924x | +7.56% | 0.329x | +67.07% | 0 |
| static-small | 21054.00 | 21440.50 | 0.982x | -1.80% | - | - | - | 0.750x | +25.03% | 1.548x | -54.79% | 1.880x | -88.05% | 0 |
| tcp-stream | 7662.50 | 5612.50 | 1.365x | +36.53% | - | - | - | 0.567x | +43.31% | 0.794x | +20.65% | 1.106x | -10.60% | 0 |
| udp-stream | 4063.50 | 3789.00 | 1.072x | +7.24% | - | - | - | 0.697x | +30.26% | 1.540x | -54.01% | 1.681x | -68.08% | 0 |
| websocket-long-connection | 5870.50 | 4829.00 | 1.216x | +21.57% | - | - | - | 0.565x | +43.52% | 0.866x | +13.36% | 0.977x | +2.26% | 0 |

- Aggregate proxysss ops/s: `85406.00`
- Aggregate nginx ops/s: `82276.50`
- Aggregate proxysss/nginx ratio: `1.038x`
- Aggregate throughput improvement: `+3.80%`
