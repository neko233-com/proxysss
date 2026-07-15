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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 23044.00 | 22854.00 | 1.008x | +0.83% | - | - | - | 0.837x | +16.26% | 1.274x | -27.37% | 1.152x | -15.21% | 0 |
| game-long-connection | 4725.50 | 3597.00 | 1.314x | +31.37% | - | - | - | 0.693x | +30.70% | 0.860x | +13.99% | 0.884x | +11.55% | 0 |
| generic-sse | 443.50 | 438.00 | 1.013x | +1.26% | - | - | - | 0.809x | +19.14% | 1.218x | -21.75% | 1.276x | -27.64% | 0 |
| https-static-small | 4314.00 | 4571.00 | 0.944x | -5.62% | - | - | - | 0.892x | +10.82% | 0.989x | +1.06% | 1.162x | -16.17% | 0 |
| qcp-transparent | 3549.50 | 3303.00 | 1.075x | +7.46% | - | - | - | 0.725x | +27.55% | 1.130x | -13.00% | 1.235x | -23.45% | 0 |
| reverse-proxy | 9854.50 | 8763.50 | 1.124x | +12.45% | - | - | - | 0.815x | +18.52% | 1.019x | -1.88% | 1.032x | -3.22% | 0 |
| static-large | 120.00 | 118.50 | 1.013x | +1.27% | - | - | - | 1.041x | -4.08% | 0.970x | +3.02% | 0.811x | +18.88% | 0 |
| static-small | 23128.50 | 24975.50 | 0.926x | -7.40% | - | - | - | 0.858x | +14.22% | 1.322x | -32.21% | 1.085x | -8.48% | 0 |
| tcp-stream | 4841.00 | 3509.00 | 1.380x | +37.96% | - | - | - | 0.657x | +34.31% | 0.869x | +13.08% | 0.924x | +7.61% | 0 |
| udp-stream | 3411.00 | 3378.00 | 1.010x | +0.98% | - | - | - | 0.777x | +22.27% | 1.237x | -23.67% | 1.202x | -20.24% | 0 |
| websocket-long-connection | 4832.00 | 3349.50 | 1.443x | +44.26% | - | - | - | 0.627x | +37.31% | 0.892x | +10.75% | 1.094x | -9.36% | 0 |

- Aggregate proxysss ops/s: `82263.50`
- Aggregate nginx ops/s: `78857.00`
- Aggregate proxysss/nginx ratio: `1.043x`
- Aggregate throughput improvement: `+4.32%`
