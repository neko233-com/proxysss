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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29538.50 | 24959.50 | 1.183x | +18.35% | - | - | - | 0.742x | +25.78% | 1.254x | -25.35% | 1.393x | -39.33% | 0 |
| game-long-connection | 4502.00 | 3884.00 | 1.159x | +15.91% | - | - | - | 0.800x | +20.00% | 1.015x | -1.54% | 1.101x | -10.13% | 0 |
| generic-sse | 1199.00 | 566.50 | 2.117x | +111.65% | - | - | - | 0.403x | +59.71% | 0.679x | +32.08% | 1.161x | -16.09% | 0 |
| https-static-small | 8576.00 | 5208.00 | 1.647x | +64.67% | - | - | - | 0.548x | +45.20% | 1.069x | -6.91% | 0.562x | +43.77% | 0 |
| qcp-transparent | 8495.50 | 3896.00 | 2.181x | +118.06% | - | - | - | 0.344x | +65.55% | 0.720x | +28.03% | 0.944x | +5.56% | 0 |
| reverse-proxy | 15279.50 | 12440.00 | 1.228x | +22.83% | - | - | - | 0.749x | +25.12% | 1.178x | -17.75% | 1.596x | -59.62% | 0 |
| static-large | 91.00 | 100.50 | 0.905x | -9.45% | - | - | - | 0.990x | +1.03% | 1.486x | -48.61% | 1.778x | -77.84% | 0 |
| static-small | 35669.00 | 24884.00 | 1.433x | +43.34% | - | - | - | 0.596x | +40.43% | 0.967x | +3.33% | 1.215x | -21.52% | 0 |
| tcp-stream | 4475.50 | 4173.00 | 1.072x | +7.25% | - | - | - | 0.828x | +17.23% | 0.966x | +3.42% | 0.901x | +9.87% | 0 |
| udp-stream | 8736.00 | 3776.50 | 2.313x | +131.33% | - | - | - | 0.343x | +65.75% | 0.662x | +33.84% | 0.812x | +18.85% | 0 |
| websocket-long-connection | 4064.50 | 3723.50 | 1.092x | +9.16% | - | - | - | 0.851x | +14.87% | 0.991x | +0.89% | 0.963x | +3.74% | 0 |

- Aggregate proxysss ops/s: `120626.50`
- Aggregate nginx ops/s: `87611.50`
- Aggregate proxysss/nginx ratio: `1.377x`
- Aggregate throughput improvement: `+37.68%`
