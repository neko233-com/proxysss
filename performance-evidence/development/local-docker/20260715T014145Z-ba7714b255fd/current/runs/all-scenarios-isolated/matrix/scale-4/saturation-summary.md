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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15175.00 | 17800.00 | 0.853x | -14.75% | - | - | - | 0.898x | +10.23% | 1.730x | -73.05% | 1.968x | -96.78% | 0 |
| game-long-connection | 4438.00 | 3520.00 | 1.261x | +26.08% | - | - | - | 0.706x | +29.35% | 0.969x | +3.10% | 0.898x | +10.25% | 0 |
| generic-sse | 506.00 | 424.00 | 1.193x | +19.34% | - | - | - | 0.776x | +22.43% | 1.075x | -7.48% | 0.799x | +20.11% | 0 |
| https-static-small | 32.00 | 2320.00 | 0.014x | -98.62% | - | - | - | 133.903x | -13290.27% | 50.680x | -4968.02% | 2.889x | -188.92% | 0 |
| qcp-transparent | 6237.00 | 3569.00 | 1.748x | +74.75% | - | - | - | 0.395x | +60.49% | 0.810x | +18.96% | 1.197x | -19.65% | 0 |
| reverse-proxy | 11175.00 | 9831.00 | 1.137x | +13.67% | - | - | - | 0.643x | +35.69% | 1.248x | -24.81% | 1.286x | -28.56% | 0 |
| static-large | 57.00 | 79.00 | 0.722x | -27.85% | - | - | - | 1.863x | -86.28% | 0.906x | +9.41% | 1.346x | -34.63% | 0 |
| static-small | 18119.00 | 13775.00 | 1.315x | +31.54% | - | - | - | 0.618x | +38.17% | 1.089x | -8.87% | 1.285x | -28.55% | 0 |
| tcp-stream | 5956.00 | 5010.00 | 1.189x | +18.88% | - | - | - | 0.612x | +38.77% | 0.863x | +13.70% | 1.016x | -1.63% | 0 |
| udp-stream | 4856.00 | 3522.00 | 1.379x | +37.88% | - | - | - | 0.514x | +48.61% | 0.920x | +8.04% | 1.937x | -93.74% | 0 |
| websocket-long-connection | 4190.00 | 3269.00 | 1.282x | +28.17% | - | - | - | 0.797x | +20.28% | 0.858x | +14.18% | 0.873x | +12.71% | 0 |

- Aggregate proxysss ops/s: `70741.00`
- Aggregate nginx ops/s: `63119.00`
- Aggregate proxysss/nginx ratio: `1.121x`
- Aggregate throughput improvement: `+12.08%`
