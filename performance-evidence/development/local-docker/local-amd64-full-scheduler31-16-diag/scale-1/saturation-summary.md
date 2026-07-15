# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-scheduler31-16-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22054.25 | 21197.45 | 1.040x | +4.04% | - | - | - | 0.773x | +22.69% | 1.112x | -11.16% | 1.096x | -9.62% | 0 |
| game-long-connection | 2646.55 | 3674.05 | 0.720x | -27.97% | - | - | - | 1.217x | -21.73% | 1.707x | -70.73% | 1.927x | -92.72% | 0 |
| generic-sse | 604.60 | 480.40 | 1.259x | +25.85% | - | - | - | 0.661x | +33.88% | 1.189x | -18.87% | 1.219x | -21.94% | 0 |
| https-static-small | 3829.95 | 5543.65 | 0.691x | -30.91% | - | - | - | 1.533x | -53.29% | 1.394x | -39.43% | 1.189x | -18.90% | 0 |
| qcp-transparent | 4759.40 | 3756.15 | 1.267x | +26.71% | - | - | - | 0.637x | +36.33% | 1.111x | -11.13% | 1.134x | -13.38% | 0 |
| reverse-proxy | 12063.25 | 11028.15 | 1.094x | +9.39% | - | - | - | 0.841x | +15.91% | 1.179x | -17.85% | 1.164x | -16.44% | 0 |
| static-large | 93.35 | 86.60 | 1.078x | +7.79% | - | - | - | 0.918x | +8.23% | 0.973x | +2.70% | 0.971x | +2.87% | 0 |
| static-small | 22275.35 | 21430.95 | 1.039x | +3.94% | - | - | - | 0.760x | +23.97% | 1.134x | -13.41% | 1.098x | -9.79% | 0 |
| tcp-stream | 2634.75 | 3645.25 | 0.723x | -27.72% | - | - | - | 1.220x | -21.96% | 1.711x | -71.11% | 1.831x | -83.14% | 0 |
| udp-stream | 4735.20 | 3766.05 | 1.257x | +25.73% | - | - | - | 0.621x | +37.93% | 1.133x | -13.26% | 1.127x | -12.74% | 0 |
| websocket-long-connection | 2500.90 | 3630.70 | 0.689x | -31.12% | - | - | - | 1.286x | -28.62% | 1.792x | -79.23% | 1.929x | -92.95% | 0 |

- Aggregate proxysss ops/s: `78197.55`
- Aggregate nginx ops/s: `78239.40`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.05%`
