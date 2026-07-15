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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 28821.00 | 24956.50 | 1.155x | +15.48% | - | - | - | 0.648x | +35.19% | 1.184x | -18.43% | 1.085x | -8.52% | 0 |
| game-long-connection | 5160.00 | 4149.00 | 1.244x | +24.37% | - | - | - | 0.532x | +46.77% | 1.226x | -22.55% | 0.977x | +2.31% | 0 |
| generic-sse | 679.00 | 440.50 | 1.541x | +54.14% | - | - | - | 0.520x | +48.00% | 0.882x | +11.80% | 0.758x | +24.15% | 0 |
| https-static-small | 5982.00 | 4706.50 | 1.271x | +27.10% | - | - | - | 0.516x | +48.42% | 1.197x | -19.73% | 0.958x | +4.24% | 0 |
| qcp-transparent | 5239.00 | 4071.00 | 1.287x | +28.69% | - | - | - | 0.575x | +42.48% | 0.993x | +0.72% | 1.130x | -13.02% | 0 |
| reverse-proxy | 13133.00 | 11382.00 | 1.154x | +15.38% | - | - | - | 0.762x | +23.78% | 1.000x | +0.01% | 1.181x | -18.06% | 0 |
| static-large | 83.50 | 85.50 | 0.977x | -2.34% | - | - | - | 1.047x | -4.66% | 1.209x | -20.91% | 0.998x | +0.25% | 0 |
| static-small | 30893.00 | 25835.00 | 1.196x | +19.58% | - | - | - | 0.646x | +35.35% | 1.140x | -14.00% | 1.284x | -28.39% | 0 |
| tcp-stream | 5542.00 | 3982.00 | 1.392x | +39.18% | - | - | - | 0.441x | +55.85% | 1.160x | -15.99% | 1.045x | -4.48% | 0 |
| udp-stream | 5115.50 | 3789.00 | 1.350x | +35.01% | - | - | - | 0.533x | +46.69% | 1.107x | -10.74% | 1.040x | -3.96% | 0 |
| websocket-long-connection | 4885.50 | 3474.50 | 1.406x | +40.61% | - | - | - | 0.498x | +50.19% | 1.082x | -8.24% | 1.069x | -6.90% | 0 |

- Aggregate proxysss ops/s: `105533.50`
- Aggregate nginx ops/s: `86871.50`
- Aggregate proxysss/nginx ratio: `1.215x`
- Aggregate throughput improvement: `+21.48%`
