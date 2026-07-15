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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32083.33 | 25068.00 | 1.280x | +27.99% | - | - | - | 0.649x | +35.11% | 1.271x | -27.09% | 1.534x | -53.43% | 0 |
| game-long-connection | 7451.33 | 3824.33 | 1.948x | +94.84% | - | - | - | 0.424x | +57.61% | 0.763x | +23.67% | 0.873x | +12.68% | 0 |
| generic-sse | 876.33 | 550.67 | 1.591x | +59.14% | - | - | - | 0.531x | +46.90% | 0.956x | +4.40% | 1.193x | -19.28% | 0 |
| https-static-small | 6837.67 | 5698.67 | 1.200x | +19.99% | - | - | - | 0.722x | +27.78% | 1.261x | -26.14% | 0.979x | +2.09% | 0 |
| qcp-transparent | 6215.33 | 3634.00 | 1.710x | +71.03% | - | - | - | 0.448x | +55.22% | 0.901x | +9.86% | 1.108x | -10.75% | 0 |
| reverse-proxy | 13457.00 | 12180.33 | 1.105x | +10.48% | - | - | - | 0.856x | +14.43% | 1.201x | -20.08% | 1.654x | -65.37% | 0 |
| static-large | 89.00 | 99.67 | 0.893x | -10.71% | - | - | - | 1.374x | -37.42% | 1.540x | -54.03% | 0.228x | +77.24% | 0 |
| static-small | 29711.67 | 24758.00 | 1.200x | +20.01% | - | - | - | 0.706x | +29.42% | 1.265x | -26.50% | 1.629x | -62.87% | 0 |
| tcp-stream | 7433.00 | 3698.67 | 2.010x | +100.96% | - | - | - | 0.418x | +58.25% | 0.751x | +24.92% | 0.822x | +17.84% | 0 |
| udp-stream | 5931.33 | 3637.33 | 1.631x | +63.07% | - | - | - | 0.483x | +51.66% | 1.011x | -1.14% | 1.530x | -52.95% | 0 |
| websocket-long-connection | 7041.00 | 3615.00 | 1.948x | +94.77% | - | - | - | 0.436x | +56.41% | 0.822x | +17.77% | 0.861x | +13.89% | 0 |

- Aggregate proxysss ops/s: `117126.99`
- Aggregate nginx ops/s: `86764.67`
- Aggregate proxysss/nginx ratio: `1.350x`
- Aggregate throughput improvement: `+34.99%`
