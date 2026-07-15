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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 3627.00 | 3624.00 | 1.001x | +0.08% | 3629.35 | 0.999x | 0.999x | 0.554x | +44.58% | 0.261x | +73.90% | 0.251x | +74.92% | 0 |
| game-long-connection | 704.00 | 704.00 | 1.000x | +0.00% | 707.96 | 0.994x | 0.994x | 0.812x | +18.76% | 0.284x | +71.62% | 0.237x | +76.32% | 0 |
| generic-sse | 75.00 | 75.00 | 1.000x | +0.00% | 75.50 | 0.993x | 0.993x | 0.842x | +15.77% | 0.277x | +72.29% | 0.549x | +45.12% | 0 |
| https-static-small | 451.00 | 451.00 | 1.000x | +0.00% | 451.98 | 0.998x | 0.998x | 0.948x | +5.16% | 0.924x | +7.63% | 0.489x | +51.08% | 0 |
| qcp-transparent | 560.00 | 560.00 | 1.000x | +0.00% | 567.74 | 0.986x | 0.986x | 0.650x | +34.98% | 0.147x | +85.26% | 0.136x | +86.44% | 0 |
| reverse-proxy | 1382.00 | 1380.00 | 1.001x | +0.14% | 1382.71 | 0.999x | 0.998x | 0.805x | +19.54% | 0.122x | +87.76% | 0.317x | +68.27% | 0 |
| static-large | 18.00 | 18.00 | 1.000x | +0.00% | 18.25 | 0.986x | 0.986x | 0.979x | +2.14% | 0.208x | +79.16% | 0.244x | +75.59% | 0 |
| static-small | 3062.00 | 3067.00 | 0.998x | -0.16% | 3068.66 | 0.998x | 0.999x | 0.559x | +44.06% | 0.114x | +88.64% | 0.422x | +57.76% | 0 |
| tcp-stream | 664.00 | 664.00 | 1.000x | +0.00% | 668.45 | 0.993x | 0.993x | 0.714x | +28.60% | 0.097x | +90.34% | 0.162x | +83.80% | 0 |
| udp-stream | 568.00 | 568.00 | 1.000x | +0.00% | 576.24 | 0.986x | 0.986x | 0.688x | +31.23% | 0.070x | +93.03% | 0.203x | +79.74% | 0 |
| websocket-long-connection | 528.00 | 528.00 | 1.000x | +0.00% | 535.73 | 0.986x | 0.986x | 0.701x | +29.95% | 0.381x | +61.89% | 0.270x | +73.03% | 0 |

- Aggregate proxysss ops/s: `11639.00`
- Aggregate nginx ops/s: `11639.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
