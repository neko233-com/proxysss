# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041851Z-502a5c14abe0/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 33183.33 | 28988.00 | 1.145x | +14.47% | - | - | - | 0.616x | +38.40% | 1.614x | -61.39% | 1.811x | -81.15% | 0 |
| game-long-connection | 3248.33 | 3740.67 | 0.868x | -13.16% | - | - | - | 0.972x | +2.85% | 1.747x | -74.69% | 1.761x | -76.08% | 0 |
| generic-sse | 569.00 | 453.67 | 1.254x | +25.42% | - | - | - | 0.610x | +39.01% | 1.496x | -49.58% | 1.569x | -56.93% | 0 |
| https-static-small | 4625.67 | 6625.00 | 0.698x | -30.18% | - | - | - | 0.902x | +9.80% | 2.720x | -172.02% | 2.251x | -125.13% | 0 |
| qcp-transparent | 4087.67 | 3667.67 | 1.115x | +11.45% | - | - | - | 0.605x | +39.53% | 1.630x | -62.99% | 1.693x | -69.30% | 0 |
| reverse-proxy | 13191.67 | 12727.67 | 1.036x | +3.65% | - | - | - | 0.784x | +21.65% | 1.605x | -60.49% | 1.611x | -61.12% | 0 |
| static-large | 99.33 | 96.00 | 1.035x | +3.47% | - | - | - | 0.951x | +4.93% | 1.042x | -4.24% | 1.217x | -21.70% | 0 |
| static-small | 32779.00 | 27824.33 | 1.178x | +17.81% | - | - | - | 0.594x | +40.61% | 1.618x | -61.80% | 1.800x | -80.05% | 0 |
| tcp-stream | 3177.33 | 3606.33 | 0.881x | -11.90% | - | - | - | 0.938x | +6.19% | 1.760x | -75.95% | 1.798x | -79.80% | 0 |
| udp-stream | 4801.33 | 3678.33 | 1.305x | +30.53% | - | - | - | 0.490x | +51.03% | 1.479x | -47.88% | 1.573x | -57.26% | 0 |
| websocket-long-connection | 2973.33 | 3497.00 | 0.850x | -14.97% | - | - | - | 0.994x | +0.64% | 1.853x | -85.34% | 1.773x | -77.31% | 0 |

- Aggregate proxysss ops/s: `102735.99`
- Aggregate nginx ops/s: `94904.67`
- Aggregate proxysss/nginx ratio: `1.083x`
- Aggregate throughput improvement: `+8.25%`
