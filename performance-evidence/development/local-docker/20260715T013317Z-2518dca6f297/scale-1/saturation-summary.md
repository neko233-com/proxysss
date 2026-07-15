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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15195.50 | 15460.00 | 0.983x | -1.71% | - | - | - | 0.862x | +13.83% | 1.093x | -9.28% | 0.874x | +12.62% | 0 |
| game-long-connection | 3113.50 | 3862.00 | 0.806x | -19.38% | - | - | - | 2.299x | -129.94% | 0.962x | +3.84% | 0.751x | +24.93% | 0 |
| generic-sse | 364.50 | 429.50 | 0.849x | -15.13% | - | - | - | 1.381x | -38.06% | 1.208x | -20.76% | 0.988x | +1.15% | 0 |
| https-static-small | 3175.00 | 3572.50 | 0.889x | -11.13% | - | - | - | 1.760x | -76.01% | 1.119x | -11.89% | 0.664x | +33.58% | 0 |
| qcp-transparent | 2687.00 | 2029.50 | 1.324x | +32.40% | - | - | - | 0.652x | +34.84% | 0.951x | +4.92% | 0.816x | +18.42% | 0 |
| reverse-proxy | 9017.00 | 5478.50 | 1.646x | +64.59% | - | - | - | 0.439x | +56.11% | 0.829x | +17.07% | 0.569x | +43.11% | 0 |
| static-large | 102.50 | 78.50 | 1.306x | +30.57% | - | - | - | 0.800x | +20.03% | 0.634x | +36.58% | 0.769x | +23.06% | 0 |
| static-small | 16100.50 | 15076.50 | 1.068x | +6.79% | - | - | - | 0.845x | +15.51% | 1.077x | -7.66% | 0.867x | +13.29% | 0 |
| tcp-stream | 3132.50 | 3537.00 | 0.886x | -11.44% | - | - | - | 1.933x | -93.35% | 0.973x | +2.69% | 0.768x | +23.24% | 0 |
| udp-stream | 2702.50 | 3119.50 | 0.866x | -13.37% | - | - | - | 1.506x | -50.58% | 1.021x | -2.06% | 0.850x | +14.98% | 0 |
| websocket-long-connection | 3262.50 | 3562.50 | 0.916x | -8.42% | - | - | - | 2.060x | -105.96% | 0.888x | +11.18% | 0.823x | +17.73% | 0 |

- Aggregate proxysss ops/s: `58853.00`
- Aggregate nginx ops/s: `56206.00`
- Aggregate proxysss/nginx ratio: `1.047x`
- Aggregate throughput improvement: `+4.71%`
