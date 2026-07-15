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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32347.50 | 25804.50 | 1.254x | +25.36% | - | - | - | 0.659x | +34.06% | 1.108x | -10.82% | 1.219x | -21.86% | 0 |
| game-long-connection | 5816.50 | 3526.00 | 1.650x | +64.96% | - | - | - | 0.495x | +50.51% | 0.944x | +5.59% | 1.054x | -5.42% | 0 |
| generic-sse | 918.50 | 453.00 | 2.028x | +102.76% | - | - | - | 0.439x | +56.07% | 0.744x | +25.60% | 0.949x | +5.13% | 0 |
| https-static-small | 8190.50 | 5508.00 | 1.487x | +48.70% | - | - | - | 0.533x | +46.73% | 1.021x | -2.10% | 0.904x | +9.56% | 0 |
| qcp-transparent | 6296.00 | 3495.50 | 1.801x | +80.12% | - | - | - | 0.473x | +52.70% | 0.746x | +25.37% | 0.753x | +24.71% | 0 |
| reverse-proxy | 14416.50 | 11959.50 | 1.205x | +20.54% | - | - | - | 0.826x | +17.37% | 1.016x | -1.57% | 1.051x | -5.13% | 0 |
| static-large | 89.50 | 91.50 | 0.978x | -2.19% | - | - | - | 1.022x | -2.20% | 1.161x | -16.13% | 2.125x | -112.48% | 0 |
| static-small | 33685.00 | 25659.50 | 1.313x | +31.28% | - | - | - | 0.670x | +33.00% | 0.982x | +1.79% | 0.925x | +7.49% | 0 |
| tcp-stream | 5639.50 | 3562.00 | 1.583x | +58.32% | - | - | - | 0.497x | +50.26% | 0.984x | +1.64% | 1.159x | -15.95% | 0 |
| udp-stream | 6563.00 | 3460.50 | 1.897x | +89.65% | - | - | - | 0.440x | +56.03% | 0.730x | +26.97% | 0.960x | +3.95% | 0 |
| websocket-long-connection | 5128.50 | 3447.50 | 1.488x | +48.76% | - | - | - | 0.580x | +42.02% | 0.993x | +0.71% | 1.047x | -4.69% | 0 |

- Aggregate proxysss ops/s: `119091.00`
- Aggregate nginx ops/s: `86967.50`
- Aggregate proxysss/nginx ratio: `1.369x`
- Aggregate throughput improvement: `+36.94%`
