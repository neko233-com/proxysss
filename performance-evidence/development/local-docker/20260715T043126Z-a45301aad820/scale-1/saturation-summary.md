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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T043126Z-a45301aad820/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22873.33 | 22082.33 | 1.036x | +3.58% | - | - | - | 0.905x | +9.52% | 1.145x | -14.54% | 1.095x | -9.46% | 0 |
| game-long-connection | 3290.00 | 3013.67 | 1.092x | +9.17% | - | - | - | 0.787x | +21.27% | 1.233x | -23.30% | 1.322x | -32.18% | 0 |
| generic-sse | 458.33 | 399.00 | 1.149x | +14.87% | - | - | - | 0.804x | +19.58% | 1.098x | -9.78% | 1.080x | -7.97% | 0 |
| https-static-small | 4332.67 | 4317.33 | 1.004x | +0.36% | - | - | - | 0.926x | +7.42% | 1.215x | -21.51% | 1.128x | -12.77% | 0 |
| qcp-transparent | 3136.00 | 2951.00 | 1.063x | +6.27% | - | - | - | 0.902x | +9.80% | 1.096x | -9.59% | 1.035x | -3.49% | 0 |
| reverse-proxy | 10490.00 | 9107.67 | 1.152x | +15.18% | - | - | - | 0.898x | +10.21% | 1.002x | -0.16% | 0.912x | +8.83% | 0 |
| static-large | 83.67 | 79.67 | 1.050x | +5.02% | - | - | - | 0.980x | +1.96% | 0.953x | +4.66% | 0.632x | +36.82% | 0 |
| static-small | 22983.00 | 23227.67 | 0.989x | -1.05% | - | - | - | 0.904x | +9.56% | 1.171x | -17.10% | 1.155x | -15.54% | 0 |
| tcp-stream | 3282.67 | 2922.00 | 1.123x | +12.34% | - | - | - | 0.763x | +23.74% | 1.242x | -24.22% | 1.374x | -37.38% | 0 |
| udp-stream | 3058.67 | 2886.67 | 1.060x | +5.96% | - | - | - | 0.954x | +4.55% | 1.091x | -9.10% | 0.973x | +2.75% | 0 |
| websocket-long-connection | 3188.00 | 2853.67 | 1.117x | +11.72% | - | - | - | 0.787x | +21.26% | 1.226x | -22.58% | 1.239x | -23.88% | 0 |

- Aggregate proxysss ops/s: `77176.34`
- Aggregate nginx ops/s: `73840.68`
- Aggregate proxysss/nginx ratio: `1.045x`
- Aggregate throughput improvement: `+4.52%`
