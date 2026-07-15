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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-soft-realtime-h23200/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22061.60 | 21300.55 | 1.036x | +3.57% | - | - | - | 0.780x | +22.02% | 1.098x | -9.75% | 1.095x | -9.54% | 0 |
| game-long-connection | 4124.90 | 3669.30 | 1.124x | +12.42% | - | - | - | 0.825x | +17.46% | 1.067x | -6.67% | 1.116x | -11.57% | 0 |
| generic-sse | 585.20 | 467.55 | 1.252x | +25.16% | - | - | - | 0.664x | +33.59% | 1.108x | -10.79% | 1.150x | -15.05% | 0 |
| https-static-small | 5506.70 | 5647.10 | 0.975x | -2.49% | - | - | - | 0.954x | +4.61% | 1.158x | -15.77% | 1.035x | -3.47% | 0 |
| qcp-transparent | 4710.20 | 3670.50 | 1.283x | +28.33% | - | - | - | 0.593x | +40.73% | 1.070x | -7.02% | 1.084x | -8.42% | 0 |
| reverse-proxy | 12478.25 | 11045.65 | 1.130x | +12.97% | - | - | - | 0.820x | +18.01% | 1.077x | -7.68% | 1.083x | -8.31% | 0 |
| static-large | 90.70 | 83.90 | 1.081x | +8.10% | - | - | - | 0.925x | +7.50% | 0.922x | +7.75% | 1.023x | -2.30% | 0 |
| static-small | 21391.95 | 20918.55 | 1.023x | +2.26% | - | - | - | 0.810x | +18.96% | 1.094x | -9.43% | 1.067x | -6.71% | 0 |
| tcp-stream | 4092.30 | 3731.10 | 1.097x | +9.68% | - | - | - | 0.843x | +15.69% | 1.097x | -9.67% | 1.125x | -12.52% | 0 |
| udp-stream | 4776.70 | 3699.95 | 1.291x | +29.10% | - | - | - | 0.585x | +41.49% | 1.071x | -7.07% | 1.111x | -11.13% | 0 |
| websocket-long-connection | 3776.80 | 3598.40 | 1.050x | +4.96% | - | - | - | 0.878x | +12.21% | 1.145x | -14.54% | 1.176x | -17.57% | 0 |

- Aggregate proxysss ops/s: `83595.30`
- Aggregate nginx ops/s: `77832.55`
- Aggregate proxysss/nginx ratio: `1.074x`
- Aggregate throughput improvement: `+7.40%`
