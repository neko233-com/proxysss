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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6783.00 | 6784.67 | 1.000x | -0.02% | 6784.00 | 1.000x | 1.000x | 0.787x | +21.31% | 1.599x | -59.87% | 2.295x | -129.53% | 0 |
| game-long-connection | 922.67 | 922.67 | 1.000x | +0.00% | 922.67 | 1.000x | 1.000x | 1.177x | -17.69% | 1.237x | -23.71% | 3.460x | -245.96% | 0 |
| generic-sse | 118.33 | 118.33 | 1.000x | +0.00% | 117.33 | 1.008x | 1.008x | 0.960x | +3.97% | 1.596x | -59.57% | 2.563x | -156.26% | 0 |
| https-static-small | 1473.00 | 1473.33 | 1.000x | -0.02% | 1472.00 | 1.001x | 1.001x | 0.959x | +4.08% | 1.283x | -28.33% | 3.412x | -241.19% | 0 |
| qcp-transparent | 970.67 | 965.33 | 1.006x | +0.55% | 970.67 | 1.000x | 0.995x | 0.947x | +5.31% | 1.771x | -77.10% | 2.778x | -177.82% | 0 |
| reverse-proxy | 3177.33 | 3176.33 | 1.000x | +0.03% | 3157.33 | 1.006x | 1.006x | 0.939x | +6.12% | 2.349x | -134.86% | 2.648x | -164.78% | 0 |
| static-large | 22.33 | 22.33 | 1.000x | +0.00% | 21.33 | 1.047x | 1.047x | 1.027x | -2.73% | 1.033x | -3.30% | 1.122x | -12.16% | 0 |
| static-small | 6973.67 | 6974.00 | 1.000x | -0.00% | 6976.00 | 1.000x | 1.000x | 0.843x | +15.73% | 2.619x | -161.94% | 2.396x | -139.62% | 0 |
| tcp-stream | 922.67 | 922.67 | 1.000x | +0.00% | 922.67 | 1.000x | 1.000x | 1.320x | -32.02% | 1.567x | -56.69% | 4.093x | -309.30% | 0 |
| udp-stream | 965.33 | 965.33 | 1.000x | +0.00% | 965.33 | 1.000x | 1.000x | 0.896x | +10.43% | 1.470x | -47.03% | 2.986x | -198.64% | 0 |
| websocket-long-connection | 901.33 | 901.33 | 1.000x | +0.00% | 901.33 | 1.000x | 1.000x | 1.019x | -1.88% | 2.110x | -111.00% | 4.757x | -375.66% | 0 |

- Aggregate proxysss ops/s: `23230.33`
- Aggregate nginx ops/s: `23226.32`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
