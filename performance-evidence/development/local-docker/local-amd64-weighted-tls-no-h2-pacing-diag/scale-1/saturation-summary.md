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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-weighted-tls-no-h2-pacing-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 14623.75 | 20053.25 | 0.729x | -27.08% | - | - | - | 0.770x | +23.03% | 2.224x | -122.38% | 2.374x | -137.41% | 0 |
| game-long-connection | 3169.75 | 3658.50 | 0.866x | -13.36% | - | - | - | 0.752x | +24.79% | 1.640x | -63.98% | 1.979x | -97.86% | 0 |
| generic-sse | 312.00 | 455.50 | 0.685x | -31.50% | - | - | - | 1.087x | -8.70% | 2.255x | -125.51% | 2.298x | -129.79% | 0 |
| https-static-small | 4688.75 | 5336.50 | 0.879x | -12.14% | - | - | - | 1.085x | -8.47% | 1.323x | -32.31% | 1.240x | -24.01% | 0 |
| qcp-transparent | 2452.50 | 3426.75 | 0.716x | -28.43% | - | - | - | 0.857x | +14.31% | 2.295x | -129.47% | 2.672x | -167.22% | 0 |
| reverse-proxy | 7482.25 | 9937.25 | 0.753x | -24.71% | - | - | - | 0.967x | +3.30% | 2.071x | -107.08% | 2.268x | -126.83% | 0 |
| static-large | 119.75 | 95.00 | 1.261x | +26.05% | - | - | - | 0.793x | +20.65% | 0.761x | +23.91% | 0.687x | +31.28% | 0 |
| static-small | 13914.25 | 20216.00 | 0.688x | -31.17% | - | - | - | 0.889x | +11.14% | 2.205x | -120.50% | 2.254x | -125.39% | 0 |
| tcp-stream | 3140.50 | 3701.75 | 0.848x | -15.16% | - | - | - | 0.785x | +21.54% | 1.622x | -62.16% | 1.882x | -88.20% | 0 |
| udp-stream | 2360.25 | 3413.50 | 0.691x | -30.86% | - | - | - | 0.913x | +8.69% | 2.377x | -137.68% | 2.690x | -168.97% | 0 |
| websocket-long-connection | 2999.75 | 3508.00 | 0.855x | -14.49% | - | - | - | 0.770x | +23.00% | 1.593x | -59.26% | 1.890x | -89.00% | 0 |

- Aggregate proxysss ops/s: `55263.50`
- Aggregate nginx ops/s: `73802.00`
- Aggregate proxysss/nginx ratio: `0.749x`
- Aggregate throughput improvement: `-25.12%`
