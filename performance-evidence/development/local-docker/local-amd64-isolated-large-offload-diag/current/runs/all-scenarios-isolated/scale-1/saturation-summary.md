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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-large-offload-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 10320.25 | 17628.00 | 0.585x | -41.46% | - | - | - | 0.736x | +26.40% | 2.690x | -168.96% | 1.887x | -88.68% | 0 |
| game-long-connection | 4612.25 | 3146.00 | 1.466x | +46.61% | - | - | - | 0.635x | +36.53% | 0.761x | +23.87% | 0.610x | +39.00% | 0 |
| generic-sse | 242.25 | 403.00 | 0.601x | -39.89% | - | - | - | 0.844x | +15.56% | 2.777x | -177.65% | 2.690x | -169.00% | 1 |
| https-static-small | 3533.25 | 4322.25 | 0.817x | -18.25% | - | - | - | 1.494x | -49.37% | 0.975x | +2.47% | 0.783x | +21.65% | 0 |
| qcp-transparent | 1798.00 | 3239.25 | 0.555x | -44.49% | - | - | - | 2.152x | -115.24% | 1.692x | -69.17% | 1.208x | -20.79% | 0 |
| reverse-proxy | 5617.25 | 8493.00 | 0.661x | -33.86% | - | - | - | 0.913x | +8.70% | 2.400x | -140.00% | 2.049x | -104.94% | 0 |
| static-large | 113.75 | 71.00 | 1.602x | +60.21% | - | - | - | 0.615x | +38.47% | 0.616x | +38.36% | 0.273x | +72.75% | 0 |
| static-small | 9921.00 | 16893.25 | 0.587x | -41.27% | - | - | - | 0.755x | +24.45% | 2.628x | -162.79% | 1.886x | -88.56% | 0 |
| tcp-stream | 4584.25 | 3019.25 | 1.518x | +51.83% | - | - | - | 0.608x | +39.16% | 0.711x | +28.93% | 0.605x | +39.49% | 0 |
| udp-stream | 1809.50 | 3112.50 | 0.581x | -41.86% | - | - | - | 2.048x | -104.78% | 1.610x | -61.02% | 1.201x | -20.14% | 0 |
| websocket-long-connection | 4472.00 | 3032.50 | 1.475x | +47.47% | - | - | - | 0.624x | +37.59% | 0.729x | +27.12% | 0.696x | +30.41% | 0 |

- Aggregate proxysss ops/s: `47023.75`
- Aggregate nginx ops/s: `63360.00`
- Aggregate proxysss/nginx ratio: `0.742x`
- Aggregate throughput improvement: `-25.78%`
