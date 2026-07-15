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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T011840Z-dc838c8ecbbd/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 2755.00 | 2756.00 | 1.000x | -0.04% | 2756.72 | 0.999x | 1.000x | 0.662x | +33.83% | 0.803x | +19.69% | 0.702x | +29.82% | 0 |
| game-long-connection | 512.00 | 512.00 | 1.000x | +0.00% | 513.48 | 0.997x | 0.997x | 0.910x | +8.99% | 0.938x | +6.20% | 0.663x | +33.69% | 0 |
| generic-sse | 63.50 | 63.50 | 1.000x | +0.00% | 63.62 | 0.998x | 0.998x | 0.903x | +9.74% | 1.075x | -7.50% | 2.665x | -166.48% | 0 |
| https-static-small | 511.00 | 511.00 | 1.000x | +0.00% | 511.48 | 0.999x | 0.999x | 0.874x | +12.55% | 0.300x | +69.98% | 0.183x | +81.66% | 0 |
| qcp-transparent | 460.00 | 460.00 | 1.000x | +0.00% | 461.23 | 0.997x | 0.997x | 0.780x | +21.99% | 0.777x | +22.34% | 0.386x | +61.38% | 0 |
| reverse-proxy | 1322.50 | 1321.50 | 1.001x | +0.08% | 1323.08 | 1.000x | 0.999x | 0.840x | +16.02% | 1.086x | -8.60% | 1.545x | -54.47% | 0 |
| static-large | 18.50 | 18.50 | 1.000x | +0.00% | 18.62 | 0.994x | 0.994x | 0.905x | +9.51% | 0.787x | +21.26% | 1.840x | -84.03% | 0 |
| static-small | 2971.50 | 2973.00 | 0.999x | -0.05% | 2974.53 | 0.999x | 0.999x | 0.689x | +31.08% | 0.752x | +24.85% | 0.765x | +23.46% | 0 |
| tcp-stream | 516.00 | 516.00 | 1.000x | +0.00% | 519.48 | 0.993x | 0.993x | 0.919x | +8.12% | 0.785x | +21.50% | 0.750x | +25.03% | 0 |
| udp-stream | 464.00 | 464.00 | 1.000x | +0.00% | 465.36 | 0.997x | 0.997x | 0.823x | +17.67% | 0.729x | +27.08% | 0.491x | +50.91% | 0 |
| websocket-long-connection | 496.00 | 496.00 | 1.000x | +0.00% | 496.49 | 0.999x | 0.999x | 0.887x | +11.26% | 0.831x | +16.92% | 0.462x | +53.82% | 0 |

- Aggregate proxysss ops/s: `10090.00`
- Aggregate nginx ops/s: `10091.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
