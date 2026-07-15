# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6443.50 | 6435.00 | 1.001x | +0.13% | 6446.41 | 1.000x | 0.998x | 0.863x | +13.74% | 1.217x | -21.69% | 2.033x | -103.34% | 0 |
| game-long-connection | 1008.00 | 1008.00 | 1.000x | +0.00% | 1011.95 | 0.996x | 0.996x | 0.955x | +4.53% | 1.769x | -76.89% | 1.058x | -5.83% | 0 |
| generic-sse | 135.00 | 135.00 | 1.000x | +0.00% | 135.62 | 0.995x | 0.995x | 1.025x | -2.50% | 1.580x | -58.00% | 0.832x | +16.82% | 0 |
| https-static-small | 1206.00 | 1205.50 | 1.000x | +0.04% | 1207.46 | 0.999x | 0.998x | 0.964x | +3.58% | 1.365x | -36.46% | 1.061x | -6.14% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 974.72 | 0.985x | 0.985x | 0.969x | +3.05% | 1.163x | -16.27% | 2.325x | -132.52% | 0 |
| reverse-proxy | 2933.50 | 2934.00 | 1.000x | -0.02% | 2937.13 | 0.999x | 0.999x | 1.036x | -3.59% | 1.367x | -36.74% | 0.869x | +13.05% | 0 |
| static-large | 20.00 | 20.00 | 1.000x | +0.00% | 20.25 | 0.988x | 0.988x | 1.000x | +0.04% | 1.029x | -2.86% | 1.076x | -7.64% | 0 |
| static-small | 6157.00 | 6155.50 | 1.000x | +0.02% | 6166.30 | 0.998x | 0.998x | 0.831x | +16.94% | 0.998x | +0.18% | 0.951x | +4.91% | 0 |
| tcp-stream | 1056.00 | 1056.00 | 1.000x | +0.00% | 1057.99 | 0.998x | 0.998x | 0.892x | +10.84% | 2.709x | -170.90% | 3.678x | -267.78% | 0 |
| udp-stream | 1248.00 | 1248.00 | 1.000x | +0.00% | 1256.87 | 0.993x | 0.993x | 0.854x | +14.58% | 1.699x | -69.89% | 2.599x | -159.88% | 0 |
| websocket-long-connection | 960.00 | 960.00 | 1.000x | +0.00% | 968.70 | 0.991x | 0.991x | 0.889x | +11.14% | 2.146x | -114.64% | 1.130x | -13.03% | 0 |

- Aggregate proxysss ops/s: `22127.00`
- Aggregate nginx ops/s: `22117.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.05%`
