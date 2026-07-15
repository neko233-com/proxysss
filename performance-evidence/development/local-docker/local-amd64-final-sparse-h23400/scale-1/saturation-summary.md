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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-final-sparse-h23400/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22751.75 | 20393.90 | 1.116x | +11.56% | - | - | - | 0.713x | +28.72% | 1.072x | -7.16% | 0.994x | +0.62% | 0 |
| game-long-connection | 4189.60 | 3671.50 | 1.141x | +14.11% | - | - | - | 0.801x | +19.90% | 1.161x | -16.08% | 0.998x | +0.19% | 0 |
| generic-sse | 657.90 | 458.05 | 1.436x | +43.63% | - | - | - | 0.558x | +44.16% | 1.074x | -7.39% | 0.925x | +7.48% | 0 |
| https-static-small | 5889.50 | 5452.85 | 1.080x | +8.01% | - | - | - | 0.866x | +13.35% | 1.180x | -18.02% | 1.000x | -0.03% | 0 |
| qcp-transparent | 5226.90 | 3668.35 | 1.425x | +42.49% | - | - | - | 0.508x | +49.16% | 1.062x | -6.19% | 0.880x | +12.02% | 0 |
| reverse-proxy | 13490.10 | 11360.80 | 1.187x | +18.74% | - | - | - | 0.759x | +24.12% | 1.102x | -10.23% | 0.895x | +10.49% | 0 |
| static-large | 91.95 | 88.60 | 1.038x | +3.78% | - | - | - | 0.977x | +2.28% | 0.971x | +2.91% | 0.761x | +23.87% | 0 |
| static-small | 22710.25 | 20684.85 | 1.098x | +9.79% | - | - | - | 0.706x | +29.37% | 1.094x | -9.40% | 0.994x | +0.57% | 0 |
| tcp-stream | 4176.30 | 3588.50 | 1.164x | +16.38% | - | - | - | 0.790x | +20.98% | 1.159x | -15.89% | 0.977x | +2.28% | 0 |
| udp-stream | 5376.30 | 3728.15 | 1.442x | +44.21% | - | - | - | 0.496x | +50.37% | 1.043x | -4.35% | 0.890x | +10.95% | 0 |
| websocket-long-connection | 3856.75 | 3578.50 | 1.078x | +7.78% | - | - | - | 0.856x | +14.42% | 1.203x | -20.34% | 1.071x | -7.12% | 0 |

- Aggregate proxysss ops/s: `88417.30`
- Aggregate nginx ops/s: `76674.05`
- Aggregate proxysss/nginx ratio: `1.153x`
- Aggregate throughput improvement: `+15.32%`
