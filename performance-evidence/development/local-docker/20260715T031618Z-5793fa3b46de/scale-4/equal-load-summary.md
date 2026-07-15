# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6212.50 | 6211.50 | 1.000x | +0.02% | 6208.00 | 1.001x | 1.001x | 0.782x | +21.79% | 0.398x | +60.17% | 0.552x | +44.84% | 0 |
| game-long-connection | 976.00 | 976.00 | 1.000x | +0.00% | 976.00 | 1.000x | 1.000x | 1.344x | -34.39% | 5.071x | -407.06% | 1.132x | -13.16% | 0 |
| generic-sse | 139.00 | 139.00 | 1.000x | +0.00% | 136.00 | 1.022x | 1.022x | 0.976x | +2.39% | 0.540x | +46.04% | 0.883x | +11.73% | 0 |
| https-static-small | 1344.50 | 1345.50 | 0.999x | -0.07% | 1344.00 | 1.000x | 1.001x | 0.992x | +0.80% | 0.748x | +25.16% | 1.944x | -94.37% | 0 |
| qcp-transparent | 976.00 | 976.00 | 1.000x | +0.00% | 976.00 | 1.000x | 1.000x | 0.709x | +29.09% | 1.330x | -32.97% | 0.396x | +60.44% | 0 |
| reverse-proxy | 2910.50 | 2909.00 | 1.001x | +0.05% | 2880.00 | 1.011x | 1.010x | 1.035x | -3.55% | 1.202x | -20.23% | 2.372x | -137.18% | 0 |
| static-large | 20.50 | 20.50 | 1.000x | +0.00% | 16.00 | 1.281x | 1.281x | 0.974x | +2.58% | 0.984x | +1.60% | 0.704x | +29.60% | 0 |
| static-small | 5760.00 | 5758.50 | 1.000x | +0.03% | 5760.00 | 1.000x | 1.000x | 0.809x | +19.10% | 0.731x | +26.88% | 1.167x | -16.72% | 0 |
| tcp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 1.262x | -26.19% | 1.234x | -23.43% | 1.846x | -84.56% | 0 |
| udp-stream | 928.00 | 928.00 | 1.000x | +0.00% | 928.00 | 1.000x | 1.000x | 0.785x | +21.53% | 0.783x | +21.66% | 0.800x | +20.00% | 0 |
| websocket-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.097x | -9.73% | 1.031x | -3.13% | 0.553x | +44.69% | 0 |

- Aggregate proxysss ops/s: `21123.00`
- Aggregate nginx ops/s: `21120.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
