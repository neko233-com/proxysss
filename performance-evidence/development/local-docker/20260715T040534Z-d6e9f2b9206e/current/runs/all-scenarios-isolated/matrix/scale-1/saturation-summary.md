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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32405.00 | 28149.33 | 1.151x | +15.12% | - | - | - | 0.759x | +24.11% | 1.058x | -5.77% | 1.083x | -8.28% | 0 |
| game-long-connection | 4245.00 | 3587.67 | 1.183x | +18.32% | - | - | - | 0.634x | +36.55% | 1.203x | -20.26% | 1.106x | -10.57% | 0 |
| generic-sse | 731.33 | 450.67 | 1.623x | +62.28% | - | - | - | 0.548x | +45.21% | 0.819x | +18.06% | 0.881x | +11.88% | 0 |
| https-static-small | 6155.33 | 5505.33 | 1.118x | +11.81% | - | - | - | 0.699x | +30.09% | 1.263x | -26.30% | 0.944x | +5.55% | 0 |
| qcp-transparent | 5686.67 | 3590.33 | 1.584x | +58.39% | - | - | - | 0.504x | +49.65% | 0.878x | +12.19% | 0.825x | +17.51% | 0 |
| reverse-proxy | 14955.67 | 11254.00 | 1.329x | +32.89% | - | - | - | 0.749x | +25.11% | 0.838x | +16.24% | 0.802x | +19.76% | 0 |
| static-large | 89.00 | 95.00 | 0.937x | -6.32% | - | - | - | 0.783x | +21.68% | 0.885x | +11.47% | 1.519x | -51.91% | 0 |
| static-small | 33624.67 | 28161.33 | 1.194x | +19.40% | - | - | - | 0.725x | +27.51% | 1.074x | -7.43% | 1.020x | -2.00% | 0 |
| tcp-stream | 4436.33 | 3598.33 | 1.233x | +23.29% | - | - | - | 0.587x | +41.30% | 1.207x | -20.70% | 1.088x | -8.77% | 0 |
| udp-stream | 5876.00 | 3586.00 | 1.639x | +63.86% | - | - | - | 0.492x | +50.81% | 0.885x | +11.51% | 0.818x | +18.19% | 0 |
| websocket-long-connection | 4018.67 | 3441.67 | 1.168x | +16.77% | - | - | - | 0.660x | +34.05% | 1.158x | -15.84% | 1.023x | -2.31% | 0 |

- Aggregate proxysss ops/s: `112223.67`
- Aggregate nginx ops/s: `91419.66`
- Aggregate proxysss/nginx ratio: `1.228x`
- Aggregate throughput improvement: `+22.76%`
