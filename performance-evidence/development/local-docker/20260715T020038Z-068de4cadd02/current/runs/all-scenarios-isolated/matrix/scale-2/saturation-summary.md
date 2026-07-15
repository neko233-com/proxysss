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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22688.00 | 25398.00 | 0.893x | -10.67% | - | - | - | 0.850x | +15.05% | 1.663x | -66.32% | 1.827x | -82.71% | 0 |
| game-long-connection | 7715.00 | 3803.00 | 2.029x | +102.87% | - | - | - | 0.395x | +60.48% | 0.666x | +33.40% | 0.611x | +38.93% | 0 |
| generic-sse | 687.00 | 429.00 | 1.601x | +60.14% | - | - | - | 0.569x | +43.13% | 1.023x | -2.31% | 1.114x | -11.36% | 0 |
| https-static-small | 3511.00 | 3416.00 | 1.028x | +2.78% | - | - | - | 0.666x | +33.41% | 0.807x | +19.31% | 0.811x | +18.86% | 0 |
| qcp-transparent | 5851.00 | 4144.00 | 1.412x | +41.19% | - | - | - | 0.429x | +57.10% | 1.103x | -10.35% | 1.190x | -19.03% | 0 |
| reverse-proxy | 7583.00 | 10063.00 | 0.754x | -24.64% | - | - | - | 1.306x | -30.58% | 1.520x | -51.96% | 2.380x | -138.02% | 0 |
| static-large | 125.00 | 125.00 | 1.000x | +0.00% | - | - | - | 1.032x | -3.23% | 0.818x | +18.18% | 0.494x | +50.60% | 0 |
| static-small | 18677.00 | 22953.00 | 0.814x | -18.63% | - | - | - | 0.811x | +18.91% | 1.646x | -64.62% | 1.664x | -66.45% | 0 |
| tcp-stream | 6769.00 | 4459.00 | 1.518x | +51.81% | - | - | - | 0.564x | +43.60% | 0.708x | +29.22% | 0.730x | +27.00% | 0 |
| udp-stream | 5859.00 | 3921.00 | 1.494x | +49.43% | - | - | - | 0.364x | +63.63% | 1.058x | -5.78% | 1.130x | -13.04% | 0 |
| websocket-long-connection | 7234.00 | 5191.00 | 1.394x | +39.36% | - | - | - | 0.775x | +22.54% | 0.845x | +15.46% | 0.756x | +24.44% | 0 |

- Aggregate proxysss ops/s: `86699.00`
- Aggregate nginx ops/s: `83902.00`
- Aggregate proxysss/nginx ratio: `1.033x`
- Aggregate throughput improvement: `+3.33%`
