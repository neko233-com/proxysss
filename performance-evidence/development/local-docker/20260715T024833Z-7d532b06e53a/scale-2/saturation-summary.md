# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30614.50 | 23181.00 | 1.321x | +32.07% | - | - | - | 0.622x | +37.82% | 1.124x | -12.38% | 0.947x | +5.27% | 0 |
| game-long-connection | 7247.00 | 5623.00 | 1.289x | +28.88% | - | - | - | 0.468x | +53.20% | 1.024x | -2.37% | 0.906x | +9.40% | 0 |
| generic-sse | 931.00 | 486.00 | 1.916x | +91.56% | - | - | - | 0.444x | +55.63% | 0.846x | +15.43% | 0.784x | +21.56% | 0 |
| https-static-small | 5595.00 | 5097.00 | 1.098x | +9.77% | - | - | - | 0.735x | +26.46% | 1.539x | -53.90% | 1.027x | -2.70% | 0 |
| qcp-transparent | 7495.00 | 4237.00 | 1.769x | +76.89% | - | - | - | 0.377x | +62.34% | 0.898x | +10.17% | 0.897x | +10.25% | 0 |
| reverse-proxy | 14390.50 | 11916.50 | 1.208x | +20.76% | - | - | - | 0.784x | +21.62% | 1.198x | -19.76% | 0.962x | +3.75% | 0 |
| static-large | 88.00 | 92.00 | 0.957x | -4.35% | - | - | - | 1.110x | -11.00% | 1.142x | -14.19% | 1.002x | -0.23% | 0 |
| static-small | 28249.00 | 23612.50 | 1.196x | +19.64% | - | - | - | 0.656x | +34.39% | 1.339x | -33.90% | 0.977x | +2.34% | 0 |
| tcp-stream | 6345.50 | 4263.00 | 1.489x | +48.85% | - | - | - | 0.429x | +57.09% | 1.063x | -6.27% | 0.911x | +8.94% | 0 |
| udp-stream | 7563.50 | 4399.50 | 1.719x | +71.92% | - | - | - | 0.394x | +60.65% | 0.913x | +8.69% | 0.925x | +7.54% | 0 |
| websocket-long-connection | 6123.00 | 4238.00 | 1.445x | +44.48% | - | - | - | 0.424x | +57.58% | 1.074x | -7.44% | 0.994x | +0.56% | 0 |

- Aggregate proxysss ops/s: `114642.00`
- Aggregate nginx ops/s: `87145.50`
- Aggregate proxysss/nginx ratio: `1.316x`
- Aggregate throughput improvement: `+31.55%`
