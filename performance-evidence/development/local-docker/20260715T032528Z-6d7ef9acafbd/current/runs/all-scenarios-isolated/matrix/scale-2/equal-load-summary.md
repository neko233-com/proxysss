# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6525.50 | 6526.00 | 1.000x | -0.01% | 6528.00 | 1.000x | 1.000x | 0.790x | +20.97% | 1.033x | -3.31% | 1.322x | -32.19% | 0 |
| game-long-connection | 1032.00 | 1032.00 | 1.000x | +0.00% | 1032.00 | 1.000x | 1.000x | 1.463x | -46.30% | 1.015x | -1.52% | 1.489x | -48.94% | 0 |
| generic-sse | 128.50 | 128.50 | 1.000x | +0.00% | 128.00 | 1.004x | 1.004x | 0.950x | +4.99% | 1.783x | -78.31% | 1.526x | -52.59% | 0 |
| https-static-small | 1313.00 | 1312.50 | 1.000x | +0.04% | 1312.00 | 1.001x | 1.000x | 0.972x | +2.80% | 0.923x | +7.66% | 0.814x | +18.64% | 0 |
| qcp-transparent | 1024.00 | 1024.00 | 1.000x | +0.00% | 1024.00 | 1.000x | 1.000x | 0.907x | +9.34% | 1.517x | -51.70% | 0.863x | +13.71% | 0 |
| reverse-proxy | 2959.00 | 2958.50 | 1.000x | +0.02% | 2944.00 | 1.005x | 1.005x | 0.926x | +7.44% | 1.312x | -31.17% | 1.498x | -49.75% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 20.00 | 1.125x | 1.125x | 1.000x | -0.03% | 0.973x | +2.66% | 1.153x | -15.29% | 0 |
| static-small | 6935.00 | 6934.00 | 1.000x | +0.01% | 6912.00 | 1.003x | 1.003x | 0.857x | +14.29% | 1.289x | -28.91% | 1.407x | -40.69% | 0 |
| tcp-stream | 1016.00 | 1016.00 | 1.000x | +0.00% | 1024.00 | 0.992x | 0.992x | 1.523x | -52.33% | 1.696x | -69.57% | 0.802x | +19.76% | 0 |
| udp-stream | 1024.00 | 1024.00 | 1.000x | +0.00% | 1024.00 | 1.000x | 1.000x | 0.806x | +19.41% | 1.087x | -8.68% | 1.014x | -1.36% | 0 |
| websocket-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 912.00 | 1.000x | 1.000x | 1.254x | -25.45% | 1.863x | -86.32% | 0.659x | +34.10% | 0 |

- Aggregate proxysss ops/s: `22891.50`
- Aggregate nginx ops/s: `22890.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
