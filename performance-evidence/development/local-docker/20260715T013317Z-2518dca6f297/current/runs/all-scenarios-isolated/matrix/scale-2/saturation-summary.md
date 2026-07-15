# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 18754.50 | 18995.00 | 0.987x | -1.27% | - | - | - | 0.918x | +8.16% | 1.318x | -31.76% | 1.297x | -29.73% | 0 |
| game-long-connection | 4230.00 | 4167.00 | 1.015x | +1.51% | - | - | - | 1.069x | -6.86% | 0.909x | +9.14% | 0.880x | +11.99% | 0 |
| generic-sse | 470.50 | 474.50 | 0.992x | -0.84% | - | - | - | 0.923x | +7.68% | 1.426x | -42.55% | 1.723x | -72.32% | 0 |
| https-static-small | 4234.00 | 3645.50 | 1.161x | +16.14% | - | - | - | 0.751x | +24.88% | 1.063x | -6.32% | 0.786x | +21.38% | 0 |
| qcp-transparent | 3292.50 | 3419.00 | 0.963x | -3.70% | - | - | - | 0.832x | +16.76% | 1.450x | -45.00% | 1.451x | -45.10% | 0 |
| reverse-proxy | 8826.00 | 9701.50 | 0.910x | -9.02% | - | - | - | 1.006x | -0.60% | 1.379x | -37.94% | 1.569x | -56.87% | 0 |
| static-large | 102.00 | 103.00 | 0.990x | -0.97% | - | - | - | 0.962x | +3.83% | 0.915x | +8.51% | 1.247x | -24.68% | 0 |
| static-small | 20592.50 | 19570.00 | 1.052x | +5.22% | - | - | - | 0.643x | +35.69% | 1.257x | -25.73% | 1.342x | -34.17% | 0 |
| tcp-stream | 4380.00 | 3400.00 | 1.288x | +28.82% | - | - | - | 0.777x | +22.27% | 0.843x | +15.69% | 0.901x | +9.93% | 0 |
| udp-stream | 3586.00 | 3632.00 | 0.987x | -1.27% | - | - | - | 0.824x | +17.59% | 1.453x | -45.28% | 1.436x | -43.61% | 0 |
| websocket-long-connection | 5493.00 | 4648.00 | 1.182x | +18.18% | - | - | - | 0.838x | +16.21% | 0.882x | +11.76% | 0.782x | +21.79% | 0 |

- Aggregate proxysss ops/s: `73961.00`
- Aggregate nginx ops/s: `71755.50`
- Aggregate proxysss/nginx ratio: `1.031x`
- Aggregate throughput improvement: `+3.07%`
