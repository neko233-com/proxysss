# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 33279.50 | 25786.00 | 1.291x | +29.06% | - | - | - | 0.669x | +33.11% | 0.985x | +1.51% | 0.992x | +0.82% | 0 |
| game-long-connection | 7981.50 | 4048.00 | 1.972x | +97.17% | - | - | - | 0.309x | +69.10% | 0.802x | +19.83% | 0.861x | +13.92% | 0 |
| generic-sse | 923.50 | 542.50 | 1.702x | +70.23% | - | - | - | 0.478x | +52.21% | 0.813x | +18.70% | 0.859x | +14.08% | 0 |
| https-static-small | 6750.00 | 4830.00 | 1.398x | +39.75% | - | - | - | 0.588x | +41.18% | 1.074x | -7.40% | 1.052x | -5.18% | 0 |
| qcp-transparent | 6876.50 | 3899.00 | 1.764x | +76.37% | - | - | - | 0.431x | +56.88% | 0.768x | +23.21% | 0.857x | +14.28% | 0 |
| reverse-proxy | 12865.00 | 11749.00 | 1.095x | +9.50% | - | - | - | 0.854x | +14.56% | 1.111x | -11.14% | 1.432x | -43.20% | 0 |
| static-large | 81.00 | 90.00 | 0.900x | -10.00% | - | - | - | 1.058x | -5.75% | 1.382x | -38.20% | 3.264x | -226.37% | 0 |
| static-small | 30826.50 | 24666.00 | 1.250x | +24.98% | - | - | - | 0.660x | +33.98% | 0.998x | +0.24% | 1.332x | -33.20% | 0 |
| tcp-stream | 6055.50 | 4232.00 | 1.431x | +43.09% | - | - | - | 0.514x | +48.58% | 0.878x | +12.18% | 0.934x | +6.61% | 0 |
| udp-stream | 7010.50 | 5027.50 | 1.394x | +39.44% | - | - | - | 0.498x | +50.16% | 0.846x | +15.41% | 0.860x | +13.98% | 0 |
| websocket-long-connection | 5731.50 | 3875.00 | 1.479x | +47.91% | - | - | - | 0.508x | +49.17% | 0.955x | +4.47% | 0.966x | +3.43% | 0 |

- Aggregate proxysss ops/s: `118381.00`
- Aggregate nginx ops/s: `88745.00`
- Aggregate proxysss/nginx ratio: `1.334x`
- Aggregate throughput improvement: `+33.39%`
