# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-udp-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 34132.75 | 34512.75 | 0.989x | -1.10% | - | - | - | 0.909x | +9.06% | 1.486x | -48.62% | 1.542x | -54.20% | 0 |
| generic-sse | 796.75 | 647.75 | 1.230x | +23.00% | - | - | - | 0.676x | +32.45% | 1.208x | -20.83% | 1.195x | -19.53% | 1 |
| qcp-transparent | 2544.00 | 5532.50 | 0.460x | -54.02% | - | - | - | 1.793x | -79.29% | 3.371x | -237.10% | 2.999x | -199.92% | 0 |
| reverse-proxy | 20446.50 | 17357.75 | 1.178x | +17.79% | - | - | - | 0.722x | +27.79% | 1.296x | -29.63% | 1.304x | -30.41% | 0 |
| static-small | 34927.75 | 34500.50 | 1.012x | +1.24% | - | - | - | 0.882x | +11.76% | 1.491x | -49.13% | 1.613x | -61.29% | 0 |
| udp-stream | 2532.25 | 5426.00 | 0.467x | -53.33% | - | - | - | 1.764x | -76.40% | 3.431x | -243.06% | 2.749x | -174.89% | 0 |

- Aggregate proxysss ops/s: `95380.00`
- Aggregate nginx ops/s: `97977.25`
- Aggregate proxysss/nginx ratio: `0.973x`
- Aggregate throughput improvement: `-2.65%`
