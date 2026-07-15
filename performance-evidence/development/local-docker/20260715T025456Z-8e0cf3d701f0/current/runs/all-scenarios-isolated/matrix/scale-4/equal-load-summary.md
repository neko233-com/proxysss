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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `qcp-transparent nginx target achievement 0.973 < 0.980 (actual=1088.00 target=1118.22); static-large nginx target achievement 0.970 < 0.980 (actual=20.00 target=20.62); udp-stream nginx target achievement 0.974 < 0.980 (actual=1120.00 target=1150.09)`
| cdn-hot-update | 5580.00 | 5581.50 | 1.000x | -0.03% | 5587.57 | 0.999x | 0.999x | 0.854x | +14.59% | 1.952x | -95.15% | 1.209x | -20.90% | 0 |
| game-long-connection | 1088.00 | 1088.00 | 1.000x | +0.00% | 1094.62 | 0.994x | 0.994x | 1.282x | -28.21% | 4.182x | -318.25% | 3.356x | -235.63% | 0 |
| generic-sse | 141.00 | 141.00 | 1.000x | +0.00% | 141.37 | 0.997x | 0.997x | 1.054x | -5.38% | 2.637x | -163.75% | 1.364x | -36.37% | 0 |
| https-static-small | 1320.00 | 1319.00 | 1.001x | +0.08% | 1321.99 | 0.998x | 0.998x | 0.995x | +0.51% | 1.594x | -59.37% | 1.073x | -7.32% | 0 |
| qcp-transparent | 1088.00 | 1088.00 | 1.000x | +0.00% | 1118.22 | 0.973x | 0.973x | 0.988x | +1.21% | 3.292x | -229.15% | 3.410x | -240.97% | 0 |
| reverse-proxy | 2867.50 | 2866.00 | 1.001x | +0.05% | 2871.37 | 0.999x | 0.998x | 1.013x | -1.29% | 2.340x | -133.97% | 1.336x | -33.57% | 0 |
| static-large | 20.00 | 20.00 | 1.000x | +0.00% | 20.62 | 0.970x | 0.970x | 1.050x | -5.01% | 1.201x | -20.05% | 1.128x | -12.80% | 0 |
| static-small | 5622.50 | 5621.50 | 1.000x | +0.02% | 5630.58 | 0.999x | 0.998x | 0.834x | +16.62% | 3.148x | -214.82% | 0.947x | +5.30% | 0 |
| tcp-stream | 1120.00 | 1120.00 | 1.000x | +0.00% | 1131.22 | 0.990x | 0.990x | 1.098x | -9.76% | 1.566x | -56.60% | 2.321x | -132.11% | 0 |
| udp-stream | 1120.00 | 1120.00 | 1.000x | +0.00% | 1150.09 | 0.974x | 0.974x | 0.842x | +15.80% | 2.453x | -145.26% | 2.799x | -179.89% | 0 |
| websocket-long-connection | 1024.00 | 1024.00 | 1.000x | +0.00% | 1043.74 | 0.981x | 0.981x | 0.913x | +8.65% | 0.862x | +13.79% | 1.502x | -50.20% | 0 |

- Aggregate proxysss ops/s: `20991.00`
- Aggregate nginx ops/s: `20989.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
