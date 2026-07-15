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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-one-minute-gate-r3/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 19621.00 | 24023.50 | 0.817x | -18.33% | - | - | - | 1.047x | -4.73% | 1.240x | -24.02% | 1.343x | -34.31% | 0 |
| game-long-connection | 4412.00 | 4160.50 | 1.060x | +6.04% | - | - | - | 0.928x | +7.19% | 0.983x | +1.72% | 1.140x | -14.03% | 0 |
| generic-sse | 481.50 | 502.00 | 0.959x | -4.08% | - | - | - | 0.859x | +14.10% | 1.435x | -43.48% | 1.417x | -41.73% | 0 |
| https-static-small | 5255.50 | 5832.50 | 0.901x | -9.89% | - | - | - | 1.024x | -2.42% | 1.166x | -16.60% | 1.052x | -5.17% | 0 |
| qcp-transparent | 3809.50 | 3831.00 | 0.994x | -0.56% | - | - | - | 0.811x | +18.92% | 1.206x | -20.58% | 1.243x | -24.30% | 0 |
| reverse-proxy | 9851.00 | 10674.50 | 0.923x | -7.71% | - | - | - | 1.094x | -9.38% | 1.187x | -18.67% | 1.046x | -4.62% | 0 |
| static-large | 101.50 | 87.00 | 1.167x | +16.67% | - | - | - | 0.850x | +14.96% | 0.810x | +19.01% | 1.105x | -10.52% | 0 |
| static-small | 19561.00 | 22939.50 | 0.853x | -14.73% | - | - | - | 0.952x | +4.80% | 1.289x | -28.87% | 1.171x | -17.09% | 0 |
| tcp-stream | 4385.50 | 4126.00 | 1.063x | +6.29% | - | - | - | 0.943x | +5.66% | 1.006x | -0.56% | 1.076x | -7.59% | 0 |
| udp-stream | 3866.00 | 3872.00 | 0.998x | -0.15% | - | - | - | 0.771x | +22.92% | 1.170x | -17.04% | 1.217x | -21.75% | 0 |
| websocket-long-connection | 4210.00 | 3941.50 | 1.068x | +6.81% | - | - | - | 0.931x | +6.86% | 1.021x | -2.11% | 1.119x | -11.88% | 0 |

- Aggregate proxysss ops/s: `75554.50`
- Aggregate nginx ops/s: `83990.00`
- Aggregate proxysss/nginx ratio: `0.900x`
- Aggregate throughput improvement: `-10.04%`
