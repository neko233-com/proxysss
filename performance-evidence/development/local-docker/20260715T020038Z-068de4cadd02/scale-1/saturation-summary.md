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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 24326.00 | 27207.00 | 0.894x | -10.59% | - | - | - | 0.933x | +6.68% | 1.359x | -35.90% | 1.128x | -12.77% | 0 |
| game-long-connection | 5900.00 | 3794.00 | 1.555x | +55.51% | - | - | - | 0.577x | +42.31% | 0.709x | +29.15% | 0.806x | +19.45% | 0 |
| generic-sse | 518.00 | 457.00 | 1.133x | +13.35% | - | - | - | 0.808x | +19.19% | 1.202x | -20.17% | 1.084x | -8.37% | 0 |
| https-static-small | 3120.00 | 3353.00 | 0.931x | -6.95% | - | - | - | 1.198x | -19.80% | 1.153x | -15.25% | 1.369x | -36.94% | 0 |
| qcp-transparent | 3688.00 | 3757.00 | 0.982x | -1.84% | - | - | - | 0.846x | +15.43% | 1.142x | -14.19% | 1.211x | -21.11% | 0 |
| reverse-proxy | 9016.00 | 8480.00 | 1.063x | +6.32% | - | - | - | 0.819x | +18.08% | 1.142x | -14.19% | 1.131x | -13.10% | 0 |
| static-large | 111.00 | 102.00 | 1.088x | +8.82% | - | - | - | 0.905x | +9.52% | 1.077x | -7.73% | 0.828x | +17.17% | 0 |
| static-small | 20965.00 | 24598.00 | 0.852x | -14.77% | - | - | - | 0.934x | +6.63% | 1.214x | -21.43% | 1.148x | -14.83% | 0 |
| tcp-stream | 5631.00 | 4080.00 | 1.380x | +38.01% | - | - | - | 0.706x | +29.41% | 0.827x | +17.32% | 0.833x | +16.70% | 0 |
| udp-stream | 3710.00 | 3521.00 | 1.054x | +5.37% | - | - | - | 0.783x | +21.65% | 1.099x | -9.91% | 1.333x | -33.34% | 0 |
| websocket-long-connection | 4965.00 | 4218.00 | 1.177x | +17.71% | - | - | - | 0.948x | +5.25% | 0.769x | +23.13% | 0.889x | +11.15% | 0 |

- Aggregate proxysss ops/s: `81950.00`
- Aggregate nginx ops/s: `83567.00`
- Aggregate proxysss/nginx ratio: `0.981x`
- Aggregate throughput improvement: `-1.93%`
