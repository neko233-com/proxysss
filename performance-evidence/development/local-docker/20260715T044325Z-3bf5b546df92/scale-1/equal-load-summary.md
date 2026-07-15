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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T044325Z-3bf5b546df92/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `generic-sse nginx target achievement 0.874 < 0.980 (actual=2.33 target=2.67)`
| cdn-hot-update | 163.67 | 163.33 | 1.002x | +0.21% | 160.00 | 1.023x | 1.021x | 1.016x | -1.56% | 0.938x | +6.15% | 0.324x | +67.60% | 0 |
| game-long-connection | 10.67 | 10.67 | 1.000x | +0.00% | 10.67 | 1.000x | 1.000x | 3.252x | -225.15% | 2.340x | -133.95% | 3.128x | -212.81% | 0 |
| generic-sse | 2.33 | 2.33 | 1.000x | +0.00% | 2.67 | 0.874x | 0.874x | 1.253x | -25.30% | 0.789x | +21.09% | 0.789x | +21.09% | 0 |
| https-static-small | 179.67 | 179.67 | 1.000x | +0.00% | 178.67 | 1.006x | 1.006x | 1.408x | -40.78% | 0.992x | +0.76% | 1.129x | -12.88% | 0 |
| qcp-transparent | 10.67 | 10.67 | 1.000x | +0.00% | 10.67 | 1.000x | 1.000x | 0.668x | +33.21% | 1.899x | -89.93% | 1.902x | -90.19% | 0 |
| reverse-proxy | 50.00 | 50.00 | 1.000x | +0.00% | 42.67 | 1.172x | 1.172x | 0.925x | +7.50% | 0.943x | +5.70% | 0.321x | +67.86% | 0 |
| static-large | 1.67 | 1.67 | 1.000x | +0.00% | 1.33 | 1.253x | 1.253x | 1.964x | -96.40% | 0.980x | +2.01% | 0.980x | +2.01% | 0 |
| static-small | 112.67 | 112.67 | 1.000x | +0.00% | 106.67 | 1.056x | 1.056x | 1.005x | -0.48% | 0.595x | +40.51% | 0.235x | +76.51% | 0 |
| tcp-stream | 13.33 | 13.33 | 1.000x | +0.00% | 13.33 | 1.000x | 1.000x | 0.417x | +58.30% | 0.056x | +94.37% | 0.064x | +93.57% | 0 |
| udp-stream | 13.33 | 13.33 | 1.000x | +0.00% | 13.33 | 1.000x | 1.000x | 0.740x | +25.97% | 4.051x | -305.15% | 3.867x | -286.75% | 0 |
| websocket-long-connection | 16.00 | 16.00 | 1.000x | +0.00% | 16.00 | 1.000x | 1.000x | 0.677x | +32.28% | 0.843x | +15.66% | 0.715x | +28.53% | 0 |

- Aggregate proxysss ops/s: `574.01`
- Aggregate nginx ops/s: `573.67`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.06%`
