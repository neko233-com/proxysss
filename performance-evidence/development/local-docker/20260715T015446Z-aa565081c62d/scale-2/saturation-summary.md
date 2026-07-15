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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 24587.00 | 22699.50 | 1.083x | +8.32% | - | - | - | 0.816x | +18.44% | 1.185x | -18.52% | 1.031x | -3.14% | 0 |
| game-long-connection | 4919.00 | 3671.00 | 1.340x | +34.00% | - | - | - | 0.731x | +26.89% | 0.778x | +22.22% | 0.675x | +32.54% | 0 |
| generic-sse | 573.00 | 458.50 | 1.250x | +24.97% | - | - | - | 0.667x | +33.28% | 1.207x | -20.75% | 1.391x | -39.05% | 0 |
| https-static-small | 5151.50 | 4524.50 | 1.139x | +13.86% | - | - | - | 0.858x | +14.20% | 0.936x | +6.39% | 0.989x | +1.10% | 0 |
| qcp-transparent | 4493.00 | 3651.00 | 1.231x | +23.06% | - | - | - | 0.712x | +28.75% | 0.922x | +7.84% | 0.858x | +14.22% | 0 |
| reverse-proxy | 10432.50 | 8278.00 | 1.260x | +26.03% | - | - | - | 0.769x | +23.10% | 0.957x | +4.31% | 1.118x | -11.80% | 0 |
| static-large | 119.50 | 122.00 | 0.980x | -2.05% | - | - | - | 1.017x | -1.68% | 0.993x | +0.69% | 1.056x | -5.56% | 0 |
| static-small | 24535.50 | 22380.00 | 1.096x | +9.63% | - | - | - | 0.858x | +14.18% | 1.137x | -13.67% | 0.752x | +24.76% | 0 |
| tcp-stream | 5322.00 | 3710.50 | 1.434x | +43.43% | - | - | - | 0.668x | +33.16% | 0.797x | +20.32% | 0.688x | +31.17% | 0 |
| udp-stream | 4308.00 | 3552.50 | 1.213x | +21.27% | - | - | - | 0.676x | +32.38% | 1.028x | -2.81% | 0.921x | +7.91% | 0 |
| websocket-long-connection | 4417.50 | 3144.00 | 1.405x | +40.51% | - | - | - | 0.687x | +31.31% | 0.887x | +11.30% | 0.817x | +18.31% | 0 |

- Aggregate proxysss ops/s: `88858.50`
- Aggregate nginx ops/s: `76191.50`
- Aggregate proxysss/nginx ratio: `1.166x`
- Aggregate throughput improvement: `+16.63%`
