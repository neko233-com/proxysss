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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 23840.50 | 23296.50 | 1.023x | +2.34% | - | - | - | 0.891x | +10.94% | 1.005x | -0.50% | 0.867x | +13.26% | 0 |
| game-long-connection | 4019.50 | 4010.00 | 1.002x | +0.24% | - | - | - | 0.998x | +0.24% | 0.915x | +8.46% | 0.970x | +2.99% | 0 |
| generic-sse | 523.00 | 390.00 | 1.341x | +34.10% | - | - | - | 0.695x | +30.50% | 0.849x | +15.12% | 0.891x | +10.88% | 0 |
| https-static-small | 3819.50 | 3481.50 | 1.097x | +9.71% | - | - | - | 0.926x | +7.38% | 0.782x | +21.79% | 0.959x | +4.11% | 0 |
| qcp-transparent | 4253.50 | 3431.50 | 1.240x | +23.95% | - | - | - | 0.711x | +28.92% | 0.898x | +10.21% | 0.950x | +4.98% | 0 |
| reverse-proxy | 9724.50 | 7879.50 | 1.234x | +23.42% | - | - | - | 0.794x | +20.64% | 0.965x | +3.46% | 0.931x | +6.88% | 0 |
| static-large | 107.50 | 103.50 | 1.039x | +3.86% | - | - | - | 1.000x | -0.02% | 0.940x | +5.99% | 0.906x | +9.37% | 0 |
| static-small | 23324.50 | 21579.50 | 1.081x | +8.09% | - | - | - | 0.854x | +14.62% | 0.958x | +4.15% | 0.853x | +14.72% | 0 |
| tcp-stream | 4313.50 | 3791.00 | 1.138x | +13.78% | - | - | - | 0.871x | +12.92% | 0.835x | +16.48% | 0.917x | +8.35% | 0 |
| udp-stream | 4069.50 | 3687.50 | 1.104x | +10.36% | - | - | - | 0.746x | +25.43% | 0.971x | +2.92% | 0.935x | +6.53% | 0 |
| websocket-long-connection | 4237.00 | 3109.00 | 1.363x | +36.28% | - | - | - | 0.662x | +33.77% | 0.772x | +22.83% | 0.854x | +14.58% | 0 |

- Aggregate proxysss ops/s: `82232.50`
- Aggregate nginx ops/s: `74759.50`
- Aggregate proxysss/nginx ratio: `1.100x`
- Aggregate throughput improvement: `+10.00%`
