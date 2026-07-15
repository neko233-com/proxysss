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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6852.00 | 6850.50 | 1.000x | +0.02% | 6848.00 | 1.001x | 1.000x | 0.837x | +16.29% | 1.801x | -80.06% | 1.787x | -78.69% | 0 |
| game-long-connection | 792.00 | 792.00 | 1.000x | +0.00% | 800.00 | 0.990x | 0.990x | 1.217x | -21.67% | 1.446x | -44.57% | 4.105x | -310.51% | 0 |
| generic-sse | 126.50 | 126.50 | 1.000x | +0.00% | 126.00 | 1.004x | 1.004x | 0.988x | +1.20% | 1.227x | -22.72% | 0.642x | +35.79% | 0 |
| https-static-small | 1478.00 | 1477.50 | 1.000x | +0.03% | 1472.00 | 1.004x | 1.004x | 0.956x | +4.38% | 1.032x | -3.16% | 1.429x | -42.93% | 0 |
| qcp-transparent | 1016.00 | 1016.00 | 1.000x | +0.00% | 1016.00 | 1.000x | 1.000x | 0.913x | +8.68% | 0.931x | +6.90% | 2.946x | -194.56% | 0 |
| reverse-proxy | 3169.50 | 3171.00 | 1.000x | -0.05% | 3168.00 | 1.000x | 1.001x | 1.010x | -0.96% | 1.802x | -80.22% | 0.980x | +2.00% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 20.00 | 1.125x | 1.125x | 0.999x | +0.14% | 1.635x | -63.50% | 1.073x | -7.25% | 0 |
| static-small | 6407.00 | 6407.00 | 1.000x | +0.00% | 6400.00 | 1.001x | 1.001x | 0.842x | +15.79% | 1.402x | -40.23% | 1.847x | -84.75% | 0 |
| tcp-stream | 792.00 | 792.00 | 1.000x | +0.00% | 792.00 | 1.000x | 1.000x | 1.124x | -12.41% | 3.343x | -234.33% | 1.038x | -3.75% | 0 |
| udp-stream | 1040.00 | 1040.00 | 1.000x | +0.00% | 1040.00 | 1.000x | 1.000x | 0.961x | +3.94% | 1.312x | -31.23% | 4.583x | -358.30% | 0 |
| websocket-long-connection | 712.00 | 712.00 | 1.000x | +0.00% | 712.00 | 1.000x | 1.000x | 1.128x | -12.80% | 1.735x | -73.49% | 1.887x | -88.67% | 0 |

- Aggregate proxysss ops/s: `22407.50`
- Aggregate nginx ops/s: `22407.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
