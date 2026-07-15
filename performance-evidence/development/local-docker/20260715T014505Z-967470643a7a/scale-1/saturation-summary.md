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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 18555.50 | 18911.50 | 0.981x | -1.88% | - | - | - | 0.839x | +16.14% | 1.030x | -3.00% | 1.074x | -7.42% | 0 |
| game-long-connection | 3532.50 | 2907.00 | 1.215x | +21.52% | - | - | - | 0.855x | +14.52% | 0.834x | +16.55% | 0.855x | +14.51% | 0 |
| generic-sse | 400.50 | 341.50 | 1.173x | +17.28% | - | - | - | 0.845x | +15.45% | 1.024x | -2.43% | 1.050x | -4.99% | 0 |
| https-static-small | 2903.50 | 2785.00 | 1.043x | +4.25% | - | - | - | 0.850x | +14.96% | 0.848x | +15.19% | 0.826x | +17.39% | 0 |
| qcp-transparent | 2988.00 | 2637.50 | 1.133x | +13.29% | - | - | - | 0.794x | +20.57% | 1.044x | -4.38% | 1.089x | -8.86% | 0 |
| reverse-proxy | 8198.00 | 6282.50 | 1.305x | +30.49% | - | - | - | 0.771x | +22.90% | 0.897x | +10.32% | 0.917x | +8.34% | 0 |
| static-large | 96.50 | 83.50 | 1.156x | +15.57% | - | - | - | 0.904x | +9.56% | 0.883x | +11.65% | 0.588x | +41.18% | 0 |
| static-small | 18636.00 | 18552.50 | 1.005x | +0.45% | - | - | - | 0.913x | +8.68% | 1.005x | -0.54% | 0.924x | +7.55% | 0 |
| tcp-stream | 3400.50 | 3097.50 | 1.098x | +9.78% | - | - | - | 0.888x | +11.23% | 0.962x | +3.75% | 0.985x | +1.55% | 0 |
| udp-stream | 3237.00 | 2484.00 | 1.303x | +30.31% | - | - | - | 0.657x | +34.27% | 0.912x | +8.81% | 1.044x | -4.40% | 0 |
| websocket-long-connection | 3591.50 | 2527.50 | 1.421x | +42.10% | - | - | - | 0.725x | +27.53% | 0.810x | +19.00% | 0.784x | +21.61% | 0 |

- Aggregate proxysss ops/s: `65539.50`
- Aggregate nginx ops/s: `60610.00`
- Aggregate proxysss/nginx ratio: `1.081x`
- Aggregate throughput improvement: `+8.13%`
