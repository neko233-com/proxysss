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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6484.33 | 6483.67 | 1.000x | +0.01% | 6485.33 | 1.000x | 1.000x | 0.866x | +13.37% | 0.899x | +10.11% | 1.508x | -50.79% | 0 |
| game-long-connection | 949.33 | 949.33 | 1.000x | +0.00% | 949.33 | 1.000x | 1.000x | 1.187x | -18.68% | 0.843x | +15.69% | 2.601x | -160.06% | 0 |
| generic-sse | 119.67 | 119.67 | 1.000x | +0.00% | 118.67 | 1.008x | 1.008x | 0.984x | +1.61% | 0.961x | +3.88% | 1.972x | -97.15% | 0 |
| https-static-small | 1606.67 | 1607.00 | 1.000x | -0.02% | 1605.33 | 1.001x | 1.001x | 0.976x | +2.43% | 0.830x | +16.96% | 0.570x | +43.01% | 0 |
| qcp-transparent | 933.33 | 933.33 | 1.000x | +0.00% | 933.33 | 1.000x | 1.000x | 0.886x | +11.37% | 1.039x | -3.88% | 3.017x | -201.71% | 0 |
| reverse-proxy | 3186.33 | 3186.33 | 1.000x | +0.00% | 3178.67 | 1.002x | 1.002x | 1.051x | -5.06% | 1.115x | -11.48% | 1.717x | -71.71% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 21.33 | 1.031x | 1.031x | 1.245x | -24.47% | 1.615x | -61.45% | 3.245x | -224.52% | 0 |
| static-small | 6561.67 | 6563.33 | 1.000x | -0.03% | 6549.33 | 1.002x | 1.002x | 0.872x | +12.85% | 0.956x | +4.39% | 1.249x | -24.92% | 0 |
| tcp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 1.152x | -15.24% | 0.830x | +16.97% | 1.196x | -19.64% | 0 |
| udp-stream | 938.67 | 938.67 | 1.000x | +0.00% | 938.67 | 1.000x | 1.000x | 0.865x | +13.47% | 0.930x | +6.96% | 3.236x | -223.65% | 0 |
| websocket-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.957x | +4.29% | 0.839x | +16.09% | 1.490x | -49.00% | 0 |

- Aggregate proxysss ops/s: `22658.00`
- Aggregate nginx ops/s: `22659.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
