# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 20881.50 | 24859.00 | 0.840x | -16.00% | - | - | - | 1.084x | -8.40% | 1.404x | -40.38% | 1.099x | -9.89% | 0 |
| game-long-connection | 4917.00 | 4225.50 | 1.164x | +16.36% | - | - | - | 0.723x | +27.65% | 0.931x | +6.90% | 0.914x | +8.61% | 0 |
| generic-sse | 445.50 | 424.50 | 1.049x | +4.95% | - | - | - | 0.852x | +14.84% | 1.383x | -38.33% | 1.172x | -17.17% | 0 |
| https-static-small | 5700.50 | 4920.50 | 1.159x | +15.85% | - | - | - | 0.725x | +27.46% | 1.020x | -1.97% | 0.748x | +25.20% | 0 |
| qcp-transparent | 3489.50 | 3856.00 | 0.905x | -9.50% | - | - | - | 0.938x | +6.18% | 1.272x | -27.17% | 1.206x | -20.65% | 0 |
| reverse-proxy | 8588.50 | 11199.00 | 0.767x | -23.31% | - | - | - | 1.131x | -13.09% | 2.106x | -110.58% | 1.591x | -59.10% | 0 |
| static-large | 86.50 | 85.50 | 1.012x | +1.17% | - | - | - | 1.073x | -7.32% | 1.134x | -13.37% | 0.657x | +34.30% | 0 |
| static-small | 21699.00 | 24736.50 | 0.877x | -12.28% | - | - | - | 1.012x | -1.17% | 1.296x | -29.56% | 1.131x | -13.10% | 0 |
| tcp-stream | 4684.50 | 3820.00 | 1.226x | +22.63% | - | - | - | 0.718x | +28.18% | 0.975x | +2.49% | 0.852x | +14.82% | 0 |
| udp-stream | 3533.00 | 3650.50 | 0.968x | -3.22% | - | - | - | 0.923x | +7.66% | 1.270x | -27.03% | 1.064x | -6.42% | 0 |
| websocket-long-connection | 4609.50 | 4145.00 | 1.112x | +11.21% | - | - | - | 0.755x | +24.53% | 1.058x | -5.84% | 0.952x | +4.79% | 0 |

- Aggregate proxysss ops/s: `78635.00`
- Aggregate nginx ops/s: `85922.00`
- Aggregate proxysss/nginx ratio: `0.915x`
- Aggregate throughput improvement: `-8.48%`
