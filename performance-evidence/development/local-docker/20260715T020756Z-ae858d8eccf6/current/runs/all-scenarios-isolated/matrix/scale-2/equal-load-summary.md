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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5018.50 | 5017.50 | 1.000x | +0.02% | 5019.61 | 1.000x | 1.000x | 0.724x | +27.62% | 0.767x | +23.30% | 2.561x | -156.09% | 0 |
| game-long-connection | 1080.00 | 1080.00 | 1.000x | +0.00% | 1085.33 | 0.995x | 0.995x | 0.943x | +5.65% | 1.350x | -34.96% | 0.925x | +7.49% | 0 |
| generic-sse | 107.00 | 107.00 | 1.000x | +0.00% | 107.25 | 0.998x | 0.998x | 0.984x | +1.61% | 1.077x | -7.74% | 0.718x | +28.21% | 0 |
| https-static-small | 1113.00 | 1113.00 | 1.000x | +0.00% | 1113.59 | 0.999x | 0.999x | 1.000x | +0.00% | 1.349x | -34.92% | 1.186x | -18.61% | 0 |
| qcp-transparent | 768.00 | 768.00 | 1.000x | +0.00% | 775.12 | 0.991x | 0.991x | 0.926x | +7.45% | 1.162x | -16.18% | 1.795x | -79.52% | 0 |
| reverse-proxy | 2165.50 | 2164.00 | 1.001x | +0.07% | 2166.33 | 1.000x | 0.999x | 1.012x | -1.20% | 1.230x | -22.96% | 1.697x | -69.72% | 0 |
| static-large | 34.00 | 34.00 | 1.000x | +0.00% | 34.25 | 0.993x | 0.993x | 0.957x | +4.33% | 1.065x | -6.51% | 1.207x | -20.66% | 0 |
| static-small | 4444.50 | 4444.00 | 1.000x | +0.01% | 4446.91 | 0.999x | 0.999x | 0.740x | +25.99% | 1.034x | -3.36% | 0.929x | +7.10% | 0 |
| tcp-stream | 1032.00 | 1032.00 | 1.000x | +0.00% | 1034.06 | 0.998x | 0.998x | 0.894x | +10.60% | 0.848x | +15.20% | 1.493x | -49.31% | 0 |
| udp-stream | 864.00 | 864.00 | 1.000x | +0.00% | 868.24 | 0.995x | 0.995x | 0.964x | +3.64% | 1.592x | -59.25% | 2.930x | -193.04% | 0 |
| websocket-long-connection | 960.00 | 952.00 | 1.008x | +0.84% | 961.83 | 0.998x | 0.990x | 0.880x | +12.04% | 1.369x | -36.92% | 2.846x | -184.59% | 0 |

- Aggregate proxysss ops/s: `17586.50`
- Aggregate nginx ops/s: `17575.50`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.06%`
