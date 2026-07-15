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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29841.00 | 27517.00 | 1.084x | +8.45% | - | - | - | 0.804x | +19.63% | 0.984x | +1.65% | 0.849x | +15.12% | 0 |
| game-long-connection | 5234.50 | 5082.50 | 1.030x | +2.99% | - | - | - | 1.034x | -3.36% | 0.977x | +2.32% | 0.882x | +11.76% | 0 |
| generic-sse | 617.00 | 446.00 | 1.383x | +38.34% | - | - | - | 0.625x | +37.49% | 0.957x | +4.26% | 0.788x | +21.17% | 0 |
| https-static-small | 5946.50 | 4788.50 | 1.242x | +24.18% | - | - | - | 0.651x | +34.87% | 0.934x | +6.65% | 0.731x | +26.88% | 0 |
| qcp-transparent | 4859.00 | 3968.00 | 1.225x | +22.45% | - | - | - | 0.668x | +33.21% | 0.960x | +3.96% | 0.860x | +14.03% | 0 |
| reverse-proxy | 12863.00 | 10792.00 | 1.192x | +19.19% | - | - | - | 0.901x | +9.90% | 0.832x | +16.81% | 0.670x | +33.00% | 0 |
| static-large | 88.50 | 88.50 | 1.000x | +0.00% | - | - | - | 1.003x | -0.30% | 1.072x | -7.18% | 1.164x | -16.40% | 0 |
| static-small | 27739.00 | 26975.50 | 1.028x | +2.83% | - | - | - | 0.837x | +16.32% | 1.133x | -13.28% | 1.013x | -1.31% | 0 |
| tcp-stream | 4900.50 | 3883.00 | 1.262x | +26.20% | - | - | - | 0.723x | +27.73% | 0.975x | +2.48% | 0.699x | +30.09% | 0 |
| udp-stream | 4678.50 | 3849.50 | 1.215x | +21.54% | - | - | - | 0.704x | +29.58% | 0.939x | +6.14% | 0.748x | +25.22% | 0 |
| websocket-long-connection | 5393.50 | 4488.00 | 1.202x | +20.18% | - | - | - | 0.711x | +28.86% | 0.961x | +3.87% | 0.815x | +18.46% | 0 |

- Aggregate proxysss ops/s: `102161.00`
- Aggregate nginx ops/s: `91878.50`
- Aggregate proxysss/nginx ratio: `1.112x`
- Aggregate throughput improvement: `+11.19%`
