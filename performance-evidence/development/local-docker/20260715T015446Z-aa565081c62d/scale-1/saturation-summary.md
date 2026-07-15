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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 26584.00 | 19329.50 | 1.375x | +37.53% | - | - | - | 0.703x | +29.74% | 0.852x | +14.80% | 0.773x | +22.74% | 0 |
| game-long-connection | 4042.50 | 2593.00 | 1.559x | +55.90% | - | - | - | 0.646x | +35.38% | 0.805x | +19.46% | 0.687x | +31.32% | 0 |
| generic-sse | 530.50 | 331.50 | 1.600x | +60.03% | - | - | - | 0.570x | +43.03% | 0.776x | +22.37% | 0.578x | +42.20% | 0 |
| https-static-small | 3993.50 | 3062.50 | 1.304x | +30.40% | - | - | - | 0.647x | +35.28% | 0.788x | +21.21% | 0.855x | +14.53% | 0 |
| qcp-transparent | 3553.00 | 2714.00 | 1.309x | +30.91% | - | - | - | 0.737x | +26.33% | 0.864x | +13.64% | 0.712x | +28.82% | 0 |
| reverse-proxy | 10868.00 | 6694.50 | 1.623x | +62.34% | - | - | - | 0.590x | +40.97% | 0.734x | +26.58% | 0.640x | +36.00% | 0 |
| static-large | 95.50 | 86.00 | 1.110x | +11.05% | - | - | - | 0.925x | +7.47% | 0.813x | +18.71% | 0.821x | +17.89% | 0 |
| static-small | 25638.00 | 16169.50 | 1.586x | +58.56% | - | - | - | 0.647x | +35.29% | 0.684x | +31.64% | 0.704x | +29.62% | 0 |
| tcp-stream | 3936.00 | 2902.50 | 1.356x | +35.61% | - | - | - | 0.742x | +25.79% | 0.812x | +18.78% | 0.783x | +21.67% | 0 |
| udp-stream | 3490.50 | 2205.50 | 1.583x | +58.26% | - | - | - | 0.556x | +44.39% | 0.721x | +27.88% | 0.637x | +36.27% | 0 |
| websocket-long-connection | 4351.50 | 2370.50 | 1.836x | +83.57% | - | - | - | 0.518x | +48.21% | 0.693x | +30.74% | 0.648x | +35.18% | 0 |

- Aggregate proxysss ops/s: `87083.00`
- Aggregate nginx ops/s: `58459.00`
- Aggregate proxysss/nginx ratio: `1.490x`
- Aggregate throughput improvement: `+48.96%`
