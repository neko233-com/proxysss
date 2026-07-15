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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T033406Z-56b2781704a6/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6236.50 | 6235.00 | 1.000x | +0.02% | 6208.00 | 1.005x | 1.004x | 0.793x | +20.65% | 1.404x | -40.42% | 1.066x | -6.64% | 0 |
| game-long-connection | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 1.552x | -55.24% | 2.680x | -167.98% | 1.569x | -56.94% | 0 |
| generic-sse | 141.50 | 141.50 | 1.000x | +0.00% | 140.00 | 1.011x | 1.011x | 0.985x | +1.51% | 1.929x | -92.86% | 1.986x | -98.59% | 0 |
| https-static-small | 1301.50 | 1301.00 | 1.000x | +0.04% | 1296.00 | 1.004x | 1.004x | 0.964x | +3.60% | 1.249x | -24.90% | 1.408x | -40.76% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.839x | +16.07% | 2.075x | -107.51% | 0.692x | +30.83% | 0 |
| reverse-proxy | 3108.50 | 3108.50 | 1.000x | +0.00% | 3072.00 | 1.012x | 1.012x | 1.017x | -1.66% | 1.246x | -24.59% | 0.521x | +47.90% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 16.00 | 1.406x | 1.406x | 1.006x | -0.63% | 1.042x | -4.25% | 1.017x | -1.69% | 0 |
| static-small | 6216.00 | 6214.50 | 1.000x | +0.02% | 6208.00 | 1.001x | 1.001x | 0.797x | +20.33% | 1.872x | -87.22% | 1.078x | -7.75% | 0 |
| tcp-stream | 1040.00 | 1040.00 | 1.000x | +0.00% | 1040.00 | 1.000x | 1.000x | 1.710x | -70.96% | 1.954x | -95.38% | 1.053x | -5.29% | 0 |
| udp-stream | 928.00 | 928.00 | 1.000x | +0.00% | 944.00 | 0.983x | 0.983x | 0.733x | +26.68% | 0.963x | +3.67% | 1.888x | -88.81% | 0 |
| websocket-long-connection | 928.00 | 928.00 | 1.000x | +0.00% | 928.00 | 1.000x | 1.000x | 1.381x | -38.10% | 1.401x | -40.14% | 1.897x | -89.74% | 0 |

- Aggregate proxysss ops/s: `21842.50`
- Aggregate nginx ops/s: `21839.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
