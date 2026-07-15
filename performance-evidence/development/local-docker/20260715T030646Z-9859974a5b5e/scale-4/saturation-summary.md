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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29586.00 | 25852.50 | 1.144x | +14.44% | - | - | - | 0.731x | +26.91% | 1.310x | -31.04% | 1.372x | -37.17% | 0 |
| game-long-connection | 8312.00 | 3513.50 | 2.366x | +136.57% | - | - | - | 0.354x | +64.64% | 0.697x | +30.33% | 0.749x | +25.14% | 0 |
| generic-sse | 910.00 | 609.50 | 1.493x | +49.30% | - | - | - | 0.575x | +42.52% | 0.936x | +6.40% | 1.145x | -14.48% | 0 |
| https-static-small | 4786.50 | 5016.00 | 0.954x | -4.58% | - | - | - | 1.087x | -8.66% | 1.709x | -70.92% | 0.761x | +23.85% | 0 |
| qcp-transparent | 6177.00 | 3849.00 | 1.605x | +60.48% | - | - | - | 0.495x | +50.49% | 0.975x | +2.53% | 1.004x | -0.37% | 0 |
| reverse-proxy | 14551.00 | 11905.50 | 1.222x | +22.22% | - | - | - | 0.793x | +20.72% | 1.184x | -18.40% | 1.070x | -7.04% | 0 |
| static-large | 91.50 | 91.50 | 1.000x | +0.00% | - | - | - | 1.029x | -2.94% | 0.860x | +13.97% | 1.442x | -44.18% | 0 |
| static-small | 28678.50 | 25314.50 | 1.133x | +13.29% | - | - | - | 0.770x | +23.04% | 1.342x | -34.20% | 1.347x | -34.74% | 0 |
| tcp-stream | 8500.50 | 3670.00 | 2.316x | +131.62% | - | - | - | 0.341x | +65.87% | 0.718x | +28.23% | 0.736x | +26.37% | 0 |
| udp-stream | 7437.50 | 4812.00 | 1.546x | +54.56% | - | - | - | 0.404x | +59.60% | 0.938x | +6.20% | 1.028x | -2.81% | 0 |
| websocket-long-connection | 9146.50 | 3542.50 | 2.582x | +158.19% | - | - | - | 0.288x | +71.25% | 0.690x | +31.04% | 0.748x | +25.23% | 0 |

- Aggregate proxysss ops/s: `118177.00`
- Aggregate nginx ops/s: `88176.50`
- Aggregate proxysss/nginx ratio: `1.340x`
- Aggregate throughput improvement: `+34.02%`
