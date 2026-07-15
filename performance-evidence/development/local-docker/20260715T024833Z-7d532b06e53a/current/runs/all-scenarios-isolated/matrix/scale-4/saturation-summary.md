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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 19342.50 | 22588.50 | 0.856x | -14.37% | - | - | - | 1.082x | -8.18% | 1.493x | -49.35% | 1.617x | -61.68% | 0 |
| game-long-connection | 7264.00 | 4290.50 | 1.693x | +69.30% | - | - | - | 0.453x | +54.68% | 0.870x | +12.98% | 0.951x | +4.94% | 0 |
| generic-sse | 1002.00 | 638.50 | 1.569x | +56.93% | - | - | - | 0.463x | +53.72% | 0.934x | +6.61% | 1.149x | -14.91% | 0 |
| https-static-small | 6571.00 | 4576.00 | 1.436x | +43.60% | - | - | - | 0.580x | +42.04% | 1.244x | -24.45% | 0.898x | +10.22% | 0 |
| qcp-transparent | 8325.50 | 4505.50 | 1.848x | +84.79% | - | - | - | 0.375x | +62.50% | 0.832x | +16.82% | 0.989x | +1.13% | 0 |
| reverse-proxy | 11784.00 | 11612.50 | 1.015x | +1.48% | - | - | - | 0.883x | +11.72% | 1.222x | -22.25% | 1.457x | -45.72% | 0 |
| static-large | 87.50 | 87.00 | 1.006x | +0.57% | - | - | - | 0.947x | +5.31% | 0.845x | +15.54% | 2.194x | -119.39% | 0 |
| static-small | 22041.50 | 21712.00 | 1.015x | +1.52% | - | - | - | 0.903x | +9.66% | 1.159x | -15.91% | 1.262x | -26.23% | 0 |
| tcp-stream | 7354.50 | 6427.50 | 1.144x | +14.42% | - | - | - | 0.798x | +20.16% | 0.939x | +6.09% | 1.066x | -6.59% | 0 |
| udp-stream | 9133.50 | 4246.50 | 2.151x | +115.08% | - | - | - | 0.270x | +73.01% | 0.819x | +18.09% | 0.914x | +8.62% | 0 |
| websocket-long-connection | 6491.00 | 3994.50 | 1.625x | +62.50% | - | - | - | 0.483x | +51.70% | 0.833x | +16.70% | 0.982x | +1.80% | 0 |

- Aggregate proxysss ops/s: `99397.00`
- Aggregate nginx ops/s: `84679.00`
- Aggregate proxysss/nginx ratio: `1.174x`
- Aggregate throughput improvement: `+17.38%`
