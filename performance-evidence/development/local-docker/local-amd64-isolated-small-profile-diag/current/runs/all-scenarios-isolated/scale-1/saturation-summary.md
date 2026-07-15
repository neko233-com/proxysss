# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `small`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-small-profile-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7545.25 | 21541.25 | 0.350x | -64.97% | - | - | - | 1.353x | -35.30% | 4.080x | -307.97% | 4.642x | -364.20% | 0 |
| game-long-connection | 4333.00 | 4112.00 | 1.054x | +5.37% | - | - | - | 0.757x | +24.26% | 1.151x | -15.08% | 1.431x | -43.07% | 0 |
| generic-sse | 149.00 | 478.50 | 0.311x | -68.86% | - | - | - | 2.185x | -118.54% | 4.929x | -392.88% | 8.342x | -734.18% | 1 |
| https-static-small | 3207.75 | 5507.00 | 0.582x | -41.75% | - | - | - | 1.653x | -65.29% | 1.703x | -70.30% | 1.933x | -93.31% | 0 |
| qcp-transparent | 4244.50 | 3848.50 | 1.103x | +10.29% | - | - | - | 0.740x | +26.03% | 1.084x | -8.44% | 1.428x | -42.77% | 0 |
| reverse-proxy | 3877.75 | 11193.00 | 0.346x | -65.36% | - | - | - | 1.869x | -86.87% | 4.680x | -367.98% | 7.239x | -623.90% | 0 |
| static-large | 89.75 | 85.75 | 1.047x | +4.66% | - | - | - | 0.894x | +10.64% | 1.417x | -41.67% | 1.018x | -1.77% | 0 |
| static-small | 7711.50 | 22873.25 | 0.337x | -66.29% | - | - | - | 1.374x | -37.39% | 4.362x | -336.17% | 4.941x | -394.14% | 0 |
| tcp-stream | 4285.50 | 4063.50 | 1.055x | +5.46% | - | - | - | 0.756x | +24.43% | 1.158x | -15.83% | 1.368x | -36.76% | 0 |
| udp-stream | 4189.50 | 3865.50 | 1.084x | +8.38% | - | - | - | 0.749x | +25.07% | 1.122x | -12.24% | 1.470x | -46.95% | 0 |
| websocket-long-connection | 4068.00 | 3615.75 | 1.125x | +12.51% | - | - | - | 0.698x | +30.18% | 1.137x | -13.65% | 1.473x | -47.27% | 0 |

- Aggregate proxysss ops/s: `43701.50`
- Aggregate nginx ops/s: `81184.00`
- Aggregate proxysss/nginx ratio: `0.538x`
- Aggregate throughput improvement: `-46.17%`
