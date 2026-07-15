# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 27367.50 | 24018.50 | 1.139x | +13.94% | - | - | - | 0.701x | +29.86% | 1.254x | -25.38% | 1.208x | -20.76% | 0 |
| game-long-connection | 6816.00 | 4202.00 | 1.622x | +62.21% | - | - | - | 0.421x | +57.86% | 0.921x | +7.94% | 0.945x | +5.48% | 0 |
| generic-sse | 844.00 | 517.50 | 1.631x | +63.09% | - | - | - | 0.470x | +52.98% | 1.021x | -2.05% | 1.154x | -15.40% | 0 |
| https-static-small | 6099.50 | 4938.50 | 1.235x | +23.51% | - | - | - | 0.635x | +36.49% | 1.093x | -9.29% | 0.826x | +17.38% | 0 |
| qcp-transparent | 7095.50 | 4110.50 | 1.726x | +72.62% | - | - | - | 0.349x | +65.11% | 0.913x | +8.72% | 1.134x | -13.43% | 0 |
| reverse-proxy | 12554.00 | 11745.50 | 1.069x | +6.88% | - | - | - | 0.838x | +16.19% | 1.228x | -22.77% | 1.342x | -34.21% | 0 |
| static-large | 89.50 | 91.50 | 0.978x | -2.19% | - | - | - | 1.154x | -15.35% | 1.512x | -51.18% | 0.587x | +41.33% | 0 |
| static-small | 26635.00 | 24751.50 | 1.076x | +7.61% | - | - | - | 0.738x | +26.24% | 1.271x | -27.08% | 1.105x | -10.50% | 0 |
| tcp-stream | 6933.00 | 4144.00 | 1.673x | +67.30% | - | - | - | 0.432x | +56.75% | 0.873x | +12.74% | 0.974x | +2.60% | 0 |
| udp-stream | 6594.00 | 3930.00 | 1.678x | +67.79% | - | - | - | 0.403x | +59.74% | 0.921x | +7.91% | 1.180x | -18.03% | 0 |
| websocket-long-connection | 6146.50 | 4833.50 | 1.272x | +27.16% | - | - | - | 0.546x | +45.37% | 1.033x | -3.34% | 1.078x | -7.81% | 0 |

- Aggregate proxysss ops/s: `107174.50`
- Aggregate nginx ops/s: `87283.00`
- Aggregate proxysss/nginx ratio: `1.228x`
- Aggregate throughput improvement: `+22.79%`
