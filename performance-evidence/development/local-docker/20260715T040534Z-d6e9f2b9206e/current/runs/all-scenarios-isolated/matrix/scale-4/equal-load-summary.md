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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6264.33 | 6265.00 | 1.000x | -0.01% | 6229.33 | 1.006x | 1.006x | 0.862x | +13.79% | 1.218x | -21.83% | 1.101x | -10.10% | 0 |
| game-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 864.00 | 1.000x | 1.000x | 1.282x | -28.21% | 1.274x | -27.42% | 2.168x | -116.77% | 0 |
| generic-sse | 139.00 | 139.00 | 1.000x | +0.00% | 138.67 | 1.002x | 1.002x | 0.984x | +1.64% | 1.032x | -3.23% | 0.972x | +2.75% | 0 |
| https-static-small | 1416.33 | 1416.67 | 1.000x | -0.02% | 1408.00 | 1.006x | 1.006x | 1.004x | -0.40% | 1.202x | -20.16% | 1.230x | -22.96% | 0 |
| qcp-transparent | 885.33 | 885.33 | 1.000x | +0.00% | 885.33 | 1.000x | 1.000x | 0.828x | +17.16% | 1.025x | -2.55% | 2.911x | -191.09% | 0 |
| reverse-proxy | 3173.33 | 3173.67 | 1.000x | -0.01% | 3157.33 | 1.005x | 1.005x | 1.020x | -1.96% | 1.292x | -29.15% | 0.825x | +17.50% | 0 |
| static-large | 22.67 | 22.67 | 1.000x | +0.00% | 21.33 | 1.063x | 1.063x | 1.026x | -2.65% | 1.104x | -10.40% | 1.868x | -86.79% | 0 |
| static-small | 6437.00 | 6439.00 | 1.000x | -0.03% | 6400.00 | 1.006x | 1.006x | 0.872x | +12.79% | 1.341x | -34.15% | 1.831x | -83.14% | 0 |
| tcp-stream | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 1.421x | -42.09% | 1.598x | -59.77% | 1.917x | -91.68% | 0 |
| udp-stream | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.857x | +14.25% | 0.861x | +13.91% | 2.629x | -162.91% | 0 |
| websocket-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 864.00 | 1.000x | 1.000x | 1.272x | -27.15% | 1.530x | -52.98% | 1.833x | -83.28% | 0 |

- Aggregate proxysss ops/s: `21836.66`
- Aggregate nginx ops/s: `21840.01`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
