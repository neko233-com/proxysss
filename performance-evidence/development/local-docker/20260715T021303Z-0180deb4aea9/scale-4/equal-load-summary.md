# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4382.50 | 4381.00 | 1.000x | +0.03% | 4386.57 | 0.999x | 0.999x | 0.872x | +12.79% | 1.321x | -32.08% | 0.951x | +4.89% | 0 |
| game-long-connection | 1184.00 | 1184.00 | 1.000x | +0.00% | 1201.34 | 0.986x | 0.986x | 1.187x | -18.72% | 1.602x | -60.17% | 1.226x | -22.61% | 0 |
| generic-sse | 156.00 | 156.00 | 1.000x | +0.00% | 156.37 | 0.998x | 0.998x | 1.056x | -5.57% | 1.887x | -88.67% | 1.390x | -38.96% | 0 |
| https-static-small | 758.50 | 758.50 | 1.000x | +0.00% | 759.50 | 0.999x | 0.999x | 1.111x | -11.13% | 1.364x | -36.39% | 0.949x | +5.08% | 0 |
| qcp-transparent | 1216.00 | 1232.00 | 0.987x | -1.30% | 1249.37 | 0.973x | 0.986x | 0.893x | +10.67% | 1.873x | -87.26% | 0.932x | +6.77% | 0 |
| reverse-proxy | 2509.50 | 2509.00 | 1.000x | +0.02% | 2512.61 | 0.999x | 0.999x | 1.076x | -7.58% | 3.410x | -241.05% | 1.476x | -47.56% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.12 | 0.995x | 0.995x | 0.988x | +1.20% | 1.221x | -22.06% | 0.758x | +24.21% | 0 |
| static-small | 4449.50 | 4450.00 | 1.000x | -0.01% | 4453.88 | 0.999x | 0.999x | 0.864x | +13.64% | 2.158x | -115.76% | 1.316x | -31.62% | 0 |
| tcp-stream | 1536.00 | 1536.00 | 1.000x | +0.00% | 1554.68 | 0.988x | 0.988x | 1.226x | -22.60% | 1.326x | -32.63% | 1.212x | -21.21% | 0 |
| udp-stream | 1184.00 | 1184.00 | 1.000x | +0.00% | 1202.47 | 0.985x | 0.985x | 0.880x | +11.98% | 1.595x | -59.54% | 1.273x | -27.32% | 0 |
| websocket-long-connection | 1248.00 | 1248.00 | 1.000x | +0.00% | 1252.10 | 0.997x | 0.997x | 0.859x | +14.07% | 2.990x | -198.96% | 1.465x | -46.51% | 0 |

- Aggregate proxysss ops/s: `18646.00`
- Aggregate nginx ops/s: `18660.50`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.08%`
