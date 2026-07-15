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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6150.00 | 6149.50 | 1.000x | +0.01% | 6144.00 | 1.001x | 1.001x | 0.832x | +16.76% | 0.346x | +65.38% | 1.074x | -7.36% | 0 |
| game-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.392x | -39.24% | 0.496x | +50.43% | 0.078x | +92.21% | 0 |
| generic-sse | 140.00 | 140.00 | 1.000x | +0.00% | 140.00 | 1.000x | 1.000x | 0.985x | +1.51% | 0.496x | +50.36% | 0.332x | +66.78% | 0 |
| https-static-small | 1394.00 | 1393.50 | 1.000x | +0.04% | 1392.00 | 1.001x | 1.001x | 0.945x | +5.47% | 1.236x | -23.61% | 0.576x | +42.39% | 0 |
| qcp-transparent | 896.00 | 896.00 | 1.000x | +0.00% | 912.00 | 0.982x | 0.982x | 0.847x | +15.26% | 0.331x | +66.85% | 0.220x | +78.00% | 0 |
| reverse-proxy | 3110.50 | 3111.00 | 1.000x | -0.02% | 3072.00 | 1.013x | 1.013x | 0.956x | +4.43% | 0.602x | +39.84% | 1.301x | -30.12% | 0 |
| static-large | 23.00 | 23.00 | 1.000x | +0.00% | 16.00 | 1.438x | 1.438x | 1.002x | -0.21% | 0.424x | +57.63% | 0.544x | +45.63% | 0 |
| static-small | 6352.50 | 6350.50 | 1.000x | +0.03% | 6336.00 | 1.003x | 1.002x | 0.768x | +23.16% | 0.311x | +68.94% | 1.299x | -29.94% | 0 |
| tcp-stream | 944.00 | 944.00 | 1.000x | +0.00% | 944.00 | 1.000x | 1.000x | 1.461x | -46.05% | 0.346x | +65.36% | 0.207x | +79.31% | 0 |
| udp-stream | 944.00 | 944.00 | 1.000x | +0.00% | 944.00 | 1.000x | 1.000x | 0.843x | +15.73% | 0.642x | +35.76% | 0.272x | +72.79% | 0 |
| websocket-long-connection | 880.00 | 880.00 | 1.000x | +0.00% | 880.00 | 1.000x | 1.000x | 1.223x | -22.33% | 0.507x | +49.26% | 0.108x | +89.20% | 0 |

- Aggregate proxysss ops/s: `21730.00`
- Aggregate nginx ops/s: `21727.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
