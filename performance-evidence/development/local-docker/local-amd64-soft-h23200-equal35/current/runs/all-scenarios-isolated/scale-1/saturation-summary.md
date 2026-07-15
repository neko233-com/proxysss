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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-soft-h23200-equal35/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22017.60 | 18283.15 | 1.204x | +20.43% | - | - | - | 0.765x | +23.51% | 0.845x | +15.52% | 0.699x | +30.10% | 0 |
| game-long-connection | 3829.15 | 3123.60 | 1.226x | +22.59% | - | - | - | 0.796x | +20.36% | 0.854x | +14.55% | 0.774x | +22.62% | 0 |
| generic-sse | 597.90 | 410.20 | 1.458x | +45.76% | - | - | - | 0.603x | +39.71% | 0.845x | +15.54% | 0.761x | +23.94% | 0 |
| https-static-small | 5357.50 | 4578.50 | 1.170x | +17.01% | - | - | - | 0.905x | +9.47% | 0.819x | +18.10% | 0.677x | +32.27% | 0 |
| qcp-transparent | 4751.10 | 3018.15 | 1.574x | +57.42% | - | - | - | 0.522x | +47.82% | 0.763x | +23.75% | 0.649x | +35.14% | 0 |
| reverse-proxy | 12542.15 | 9116.45 | 1.376x | +37.58% | - | - | - | 0.696x | +30.36% | 0.784x | +21.60% | 0.676x | +32.43% | 0 |
| static-large | 86.75 | 76.00 | 1.141x | +14.14% | - | - | - | 0.874x | +12.57% | 0.828x | +17.21% | 0.867x | +13.29% | 0 |
| static-small | 22314.75 | 18122.80 | 1.231x | +23.13% | - | - | - | 0.754x | +24.60% | 0.809x | +19.13% | 0.712x | +28.75% | 0 |
| tcp-stream | 3805.30 | 3158.75 | 1.205x | +20.47% | - | - | - | 0.816x | +18.36% | 0.889x | +11.12% | 0.775x | +22.46% | 0 |
| udp-stream | 4682.35 | 3054.00 | 1.533x | +53.32% | - | - | - | 0.534x | +46.58% | 0.788x | +21.18% | 0.678x | +32.19% | 0 |
| websocket-long-connection | 3566.35 | 2998.20 | 1.189x | +18.95% | - | - | - | 0.831x | +16.90% | 0.891x | +10.91% | 0.772x | +22.81% | 0 |

- Aggregate proxysss ops/s: `83550.90`
- Aggregate nginx ops/s: `65939.80`
- Aggregate proxysss/nginx ratio: `1.267x`
- Aggregate throughput improvement: `+26.71%`
