# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 21443.50 | 22616.50 | 0.948x | -5.19% | - | - | - | 0.991x | +0.90% | 1.389x | -38.88% | 0.996x | +0.42% | 0 |
| game-long-connection | 7731.00 | 4620.50 | 1.673x | +67.32% | - | - | - | 0.462x | +53.76% | 0.795x | +20.50% | 0.923x | +7.72% | 0 |
| generic-sse | 974.00 | 562.50 | 1.732x | +73.16% | - | - | - | 0.451x | +54.93% | 0.893x | +10.73% | 0.865x | +13.52% | 0 |
| https-static-small | 5833.00 | 4440.50 | 1.314x | +31.36% | - | - | - | 0.811x | +18.91% | 0.772x | +22.83% | 0.706x | +29.36% | 0 |
| qcp-transparent | 11079.00 | 4695.50 | 2.359x | +135.95% | - | - | - | 0.204x | +79.55% | 0.674x | +32.64% | 1.104x | -10.37% | 0 |
| reverse-proxy | 11153.50 | 11029.50 | 1.011x | +1.12% | - | - | - | 0.870x | +13.02% | 1.316x | -31.62% | 1.053x | -5.28% | 0 |
| static-large | 75.00 | 89.50 | 0.838x | -16.20% | - | - | - | 1.112x | -11.20% | 1.212x | -21.25% | 3.006x | -200.60% | 0 |
| static-small | 20571.00 | 22486.00 | 0.915x | -8.52% | - | - | - | 1.012x | -1.22% | 1.546x | -54.64% | 1.788x | -78.83% | 0 |
| tcp-stream | 7386.00 | 6427.00 | 1.149x | +14.92% | - | - | - | 0.877x | +12.34% | 0.721x | +27.90% | 0.877x | +12.28% | 0 |
| udp-stream | 7462.50 | 5194.50 | 1.437x | +43.66% | - | - | - | 0.559x | +44.10% | 0.966x | +3.39% | 1.186x | -18.61% | 0 |
| websocket-long-connection | 7067.50 | 3911.00 | 1.807x | +80.71% | - | - | - | 0.417x | +58.33% | 0.680x | +31.96% | 0.787x | +21.27% | 0 |

- Aggregate proxysss ops/s: `100776.00`
- Aggregate nginx ops/s: `86073.00`
- Aggregate proxysss/nginx ratio: `1.171x`
- Aggregate throughput improvement: `+17.08%`
