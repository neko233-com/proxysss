# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22351.00 | 23200.50 | 0.963x | -3.66% | - | - | - | 1.028x | -2.81% | 1.603x | -60.30% | 1.420x | -41.96% | 0 |
| game-long-connection | 6731.00 | 4378.50 | 1.537x | +53.73% | - | - | - | 0.531x | +46.86% | 0.864x | +13.62% | 0.748x | +25.25% | 0 |
| generic-sse | 962.00 | 565.50 | 1.701x | +70.11% | - | - | - | 0.516x | +48.37% | 0.923x | +7.69% | 0.645x | +35.49% | 0 |
| https-static-small | 5903.50 | 5288.00 | 1.116x | +11.64% | - | - | - | 0.718x | +28.18% | 1.051x | -5.08% | 1.217x | -21.67% | 0 |
| qcp-transparent | 8836.00 | 4473.00 | 1.975x | +97.54% | - | - | - | 0.312x | +68.83% | 0.729x | +27.12% | 0.888x | +11.21% | 0 |
| reverse-proxy | 11485.50 | 12344.50 | 0.930x | -6.96% | - | - | - | 0.939x | +6.10% | 1.392x | -39.18% | 1.269x | -26.86% | 0 |
| static-large | 82.50 | 91.00 | 0.907x | -9.34% | - | - | - | 1.200x | -20.04% | 0.968x | +3.25% | 0.982x | +1.77% | 0 |
| static-small | 25468.50 | 22522.50 | 1.131x | +13.08% | - | - | - | 0.738x | +26.15% | 1.207x | -20.72% | 1.373x | -37.28% | 0 |
| tcp-stream | 8807.50 | 4525.00 | 1.946x | +94.64% | - | - | - | 0.336x | +66.41% | 0.899x | +10.15% | 0.898x | +10.20% | 0 |
| udp-stream | 8848.00 | 4600.50 | 1.923x | +92.33% | - | - | - | 0.341x | +65.90% | 0.810x | +18.96% | 1.045x | -4.54% | 0 |
| websocket-long-connection | 6080.50 | 4175.00 | 1.456x | +45.64% | - | - | - | 0.513x | +48.66% | 0.977x | +2.32% | 1.087x | -8.66% | 0 |

- Aggregate proxysss ops/s: `105556.00`
- Aggregate nginx ops/s: `86164.00`
- Aggregate proxysss/nginx ratio: `1.225x`
- Aggregate throughput improvement: `+22.51%`
