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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `qcp-transparent nginx target achievement 0.977 < 0.980 (actual=688.00 target=704.49); static-large nginx target achievement 0.977 < 0.980 (actual=21.50 target=22.00); udp-stream nginx target achievement 0.976 < 0.980 (actual=560.00 target=573.50)`
| cdn-hot-update | 2457.00 | 2456.50 | 1.000x | +0.02% | 2458.37 | 0.999x | 0.999x | 0.903x | +9.66% | 2.985x | -198.48% | 2.275x | -127.47% | 0 |
| game-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 908.11 | 0.987x | 0.987x | 1.310x | -30.99% | 3.181x | -218.15% | 1.130x | -12.97% | 0 |
| generic-sse | 63.00 | 63.00 | 1.000x | +0.00% | 63.12 | 0.998x | 0.998x | 1.015x | -1.46% | 1.897x | -89.71% | 0.782x | +21.77% | 0 |
| https-static-small | 592.00 | 592.00 | 1.000x | +0.00% | 592.87 | 0.999x | 0.999x | 1.138x | -13.82% | 2.149x | -114.88% | 1.811x | -81.13% | 0 |
| qcp-transparent | 704.00 | 688.00 | 1.023x | +2.33% | 704.49 | 0.999x | 0.977x | 0.874x | +12.61% | 2.116x | -111.64% | 0.998x | +0.17% | 0 |
| reverse-proxy | 1461.00 | 1461.00 | 1.000x | +0.00% | 1462.49 | 0.999x | 0.999x | 1.064x | -6.39% | 2.632x | -163.20% | 3.302x | -230.16% | 0 |
| static-large | 21.50 | 21.50 | 1.000x | +0.00% | 22.00 | 0.977x | 0.977x | 1.052x | -5.22% | 1.224x | -22.37% | 1.035x | -3.48% | 0 |
| static-small | 2394.00 | 2393.00 | 1.000x | +0.04% | 2395.34 | 0.999x | 0.999x | 0.901x | +9.90% | 4.453x | -345.26% | 2.871x | -187.13% | 0 |
| tcp-stream | 1008.00 | 1008.00 | 1.000x | +0.00% | 1014.23 | 0.994x | 0.994x | 1.160x | -16.03% | 2.126x | -112.65% | 0.592x | +40.82% | 0 |
| udp-stream | 560.00 | 560.00 | 1.000x | +0.00% | 573.50 | 0.976x | 0.976x | 0.805x | +19.54% | 1.581x | -58.06% | 3.366x | -236.60% | 0 |
| websocket-long-connection | 736.00 | 732.00 | 1.005x | +0.55% | 736.87 | 0.999x | 0.993x | 0.959x | +4.14% | 1.274x | -27.40% | 0.597x | +40.34% | 0 |

- Aggregate proxysss ops/s: `10892.50`
- Aggregate nginx ops/s: `10871.00`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.20%`
