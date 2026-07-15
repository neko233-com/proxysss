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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 9833.50 | 16808.00 | 0.585x | -41.50% | - | - | - | 1.102x | -10.25% | 2.433x | -143.27% | 4.863x | -386.32% | 0 |
| game-long-connection | 4275.00 | 3632.50 | 1.177x | +17.69% | - | - | - | 0.678x | +32.20% | 1.022x | -2.22% | 1.238x | -23.80% | 0 |
| generic-sse | 252.50 | 475.00 | 0.532x | -46.84% | - | - | - | 0.971x | +2.90% | 2.574x | -157.36% | 9.133x | -813.34% | 0 |
| https-static-small | 2371.50 | 5337.50 | 0.444x | -55.57% | - | - | - | 1.491x | -49.06% | 1.079x | -7.90% | 1.617x | -61.71% | 0 |
| qcp-transparent | 2818.00 | 3539.50 | 0.796x | -20.38% | - | - | - | 0.661x | +33.85% | 1.907x | -90.67% | 4.578x | -357.84% | 0 |
| reverse-proxy | 5850.00 | 9288.50 | 0.630x | -37.02% | - | - | - | 0.941x | +5.92% | 2.399x | -139.94% | 4.867x | -386.74% | 0 |
| static-large | 88.00 | 88.00 | 1.000x | +0.00% | - | - | - | 1.173x | -17.32% | 1.128x | -12.82% | 0.568x | +43.20% | 0 |
| static-small | 9581.50 | 15863.50 | 0.604x | -39.60% | - | - | - | 1.073x | -7.28% | 2.521x | -152.07% | 5.040x | -404.02% | 0 |
| tcp-stream | 4270.50 | 4057.00 | 1.053x | +5.26% | - | - | - | 0.829x | +17.09% | 0.821x | +17.94% | 1.212x | -21.19% | 0 |
| udp-stream | 2294.00 | 3641.00 | 0.630x | -37.00% | - | - | - | 0.912x | +8.80% | 2.238x | -123.85% | 4.944x | -394.35% | 0 |
| websocket-long-connection | 3656.00 | 2947.50 | 1.240x | +24.04% | - | - | - | 0.676x | +32.37% | 1.035x | -3.46% | 1.353x | -35.30% | 0 |

- Aggregate proxysss ops/s: `45290.50`
- Aggregate nginx ops/s: `65678.00`
- Aggregate proxysss/nginx ratio: `0.690x`
- Aggregate throughput improvement: `-31.04%`
