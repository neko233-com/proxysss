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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T043557Z-2be6ba4712a1/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 2739.67 | 2739.00 | 1.000x | +0.02% | 2730.67 | 1.003x | 1.003x | 0.860x | +13.95% | 0.311x | +68.95% | 0.429x | +57.07% | 0 |
| game-long-connection | 378.67 | 378.67 | 1.000x | +0.00% | 378.67 | 1.000x | 1.000x | 1.027x | -2.73% | 0.313x | +68.70% | 0.430x | +57.03% | 0 |
| generic-sse | 58.33 | 58.33 | 1.000x | +0.00% | 58.00 | 1.006x | 1.006x | 1.032x | -3.25% | 0.251x | +74.94% | 0.325x | +67.45% | 0 |
| https-static-small | 675.00 | 675.00 | 1.000x | +0.00% | 674.67 | 1.000x | 1.000x | 1.157x | -15.73% | 0.951x | +4.93% | 0.806x | +19.38% | 0 |
| qcp-transparent | 370.67 | 370.67 | 1.000x | +0.00% | 370.67 | 1.000x | 1.000x | 0.964x | +3.59% | 0.188x | +81.24% | 0.654x | +34.61% | 0 |
| reverse-proxy | 1181.00 | 1180.67 | 1.000x | +0.03% | 1173.33 | 1.007x | 1.006x | 1.034x | -3.41% | 0.262x | +73.80% | 1.018x | -1.83% | 0 |
| static-large | 13.67 | 13.67 | 1.000x | +0.00% | 13.33 | 1.025x | 1.025x | 0.981x | +1.88% | 0.225x | +77.48% | 0.455x | +54.55% | 0 |
| static-small | 2838.67 | 2839.33 | 1.000x | -0.02% | 2837.33 | 1.000x | 1.001x | 0.871x | +12.94% | 0.326x | +67.43% | 0.338x | +66.20% | 0 |
| tcp-stream | 386.67 | 386.67 | 1.000x | +0.00% | 386.67 | 1.000x | 1.000x | 1.004x | -0.39% | 0.269x | +73.10% | 0.779x | +22.10% | 0 |
| udp-stream | 365.33 | 365.33 | 1.000x | +0.00% | 365.33 | 1.000x | 1.000x | 0.932x | +6.80% | 0.228x | +77.21% | 0.225x | +77.47% | 0 |
| websocket-long-connection | 368.00 | 368.00 | 1.000x | +0.00% | 368.00 | 1.000x | 1.000x | 1.048x | -4.82% | 0.388x | +61.25% | 0.668x | +33.19% | 0 |

- Aggregate proxysss ops/s: `9375.68`
- Aggregate nginx ops/s: `9375.34`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
