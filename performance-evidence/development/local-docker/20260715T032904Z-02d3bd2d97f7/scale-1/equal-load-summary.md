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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6685.00 | 6688.00 | 1.000x | -0.04% | 6688.00 | 1.000x | 1.000x | 0.871x | +12.87% | 0.727x | +27.33% | 0.705x | +29.47% | 0 |
| game-long-connection | 860.00 | 860.00 | 1.000x | +0.00% | 860.00 | 1.000x | 1.000x | 1.233x | -23.33% | 1.290x | -29.00% | 0.405x | +59.54% | 0 |
| generic-sse | 106.00 | 106.00 | 1.000x | +0.00% | 106.00 | 1.000x | 1.000x | 1.038x | -3.77% | 1.248x | -24.77% | 1.582x | -58.16% | 0 |
| https-static-small | 1318.00 | 1318.00 | 1.000x | +0.00% | 1316.00 | 1.002x | 1.002x | 0.975x | +2.49% | 0.947x | +5.29% | 0.902x | +9.82% | 0 |
| qcp-transparent | 916.00 | 916.00 | 1.000x | +0.00% | 916.00 | 1.000x | 1.000x | 0.913x | +8.68% | 1.368x | -36.77% | 1.444x | -44.35% | 0 |
| reverse-proxy | 2731.50 | 2731.50 | 1.000x | +0.00% | 2720.00 | 1.004x | 1.004x | 1.032x | -3.25% | 1.301x | -30.09% | 0.714x | +28.61% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 20.00 | 1.050x | 1.050x | 1.012x | -1.18% | 1.086x | -8.64% | 0.956x | +4.41% | 0 |
| static-small | 6770.00 | 6771.50 | 1.000x | -0.02% | 6768.00 | 1.000x | 1.001x | 0.856x | +14.36% | 0.642x | +35.81% | 0.583x | +41.66% | 0 |
| tcp-stream | 840.00 | 840.00 | 1.000x | +0.00% | 844.00 | 0.995x | 0.995x | 1.185x | -18.50% | 1.302x | -30.20% | 0.670x | +32.97% | 0 |
| udp-stream | 844.00 | 844.00 | 1.000x | +0.00% | 844.00 | 1.000x | 1.000x | 0.913x | +8.71% | 1.098x | -9.75% | 1.023x | -2.34% | 0 |
| websocket-long-connection | 832.00 | 832.00 | 1.000x | +0.00% | 832.00 | 1.000x | 1.000x | 1.164x | -16.43% | 1.284x | -28.41% | 1.125x | -12.46% | 0 |

- Aggregate proxysss ops/s: `21923.50`
- Aggregate nginx ops/s: `21928.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
