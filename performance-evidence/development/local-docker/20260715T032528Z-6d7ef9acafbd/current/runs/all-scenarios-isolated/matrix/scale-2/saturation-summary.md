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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 34582.50 | 26124.00 | 1.324x | +32.38% | - | - | - | 0.695x | +30.54% | 0.857x | +14.32% | 1.014x | -1.41% | 0 |
| game-long-connection | 5564.00 | 4151.00 | 1.340x | +34.04% | - | - | - | 0.562x | +43.77% | 1.185x | -18.49% | 1.357x | -35.75% | 0 |
| generic-sse | 977.50 | 514.50 | 1.900x | +89.99% | - | - | - | 0.459x | +54.07% | 0.758x | +24.24% | 0.850x | +15.05% | 0 |
| https-static-small | 5832.00 | 5256.00 | 1.110x | +10.96% | - | - | - | 0.821x | +17.89% | 1.202x | -20.22% | 0.929x | +7.05% | 0 |
| qcp-transparent | 6838.00 | 4106.50 | 1.665x | +66.52% | - | - | - | 0.501x | +49.87% | 0.851x | +14.87% | 0.947x | +5.31% | 0 |
| reverse-proxy | 15859.00 | 11843.00 | 1.339x | +33.91% | - | - | - | 0.766x | +23.35% | 0.784x | +21.58% | 0.755x | +24.48% | 0 |
| static-large | 91.50 | 99.00 | 0.924x | -7.58% | - | - | - | 1.069x | -6.89% | 1.081x | -8.10% | 1.202x | -20.18% | 0 |
| static-small | 34003.50 | 27748.00 | 1.225x | +22.54% | - | - | - | 0.709x | +29.13% | 1.127x | -12.66% | 1.155x | -15.46% | 0 |
| tcp-stream | 5956.00 | 4096.00 | 1.454x | +45.41% | - | - | - | 0.515x | +48.50% | 1.073x | -7.30% | 1.292x | -29.16% | 0 |
| udp-stream | 7430.50 | 4114.00 | 1.806x | +80.61% | - | - | - | 0.454x | +54.64% | 0.753x | +24.65% | 0.803x | +19.73% | 0 |
| websocket-long-connection | 5632.00 | 3679.50 | 1.531x | +53.06% | - | - | - | 0.480x | +51.99% | 1.153x | -15.32% | 1.262x | -26.25% | 0 |

- Aggregate proxysss ops/s: `122766.50`
- Aggregate nginx ops/s: `91731.50`
- Aggregate proxysss/nginx ratio: `1.338x`
- Aggregate throughput improvement: `+33.83%`
