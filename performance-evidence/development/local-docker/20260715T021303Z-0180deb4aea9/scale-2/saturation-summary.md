# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 23902.00 | 21825.00 | 1.095x | +9.52% | - | - | - | 0.821x | +17.88% | 0.951x | +4.88% | 1.123x | -12.27% | 0 |
| game-long-connection | 5798.00 | 4772.50 | 1.215x | +21.49% | - | - | - | 0.765x | +23.46% | 0.851x | +14.87% | 0.701x | +29.91% | 0 |
| generic-sse | 777.00 | 520.50 | 1.493x | +49.28% | - | - | - | 0.507x | +49.29% | 1.003x | -0.30% | 1.199x | -19.88% | 0 |
| https-static-small | 4795.50 | 3798.50 | 1.262x | +26.25% | - | - | - | 0.808x | +19.15% | 0.857x | +14.29% | 0.639x | +36.09% | 0 |
| qcp-transparent | 6288.00 | 4753.00 | 1.323x | +32.30% | - | - | - | 0.591x | +40.93% | 0.891x | +10.89% | 1.045x | -4.50% | 0 |
| reverse-proxy | 11034.50 | 8699.00 | 1.268x | +26.85% | - | - | - | 0.610x | +39.01% | 1.058x | -5.78% | 1.188x | -18.83% | 0 |
| static-large | 97.00 | 108.50 | 0.894x | -10.60% | - | - | - | 1.029x | -2.89% | 1.444x | -44.39% | 0.959x | +4.06% | 0 |
| static-small | 25726.00 | 25882.50 | 0.994x | -0.60% | - | - | - | 0.869x | +13.08% | 1.216x | -21.65% | 1.226x | -22.63% | 0 |
| tcp-stream | 9158.50 | 4376.00 | 2.093x | +109.29% | - | - | - | 0.387x | +61.25% | 0.712x | +28.81% | 0.694x | +30.65% | 0 |
| udp-stream | 6079.50 | 3718.50 | 1.635x | +63.49% | - | - | - | 0.414x | +58.58% | 0.789x | +21.14% | 0.944x | +5.58% | 0 |
| websocket-long-connection | 5698.00 | 5060.50 | 1.126x | +12.60% | - | - | - | 0.837x | +16.34% | 0.835x | +16.45% | 0.888x | +11.24% | 0 |

- Aggregate proxysss ops/s: `99354.00`
- Aggregate nginx ops/s: `83514.50`
- Aggregate proxysss/nginx ratio: `1.190x`
- Aggregate throughput improvement: `+18.97%`
