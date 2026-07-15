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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-udp-shared-realtime-per-core-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 9086.00 | 19409.00 | 0.468x | -53.19% | - | - | - | 1.090x | -8.99% | 3.309x | -230.94% | 2.718x | -171.77% | 0 |
| game-long-connection | 4924.25 | 3677.00 | 1.339x | +33.92% | - | - | - | 0.643x | +35.68% | 0.898x | +10.18% | 0.850x | +15.01% | 0 |
| generic-sse | 200.75 | 460.00 | 0.436x | -56.36% | - | - | - | 1.292x | -29.17% | 3.861x | -286.09% | 3.936x | -293.60% | 1 |
| https-static-small | 3513.00 | 5376.75 | 0.653x | -34.66% | - | - | - | 1.722x | -72.23% | 1.299x | -29.91% | 1.193x | -19.32% | 0 |
| qcp-transparent | 1626.75 | 3485.50 | 0.467x | -53.33% | - | - | - | 1.001x | -0.10% | 3.878x | -287.82% | 3.817x | -281.70% | 0 |
| reverse-proxy | 5168.75 | 9948.75 | 0.520x | -48.05% | - | - | - | 1.130x | -13.03% | 3.142x | -214.23% | 2.891x | -189.09% | 0 |
| static-large | 115.00 | 89.75 | 1.281x | +28.13% | - | - | - | 0.780x | +21.97% | 0.737x | +26.28% | 0.523x | +47.70% | 0 |
| static-small | 9529.75 | 20384.75 | 0.467x | -53.25% | - | - | - | 0.967x | +3.28% | 3.279x | -227.91% | 2.681x | -168.15% | 0 |
| tcp-stream | 4910.25 | 3766.75 | 1.304x | +30.36% | - | - | - | 0.659x | +34.06% | 0.917x | +8.34% | 0.897x | +10.30% | 0 |
| udp-stream | 1571.50 | 3614.25 | 0.435x | -56.52% | - | - | - | 1.084x | -8.39% | 4.118x | -311.85% | 3.961x | -296.11% | 0 |
| websocket-long-connection | 4689.50 | 3399.25 | 1.380x | +37.96% | - | - | - | 0.603x | +39.72% | 0.902x | +9.80% | 0.938x | +6.15% | 0 |

- Aggregate proxysss ops/s: `45335.50`
- Aggregate nginx ops/s: `73611.75`
- Aggregate proxysss/nginx ratio: `0.616x`
- Aggregate throughput improvement: `-38.41%`
