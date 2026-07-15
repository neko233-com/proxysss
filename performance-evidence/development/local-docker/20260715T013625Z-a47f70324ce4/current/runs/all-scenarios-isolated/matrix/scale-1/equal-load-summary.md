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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 2934.00 | 2931.00 | 1.001x | +0.10% | 2935.24 | 1.000x | 0.999x | 0.940x | +5.98% | 1.293x | -29.27% | 1.690x | -69.01% | 0 |
| game-long-connection | 752.00 | 752.00 | 1.000x | +0.00% | 754.93 | 0.996x | 0.996x | 1.108x | -10.80% | 1.296x | -29.57% | 1.214x | -21.39% | 0 |
| generic-sse | 80.00 | 80.00 | 1.000x | +0.00% | 80.75 | 0.991x | 0.991x | 0.881x | +11.88% | 1.222x | -22.22% | 1.916x | -91.56% | 0 |
| https-static-small | 438.00 | 438.00 | 1.000x | +0.00% | 438.98 | 0.998x | 0.998x | 0.911x | +8.86% | 1.244x | -24.45% | 1.250x | -24.99% | 0 |
| qcp-transparent | 544.00 | 544.00 | 1.000x | +0.00% | 548.25 | 0.992x | 0.992x | 0.834x | +16.60% | 2.072x | -107.20% | 1.314x | -31.40% | 0 |
| reverse-proxy | 1323.00 | 1322.00 | 1.001x | +0.08% | 1324.45 | 0.999x | 0.998x | 1.197x | -19.66% | 1.748x | -74.85% | 1.994x | -99.43% | 0 |
| static-large | 18.00 | 18.00 | 1.000x | +0.00% | 18.25 | 0.986x | 0.986x | 0.881x | +11.92% | 1.422x | -42.19% | 1.997x | -99.74% | 0 |
| static-small | 2887.00 | 2885.00 | 1.001x | +0.07% | 2889.65 | 0.999x | 0.998x | 0.748x | +25.21% | 1.235x | -23.49% | 1.839x | -83.88% | 0 |
| tcp-stream | 544.00 | 544.00 | 1.000x | +0.00% | 546.75 | 0.995x | 0.995x | 1.049x | -4.89% | 1.044x | -4.37% | 0.945x | +5.48% | 0 |
| udp-stream | 576.00 | 576.00 | 1.000x | +0.00% | 582.71 | 0.988x | 0.988x | 0.843x | +15.73% | 1.910x | -90.99% | 1.176x | -17.59% | 0 |
| websocket-long-connection | 728.00 | 728.00 | 1.000x | +0.00% | 729.73 | 0.998x | 0.998x | 0.857x | +14.25% | 1.397x | -39.67% | 0.855x | +14.45% | 0 |

- Aggregate proxysss ops/s: `10824.00`
- Aggregate nginx ops/s: `10818.00`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.06%`
