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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 33925.00 | 26125.67 | 1.299x | +29.85% | - | - | - | 0.624x | +37.58% | 1.132x | -13.20% | 1.136x | -13.64% | 0 |
| game-long-connection | 4159.00 | 3655.00 | 1.138x | +13.79% | - | - | - | 0.722x | +27.82% | 1.291x | -29.10% | 1.166x | -16.57% | 0 |
| generic-sse | 691.00 | 436.33 | 1.584x | +58.37% | - | - | - | 0.537x | +46.29% | 0.942x | +5.80% | 1.066x | -6.60% | 0 |
| https-static-small | 7706.67 | 6362.33 | 1.211x | +21.13% | - | - | - | 0.616x | +38.45% | 1.367x | -36.72% | 1.036x | -3.59% | 0 |
| qcp-transparent | 5513.67 | 3876.33 | 1.422x | +42.24% | - | - | - | 0.553x | +44.74% | 1.092x | -9.15% | 1.089x | -8.92% | 0 |
| reverse-proxy | 14607.33 | 12505.67 | 1.168x | +16.81% | - | - | - | 0.806x | +19.43% | 1.136x | -13.63% | 0.971x | +2.95% | 0 |
| static-large | 97.33 | 95.33 | 1.021x | +2.10% | - | - | - | 0.972x | +2.84% | 0.979x | +2.14% | 1.028x | -2.83% | 0 |
| static-small | 33886.33 | 28442.67 | 1.191x | +19.14% | - | - | - | 0.679x | +32.06% | 1.216x | -21.64% | 1.267x | -26.72% | 0 |
| tcp-stream | 4118.67 | 3500.67 | 1.177x | +17.65% | - | - | - | 0.695x | +30.45% | 1.300x | -30.04% | 1.131x | -13.09% | 0 |
| udp-stream | 5362.67 | 3663.67 | 1.464x | +46.37% | - | - | - | 0.546x | +45.40% | 1.056x | -5.58% | 1.076x | -7.63% | 0 |
| websocket-long-connection | 3805.67 | 3679.00 | 1.034x | +3.44% | - | - | - | 0.828x | +17.19% | 1.400x | -39.99% | 1.187x | -18.68% | 0 |

- Aggregate proxysss ops/s: `113873.34`
- Aggregate nginx ops/s: `92342.67`
- Aggregate proxysss/nginx ratio: `1.233x`
- Aggregate throughput improvement: `+23.32%`
