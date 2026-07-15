# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-read-batch-1x2x4x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22723.80 | 21509.70 | 1.056x | +5.64% | - | - | - | 0.739x | +26.11% | 1.080x | -8.00% | 1.099x | -9.86% | 0 |
| game-long-connection | 4238.90 | 3880.10 | 1.092x | +9.25% | - | - | - | 0.815x | +18.46% | 1.187x | -18.74% | 1.255x | -25.46% | 0 |
| generic-sse | 657.90 | 491.50 | 1.339x | +33.86% | - | - | - | 0.591x | +40.88% | 1.108x | -10.78% | 1.195x | -19.46% | 0 |
| https-static-small | 5808.45 | 6037.90 | 0.962x | -3.80% | - | - | - | 0.946x | +5.44% | 1.192x | -19.21% | 1.023x | -2.30% | 0 |
| qcp-transparent | 5398.60 | 3991.15 | 1.353x | +35.26% | - | - | - | 0.519x | +48.07% | 1.107x | -10.71% | 1.093x | -9.26% | 0 |
| reverse-proxy | 13716.90 | 11946.65 | 1.148x | +14.82% | - | - | - | 0.777x | +22.33% | 1.113x | -11.33% | 1.086x | -8.58% | 0 |
| static-large | 93.30 | 84.55 | 1.103x | +10.35% | - | - | - | 0.901x | +9.92% | 0.955x | +4.47% | 0.902x | +9.77% | 0 |
| static-small | 22856.75 | 21296.30 | 1.073x | +7.33% | - | - | - | 0.727x | +27.32% | 1.088x | -8.81% | 1.083x | -8.28% | 0 |
| tcp-stream | 4228.80 | 3999.45 | 1.057x | +5.73% | - | - | - | 0.879x | +12.08% | 1.222x | -22.23% | 1.239x | -23.87% | 0 |
| udp-stream | 5483.60 | 4001.25 | 1.370x | +37.05% | - | - | - | 0.510x | +49.01% | 1.087x | -8.66% | 1.059x | -5.92% | 0 |
| websocket-long-connection | 3871.70 | 3852.25 | 1.005x | +0.50% | - | - | - | 0.924x | +7.64% | 1.274x | -27.39% | 1.320x | -31.96% | 0 |

- Aggregate proxysss ops/s: `89078.70`
- Aggregate nginx ops/s: `81090.80`
- Aggregate proxysss/nginx ratio: `1.099x`
- Aggregate throughput improvement: `+9.85%`
