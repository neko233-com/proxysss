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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.966 < 0.980 (actual=21.00 target=21.75)`
| cdn-hot-update | 4828.00 | 4832.00 | 0.999x | -0.08% | 4835.48 | 0.998x | 0.999x | 0.840x | +16.01% | 0.405x | +59.48% | 0.901x | +9.90% | 0 |
| game-long-connection | 1056.00 | 1056.00 | 1.000x | +0.00% | 1072.60 | 0.985x | 0.985x | 1.402x | -40.17% | 0.996x | +0.45% | 0.431x | +56.91% | 0 |
| generic-sse | 159.00 | 159.00 | 1.000x | +0.00% | 159.62 | 0.996x | 0.996x | 1.010x | -0.98% | 0.507x | +49.34% | 0.743x | +25.70% | 0 |
| https-static-small | 1142.50 | 1143.00 | 1.000x | -0.04% | 1143.96 | 0.999x | 0.999x | 1.032x | -3.23% | 0.833x | +16.67% | 0.996x | +0.38% | 0 |
| qcp-transparent | 1120.00 | 1120.00 | 1.000x | +0.00% | 1126.36 | 0.994x | 0.994x | 0.891x | +10.93% | 0.606x | +39.37% | 0.652x | +34.83% | 0 |
| reverse-proxy | 2899.50 | 2901.50 | 0.999x | -0.07% | 2903.09 | 0.999x | 0.999x | 1.053x | -5.33% | 0.438x | +56.17% | 0.759x | +24.08% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 21.75 | 0.966x | 0.966x | 0.974x | +2.64% | 0.862x | +13.84% | 0.844x | +15.55% | 0 |
| static-small | 5419.00 | 5419.50 | 1.000x | -0.01% | 5427.87 | 0.998x | 0.998x | 0.862x | +13.81% | 0.618x | +38.20% | 0.920x | +7.98% | 0 |
| tcp-stream | 1600.00 | 1600.00 | 1.000x | +0.00% | 1606.83 | 0.996x | 0.996x | 1.230x | -22.95% | 1.010x | -0.96% | 0.573x | +42.71% | 0 |
| udp-stream | 1056.00 | 1056.00 | 1.000x | +0.00% | 1061.61 | 0.995x | 0.995x | 0.983x | +1.73% | 1.206x | -20.65% | 1.879x | -87.88% | 0 |
| websocket-long-connection | 992.00 | 992.00 | 1.000x | +0.00% | 998.60 | 0.993x | 0.993x | 0.965x | +3.49% | 0.625x | +37.49% | 0.838x | +16.21% | 0 |

- Aggregate proxysss ops/s: `20293.00`
- Aggregate nginx ops/s: `20300.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.03%`
