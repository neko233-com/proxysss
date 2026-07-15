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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6527.33 | 6527.33 | 1.000x | +0.00% | 6528.00 | 1.000x | 1.000x | 0.852x | +14.79% | 1.186x | -18.62% | 2.747x | -174.72% | 0 |
| game-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 912.00 | 1.000x | 1.000x | 0.996x | +0.41% | 1.726x | -72.59% | 0.887x | +11.27% | 0 |
| generic-sse | 109.00 | 109.00 | 1.000x | +0.00% | 108.67 | 1.003x | 1.003x | 0.966x | +3.38% | 1.613x | -61.26% | 1.414x | -41.42% | 0 |
| https-static-small | 1590.00 | 1590.00 | 1.000x | +0.00% | 1589.33 | 1.000x | 1.000x | 0.960x | +4.03% | 0.996x | +0.40% | 1.087x | -8.71% | 0 |
| qcp-transparent | 968.00 | 968.00 | 1.000x | +0.00% | 968.00 | 1.000x | 1.000x | 0.908x | +9.24% | 1.126x | -12.62% | 1.079x | -7.88% | 0 |
| reverse-proxy | 3124.33 | 3125.00 | 1.000x | -0.02% | 3125.33 | 1.000x | 1.000x | 1.006x | -0.64% | 1.849x | -84.85% | 3.950x | -294.99% | 0 |
| static-large | 23.67 | 23.67 | 1.000x | +0.00% | 22.67 | 1.044x | 1.044x | 0.897x | +10.34% | 0.890x | +11.01% | 1.026x | -2.60% | 0 |
| static-small | 7106.00 | 7108.00 | 1.000x | -0.03% | 7104.00 | 1.000x | 1.001x | 0.814x | +18.58% | 1.167x | -16.70% | 2.307x | -130.71% | 0 |
| tcp-stream | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 1.028x | -2.83% | 1.918x | -91.79% | 0.981x | +1.90% | 0 |
| udp-stream | 914.67 | 914.67 | 1.000x | +0.00% | 914.67 | 1.000x | 1.000x | 0.913x | +8.68% | 1.366x | -36.60% | 1.466x | -46.57% | 0 |
| websocket-long-connection | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 0.993x | +0.74% | 2.436x | -143.58% | 0.691x | +30.87% | 0 |

- Aggregate proxysss ops/s: `23067.00`
- Aggregate nginx ops/s: `23069.67`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
