# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-qos-rebalance-1x2x4x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 17113.00 | 15914.55 | 1.075x | +7.53% | - | - | - | 0.877x | +12.30% | 0.929x | +7.09% | 0.895x | +10.46% | 0 |
| game-long-connection | 3732.35 | 2922.70 | 1.277x | +27.70% | - | - | - | 0.817x | +18.27% | 0.773x | +22.73% | 0.779x | +22.14% | 0 |
| generic-sse | 434.65 | 380.45 | 1.142x | +14.25% | - | - | - | 0.785x | +21.54% | 1.061x | -6.09% | 1.025x | -2.51% | 0 |
| https-static-small | 4572.95 | 3945.20 | 1.159x | +15.91% | - | - | - | 0.860x | +14.04% | 0.849x | +15.13% | 0.775x | +22.55% | 0 |
| qcp-transparent | 3457.80 | 2843.50 | 1.216x | +21.60% | - | - | - | 0.709x | +29.14% | 0.973x | +2.66% | 0.903x | +9.74% | 0 |
| reverse-proxy | 9469.15 | 8249.10 | 1.148x | +14.79% | - | - | - | 0.857x | +14.29% | 0.931x | +6.88% | 0.903x | +9.74% | 0 |
| static-large | 83.50 | 70.05 | 1.192x | +19.20% | - | - | - | 0.837x | +16.30% | 0.804x | +19.57% | 0.873x | +12.66% | 0 |
| static-small | 17277.05 | 15539.75 | 1.112x | +11.18% | - | - | - | 0.815x | +18.52% | 0.886x | +11.40% | 0.867x | +13.29% | 0 |
| tcp-stream | 3759.65 | 2983.20 | 1.260x | +26.03% | - | - | - | 0.831x | +16.94% | 0.784x | +21.63% | 0.783x | +21.74% | 0 |
| udp-stream | 3455.30 | 2837.55 | 1.218x | +21.77% | - | - | - | 0.717x | +28.29% | 0.941x | +5.88% | 0.887x | +11.35% | 0 |
| websocket-long-connection | 3479.15 | 2876.95 | 1.209x | +20.93% | - | - | - | 0.858x | +14.23% | 0.815x | +18.47% | 0.751x | +24.90% | 0 |

- Aggregate proxysss ops/s: `66834.55`
- Aggregate nginx ops/s: `58563.00`
- Aggregate proxysss/nginx ratio: `1.141x`
- Aggregate throughput improvement: `+14.12%`
