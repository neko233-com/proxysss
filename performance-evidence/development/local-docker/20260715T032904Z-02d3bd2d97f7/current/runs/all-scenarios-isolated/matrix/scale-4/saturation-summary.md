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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32669.00 | 25306.00 | 1.291x | +29.10% | - | - | - | 0.660x | +33.99% | 1.073x | -7.27% | 1.529x | -52.92% | 0 |
| game-long-connection | 7028.00 | 3777.00 | 1.861x | +86.07% | - | - | - | 0.427x | +57.30% | 0.929x | +7.10% | 1.105x | -10.47% | 0 |
| generic-sse | 1226.50 | 542.00 | 2.263x | +126.29% | - | - | - | 0.417x | +58.26% | 0.656x | +34.37% | 0.704x | +29.58% | 0 |
| https-static-small | 8824.00 | 4996.00 | 1.766x | +76.62% | - | - | - | 0.533x | +46.73% | 0.959x | +4.13% | 0.546x | +45.38% | 0 |
| qcp-transparent | 8334.00 | 3853.50 | 2.163x | +116.27% | - | - | - | 0.373x | +62.67% | 0.671x | +32.94% | 0.904x | +9.55% | 0 |
| reverse-proxy | 15057.00 | 12453.00 | 1.209x | +20.91% | - | - | - | 0.770x | +22.96% | 1.051x | -5.06% | 1.195x | -19.52% | 0 |
| static-large | 89.50 | 98.50 | 0.909x | -9.14% | - | - | - | 1.181x | -18.15% | 1.184x | -18.41% | 1.195x | -19.47% | 0 |
| static-small | 28319.00 | 24629.00 | 1.150x | +14.98% | - | - | - | 0.731x | +26.91% | 1.402x | -40.25% | 1.737x | -73.75% | 0 |
| tcp-stream | 6894.00 | 4273.00 | 1.613x | +61.34% | - | - | - | 0.471x | +52.91% | 0.907x | +9.33% | 1.067x | -6.70% | 0 |
| udp-stream | 8525.00 | 4187.00 | 2.036x | +103.61% | - | - | - | 0.389x | +61.07% | 0.665x | +33.52% | 0.830x | +16.97% | 0 |
| websocket-long-connection | 6674.50 | 3682.50 | 1.812x | +81.25% | - | - | - | 0.438x | +56.19% | 0.960x | +4.05% | 1.016x | -1.60% | 0 |

- Aggregate proxysss ops/s: `123640.50`
- Aggregate nginx ops/s: `87797.50`
- Aggregate proxysss/nginx ratio: `1.408x`
- Aggregate throughput improvement: `+40.82%`
