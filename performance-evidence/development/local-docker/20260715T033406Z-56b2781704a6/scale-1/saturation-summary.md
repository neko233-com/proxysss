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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 36070.00 | 27513.50 | 1.311x | +31.10% | - | - | - | 0.685x | +31.54% | 0.936x | +6.41% | 1.006x | -0.60% | 0 |
| game-long-connection | 1718.50 | 3684.50 | 0.466x | -53.36% | - | - | - | 2.313x | -131.26% | 2.173x | -117.28% | 1.683x | -68.29% | 0 |
| generic-sse | 848.50 | 429.50 | 1.976x | +97.56% | - | - | - | 0.429x | +57.12% | 0.736x | +26.38% | 0.692x | +30.78% | 0 |
| https-static-small | 9194.00 | 5196.00 | 1.769x | +76.94% | - | - | - | 0.443x | +55.69% | 0.829x | +17.07% | 0.607x | +39.26% | 0 |
| qcp-transparent | 6308.00 | 3620.00 | 1.743x | +74.25% | - | - | - | 0.443x | +55.72% | 0.794x | +20.64% | 0.800x | +19.98% | 0 |
| reverse-proxy | 15098.00 | 11522.00 | 1.310x | +31.04% | - | - | - | 0.739x | +26.07% | 0.917x | +8.30% | 0.840x | +15.98% | 0 |
| static-large | 89.50 | 91.50 | 0.978x | -2.19% | - | - | - | 0.983x | +1.71% | 1.047x | -4.67% | 1.250x | -25.04% | 0 |
| static-small | 37090.50 | 28462.50 | 1.303x | +30.31% | - | - | - | 0.676x | +32.37% | 0.894x | +10.60% | 1.121x | -12.05% | 0 |
| tcp-stream | 1754.00 | 3610.50 | 0.486x | -51.42% | - | - | - | 2.176x | -117.64% | 2.147x | -114.71% | 1.653x | -65.26% | 0 |
| udp-stream | 6391.00 | 3634.00 | 1.759x | +75.87% | - | - | - | 0.424x | +57.65% | 0.827x | +17.27% | 0.874x | +12.63% | 0 |
| websocket-long-connection | 1685.50 | 3368.50 | 0.500x | -49.96% | - | - | - | 2.238x | -123.79% | 2.073x | -107.32% | 1.588x | -58.84% | 0 |

- Aggregate proxysss ops/s: `116247.50`
- Aggregate nginx ops/s: `91132.50`
- Aggregate proxysss/nginx ratio: `1.276x`
- Aggregate throughput improvement: `+27.56%`
