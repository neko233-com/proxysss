# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5668.50 | 5670.00 | 1.000x | -0.03% | 5674.77 | 0.999x | 0.999x | 0.794x | +20.57% | 0.850x | +15.04% | 0.206x | +79.43% | 0 |
| game-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 917.75 | 0.994x | 0.994x | 0.916x | +8.36% | 0.951x | +4.93% | 2.254x | -125.41% | 0 |
| generic-sse | 114.50 | 114.50 | 1.000x | +0.00% | 114.62 | 0.999x | 0.999x | 1.004x | -0.42% | 0.947x | +5.29% | 0.404x | +59.64% | 0 |
| https-static-small | 1130.50 | 1130.50 | 1.000x | +0.00% | 1131.06 | 1.000x | 1.000x | 1.004x | -0.39% | 3.550x | -255.03% | 16.427x | -1542.73% | 0 |
| qcp-transparent | 904.00 | 912.00 | 0.991x | -0.88% | 912.72 | 0.990x | 0.999x | 0.848x | +15.16% | 1.096x | -9.60% | 2.451x | -145.07% | 0 |
| reverse-proxy | 2067.50 | 2067.50 | 1.000x | +0.00% | 2069.46 | 0.999x | 0.999x | 0.960x | +4.02% | 0.906x | +9.36% | 0.534x | +46.59% | 0 |
| static-large | 29.50 | 29.50 | 1.000x | +0.00% | 29.87 | 0.988x | 0.988x | 0.987x | +1.30% | 0.928x | +7.17% | 0.671x | +32.93% | 0 |
| static-small | 5589.50 | 5593.50 | 0.999x | -0.07% | 5594.89 | 0.999x | 1.000x | 0.806x | +19.41% | 0.995x | +0.52% | 1.398x | -39.82% | 0 |
| tcp-stream | 920.00 | 920.00 | 1.000x | +0.00% | 927.59 | 0.992x | 0.992x | 0.986x | +1.40% | 0.729x | +27.08% | 0.318x | +68.25% | 0 |
| udp-stream | 880.00 | 880.00 | 1.000x | +0.00% | 888.10 | 0.991x | 0.991x | 0.820x | +17.98% | 1.146x | -14.63% | 1.351x | -35.08% | 0 |
| websocket-long-connection | 784.00 | 784.00 | 1.000x | +0.00% | 785.97 | 0.997x | 0.997x | 0.950x | +5.00% | 1.376x | -37.58% | 1.201x | -20.10% | 0 |

- Aggregate proxysss ops/s: `19000.00`
- Aggregate nginx ops/s: `19013.50`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.07%`
