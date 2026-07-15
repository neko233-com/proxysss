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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 21922.00 | 22725.00 | 0.965x | -3.53% | - | - | - | 0.856x | +14.38% | 1.398x | -39.76% | 1.407x | -40.74% | 0 |
| game-long-connection | 7605.00 | 4793.50 | 1.587x | +58.65% | - | - | - | 0.458x | +54.24% | 0.952x | +4.79% | 1.067x | -6.72% | 0 |
| generic-sse | 1090.50 | 576.00 | 1.893x | +89.32% | - | - | - | 0.423x | +57.67% | 0.853x | +14.73% | 0.806x | +19.41% | 0 |
| https-static-small | 7112.50 | 4374.00 | 1.626x | +62.61% | - | - | - | 0.553x | +44.66% | 0.940x | +5.99% | 0.759x | +24.14% | 0 |
| qcp-transparent | 8404.00 | 6342.00 | 1.325x | +32.51% | - | - | - | 0.702x | +29.82% | 0.853x | +14.73% | 0.999x | +0.14% | 0 |
| reverse-proxy | 11761.50 | 10895.00 | 1.080x | +7.95% | - | - | - | 0.897x | +10.30% | 1.294x | -29.36% | 0.762x | +23.84% | 0 |
| static-large | 79.50 | 96.50 | 0.824x | -17.62% | - | - | - | 1.085x | -8.49% | 2.304x | -130.35% | 1.215x | -21.53% | 0 |
| static-small | 19650.50 | 21459.00 | 0.916x | -8.43% | - | - | - | 0.969x | +3.13% | 1.471x | -47.05% | 1.622x | -62.24% | 0 |
| tcp-stream | 7162.50 | 4375.00 | 1.637x | +63.71% | - | - | - | 0.444x | +55.61% | 0.922x | +7.78% | 1.029x | -2.86% | 0 |
| udp-stream | 9627.50 | 4967.50 | 1.938x | +93.81% | - | - | - | 0.344x | +65.57% | 0.784x | +21.61% | 0.859x | +14.11% | 0 |
| websocket-long-connection | 6240.50 | 4017.50 | 1.553x | +55.33% | - | - | - | 0.534x | +46.59% | 0.964x | +3.58% | 0.854x | +14.55% | 0 |

- Aggregate proxysss ops/s: `100656.00`
- Aggregate nginx ops/s: `84621.00`
- Aggregate proxysss/nginx ratio: `1.189x`
- Aggregate throughput improvement: `+18.95%`
