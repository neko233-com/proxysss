# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 34053.33 | 27144.67 | 1.255x | +25.45% | - | - | - | 0.679x | +32.14% | 1.202x | -20.17% | 1.240x | -23.98% | 0 |
| game-long-connection | 6205.33 | 3712.00 | 1.672x | +67.17% | - | - | - | 0.470x | +53.00% | 0.943x | +5.72% | 0.893x | +10.72% | 0 |
| generic-sse | 925.33 | 474.00 | 1.952x | +95.22% | - | - | - | 0.432x | +56.77% | 0.854x | +14.64% | 1.055x | -5.46% | 0 |
| https-static-small | 8643.00 | 5895.67 | 1.466x | +46.60% | - | - | - | 0.486x | +51.38% | 1.345x | -34.47% | 0.893x | +10.66% | 0 |
| qcp-transparent | 5794.33 | 3884.67 | 1.492x | +49.16% | - | - | - | 0.551x | +44.93% | 1.033x | -3.33% | 1.054x | -5.44% | 0 |
| reverse-proxy | 14364.67 | 12714.00 | 1.130x | +12.98% | - | - | - | 0.837x | +16.29% | 1.186x | -18.56% | 1.212x | -21.19% | 0 |
| static-large | 90.00 | 98.67 | 0.912x | -8.79% | - | - | - | 1.343x | -34.28% | 1.278x | -27.79% | 0.971x | +2.94% | 0 |
| static-small | 33814.00 | 27914.00 | 1.211x | +21.14% | - | - | - | 0.691x | +30.93% | 1.168x | -16.79% | 1.246x | -24.57% | 0 |
| tcp-stream | 6319.33 | 3697.33 | 1.709x | +70.92% | - | - | - | 0.452x | +54.82% | 0.958x | +4.18% | 0.958x | +4.16% | 0 |
| udp-stream | 5783.67 | 3869.33 | 1.495x | +49.47% | - | - | - | 0.557x | +44.33% | 1.023x | -2.29% | 1.100x | -10.03% | 0 |
| websocket-long-connection | 5607.67 | 3620.33 | 1.549x | +54.89% | - | - | - | 0.519x | +48.08% | 1.012x | -1.22% | 1.055x | -5.46% | 0 |

- Aggregate proxysss ops/s: `121600.66`
- Aggregate nginx ops/s: `93024.67`
- Aggregate proxysss/nginx ratio: `1.307x`
- Aggregate throughput improvement: `+30.72%`
