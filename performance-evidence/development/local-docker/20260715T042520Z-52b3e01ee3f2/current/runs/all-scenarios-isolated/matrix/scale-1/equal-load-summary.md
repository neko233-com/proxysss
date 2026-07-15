# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for `game-long-connection, qcp-transparent, tcp-stream, udp-stream, websocket-long-connection`
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=true, strict=true)
- Saturation ops gate: `false`
- Equal-load latency gate: `true`
- Minimum fixed-load completion: `0.980`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T042520Z-52b3e01ee3f2/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4729.67 | 4728.33 | 1.000x | +0.03% | 4725.33 | 1.001x | 1.001x | 0.824x | +17.61% | 1.279x | -27.85% | 1.200x | -19.98% | 0 |
| game-long-connection | 906.67 | 906.67 | 1.000x | +0.00% | 909.33 | 0.997x | 0.997x | 0.947x | +5.33% | 1.060x | -6.02% | 1.586x | -58.56% | 0 |
| generic-sse | 88.00 | 88.00 | 1.000x | +0.00% | 88.00 | 1.000x | 1.000x | 0.999x | +0.15% | 1.003x | -0.34% | 1.140x | -13.97% | 0 |
| https-static-small | 1454.33 | 1454.00 | 1.000x | +0.02% | 1453.33 | 1.001x | 1.000x | 0.954x | +4.56% | 1.032x | -3.18% | 1.974x | -97.43% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.893x | +10.75% | 1.136x | -13.62% | 2.005x | -100.47% | 0 |
| reverse-proxy | 2337.00 | 2337.33 | 1.000x | -0.01% | 2336.00 | 1.000x | 1.001x | 0.941x | +5.94% | 1.401x | -40.12% | 1.701x | -70.06% | 0 |
| static-large | 23.33 | 23.33 | 1.000x | +0.00% | 22.67 | 1.029x | 1.029x | 0.854x | +14.59% | 0.838x | +16.24% | 1.766x | -76.62% | 0 |
| static-small | 4607.67 | 4606.67 | 1.000x | +0.02% | 4608.00 | 1.000x | 1.000x | 0.830x | +16.98% | 1.370x | -36.96% | 1.399x | -39.91% | 0 |
| tcp-stream | 904.00 | 904.00 | 1.000x | +0.00% | 904.00 | 1.000x | 1.000x | 0.950x | +5.00% | 1.309x | -30.90% | 1.949x | -94.91% | 0 |
| udp-stream | 933.33 | 933.33 | 1.000x | +0.00% | 933.33 | 1.000x | 1.000x | 0.904x | +9.57% | 1.240x | -24.01% | 1.847x | -84.70% | 0 |
| websocket-long-connection | 858.67 | 858.67 | 1.000x | +0.00% | 858.67 | 1.000x | 1.000x | 0.949x | +5.11% | 1.336x | -33.62% | 1.654x | -65.40% | 0 |

- Aggregate proxysss ops/s: `17802.67`
- Aggregate nginx ops/s: `17800.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
