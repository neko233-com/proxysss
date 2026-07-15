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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29545.00 | 24619.00 | 1.200x | +20.01% | - | - | - | 0.733x | +26.70% | 1.254x | -25.42% | 1.632x | -63.17% | 0 |
| game-long-connection | 6730.50 | 3603.50 | 1.868x | +86.78% | - | - | - | 0.472x | +52.84% | 0.884x | +11.64% | 0.946x | +5.40% | 0 |
| generic-sse | 1103.00 | 562.00 | 1.963x | +96.26% | - | - | - | 0.468x | +53.17% | 0.743x | +25.72% | 0.863x | +13.67% | 0 |
| https-static-small | 6390.00 | 5580.00 | 1.145x | +14.52% | - | - | - | 0.831x | +16.94% | 1.307x | -30.68% | 0.593x | +40.67% | 0 |
| qcp-transparent | 7062.00 | 3649.00 | 1.935x | +93.53% | - | - | - | 0.431x | +56.91% | 0.819x | +18.08% | 1.089x | -8.89% | 0 |
| reverse-proxy | 13940.50 | 12456.00 | 1.119x | +11.92% | - | - | - | 0.807x | +19.28% | 1.341x | -34.07% | 1.701x | -70.06% | 0 |
| static-large | 92.50 | 97.50 | 0.949x | -5.13% | - | - | - | 1.071x | -7.12% | 1.098x | -9.79% | 1.161x | -16.10% | 0 |
| static-small | 32249.50 | 25428.00 | 1.268x | +26.83% | - | - | - | 0.734x | +26.59% | 1.092x | -9.22% | 1.297x | -29.69% | 0 |
| tcp-stream | 6809.50 | 3800.00 | 1.792x | +79.20% | - | - | - | 0.470x | +52.95% | 0.915x | +8.48% | 0.975x | +2.48% | 0 |
| udp-stream | 7603.50 | 3795.50 | 2.003x | +100.33% | - | - | - | 0.412x | +58.83% | 0.761x | +23.92% | 0.987x | +1.27% | 0 |
| websocket-long-connection | 6417.00 | 3565.00 | 1.800x | +80.00% | - | - | - | 0.469x | +53.05% | 0.895x | +10.53% | 1.119x | -11.85% | 0 |

- Aggregate proxysss ops/s: `117943.00`
- Aggregate nginx ops/s: `87155.50`
- Aggregate proxysss/nginx ratio: `1.353x`
- Aggregate throughput improvement: `+35.32%`
