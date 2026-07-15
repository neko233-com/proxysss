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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29689.00 | 26771.50 | 1.109x | +10.90% | - | - | - | 0.725x | +27.51% | 1.133x | -13.27% | 1.117x | -11.66% | 0 |
| game-long-connection | 4456.00 | 3447.00 | 1.293x | +29.27% | - | - | - | 0.600x | +40.05% | 1.122x | -12.18% | 0.944x | +5.56% | 0 |
| generic-sse | 652.50 | 425.00 | 1.535x | +53.53% | - | - | - | 0.550x | +45.04% | 0.909x | +9.12% | 0.847x | +15.27% | 0 |
| https-static-small | 5276.00 | 5349.50 | 0.986x | -1.37% | - | - | - | 0.787x | +21.28% | 1.409x | -40.89% | 0.892x | +10.77% | 0 |
| qcp-transparent | 4920.00 | 3672.50 | 1.340x | +33.97% | - | - | - | 0.556x | +44.38% | 0.937x | +6.26% | 0.816x | +18.41% | 0 |
| reverse-proxy | 13657.00 | 10931.50 | 1.249x | +24.93% | - | - | - | 0.769x | +23.13% | 0.909x | +9.07% | 0.873x | +12.74% | 0 |
| static-large | 85.50 | 88.50 | 0.966x | -3.39% | - | - | - | 1.154x | -15.38% | 1.133x | -13.26% | 1.818x | -81.82% | 0 |
| static-small | 31240.50 | 27104.00 | 1.153x | +15.26% | - | - | - | 0.694x | +30.59% | 1.110x | -10.99% | 1.028x | -2.82% | 0 |
| tcp-stream | 4566.50 | 3376.50 | 1.352x | +35.24% | - | - | - | 0.564x | +43.55% | 1.087x | -8.73% | 0.929x | +7.11% | 0 |
| udp-stream | 4981.50 | 3381.00 | 1.473x | +47.34% | - | - | - | 0.512x | +48.83% | 0.854x | +14.55% | 0.775x | +22.51% | 0 |
| websocket-long-connection | 4159.00 | 3332.50 | 1.248x | +24.80% | - | - | - | 0.610x | +39.01% | 1.143x | -14.34% | 0.868x | +13.24% | 0 |

- Aggregate proxysss ops/s: `103683.50`
- Aggregate nginx ops/s: `87879.50`
- Aggregate proxysss/nginx ratio: `1.180x`
- Aggregate throughput improvement: `+17.98%`
