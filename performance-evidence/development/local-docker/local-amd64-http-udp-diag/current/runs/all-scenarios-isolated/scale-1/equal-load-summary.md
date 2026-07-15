# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-udp-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.963 < 0.980 (actual=22989.00 target=23880.60); generic-sse nginx target achievement 0.977 < 0.980 (actual=443.00 target=453.41); reverse-proxy nginx target achievement 0.966 < 0.980 (actual=11733.00 target=12148.82); static-small nginx target achievement 0.964 < 0.980 (actual=23267.25 target=24132.73)`
| cdn-hot-update | 18619.25 | 22989.00 | 0.810x | -19.01% | 23880.60 | 0.780x | 0.963x | 1.327x | -32.71% | 3.410x | -241.01% | 4.507x | -350.67% | 0 |
| generic-sse | 384.25 | 443.00 | 0.867x | -13.26% | 453.41 | 0.847x | 0.977x | 1.840x | -83.99% | 3.191x | -219.15% | 3.629x | -262.86% | 1 |
| qcp-transparent | 1586.50 | 1774.75 | 0.894x | -10.61% | 1780.55 | 0.891x | 0.997x | 3.017x | -201.73% | 4.456x | -345.57% | 4.310x | -330.97% | 0 |
| reverse-proxy | 9878.50 | 11733.00 | 0.842x | -15.81% | 12148.82 | 0.813x | 0.966x | 1.811x | -81.10% | 3.496x | -249.58% | 4.549x | -354.93% | 0 |
| static-small | 19242.25 | 23267.25 | 0.827x | -17.30% | 24132.73 | 0.797x | 0.964x | 1.265x | -26.49% | 3.303x | -230.29% | 4.277x | -327.70% | 0 |
| udp-stream | 1588.75 | 1767.00 | 0.899x | -10.09% | 1772.26 | 0.896x | 0.997x | 3.128x | -212.84% | 4.669x | -366.93% | 3.914x | -291.44% | 0 |

- Aggregate proxysss ops/s: `51299.50`
- Aggregate nginx ops/s: `61974.00`
- Aggregate proxysss/nginx ratio: `0.828x`
- Aggregate throughput improvement: `-17.22%`
