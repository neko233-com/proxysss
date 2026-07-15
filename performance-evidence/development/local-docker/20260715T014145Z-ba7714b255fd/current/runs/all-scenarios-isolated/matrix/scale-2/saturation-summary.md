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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 13515.00 | 13164.00 | 1.027x | +2.67% | - | - | - | 0.762x | +23.82% | 1.423x | -42.34% | 1.357x | -35.68% | 0 |
| game-long-connection | 4201.00 | 2568.00 | 1.636x | +63.59% | - | - | - | 0.531x | +46.87% | 0.828x | +17.24% | 0.871x | +12.90% | 0 |
| generic-sse | 380.00 | 297.00 | 1.279x | +27.95% | - | - | - | 0.719x | +28.15% | 1.370x | -36.97% | 0.261x | +73.88% | 0 |
| https-static-small | 3227.00 | 1677.00 | 1.924x | +92.43% | - | - | - | 0.694x | +30.63% | 0.618x | +38.22% | 0.656x | +34.42% | 0 |
| qcp-transparent | 2818.00 | 2938.00 | 0.959x | -4.08% | - | - | - | 0.843x | +15.73% | 1.387x | -38.75% | 1.528x | -52.82% | 0 |
| reverse-proxy | 5782.00 | 7605.00 | 0.760x | -23.97% | - | - | - | 1.238x | -23.77% | 1.501x | -50.06% | 1.774x | -77.43% | 0 |
| static-large | 80.00 | 85.00 | 0.941x | -5.88% | - | - | - | 0.837x | +16.27% | 1.645x | -64.51% | 1.710x | -71.03% | 0 |
| static-small | 14491.00 | 14890.00 | 0.973x | -2.68% | - | - | - | 0.847x | +15.33% | 1.296x | -29.62% | 1.163x | -16.29% | 0 |
| tcp-stream | 3627.00 | 3120.00 | 1.163x | +16.25% | - | - | - | 0.866x | +13.36% | 0.770x | +23.04% | 1.031x | -3.11% | 0 |
| udp-stream | 2912.00 | 2864.00 | 1.017x | +1.68% | - | - | - | 0.821x | +17.91% | 1.353x | -35.34% | 1.264x | -26.37% | 0 |
| websocket-long-connection | 3299.00 | 2608.00 | 1.265x | +26.50% | - | - | - | 0.834x | +16.55% | 0.685x | +31.49% | 0.899x | +10.06% | 0 |

- Aggregate proxysss ops/s: `54332.00`
- Aggregate nginx ops/s: `51816.00`
- Aggregate proxysss/nginx ratio: `1.049x`
- Aggregate throughput improvement: `+4.86%`
