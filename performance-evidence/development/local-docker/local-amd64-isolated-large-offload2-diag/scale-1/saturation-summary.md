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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-large-offload2-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 3233.25 | 22065.25 | 0.147x | -85.35% | - | - | - | 1.429x | -42.90% | 13.434x | -1243.43% | 22.568x | -2156.84% | 0 |
| game-long-connection | 1562.25 | 3854.50 | 0.405x | -59.47% | - | - | - | 1.118x | -11.78% | 5.335x | -433.53% | 7.558x | -655.85% | 0 |
| generic-sse | 67.00 | 484.00 | 0.138x | -86.16% | - | - | - | 3.195x | -219.54% | 17.594x | -1659.37% | 24.092x | -2309.21% | 1 |
| https-static-small | 1229.00 | 5568.50 | 0.221x | -77.93% | - | - | - | 2.087x | -108.70% | 4.033x | -303.26% | 8.309x | -730.92% | 0 |
| qcp-transparent | 692.50 | 3850.25 | 0.180x | -82.01% | - | - | - | 3.338x | -233.81% | 9.840x | -883.96% | 14.116x | -1311.61% | 0 |
| reverse-proxy | 1601.75 | 11308.50 | 0.142x | -85.84% | - | - | - | 3.084x | -208.39% | 18.448x | -1744.79% | 29.002x | -2800.19% | 0 |
| static-large | 54.00 | 88.25 | 0.612x | -38.81% | - | - | - | 1.066x | -6.64% | 3.007x | -200.73% | 4.761x | -376.06% | 0 |
| static-small | 3202.75 | 21351.75 | 0.150x | -85.00% | - | - | - | 1.671x | -67.12% | 12.063x | -1106.28% | 21.869x | -2086.90% | 0 |
| tcp-stream | 1564.50 | 3899.00 | 0.401x | -59.87% | - | - | - | 1.183x | -18.34% | 4.632x | -363.25% | 8.392x | -739.19% | 0 |
| udp-stream | 698.00 | 3861.25 | 0.181x | -81.92% | - | - | - | 3.423x | -242.34% | 9.726x | -872.58% | 17.876x | -1687.64% | 0 |
| websocket-long-connection | 1509.50 | 3703.25 | 0.408x | -59.24% | - | - | - | 1.174x | -17.40% | 4.428x | -342.84% | 8.891x | -789.14% | 0 |

- Aggregate proxysss ops/s: `15414.50`
- Aggregate nginx ops/s: `80034.50`
- Aggregate proxysss/nginx ratio: `0.193x`
- Aggregate throughput improvement: `-80.74%`
