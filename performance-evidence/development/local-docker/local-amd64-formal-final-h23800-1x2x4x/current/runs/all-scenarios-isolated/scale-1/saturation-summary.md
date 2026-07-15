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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-final-h23800-1x2x4x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 17529.80 | 17041.45 | 1.029x | +2.87% | - | - | - | 0.830x | +17.02% | 1.074x | -7.40% | 1.080x | -7.98% | 0 |
| game-long-connection | 3303.80 | 3386.50 | 0.976x | -2.44% | - | - | - | 1.006x | -0.58% | 1.090x | -9.00% | 1.154x | -15.43% | 0 |
| generic-sse | 464.65 | 421.25 | 1.103x | +10.30% | - | - | - | 0.766x | +23.40% | 1.238x | -23.77% | 1.341x | -34.06% | 0 |
| https-static-small | 4368.25 | 4414.15 | 0.990x | -1.04% | - | - | - | 0.977x | +2.34% | 1.057x | -5.66% | 1.007x | -0.68% | 0 |
| qcp-transparent | 3795.20 | 3314.60 | 1.145x | +14.50% | - | - | - | 0.701x | +29.90% | 1.132x | -13.21% | 1.253x | -25.28% | 0 |
| reverse-proxy | 9891.40 | 9365.65 | 1.056x | +5.61% | - | - | - | 0.871x | +12.95% | 1.144x | -14.38% | 1.184x | -18.45% | 0 |
| static-large | 86.85 | 74.10 | 1.172x | +17.21% | - | - | - | 0.830x | +16.97% | 0.867x | +13.31% | 0.858x | +14.25% | 0 |
| static-small | 17515.05 | 17164.30 | 1.020x | +2.04% | - | - | - | 0.837x | +16.31% | 1.077x | -7.72% | 1.071x | -7.10% | 0 |
| tcp-stream | 3294.15 | 3193.45 | 1.032x | +3.15% | - | - | - | 0.959x | +4.09% | 1.063x | -6.29% | 1.110x | -11.03% | 0 |
| udp-stream | 3762.35 | 3328.45 | 1.130x | +13.04% | - | - | - | 0.703x | +29.74% | 1.165x | -16.47% | 1.264x | -26.39% | 0 |
| websocket-long-connection | 3056.70 | 3170.40 | 0.964x | -3.59% | - | - | - | 1.007x | -0.72% | 1.131x | -13.09% | 1.198x | -19.79% | 0 |

- Aggregate proxysss ops/s: `67068.20`
- Aggregate nginx ops/s: `64874.30`
- Aggregate proxysss/nginx ratio: `1.034x`
- Aggregate throughput improvement: `+3.38%`
