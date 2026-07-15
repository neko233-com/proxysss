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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 36899.50 | 27428.00 | 1.345x | +34.53% | - | - | - | 0.657x | +34.34% | 0.918x | +8.18% | 1.045x | -4.54% | 0 |
| game-long-connection | 3201.50 | 3887.50 | 0.824x | -17.65% | - | - | - | 1.222x | -22.16% | 1.379x | -37.88% | 1.201x | -20.06% | 0 |
| generic-sse | 1165.00 | 508.00 | 2.293x | +129.33% | - | - | - | 0.380x | +62.03% | 0.592x | +40.78% | 0.671x | +32.93% | 0 |
| https-static-small | 9251.00 | 5916.00 | 1.564x | +56.37% | - | - | - | 0.499x | +50.13% | 1.109x | -10.93% | 0.763x | +23.72% | 0 |
| qcp-transparent | 8294.50 | 4089.00 | 2.028x | +102.85% | - | - | - | 0.355x | +64.50% | 0.786x | +21.43% | 0.900x | +10.00% | 0 |
| reverse-proxy | 14578.50 | 12689.50 | 1.149x | +14.89% | - | - | - | 0.862x | +13.80% | 1.092x | -9.19% | 1.014x | -1.38% | 0 |
| static-large | 91.50 | 98.00 | 0.934x | -6.63% | - | - | - | 1.305x | -30.51% | 1.052x | -5.21% | 0.206x | +79.42% | 0 |
| static-small | 40055.50 | 25647.50 | 1.562x | +56.18% | - | - | - | 0.588x | +41.20% | 0.732x | +26.82% | 1.062x | -6.19% | 0 |
| tcp-stream | 3175.50 | 3927.50 | 0.809x | -19.15% | - | - | - | 1.227x | -22.66% | 1.300x | -30.03% | 1.050x | -4.96% | 0 |
| udp-stream | 8442.00 | 4190.50 | 2.015x | +101.46% | - | - | - | 0.356x | +64.36% | 0.763x | +23.71% | 0.880x | +12.05% | 0 |
| websocket-long-connection | 2872.50 | 3886.00 | 0.739x | -26.08% | - | - | - | 1.289x | -28.89% | 1.389x | -38.94% | 1.299x | -29.88% | 0 |

- Aggregate proxysss ops/s: `128027.00`
- Aggregate nginx ops/s: `92267.50`
- Aggregate proxysss/nginx ratio: `1.388x`
- Aggregate throughput improvement: `+38.76%`
