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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-baseline-1x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 16514.40 | 16793.75 | 0.983x | -1.66% | - | - | - | 0.823x | +17.72% | 1.470x | -46.99% | 1.149x | -14.86% | 0 |
| game-long-connection | 3327.45 | 2852.05 | 1.167x | +16.67% | - | - | - | 0.716x | +28.44% | 1.129x | -12.92% | 0.786x | +21.36% | 0 |
| generic-sse | 382.60 | 378.05 | 1.012x | +1.20% | - | - | - | 0.842x | +15.76% | 1.381x | -38.06% | 1.178x | -17.77% | 1 |
| https-static-small | 3798.40 | 4385.30 | 0.866x | -13.38% | - | - | - | 1.549x | -54.92% | 1.097x | -9.69% | 0.714x | +28.56% | 0 |
| qcp-transparent | 2913.20 | 2800.75 | 1.040x | +4.01% | - | - | - | 0.745x | +25.51% | 1.501x | -50.09% | 1.280x | -27.99% | 0 |
| reverse-proxy | 8534.10 | 8372.45 | 1.019x | +1.93% | - | - | - | 0.893x | +10.67% | 1.385x | -38.53% | 0.993x | +0.73% | 0 |
| static-large | 108.95 | 73.10 | 1.490x | +49.04% | - | - | - | 0.721x | +27.93% | 0.559x | +44.11% | 0.382x | +61.77% | 0 |
| static-small | 16446.10 | 16448.00 | 1.000x | -0.01% | - | - | - | 0.815x | +18.55% | 1.471x | -47.10% | 1.144x | -14.39% | 0 |
| tcp-stream | 3337.10 | 2848.95 | 1.171x | +17.13% | - | - | - | 0.694x | +30.62% | 1.119x | -11.91% | 0.783x | +21.75% | 0 |
| udp-stream | 2953.40 | 2893.60 | 1.021x | +2.07% | - | - | - | 0.759x | +24.10% | 1.474x | -47.42% | 1.255x | -25.53% | 0 |
| websocket-long-connection | 3183.10 | 2658.35 | 1.197x | +19.74% | - | - | - | 0.684x | +31.58% | 1.139x | -13.89% | 0.777x | +22.32% | 0 |

- Aggregate proxysss ops/s: `61498.80`
- Aggregate nginx ops/s: `60504.35`
- Aggregate proxysss/nginx ratio: `1.016x`
- Aggregate throughput improvement: `+1.64%`
