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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32822.00 | 25938.33 | 1.265x | +26.54% | - | - | - | 0.649x | +35.13% | 1.109x | -10.92% | 1.059x | -5.93% | 0 |
| game-long-connection | 5196.00 | 3722.67 | 1.396x | +39.58% | - | - | - | 0.645x | +35.47% | 1.034x | -3.42% | 1.008x | -0.82% | 0 |
| generic-sse | 837.00 | 489.00 | 1.712x | +71.17% | - | - | - | 0.499x | +50.07% | 0.861x | +13.95% | 1.021x | -2.10% | 0 |
| https-static-small | 5933.67 | 6595.00 | 0.900x | -10.03% | - | - | - | 0.965x | +3.54% | 1.536x | -53.60% | 1.169x | -16.92% | 0 |
| qcp-transparent | 5588.00 | 3745.33 | 1.492x | +49.20% | - | - | - | 0.557x | +44.27% | 0.977x | +2.31% | 1.091x | -9.12% | 0 |
| reverse-proxy | 14903.33 | 12757.00 | 1.168x | +16.82% | - | - | - | 0.836x | +16.35% | 0.962x | +3.75% | 0.889x | +11.12% | 0 |
| static-large | 95.67 | 97.33 | 0.983x | -1.71% | - | - | - | 1.232x | -23.18% | 1.029x | -2.87% | 0.187x | +81.32% | 0 |
| static-small | 33427.67 | 26444.33 | 1.264x | +26.41% | - | - | - | 0.623x | +37.67% | 1.095x | -9.52% | 1.019x | -1.87% | 0 |
| tcp-stream | 5328.00 | 3853.33 | 1.383x | +38.27% | - | - | - | 0.623x | +37.65% | 1.040x | -3.98% | 0.969x | +3.11% | 0 |
| udp-stream | 6252.33 | 3860.00 | 1.620x | +61.98% | - | - | - | 0.503x | +49.65% | 0.937x | +6.28% | 0.931x | +6.93% | 0 |
| websocket-long-connection | 5112.67 | 3737.00 | 1.368x | +36.81% | - | - | - | 0.652x | +34.84% | 1.031x | -3.07% | 1.081x | -8.12% | 0 |

- Aggregate proxysss ops/s: `115496.34`
- Aggregate nginx ops/s: `91239.32`
- Aggregate proxysss/nginx ratio: `1.266x`
- Aggregate throughput improvement: `+26.59%`
