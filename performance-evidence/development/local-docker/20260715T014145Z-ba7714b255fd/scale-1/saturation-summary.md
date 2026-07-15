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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15865.00 | 14518.00 | 1.093x | +9.28% | - | - | - | 0.913x | +8.74% | 0.911x | +8.92% | 0.950x | +4.98% | 0 |
| game-long-connection | 3370.00 | 2832.00 | 1.190x | +19.00% | - | - | - | 0.787x | +21.29% | 0.855x | +14.47% | 0.777x | +22.27% | 0 |
| generic-sse | 335.00 | 302.00 | 1.109x | +10.93% | - | - | - | 0.856x | +14.43% | 1.031x | -3.05% | 1.183x | -18.35% | 0 |
| https-static-small | 1808.00 | 2134.00 | 0.847x | -15.28% | - | - | - | 0.911x | +8.89% | 0.922x | +7.84% | 0.811x | +18.92% | 0 |
| qcp-transparent | 2791.00 | 2271.00 | 1.229x | +22.90% | - | - | - | 0.759x | +24.11% | 0.903x | +9.71% | 0.752x | +24.84% | 0 |
| reverse-proxy | 6167.00 | 5531.00 | 1.115x | +11.50% | - | - | - | 0.939x | +6.14% | 1.103x | -10.28% | 0.691x | +30.94% | 0 |
| static-large | 81.00 | 73.00 | 1.110x | +10.96% | - | - | - | 0.851x | +14.90% | 1.182x | -18.24% | 0.938x | +6.19% | 0 |
| static-small | 15330.00 | 12275.00 | 1.249x | +24.89% | - | - | - | 0.755x | +24.46% | 0.840x | +15.98% | 0.867x | +13.33% | 0 |
| tcp-stream | 2873.00 | 2674.00 | 1.074x | +7.44% | - | - | - | 0.909x | +9.11% | 0.963x | +3.68% | 1.051x | -5.15% | 0 |
| udp-stream | 2680.00 | 2305.00 | 1.163x | +16.27% | - | - | - | 0.833x | +16.71% | 0.980x | +1.98% | 0.898x | +10.23% | 0 |
| websocket-long-connection | 2592.00 | 2143.00 | 1.210x | +20.95% | - | - | - | 0.821x | +17.93% | 0.892x | +10.78% | 0.825x | +17.47% | 0 |

- Aggregate proxysss ops/s: `53892.00`
- Aggregate nginx ops/s: `47058.00`
- Aggregate proxysss/nginx ratio: `1.145x`
- Aggregate throughput improvement: `+14.52%`
