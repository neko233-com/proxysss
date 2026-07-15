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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30229.00 | 26998.50 | 1.120x | +11.97% | - | - | - | 0.765x | +23.45% | 1.111x | -11.06% | 0.921x | +7.89% | 0 |
| game-long-connection | 6544.00 | 4395.00 | 1.489x | +48.90% | - | - | - | 0.437x | +56.32% | 0.971x | +2.87% | 0.951x | +4.94% | 0 |
| generic-sse | 803.50 | 539.50 | 1.489x | +48.93% | - | - | - | 0.535x | +46.46% | 0.909x | +9.08% | 1.104x | -10.42% | 0 |
| https-static-small | 5365.00 | 4860.00 | 1.104x | +10.39% | - | - | - | 0.914x | +8.60% | 1.153x | -15.34% | 1.034x | -3.44% | 0 |
| qcp-transparent | 7301.00 | 4556.50 | 1.602x | +60.23% | - | - | - | 0.429x | +57.12% | 0.876x | +12.40% | 1.015x | -1.53% | 0 |
| reverse-proxy | 13800.50 | 12584.00 | 1.097x | +9.67% | - | - | - | 0.846x | +15.37% | 1.063x | -6.31% | 1.011x | -1.14% | 0 |
| static-large | 92.00 | 86.00 | 1.070x | +6.98% | - | - | - | 1.131x | -13.13% | 0.998x | +0.19% | 0.233x | +76.66% | 0 |
| static-small | 30488.00 | 25212.00 | 1.209x | +20.93% | - | - | - | 0.665x | +33.47% | 1.104x | -10.35% | 1.042x | -4.19% | 0 |
| tcp-stream | 6603.50 | 4301.50 | 1.535x | +53.52% | - | - | - | 0.415x | +58.50% | 0.947x | +5.30% | 0.830x | +17.02% | 0 |
| udp-stream | 6812.50 | 4334.50 | 1.572x | +57.17% | - | - | - | 0.525x | +47.49% | 0.793x | +20.67% | 0.686x | +31.44% | 0 |
| websocket-long-connection | 6064.00 | 4092.50 | 1.482x | +48.17% | - | - | - | 0.484x | +51.63% | 0.851x | +14.94% | 0.926x | +7.37% | 0 |

- Aggregate proxysss ops/s: `114103.00`
- Aggregate nginx ops/s: `91960.00`
- Aggregate proxysss/nginx ratio: `1.241x`
- Aggregate throughput improvement: `+24.08%`
