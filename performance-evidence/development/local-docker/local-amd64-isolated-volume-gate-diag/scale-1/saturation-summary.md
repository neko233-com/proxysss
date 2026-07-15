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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-volume-gate-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 9815.25 | 12426.50 | 0.790x | -21.01% | - | - | - | 0.820x | +17.95% | 1.747x | -74.73% | 1.212x | -21.20% | 0 |
| game-long-connection | 3864.00 | 2138.50 | 1.807x | +80.69% | - | - | - | 0.547x | +45.29% | 0.550x | +44.97% | 0.483x | +51.68% | 0 |
| generic-sse | 231.00 | 295.50 | 0.782x | -21.83% | - | - | - | 0.734x | +26.57% | 1.943x | -94.34% | 1.634x | -63.36% | 1 |
| https-static-small | 3195.25 | 3131.25 | 1.020x | +2.04% | - | - | - | 1.751x | -75.14% | 0.747x | +25.26% | 0.642x | +35.76% | 0 |
| qcp-transparent | 1620.00 | 2020.50 | 0.802x | -19.82% | - | - | - | 1.642x | -64.24% | 1.054x | -5.41% | 0.760x | +24.05% | 0 |
| reverse-proxy | 5202.75 | 6006.00 | 0.866x | -13.37% | - | - | - | 0.792x | +20.82% | 1.607x | -60.69% | 1.084x | -8.36% | 0 |
| static-large | 109.50 | 64.00 | 1.711x | +71.09% | - | - | - | 0.653x | +34.70% | 0.544x | +45.63% | 0.264x | +73.55% | 0 |
| static-small | 10103.00 | 12746.50 | 0.793x | -20.74% | - | - | - | 0.835x | +16.46% | 1.676x | -67.59% | 1.220x | -21.95% | 0 |
| tcp-stream | 3911.00 | 2113.75 | 1.850x | +85.03% | - | - | - | 0.553x | +44.71% | 0.539x | +46.14% | 0.491x | +50.89% | 0 |
| udp-stream | 1655.00 | 2016.25 | 0.821x | -17.92% | - | - | - | 1.535x | -53.46% | 1.023x | -2.26% | 0.758x | +24.18% | 0 |
| websocket-long-connection | 3727.50 | 2057.50 | 1.812x | +81.17% | - | - | - | 0.561x | +43.91% | 0.553x | +44.68% | 0.396x | +60.40% | 0 |

- Aggregate proxysss ops/s: `43434.25`
- Aggregate nginx ops/s: `45016.25`
- Aggregate proxysss/nginx ratio: `0.965x`
- Aggregate throughput improvement: `-3.51%`
