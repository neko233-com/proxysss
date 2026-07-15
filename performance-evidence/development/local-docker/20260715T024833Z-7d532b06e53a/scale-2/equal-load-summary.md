# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.955 < 0.980 (actual=21.00 target=22.00)`
| cdn-hot-update | 5791.00 | 5789.50 | 1.000x | +0.03% | 5795.00 | 0.999x | 0.999x | 0.823x | +17.70% | 1.034x | -3.38% | 1.538x | -53.78% | 0 |
| game-long-connection | 1392.00 | 1392.00 | 1.000x | +0.00% | 1405.73 | 0.990x | 0.990x | 0.949x | +5.12% | 1.901x | -90.15% | 0.641x | +35.92% | 0 |
| generic-sse | 121.00 | 121.00 | 1.000x | +0.00% | 121.50 | 0.996x | 0.996x | 1.005x | -0.50% | 0.741x | +25.94% | 1.474x | -47.43% | 0 |
| https-static-small | 1272.50 | 1272.50 | 1.000x | +0.00% | 1274.19 | 0.999x | 0.999x | 0.953x | +4.70% | 0.852x | +14.84% | 1.066x | -6.57% | 0 |
| qcp-transparent | 1056.00 | 1056.00 | 1.000x | +0.00% | 1059.18 | 0.997x | 0.997x | 0.783x | +21.72% | 1.522x | -52.23% | 1.410x | -41.04% | 0 |
| reverse-proxy | 2976.50 | 2974.00 | 1.001x | +0.08% | 2979.10 | 0.999x | 0.998x | 1.028x | -2.77% | 1.051x | -5.11% | 0.996x | +0.40% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 22.00 | 0.955x | 0.955x | 0.938x | +6.22% | 0.730x | +27.02% | 1.109x | -10.87% | 0 |
| static-small | 5896.00 | 5895.00 | 1.000x | +0.02% | 5902.97 | 0.999x | 0.999x | 0.811x | +18.89% | 1.081x | -8.13% | 1.360x | -35.99% | 0 |
| tcp-stream | 1056.00 | 1056.00 | 1.000x | +0.00% | 1065.74 | 0.991x | 0.991x | 1.089x | -8.89% | 1.008x | -0.78% | 0.537x | +46.30% | 0 |
| udp-stream | 1088.00 | 1088.00 | 1.000x | +0.00% | 1099.81 | 0.989x | 0.989x | 0.909x | +9.12% | 0.855x | +14.48% | 0.531x | +46.95% | 0 |
| websocket-long-connection | 1056.00 | 1056.00 | 1.000x | +0.00% | 1059.46 | 0.997x | 0.997x | 0.852x | +14.85% | 1.006x | -0.64% | 0.948x | +5.20% | 0 |

- Aggregate proxysss ops/s: `21726.00`
- Aggregate nginx ops/s: `21721.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
