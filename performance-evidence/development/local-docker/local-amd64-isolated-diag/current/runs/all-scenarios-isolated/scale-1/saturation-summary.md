# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `unspecified`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 17329.25 | 20519.25 | 0.845x | -15.55% | - | - | - | 0.873x | +12.74% | 1.558x | -55.77% | 1.732x | -73.24% | 0 |
| game-long-connection | 9264.75 | 11416.25 | 0.812x | -18.85% | - | - | - | 0.899x | +10.14% | 1.507x | -50.72% | 1.287x | -28.72% | 0 |
| generic-sse | 521.75 | 859.25 | 0.607x | -39.28% | - | - | - | 1.106x | -10.57% | 2.072x | -107.22% | 2.245x | -124.49% | 0 |
| https-static-small | 4064.50 | 6050.25 | 0.672x | -32.82% | - | - | - | 1.838x | -83.83% | 1.320x | -32.01% | 1.143x | -14.29% | 0 |
| qcp-transparent | 16154.75 | 20549.75 | 0.786x | -21.39% | - | - | - | 0.961x | +3.93% | 1.420x | -42.02% | 3.112x | -211.15% | 0 |
| reverse-proxy | 9508.25 | 15399.75 | 0.617x | -38.26% | - | - | - | 1.350x | -35.00% | 1.893x | -89.30% | 1.992x | -99.23% | 0 |
| static-large | 110.25 | 97.75 | 1.128x | +12.79% | - | - | - | 0.928x | +7.16% | 0.874x | +12.58% | 0.876x | +12.35% | 0 |
| static-small | 20773.50 | 23062.50 | 0.901x | -9.93% | - | - | - | 0.959x | +4.09% | 1.246x | -24.60% | 1.524x | -52.40% | 0 |
| tcp-stream | 12086.50 | 14718.25 | 0.821x | -17.88% | - | - | - | 0.861x | +13.91% | 1.585x | -58.54% | 1.673x | -67.26% | 0 |
| udp-stream | 12321.50 | 17896.75 | 0.688x | -31.15% | - | - | - | 1.021x | -2.10% | 2.563x | -156.30% | 2.823x | -182.27% | 0 |
| websocket-long-connection | 6581.50 | 8875.50 | 0.742x | -25.85% | - | - | - | 1.278x | -27.80% | 1.517x | -51.73% | 1.298x | -29.81% | 0 |

- Aggregate proxysss ops/s: `108716.50`
- Aggregate nginx ops/s: `139445.25`
- Aggregate proxysss/nginx ratio: `0.780x`
- Aggregate throughput improvement: `-22.04%`
