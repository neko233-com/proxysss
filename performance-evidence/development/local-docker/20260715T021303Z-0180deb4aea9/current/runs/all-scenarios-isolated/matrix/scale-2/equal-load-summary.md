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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5449.50 | 5446.00 | 1.001x | +0.06% | 5456.10 | 0.999x | 0.998x | 0.785x | +21.51% | 0.734x | +26.56% | 0.599x | +40.13% | 0 |
| game-long-connection | 1184.00 | 1184.00 | 1.000x | +0.00% | 1193.05 | 0.992x | 0.992x | 0.972x | +2.83% | 1.012x | -1.20% | 2.590x | -158.99% | 0 |
| generic-sse | 129.50 | 129.00 | 1.004x | +0.39% | 130.12 | 0.995x | 0.991x | 0.971x | +2.88% | 0.910x | +8.95% | 0.682x | +31.80% | 0 |
| https-static-small | 948.00 | 948.00 | 1.000x | +0.00% | 949.61 | 0.998x | 0.998x | 0.987x | +1.26% | 1.697x | -69.74% | 1.039x | -3.91% | 0 |
| qcp-transparent | 1184.00 | 1184.00 | 1.000x | +0.00% | 1188.18 | 0.996x | 0.996x | 0.831x | +16.89% | 1.649x | -64.93% | 1.625x | -62.48% | 0 |
| reverse-proxy | 2172.50 | 2170.00 | 1.001x | +0.12% | 2174.73 | 0.999x | 0.998x | 0.971x | +2.93% | 1.121x | -12.11% | 0.723x | +27.72% | 0 |
| static-large | 24.00 | 24.00 | 1.000x | +0.00% | 24.25 | 0.990x | 0.990x | 0.981x | +1.93% | 0.983x | +1.67% | 0.949x | +5.09% | 0 |
| static-small | 6422.00 | 6424.50 | 1.000x | -0.04% | 6430.87 | 0.999x | 0.999x | 0.817x | +18.33% | 1.254x | -25.37% | 0.958x | +4.23% | 0 |
| tcp-stream | 1088.00 | 1088.00 | 1.000x | +0.00% | 1093.94 | 0.995x | 0.995x | 0.983x | +1.70% | 1.412x | -41.20% | 2.016x | -101.58% | 0 |
| udp-stream | 928.00 | 928.00 | 1.000x | +0.00% | 929.58 | 0.998x | 0.998x | 0.841x | +15.86% | 1.579x | -57.92% | 1.929x | -92.95% | 0 |
| websocket-long-connection | 1248.00 | 1248.00 | 1.000x | +0.00% | 1265.12 | 0.986x | 0.986x | 0.984x | +1.60% | 1.432x | -43.19% | 1.471x | -47.08% | 0 |

- Aggregate proxysss ops/s: `20777.50`
- Aggregate nginx ops/s: `20773.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
