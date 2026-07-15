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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6480.67 | 6479.67 | 1.000x | +0.02% | 6464.00 | 1.003x | 1.002x | 0.771x | +22.86% | 2.139x | -113.95% | 1.312x | -31.17% | 0 |
| game-long-connection | 928.00 | 928.00 | 1.000x | +0.00% | 928.00 | 1.000x | 1.000x | 0.976x | +2.37% | 1.019x | -1.91% | 3.566x | -256.65% | 0 |
| generic-sse | 122.00 | 122.00 | 1.000x | +0.00% | 121.33 | 1.005x | 1.005x | 0.932x | +6.79% | 1.766x | -76.64% | 1.378x | -37.77% | 0 |
| https-static-small | 1482.67 | 1482.67 | 1.000x | +0.00% | 1482.67 | 1.000x | 1.000x | 0.967x | +3.29% | 1.117x | -11.70% | 2.443x | -144.28% | 0 |
| qcp-transparent | 933.33 | 933.33 | 1.000x | +0.00% | 933.33 | 1.000x | 1.000x | 0.923x | +7.74% | 1.747x | -74.72% | 3.970x | -296.96% | 0 |
| reverse-proxy | 3188.00 | 3187.33 | 1.000x | +0.02% | 3178.67 | 1.003x | 1.003x | 1.054x | -5.37% | 2.578x | -157.82% | 1.226x | -22.57% | 0 |
| static-large | 23.67 | 23.67 | 1.000x | +0.00% | 21.33 | 1.110x | 1.110x | 0.893x | +10.67% | 0.950x | +4.97% | 1.918x | -91.78% | 0 |
| static-small | 6608.67 | 6607.33 | 1.000x | +0.02% | 6592.00 | 1.003x | 1.002x | 0.808x | +19.23% | 2.306x | -130.56% | 1.140x | -14.04% | 0 |
| tcp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.951x | +4.92% | 1.047x | -4.69% | 1.956x | -95.65% | 0 |
| udp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.892x | +10.84% | 1.433x | -43.28% | 1.312x | -31.25% | 0 |
| websocket-long-connection | 933.33 | 933.33 | 1.000x | +0.00% | 933.33 | 1.000x | 1.000x | 0.944x | +5.59% | 1.034x | -3.45% | 1.646x | -64.61% | 0 |

- Aggregate proxysss ops/s: `22620.34`
- Aggregate nginx ops/s: `22617.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
