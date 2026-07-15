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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-sync-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 10942.75 | 20197.50 | 0.542x | -45.82% | - | - | - | 0.870x | +12.99% | 2.904x | -190.39% | 2.475x | -147.48% | 0 |
| game-long-connection | 4492.25 | 4055.50 | 1.108x | +10.77% | - | - | - | 0.736x | +26.42% | 1.154x | -15.39% | 1.189x | -18.89% | 0 |
| generic-sse | 249.50 | 473.75 | 0.527x | -47.34% | - | - | - | 1.182x | -18.15% | 3.538x | -253.77% | 3.186x | -218.63% | 1 |
| https-static-small | 3640.25 | 5258.75 | 0.692x | -30.78% | - | - | - | 1.597x | -59.75% | 1.382x | -38.25% | 1.147x | -14.74% | 0 |
| qcp-transparent | 1895.00 | 3817.75 | 0.496x | -50.36% | - | - | - | 2.184x | -118.38% | 2.041x | -104.11% | 1.731x | -73.09% | 0 |
| reverse-proxy | 5947.50 | 11309.25 | 0.526x | -47.41% | - | - | - | 1.202x | -20.23% | 3.279x | -227.86% | 3.167x | -216.74% | 0 |
| static-large | 121.25 | 91.50 | 1.325x | +32.51% | - | - | - | 0.754x | +24.56% | 0.761x | +23.95% | 0.725x | +27.52% | 0 |
| static-small | 11478.50 | 21631.75 | 0.531x | -46.94% | - | - | - | 0.895x | +10.47% | 3.104x | -210.36% | 2.597x | -159.72% | 0 |
| tcp-stream | 4540.25 | 3852.50 | 1.179x | +17.85% | - | - | - | 0.693x | +30.66% | 1.107x | -10.73% | 1.116x | -11.60% | 0 |
| udp-stream | 1910.25 | 3888.25 | 0.491x | -50.87% | - | - | - | 2.197x | -119.73% | 2.038x | -103.83% | 1.775x | -77.50% | 0 |
| websocket-long-connection | 4301.25 | 4004.25 | 1.074x | +7.42% | - | - | - | 0.754x | +24.55% | 1.215x | -21.49% | 1.217x | -21.67% | 0 |

- Aggregate proxysss ops/s: `49518.75`
- Aggregate nginx ops/s: `78580.75`
- Aggregate proxysss/nginx ratio: `0.630x`
- Aggregate throughput improvement: `-36.98%`
