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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/old-dbfe-current-harness/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 13970.00 | 21700.50 | 0.644x | -35.62% | - | - | - | 0.949x | +5.14% | 2.383x | -138.26% | 2.926x | -192.63% | 0 |
| game-long-connection | 3583.25 | 3554.00 | 1.008x | +0.82% | - | - | - | 0.745x | +25.48% | 1.362x | -36.16% | 1.354x | -35.39% | 0 |
| generic-sse | 328.75 | 444.75 | 0.739x | -26.08% | - | - | - | 0.948x | +5.19% | 2.335x | -133.54% | 2.985x | -198.53% | 1 |
| https-static-small | 3509.00 | 5587.50 | 0.628x | -37.20% | - | - | - | 1.721x | -72.08% | 1.537x | -53.72% | 1.316x | -31.58% | 0 |
| qcp-transparent | 2391.25 | 3377.50 | 0.708x | -29.20% | - | - | - | 0.879x | +12.06% | 2.398x | -139.78% | 3.050x | -205.00% | 0 |
| reverse-proxy | 7353.25 | 10141.00 | 0.725x | -27.49% | - | - | - | 1.028x | -2.77% | 2.210x | -121.00% | 2.754x | -175.39% | 0 |
| static-large | 103.00 | 80.25 | 1.283x | +28.35% | - | - | - | 0.755x | +24.46% | 0.794x | +20.61% | 0.717x | +28.31% | 0 |
| static-small | 14458.25 | 20524.00 | 0.704x | -29.55% | - | - | - | 0.837x | +16.33% | 2.231x | -123.12% | 2.731x | -173.10% | 0 |
| tcp-stream | 3533.75 | 3575.00 | 0.988x | -1.15% | - | - | - | 0.753x | +24.73% | 1.371x | -37.09% | 1.468x | -46.82% | 0 |
| udp-stream | 2466.25 | 3452.75 | 0.714x | -28.57% | - | - | - | 0.892x | +10.80% | 2.367x | -136.72% | 3.213x | -221.32% | 0 |
| websocket-long-connection | 3367.25 | 3275.25 | 1.028x | +2.81% | - | - | - | 0.751x | +24.88% | 1.318x | -31.79% | 1.247x | -24.74% | 0 |

- Aggregate proxysss ops/s: `55064.00`
- Aggregate nginx ops/s: `75712.50`
- Aggregate proxysss/nginx ratio: `0.727x`
- Aggregate throughput improvement: `-27.27%`
