# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T011840Z-dc838c8ecbbd/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 12501.00 | 11027.00 | 1.134x | +13.37% | - | - | - | 0.804x | +19.57% | 1.012x | -1.23% | 1.071x | -7.11% | 0 |
| game-long-connection | 2502.50 | 2054.00 | 1.218x | +21.84% | - | - | - | 0.928x | +7.20% | 0.820x | +18.05% | 0.671x | +32.89% | 0 |
| generic-sse | 254.50 | 273.00 | 0.932x | -6.78% | - | - | - | 1.001x | -0.05% | 1.366x | -36.58% | 1.011x | -1.08% | 0 |
| https-static-small | 2081.00 | 2046.00 | 1.017x | +1.71% | - | - | - | 0.938x | +6.16% | 0.897x | +10.33% | 1.006x | -0.56% | 0 |
| qcp-transparent | 1845.00 | 1911.00 | 0.965x | -3.45% | - | - | - | 0.887x | +11.30% | 1.176x | -17.58% | 1.144x | -14.39% | 0 |
| reverse-proxy | 5418.50 | 5292.50 | 1.024x | +2.38% | - | - | - | 0.932x | +6.83% | 0.956x | +4.42% | 0.693x | +30.73% | 0 |
| static-large | 88.00 | 74.50 | 1.181x | +18.12% | - | - | - | 0.869x | +13.07% | 0.814x | +18.60% | 0.796x | +20.41% | 0 |
| static-small | 11899.00 | 12428.00 | 0.957x | -4.26% | - | - | - | 0.938x | +6.18% | 1.071x | -7.13% | 1.141x | -14.09% | 0 |
| tcp-stream | 2506.50 | 2078.00 | 1.206x | +20.62% | - | - | - | 0.855x | +14.48% | 0.811x | +18.90% | 0.615x | +38.52% | 0 |
| udp-stream | 1861.50 | 1944.00 | 0.958x | -4.24% | - | - | - | 0.950x | +5.00% | 1.187x | -18.70% | 1.033x | -3.29% | 0 |
| websocket-long-connection | 2136.00 | 1986.00 | 1.076x | +7.55% | - | - | - | 1.014x | -1.38% | 0.886x | +11.43% | 0.785x | +21.47% | 0 |

- Aggregate proxysss ops/s: `43093.50`
- Aggregate nginx ops/s: `41114.00`
- Aggregate proxysss/nginx ratio: `1.048x`
- Aggregate throughput improvement: `+4.81%`
