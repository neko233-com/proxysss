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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-final-sparse-h23400-equal33/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 20288.75 | 16817.25 | 1.206x | +20.64% | - | - | - | 0.778x | +22.17% | 0.771x | +22.86% | 0.737x | +26.30% | 0 |
| game-long-connection | 3793.30 | 2971.35 | 1.277x | +27.66% | - | - | - | 0.829x | +17.07% | 0.761x | +23.91% | 0.674x | +32.62% | 0 |
| generic-sse | 533.60 | 384.60 | 1.387x | +38.74% | - | - | - | 0.604x | +39.58% | 0.838x | +16.24% | 0.797x | +20.32% | 0 |
| https-static-small | 5167.90 | 4031.40 | 1.282x | +28.19% | - | - | - | 0.849x | +15.10% | 0.737x | +26.33% | 0.631x | +36.85% | 0 |
| qcp-transparent | 4063.40 | 2896.80 | 1.403x | +40.27% | - | - | - | 0.585x | +41.46% | 0.787x | +21.31% | 0.701x | +29.89% | 0 |
| reverse-proxy | 11096.45 | 8711.20 | 1.274x | +27.38% | - | - | - | 0.745x | +25.52% | 0.777x | +22.31% | 0.732x | +26.77% | 0 |
| static-large | 89.20 | 74.55 | 1.197x | +19.65% | - | - | - | 0.864x | +13.58% | 0.765x | +23.48% | 0.555x | +44.45% | 0 |
| static-small | 19159.90 | 16575.05 | 1.156x | +15.59% | - | - | - | 0.811x | +18.89% | 0.803x | +19.66% | 0.755x | +24.52% | 0 |
| tcp-stream | 3783.80 | 3013.90 | 1.255x | +25.54% | - | - | - | 0.838x | +16.16% | 0.766x | +23.45% | 0.744x | +25.56% | 0 |
| udp-stream | 4104.20 | 2861.55 | 1.434x | +43.43% | - | - | - | 0.569x | +43.10% | 0.787x | +21.28% | 0.717x | +28.27% | 0 |
| websocket-long-connection | 3509.15 | 3015.95 | 1.164x | +16.35% | - | - | - | 0.913x | +8.74% | 0.822x | +17.83% | 0.793x | +20.68% | 0 |

- Aggregate proxysss ops/s: `75589.65`
- Aggregate nginx ops/s: `61353.60`
- Aggregate proxysss/nginx ratio: `1.232x`
- Aggregate throughput improvement: `+23.20%`
