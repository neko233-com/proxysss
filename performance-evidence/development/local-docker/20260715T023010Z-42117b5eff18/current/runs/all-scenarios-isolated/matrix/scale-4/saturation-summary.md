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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 19984.00 | 21348.00 | 0.936x | -6.39% | - | - | - | 0.883x | +11.74% | 1.518x | -51.76% | 1.549x | -54.86% | 0 |
| game-long-connection | 7239.50 | 7236.00 | 1.000x | +0.05% | - | - | - | 2.149x | -114.87% | 0.904x | +9.60% | 1.061x | -6.08% | 0 |
| generic-sse | 791.00 | 549.00 | 1.441x | +44.08% | - | - | - | 0.497x | +50.30% | 1.344x | -34.43% | 0.760x | +24.00% | 0 |
| https-static-small | 8349.50 | 4793.50 | 1.742x | +74.18% | - | - | - | 0.392x | +60.76% | 0.812x | +18.82% | 0.728x | +27.18% | 0 |
| qcp-transparent | 6692.50 | 3916.00 | 1.709x | +70.90% | - | - | - | 0.367x | +63.29% | 1.074x | -7.37% | 1.255x | -25.49% | 0 |
| reverse-proxy | 9074.50 | 11195.00 | 0.811x | -18.94% | - | - | - | 1.072x | -7.17% | 1.709x | -70.89% | 1.461x | -46.12% | 0 |
| static-large | 93.00 | 91.50 | 1.016x | +1.64% | - | - | - | 1.522x | -52.22% | 0.207x | +79.32% | 0.666x | +33.35% | 0 |
| static-small | 19740.00 | 20375.00 | 0.969x | -3.12% | - | - | - | 0.954x | +4.64% | 1.411x | -41.10% | 1.120x | -12.03% | 0 |
| tcp-stream | 6560.50 | 3855.50 | 1.702x | +70.16% | - | - | - | 0.443x | +55.67% | 0.842x | +15.77% | 1.028x | -2.82% | 0 |
| udp-stream | 8219.00 | 3961.50 | 2.075x | +107.47% | - | - | - | 0.246x | +75.40% | 0.968x | +3.24% | 0.971x | +2.90% | 0 |
| websocket-long-connection | 6474.50 | 4446.00 | 1.456x | +45.63% | - | - | - | 0.471x | +52.91% | 0.844x | +15.61% | 0.921x | +7.87% | 0 |

- Aggregate proxysss ops/s: `93218.00`
- Aggregate nginx ops/s: `81767.00`
- Aggregate proxysss/nginx ratio: `1.140x`
- Aggregate throughput improvement: `+14.00%`
