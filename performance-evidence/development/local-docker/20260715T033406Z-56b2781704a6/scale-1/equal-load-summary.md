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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6873.00 | 6873.00 | 1.000x | +0.00% | 6864.00 | 1.001x | 1.001x | 0.750x | +25.00% | 0.576x | +42.36% | 0.537x | +46.34% | 0 |
| game-long-connection | 428.00 | 428.00 | 1.000x | +0.00% | 428.00 | 1.000x | 1.000x | 1.220x | -22.02% | 1.357x | -35.71% | 0.680x | +32.00% | 0 |
| generic-sse | 107.00 | 107.00 | 1.000x | +0.00% | 107.00 | 1.000x | 1.000x | 0.968x | +3.17% | 0.456x | +54.43% | 0.535x | +46.49% | 0 |
| https-static-small | 1298.50 | 1297.50 | 1.001x | +0.08% | 1296.00 | 1.002x | 1.001x | 0.971x | +2.88% | 1.002x | -0.21% | 0.684x | +31.61% | 0 |
| qcp-transparent | 904.00 | 904.00 | 1.000x | +0.00% | 904.00 | 1.000x | 1.000x | 0.896x | +10.41% | 1.144x | -14.40% | 0.716x | +28.43% | 0 |
| reverse-proxy | 2878.50 | 2877.50 | 1.000x | +0.03% | 2880.00 | 0.999x | 0.999x | 0.945x | +5.52% | 1.207x | -20.71% | 0.839x | +16.13% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.00 | 1.000x | 1.000x | 1.016x | -1.57% | 1.234x | -23.43% | 0.701x | +29.89% | 0 |
| static-small | 7110.00 | 7111.00 | 1.000x | -0.01% | 7104.00 | 1.001x | 1.001x | 0.842x | +15.76% | 0.851x | +14.86% | 0.646x | +35.40% | 0 |
| tcp-stream | 436.00 | 436.00 | 1.000x | +0.00% | 436.00 | 1.000x | 1.000x | 1.228x | -22.78% | 1.278x | -27.77% | 0.800x | +20.03% | 0 |
| udp-stream | 904.00 | 908.00 | 0.996x | -0.44% | 908.00 | 0.996x | 1.000x | 0.836x | +16.42% | 0.882x | +11.85% | 0.613x | +38.72% | 0 |
| websocket-long-connection | 420.00 | 420.00 | 1.000x | +0.00% | 420.00 | 1.000x | 1.000x | 1.022x | -2.20% | 1.553x | -55.28% | 0.858x | +14.25% | 0 |

- Aggregate proxysss ops/s: `21381.00`
- Aggregate nginx ops/s: `21384.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
