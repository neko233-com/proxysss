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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 35863.67 | 25944.00 | 1.382x | +38.23% | - | - | - | 0.653x | +34.65% | 0.943x | +5.68% | 0.884x | +11.56% | 0 |
| game-long-connection | 5965.33 | 3802.67 | 1.569x | +56.87% | - | - | - | 0.537x | +46.29% | 0.910x | +9.00% | 0.836x | +16.37% | 0 |
| generic-sse | 943.00 | 479.00 | 1.969x | +96.87% | - | - | - | 0.450x | +55.04% | 0.727x | +27.31% | 0.738x | +26.16% | 0 |
| https-static-small | 7867.33 | 6429.33 | 1.224x | +22.37% | - | - | - | 0.667x | +33.25% | 1.178x | -17.83% | 0.831x | +16.87% | 0 |
| qcp-transparent | 6559.00 | 3745.33 | 1.751x | +75.12% | - | - | - | 0.470x | +52.98% | 0.814x | +18.63% | 0.775x | +22.52% | 0 |
| reverse-proxy | 14756.67 | 12752.67 | 1.157x | +15.71% | - | - | - | 0.867x | +13.28% | 1.035x | -3.47% | 0.908x | +9.23% | 0 |
| static-large | 89.00 | 95.33 | 0.934x | -6.64% | - | - | - | 1.132x | -13.16% | 1.236x | -23.56% | 1.195x | -19.49% | 0 |
| static-small | 34248.67 | 26262.00 | 1.304x | +30.41% | - | - | - | 0.665x | +33.52% | 1.050x | -5.04% | 0.980x | +2.04% | 0 |
| tcp-stream | 6005.67 | 3853.00 | 1.559x | +55.87% | - | - | - | 0.532x | +46.80% | 0.990x | +0.99% | 0.844x | +15.63% | 0 |
| udp-stream | 6516.67 | 3771.00 | 1.728x | +72.81% | - | - | - | 0.500x | +50.02% | 0.849x | +15.07% | 0.828x | +17.21% | 0 |
| websocket-long-connection | 5651.00 | 3600.67 | 1.569x | +56.94% | - | - | - | 0.555x | +44.46% | 0.881x | +11.90% | 0.813x | +18.70% | 0 |

- Aggregate proxysss ops/s: `124466.01`
- Aggregate nginx ops/s: `90735.00`
- Aggregate proxysss/nginx ratio: `1.372x`
- Aggregate throughput improvement: `+37.18%`
