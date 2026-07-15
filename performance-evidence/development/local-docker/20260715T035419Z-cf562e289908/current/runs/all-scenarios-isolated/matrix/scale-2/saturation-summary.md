# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32170.33 | 25608.00 | 1.256x | +25.63% | - | - | - | 0.632x | +36.84% | 1.275x | -27.47% | 1.301x | -30.08% | 0 |
| game-long-connection | 6477.00 | 3876.00 | 1.671x | +67.11% | - | - | - | 0.430x | +57.01% | 0.923x | +7.65% | 0.914x | +8.58% | 0 |
| generic-sse | 739.67 | 470.00 | 1.574x | +57.38% | - | - | - | 0.514x | +48.57% | 1.065x | -6.51% | 1.245x | -24.55% | 0 |
| https-static-small | 4266.33 | 5882.67 | 0.725x | -27.48% | - | - | - | 1.249x | -24.87% | 2.059x | -105.92% | 1.264x | -26.39% | 0 |
| qcp-transparent | 5562.33 | 4033.00 | 1.379x | +37.92% | - | - | - | 0.544x | +45.57% | 1.167x | -16.75% | 1.307x | -30.65% | 0 |
| reverse-proxy | 15107.67 | 12510.00 | 1.208x | +20.76% | - | - | - | 0.760x | +23.97% | 1.138x | -13.81% | 1.220x | -21.97% | 0 |
| static-large | 92.67 | 95.00 | 0.975x | -2.45% | - | - | - | 0.909x | +9.13% | 1.052x | -5.18% | 6.293x | -529.25% | 0 |
| static-small | 32040.67 | 25480.33 | 1.257x | +25.75% | - | - | - | 0.603x | +39.73% | 1.207x | -20.75% | 1.276x | -27.56% | 0 |
| tcp-stream | 6358.33 | 3891.67 | 1.634x | +63.38% | - | - | - | 0.439x | +56.11% | 0.926x | +7.44% | 0.915x | +8.49% | 0 |
| udp-stream | 6263.33 | 3879.00 | 1.615x | +61.47% | - | - | - | 0.459x | +54.08% | 1.084x | -8.45% | 1.079x | -7.93% | 0 |
| websocket-long-connection | 6182.00 | 3914.33 | 1.579x | +57.93% | - | - | - | 0.475x | +52.45% | 0.924x | +7.55% | 0.887x | +11.33% | 0 |

- Aggregate proxysss ops/s: `115260.33`
- Aggregate nginx ops/s: `89640.00`
- Aggregate proxysss/nginx ratio: `1.286x`
- Aggregate throughput improvement: `+28.58%`
