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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5899.00 | 5900.50 | 1.000x | -0.03% | 5911.69 | 0.998x | 0.998x | 0.792x | +20.79% | 0.927x | +7.30% | 0.854x | +14.62% | 0 |
| game-long-connection | 1200.00 | 1200.00 | 1.000x | +0.00% | 1206.09 | 0.995x | 0.995x | 1.325x | -32.51% | 1.005x | -0.50% | 1.439x | -43.94% | 0 |
| generic-sse | 121.00 | 121.00 | 1.000x | +0.00% | 121.25 | 0.998x | 0.998x | 0.989x | +1.14% | 1.063x | -6.29% | 1.115x | -11.48% | 0 |
| https-static-small | 1337.00 | 1335.50 | 1.001x | +0.11% | 1337.90 | 0.999x | 0.998x | 1.035x | -3.49% | 1.068x | -6.75% | 1.056x | -5.59% | 0 |
| qcp-transparent | 1024.00 | 1024.00 | 1.000x | +0.00% | 1040.24 | 0.984x | 0.984x | 0.853x | +14.75% | 1.040x | -3.98% | 0.932x | +6.80% | 0 |
| reverse-proxy | 2835.00 | 2832.00 | 1.001x | +0.11% | 2836.50 | 0.999x | 0.998x | 1.005x | -0.45% | 0.894x | +10.63% | 1.297x | -29.65% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.25 | 0.989x | 0.989x | 0.969x | +3.14% | 0.844x | +15.62% | 1.156x | -15.60% | 0 |
| static-small | 5933.00 | 5934.00 | 1.000x | -0.02% | 5938.57 | 0.999x | 0.999x | 0.804x | +19.62% | 1.084x | -8.37% | 0.675x | +32.52% | 0 |
| tcp-stream | 1120.00 | 1120.00 | 1.000x | +0.00% | 1132.34 | 0.989x | 0.989x | 1.291x | -29.14% | 1.260x | -26.03% | 1.147x | -14.74% | 0 |
| udp-stream | 1216.00 | 1216.00 | 1.000x | +0.00% | 1219.33 | 0.997x | 0.997x | 0.850x | +14.96% | 0.642x | +35.82% | 1.144x | -14.35% | 0 |
| websocket-long-connection | 960.00 | 960.00 | 1.000x | +0.00% | 962.87 | 0.997x | 0.997x | 1.107x | -10.67% | 1.402x | -40.23% | 1.421x | -42.10% | 0 |

- Aggregate proxysss ops/s: `21667.00`
- Aggregate nginx ops/s: `21665.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
