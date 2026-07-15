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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6460.00 | 6462.00 | 1.000x | -0.03% | 6400.00 | 1.009x | 1.010x | 0.799x | +20.11% | 0.739x | +26.14% | 0.624x | +37.59% | 0 |
| game-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 864.00 | 1.000x | 1.000x | 1.089x | -8.87% | 0.982x | +1.83% | 1.236x | -23.58% | 0 |
| generic-sse | 152.00 | 152.00 | 1.000x | +0.00% | 152.00 | 1.000x | 1.000x | 0.980x | +1.99% | 1.263x | -26.33% | 0.853x | +14.68% | 0 |
| https-static-small | 1195.50 | 1195.50 | 1.000x | +0.00% | 1184.00 | 1.010x | 1.010x | 1.000x | +0.00% | 1.219x | -21.86% | 0.568x | +43.21% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.716x | +28.36% | 2.017x | -101.73% | 2.852x | -185.17% | 0 |
| reverse-proxy | 2975.50 | 2973.50 | 1.001x | +0.07% | 2944.00 | 1.011x | 1.010x | 0.972x | +2.83% | 0.911x | +8.87% | 1.102x | -10.22% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 16.00 | 1.406x | 1.406x | 1.004x | -0.41% | 1.347x | -34.70% | 0.914x | +8.57% | 0 |
| static-small | 6324.50 | 6324.00 | 1.000x | +0.01% | 6272.00 | 1.008x | 1.008x | 0.849x | +15.14% | 1.183x | -18.29% | 1.181x | -18.07% | 0 |
| tcp-stream | 912.00 | 912.00 | 1.000x | +0.00% | 912.00 | 1.000x | 1.000x | 1.327x | -32.71% | 0.488x | +51.19% | 0.603x | +39.71% | 0 |
| udp-stream | 1200.00 | 1200.00 | 1.000x | +0.00% | 1200.00 | 1.000x | 1.000x | 0.834x | +16.55% | 2.417x | -141.65% | 0.854x | +14.58% | 0 |
| websocket-long-connection | 880.00 | 880.00 | 1.000x | +0.00% | 880.00 | 1.000x | 1.000x | 0.985x | +1.47% | 1.411x | -41.11% | 2.394x | -139.43% | 0 |

- Aggregate proxysss ops/s: `21946.00`
- Aggregate nginx ops/s: `21945.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
