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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-http-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 42620.75 | 38415.25 | 1.109x | +10.95% | - | - | - | 0.928x | +7.21% | 1.044x | -4.37% | 1.098x | -9.81% | 0 |
| generic-sse | 966.75 | 772.50 | 1.251x | +25.15% | - | - | - | 0.750x | +25.01% | 0.951x | +4.90% | 1.042x | -4.16% | 1 |
| reverse-proxy | 23910.00 | 20618.75 | 1.160x | +15.96% | - | - | - | 0.816x | +18.38% | 1.021x | -2.06% | 1.116x | -11.56% | 0 |
| static-small | 41703.50 | 37232.50 | 1.120x | +12.01% | - | - | - | 0.917x | +8.33% | 1.025x | -2.48% | 1.091x | -9.13% | 0 |

- Aggregate proxysss ops/s: `109201.00`
- Aggregate nginx ops/s: `97039.00`
- Aggregate proxysss/nginx ratio: `1.125x`
- Aggregate throughput improvement: `+12.53%`
