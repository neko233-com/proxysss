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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5076.50 | 5077.00 | 1.000x | -0.01% | 5079.97 | 0.999x | 0.999x | 0.789x | +21.05% | 0.212x | +78.82% | 0.693x | +30.70% | 0 |
| game-long-connection | 944.00 | 944.00 | 1.000x | +0.00% | 954.11 | 0.989x | 0.989x | 1.003x | -0.26% | 0.200x | +79.99% | 0.216x | +78.38% | 0 |
| generic-sse | 130.50 | 130.50 | 1.000x | +0.00% | 130.87 | 0.997x | 0.997x | 0.952x | +4.77% | 0.258x | +74.23% | 0.396x | +60.37% | 0 |
| https-static-small | 1121.00 | 1121.00 | 1.000x | +0.00% | 1122.10 | 0.999x | 0.999x | 1.008x | -0.76% | 0.383x | +61.75% | 0.544x | +45.62% | 0 |
| qcp-transparent | 800.00 | 800.00 | 1.000x | +0.00% | 808.61 | 0.989x | 0.989x | 0.780x | +21.96% | 0.971x | +2.88% | 0.290x | +70.99% | 0 |
| reverse-proxy | 2494.00 | 2495.00 | 1.000x | -0.04% | 2496.10 | 0.999x | 1.000x | 0.936x | +6.40% | 0.175x | +82.46% | 0.284x | +71.58% | 0 |
| static-large | 30.50 | 30.50 | 1.000x | +0.00% | 30.87 | 0.988x | 0.988x | 0.989x | +1.12% | 0.440x | +55.96% | 0.247x | +75.27% | 0 |
| static-small | 5064.00 | 5064.00 | 1.000x | +0.00% | 5066.30 | 1.000x | 1.000x | 0.773x | +22.65% | 0.387x | +61.26% | 0.976x | +2.39% | 0 |
| tcp-stream | 1136.00 | 1136.00 | 1.000x | +0.00% | 1149.47 | 0.988x | 0.988x | 1.058x | -5.78% | 0.390x | +60.95% | 0.345x | +65.53% | 0 |
| udp-stream | 880.00 | 880.00 | 1.000x | +0.00% | 882.98 | 0.997x | 0.997x | 0.774x | +22.57% | 0.113x | +88.68% | 0.107x | +89.29% | 0 |
| websocket-long-connection | 928.00 | 928.00 | 1.000x | +0.00% | 933.49 | 0.994x | 0.994x | 0.878x | +12.23% | 0.265x | +73.49% | 0.617x | +38.28% | 0 |

- Aggregate proxysss ops/s: `18604.50`
- Aggregate nginx ops/s: `18606.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
