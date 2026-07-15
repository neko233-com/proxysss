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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T043557Z-2be6ba4712a1/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 18288.33 | 10964.33 | 1.668x | +66.80% | - | - | - | 0.702x | +29.80% | 0.556x | +44.44% | 0.399x | +60.09% | 0 |
| game-long-connection | 2210.33 | 1520.00 | 1.454x | +45.42% | - | - | - | 0.806x | +19.38% | 0.616x | +38.37% | 0.471x | +52.92% | 0 |
| generic-sse | 356.33 | 234.67 | 1.518x | +51.84% | - | - | - | 0.730x | +26.99% | 0.608x | +39.15% | 0.466x | +53.45% | 0 |
| https-static-small | 2701.67 | 6848.33 | 0.395x | -60.55% | - | - | - | 11.195x | -1019.52% | 1.523x | -52.35% | 1.256x | -25.61% | 0 |
| qcp-transparent | 2516.33 | 1489.33 | 1.690x | +68.96% | - | - | - | 0.686x | +31.35% | 0.469x | +53.11% | 0.339x | +66.08% | 0 |
| reverse-proxy | 7575.00 | 4725.67 | 1.603x | +60.29% | - | - | - | 0.735x | +26.47% | 0.511x | +48.90% | 0.413x | +58.66% | 0 |
| static-large | 71.00 | 55.00 | 1.291x | +29.09% | - | - | - | 1.104x | -10.41% | 0.662x | +33.81% | 0.345x | +65.52% | 0 |
| static-small | 18413.67 | 11359.33 | 1.621x | +62.10% | - | - | - | 0.712x | +28.84% | 0.575x | +42.53% | 0.396x | +60.41% | 0 |
| tcp-stream | 2477.67 | 1550.67 | 1.598x | +59.78% | - | - | - | 0.723x | +27.73% | 0.561x | +43.88% | 0.481x | +51.87% | 0 |
| udp-stream | 2554.67 | 1468.00 | 1.740x | +74.02% | - | - | - | 0.668x | +33.25% | 0.471x | +52.87% | 0.373x | +62.67% | 0 |
| websocket-long-connection | 2216.00 | 1480.33 | 1.497x | +49.70% | - | - | - | 0.768x | +23.17% | 0.586x | +41.38% | 0.437x | +56.35% | 0 |

- Aggregate proxysss ops/s: `59381.00`
- Aggregate nginx ops/s: `41695.66`
- Aggregate proxysss/nginx ratio: `1.424x`
- Aggregate throughput improvement: `+42.42%`
