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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.977 < 0.980 (actual=21.00 target=21.50)`
| cdn-hot-update | 6739.00 | 6741.00 | 1.000x | -0.03% | 6748.92 | 0.999x | 0.999x | 0.840x | +16.04% | 1.031x | -3.14% | 1.067x | -6.71% | 0 |
| game-long-connection | 1088.00 | 1088.00 | 1.000x | +0.00% | 1098.67 | 0.990x | 0.990x | 1.141x | -14.10% | 1.525x | -52.54% | 0.772x | +22.81% | 0 |
| generic-sse | 134.00 | 134.00 | 1.000x | +0.00% | 134.87 | 0.994x | 0.994x | 1.083x | -8.28% | 1.308x | -30.82% | 1.161x | -16.07% | 0 |
| https-static-small | 1213.50 | 1213.00 | 1.000x | +0.04% | 1214.97 | 0.999x | 0.998x | 1.043x | -4.33% | 1.024x | -2.44% | 1.061x | -6.07% | 0 |
| qcp-transparent | 1136.00 | 1136.00 | 1.000x | +0.00% | 1139.11 | 0.997x | 0.997x | 0.947x | +5.28% | 2.912x | -191.17% | 0.766x | +23.37% | 0 |
| reverse-proxy | 3142.00 | 3141.50 | 1.000x | +0.02% | 3145.89 | 0.999x | 0.999x | 1.060x | -5.97% | 0.650x | +34.98% | 0.664x | +33.55% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 21.50 | 0.977x | 0.977x | 1.031x | -3.10% | 0.986x | +1.43% | 0.956x | +4.38% | 0 |
| static-small | 6299.50 | 6298.50 | 1.000x | +0.02% | 6302.93 | 0.999x | 0.999x | 0.871x | +12.92% | 1.049x | -4.90% | 0.934x | +6.62% | 0 |
| tcp-stream | 1072.00 | 1072.00 | 1.000x | +0.00% | 1075.34 | 0.997x | 0.997x | 1.090x | -8.99% | 0.883x | +11.72% | 2.142x | -114.20% | 0 |
| udp-stream | 1072.00 | 1072.00 | 1.000x | +0.00% | 1083.57 | 0.989x | 0.989x | 1.015x | -1.52% | 1.332x | -33.24% | 0.852x | +14.77% | 0 |
| websocket-long-connection | 1008.00 | 1008.00 | 1.000x | +0.00% | 1023.08 | 0.985x | 0.985x | 1.171x | -17.14% | 1.146x | -14.64% | 2.825x | -182.52% | 0 |

- Aggregate proxysss ops/s: `22925.00`
- Aggregate nginx ops/s: `22925.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
