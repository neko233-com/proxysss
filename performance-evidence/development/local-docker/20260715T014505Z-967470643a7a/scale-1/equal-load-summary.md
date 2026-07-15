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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4635.50 | 4633.50 | 1.000x | +0.04% | 4638.35 | 0.999x | 0.999x | 0.828x | +17.17% | 1.154x | -15.36% | 0.781x | +21.91% | 0 |
| game-long-connection | 724.00 | 724.00 | 1.000x | +0.00% | 726.74 | 0.996x | 0.996x | 1.045x | -4.47% | 1.306x | -30.60% | 1.358x | -35.83% | 0 |
| generic-sse | 85.00 | 85.00 | 1.000x | +0.00% | 85.37 | 0.996x | 0.996x | 1.018x | -1.79% | 1.388x | -38.79% | 0.692x | +30.80% | 0 |
| https-static-small | 695.50 | 695.50 | 1.000x | +0.00% | 696.20 | 0.999x | 0.999x | 1.074x | -7.37% | 1.190x | -19.04% | 0.587x | +41.27% | 0 |
| qcp-transparent | 656.00 | 656.00 | 1.000x | +0.00% | 659.36 | 0.995x | 0.995x | 0.897x | +10.31% | 1.186x | -18.64% | 0.944x | +5.60% | 0 |
| reverse-proxy | 1567.50 | 1569.50 | 0.999x | -0.13% | 1570.55 | 0.998x | 0.999x | 1.018x | -1.78% | 0.811x | +18.88% | 1.632x | -63.18% | 0 |
| static-large | 20.50 | 20.50 | 1.000x | +0.00% | 20.87 | 0.982x | 0.982x | 0.989x | +1.14% | 0.846x | +15.37% | 0.774x | +22.58% | 0 |
| static-small | 4634.00 | 4636.50 | 0.999x | -0.05% | 4637.68 | 0.999x | 1.000x | 0.832x | +16.75% | 1.081x | -8.07% | 1.222x | -22.21% | 0 |
| tcp-stream | 772.00 | 772.00 | 1.000x | +0.00% | 774.37 | 0.997x | 0.997x | 1.103x | -10.26% | 1.440x | -44.01% | 0.879x | +12.10% | 0 |
| udp-stream | 620.00 | 620.00 | 1.000x | +0.00% | 620.97 | 0.998x | 0.998x | 0.931x | +6.93% | 1.427x | -42.74% | 0.823x | +17.72% | 0 |
| websocket-long-connection | 628.00 | 628.00 | 1.000x | +0.00% | 631.86 | 0.994x | 0.994x | 1.000x | +0.00% | 0.969x | +3.11% | 0.611x | +38.93% | 0 |

- Aggregate proxysss ops/s: `15038.00`
- Aggregate nginx ops/s: `15040.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
