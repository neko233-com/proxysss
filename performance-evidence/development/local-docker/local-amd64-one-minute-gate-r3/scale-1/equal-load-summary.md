# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for `game-long-connection, qcp-transparent, tcp-stream, udp-stream, websocket-long-connection`
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=true, strict=true)
- Saturation ops gate: `false`
- Equal-load latency gate: `true`
- Minimum fixed-load completion: `0.980`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-one-minute-gate-r3/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4903.00 | 4900.00 | 1.001x | +0.06% | 4904.97 | 1.000x | 0.999x | 0.714x | +28.57% | 0.650x | +35.04% | 0.782x | +21.76% | 0 |
| game-long-connection | 1036.00 | 1036.00 | 1.000x | +0.00% | 1040.04 | 0.996x | 0.996x | 1.004x | -0.36% | 0.587x | +41.29% | 0.770x | +23.05% | 0 |
| generic-sse | 120.00 | 120.00 | 1.000x | +0.00% | 120.37 | 0.997x | 0.997x | 0.969x | +3.13% | 0.580x | +42.01% | 0.226x | +77.41% | 0 |
| https-static-small | 1313.00 | 1312.50 | 1.000x | +0.04% | 1313.84 | 0.999x | 0.999x | 1.012x | -1.16% | 1.130x | -13.01% | 1.462x | -46.21% | 0 |
| qcp-transparent | 948.00 | 948.00 | 1.000x | +0.00% | 952.27 | 0.996x | 0.996x | 0.846x | +15.36% | 1.479x | -47.91% | 0.723x | +27.74% | 0 |
| reverse-proxy | 2461.50 | 2460.50 | 1.000x | +0.04% | 2462.68 | 1.000x | 0.999x | 0.915x | +8.51% | 0.840x | +16.02% | 0.227x | +77.28% | 0 |
| static-large | 21.50 | 21.50 | 1.000x | +0.00% | 21.75 | 0.989x | 0.989x | 0.997x | +0.26% | 1.057x | -5.65% | 0.807x | +19.26% | 0 |
| static-small | 4887.50 | 4889.00 | 1.000x | -0.03% | 4889.98 | 0.999x | 1.000x | 0.740x | +25.97% | 0.614x | +38.58% | 0.630x | +36.98% | 0 |
| tcp-stream | 1028.00 | 1028.00 | 1.000x | +0.00% | 1031.46 | 0.997x | 0.997x | 0.985x | +1.53% | 0.723x | +27.66% | 0.877x | +12.29% | 0 |
| udp-stream | 964.00 | 964.00 | 1.000x | +0.00% | 966.42 | 0.997x | 0.997x | 0.850x | +15.04% | 0.869x | +13.06% | 1.377x | -37.71% | 0 |
| websocket-long-connection | 984.00 | 984.00 | 1.000x | +0.00% | 985.34 | 0.999x | 0.999x | 0.968x | +3.22% | 0.958x | +4.20% | 0.900x | +10.01% | 0 |

- Aggregate proxysss ops/s: `18666.50`
- Aggregate nginx ops/s: `18663.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
