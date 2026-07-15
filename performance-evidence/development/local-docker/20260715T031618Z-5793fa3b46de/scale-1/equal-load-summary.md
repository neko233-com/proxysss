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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6563.50 | 6561.00 | 1.000x | +0.04% | 6560.00 | 1.001x | 1.000x | 0.811x | +18.86% | 0.809x | +19.11% | 0.760x | +23.98% | 0 |
| game-long-connection | 956.00 | 956.00 | 1.000x | +0.00% | 956.00 | 1.000x | 1.000x | 0.976x | +2.42% | 1.344x | -34.43% | 1.588x | -58.80% | 0 |
| generic-sse | 109.50 | 109.50 | 1.000x | +0.00% | 109.00 | 1.005x | 1.005x | 0.985x | +1.49% | 0.758x | +24.21% | 0.680x | +32.05% | 0 |
| https-static-small | 1324.00 | 1324.00 | 1.000x | +0.00% | 1324.00 | 1.000x | 1.000x | 0.972x | +2.82% | 1.496x | -49.56% | 2.012x | -101.21% | 0 |
| qcp-transparent | 972.00 | 972.00 | 1.000x | +0.00% | 972.00 | 1.000x | 1.000x | 0.932x | +6.80% | 1.052x | -5.20% | 0.859x | +14.08% | 0 |
| reverse-proxy | 3117.00 | 3118.00 | 1.000x | -0.03% | 3104.00 | 1.004x | 1.005x | 0.948x | +5.23% | 0.796x | +20.38% | 1.489x | -48.91% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 20.00 | 1.050x | 1.050x | 0.989x | +1.06% | 1.329x | -32.94% | 0.789x | +21.06% | 0 |
| static-small | 6564.50 | 6567.00 | 1.000x | -0.04% | 6560.00 | 1.001x | 1.001x | 0.843x | +15.73% | 0.940x | +5.98% | 0.931x | +6.89% | 0 |
| tcp-stream | 948.00 | 948.00 | 1.000x | +0.00% | 952.00 | 0.996x | 0.996x | 1.033x | -3.25% | 1.509x | -50.86% | 1.571x | -57.13% | 0 |
| udp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.939x | +6.07% | 1.185x | -18.53% | 1.024x | -2.36% | 0 |
| websocket-long-connection | 900.00 | 900.00 | 1.000x | +0.00% | 900.00 | 1.000x | 1.000x | 1.072x | -7.17% | 0.846x | +15.40% | 0.662x | +33.84% | 0 |

- Aggregate proxysss ops/s: `22435.50`
- Aggregate nginx ops/s: `22436.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.00%`
