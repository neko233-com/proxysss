# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 20799.00 | 20911.00 | 0.995x | -0.54% | - | - | - | 0.761x | +23.90% | 1.659x | -65.86% | 1.373x | -37.27% | 0 |
| game-long-connection | 7842.00 | 4147.00 | 1.891x | +89.10% | - | - | - | 0.391x | +60.94% | 0.722x | +27.76% | 1.020x | -1.98% | 0 |
| generic-sse | 717.00 | 509.00 | 1.409x | +40.86% | - | - | - | 0.633x | +36.70% | 1.166x | -16.63% | 0.605x | +39.55% | 0 |
| https-static-small | 4918.00 | 3705.00 | 1.327x | +32.74% | - | - | - | 0.819x | +18.07% | 0.993x | +0.65% | 0.598x | +40.19% | 0 |
| qcp-transparent | 6159.00 | 7705.00 | 0.799x | -20.06% | - | - | - | 2.946x | -194.57% | 1.057x | -5.70% | 1.361x | -36.15% | 0 |
| reverse-proxy | 11548.00 | 9515.00 | 1.214x | +21.37% | - | - | - | 0.790x | +20.99% | 1.184x | -18.38% | 0.804x | +19.56% | 0 |
| static-large | 90.00 | 134.00 | 0.672x | -32.84% | - | - | - | 1.618x | -61.82% | 1.445x | -44.51% | 1.062x | -6.19% | 0 |
| static-small | 22487.00 | 20861.00 | 1.078x | +7.79% | - | - | - | 0.695x | +30.49% | 1.358x | -35.77% | 1.296x | -29.64% | 0 |
| tcp-stream | 7799.00 | 5515.00 | 1.414x | +41.41% | - | - | - | 0.594x | +40.60% | 0.749x | +25.08% | 0.823x | +17.73% | 0 |
| udp-stream | 5366.00 | 4741.00 | 1.132x | +13.18% | - | - | - | 0.650x | +34.97% | 1.259x | -25.88% | 1.651x | -65.11% | 0 |
| websocket-long-connection | 7991.00 | 3958.00 | 2.019x | +101.89% | - | - | - | 0.397x | +60.28% | 0.714x | +28.62% | 0.858x | +14.19% | 0 |

- Aggregate proxysss ops/s: `95716.00`
- Aggregate nginx ops/s: `81701.00`
- Aggregate proxysss/nginx ratio: `1.172x`
- Aggregate throughput improvement: `+17.15%`
