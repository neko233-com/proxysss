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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6277.00 | 6277.67 | 1.000x | -0.01% | 6272.00 | 1.001x | 1.001x | 0.835x | +16.47% | 0.917x | +8.26% | 0.553x | +44.74% | 0 |
| game-long-connection | 877.33 | 877.33 | 1.000x | +0.00% | 877.33 | 1.000x | 1.000x | 1.152x | -15.16% | 1.398x | -39.80% | 2.227x | -122.68% | 0 |
| generic-sse | 109.33 | 109.33 | 1.000x | +0.00% | 109.33 | 1.000x | 1.000x | 0.980x | +1.99% | 0.820x | +18.04% | 0.546x | +45.42% | 0 |
| https-static-small | 1200.33 | 1200.33 | 1.000x | +0.00% | 1200.00 | 1.000x | 1.000x | 0.959x | +4.15% | 0.857x | +14.29% | 0.544x | +45.58% | 0 |
| qcp-transparent | 845.33 | 845.33 | 1.000x | +0.00% | 845.33 | 1.000x | 1.000x | 0.879x | +12.06% | 0.914x | +8.62% | 0.790x | +21.04% | 0 |
| reverse-proxy | 2696.33 | 2696.00 | 1.000x | +0.01% | 2688.00 | 1.003x | 1.003x | 1.016x | -1.64% | 1.009x | -0.92% | 1.355x | -35.49% | 0 |
| static-large | 22.67 | 22.67 | 1.000x | +0.00% | 22.67 | 1.000x | 1.000x | 0.981x | +1.88% | 0.971x | +2.90% | 0.969x | +3.12% | 0 |
| static-small | 6493.33 | 6495.00 | 1.000x | -0.03% | 6496.00 | 1.000x | 1.000x | 0.844x | +15.57% | 0.794x | +20.58% | 0.800x | +20.03% | 0 |
| tcp-stream | 882.67 | 882.67 | 1.000x | +0.00% | 882.67 | 1.000x | 1.000x | 1.108x | -10.84% | 1.402x | -40.16% | 1.109x | -10.85% | 0 |
| udp-stream | 882.67 | 882.67 | 1.000x | +0.00% | 882.67 | 1.000x | 1.000x | 0.851x | +14.86% | 1.000x | +0.00% | 1.269x | -26.86% | 0 |
| websocket-long-connection | 818.67 | 818.67 | 1.000x | +0.00% | 818.67 | 1.000x | 1.000x | 1.055x | -5.50% | 1.346x | -34.63% | 1.627x | -62.65% | 0 |

- Aggregate proxysss ops/s: `21105.66`
- Aggregate nginx ops/s: `21107.67`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
