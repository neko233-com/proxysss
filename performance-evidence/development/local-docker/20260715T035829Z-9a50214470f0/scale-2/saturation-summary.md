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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 35173.67 | 26291.00 | 1.338x | +33.79% | - | - | - | 0.673x | +32.67% | 0.969x | +3.08% | 0.931x | +6.95% | 0 |
| game-long-connection | 5959.67 | 3774.67 | 1.579x | +57.89% | - | - | - | 0.521x | +47.89% | 0.900x | +9.95% | 0.884x | +11.61% | 0 |
| generic-sse | 909.33 | 484.33 | 1.878x | +87.75% | - | - | - | 0.470x | +53.02% | 0.775x | +22.54% | 0.767x | +23.26% | 0 |
| https-static-small | 5337.00 | 5434.67 | 0.982x | -1.80% | - | - | - | 0.911x | +8.90% | 1.361x | -36.14% | 1.042x | -4.16% | 0 |
| qcp-transparent | 6213.00 | 3669.67 | 1.693x | +69.31% | - | - | - | 0.518x | +48.16% | 0.860x | +14.04% | 0.775x | +22.51% | 0 |
| reverse-proxy | 15876.00 | 12440.33 | 1.276x | +27.62% | - | - | - | 0.784x | +21.58% | 0.952x | +4.84% | 0.837x | +16.29% | 0 |
| static-large | 92.67 | 96.33 | 0.962x | -3.80% | - | - | - | 1.185x | -18.45% | 1.110x | -11.00% | 0.762x | +23.81% | 0 |
| static-small | 34019.33 | 25782.33 | 1.319x | +31.95% | - | - | - | 0.699x | +30.08% | 0.938x | +6.23% | 0.802x | +19.76% | 0 |
| tcp-stream | 6043.00 | 3617.00 | 1.671x | +67.07% | - | - | - | 0.497x | +50.28% | 0.902x | +9.84% | 0.904x | +9.59% | 0 |
| udp-stream | 7313.67 | 3684.67 | 1.985x | +98.49% | - | - | - | 0.415x | +58.54% | 0.766x | +23.37% | 0.794x | +20.60% | 0 |
| websocket-long-connection | 5671.00 | 3644.00 | 1.556x | +55.63% | - | - | - | 0.553x | +44.73% | 0.938x | +6.22% | 0.883x | +11.70% | 0 |

- Aggregate proxysss ops/s: `122608.34`
- Aggregate nginx ops/s: `88919.00`
- Aggregate proxysss/nginx ratio: `1.379x`
- Aggregate throughput improvement: `+37.89%`
