# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-soft-realtime-h23200/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.969 < 0.980 (actual=2668.35 target=2752.92)`
| cdn-hot-update | 10254.10 | 10441.35 | 0.982x | -1.79% | 10648.92 | 0.963x | 0.981x | 0.737x | +26.32% | 1.004x | -0.37% | 1.511x | -51.08% | 0 |
| game-long-connection | 1798.60 | 1814.55 | 0.991x | -0.88% | 1834.44 | 0.980x | 0.989x | 1.035x | -3.54% | 1.330x | -33.03% | 1.605x | -60.46% | 0 |
| generic-sse | 227.85 | 230.75 | 0.987x | -1.26% | 233.75 | 0.975x | 0.987x | 0.839x | +16.12% | 1.156x | -15.61% | 1.702x | -70.22% | 0 |
| https-static-small | 2583.90 | 2668.35 | 0.968x | -3.16% | 2752.92 | 0.939x | 0.969x | 0.966x | +3.44% | 1.033x | -3.33% | 1.319x | -31.88% | 0 |
| qcp-transparent | 1784.55 | 1814.40 | 0.984x | -1.65% | 1834.86 | 0.973x | 0.989x | 0.770x | +22.98% | 1.135x | -13.53% | 1.803x | -80.34% | 0 |
| reverse-proxy | 5340.25 | 5419.85 | 0.985x | -1.47% | 5522.00 | 0.967x | 0.982x | 0.776x | +22.39% | 1.114x | -11.41% | 1.738x | -73.75% | 0 |
| static-large | 41.90 | 41.90 | 1.000x | +0.00% | 41.95 | 0.999x | 0.999x | 0.948x | +5.25% | 1.135x | -13.45% | 1.270x | -26.99% | 0 |
| static-small | 10079.65 | 10277.45 | 0.981x | -1.92% | 10457.52 | 0.964x | 0.983x | 0.742x | +25.82% | 1.035x | -3.45% | 1.490x | -49.03% | 0 |
| tcp-stream | 1826.80 | 1845.20 | 0.990x | -1.00% | 1865.24 | 0.979x | 0.989x | 1.001x | -0.10% | 1.344x | -34.38% | 1.683x | -68.30% | 0 |
| udp-stream | 1797.40 | 1829.55 | 0.982x | -1.76% | 1849.71 | 0.972x | 0.989x | 0.749x | +25.08% | 1.146x | -14.64% | 1.578x | -57.82% | 0 |
| websocket-long-connection | 1763.70 | 1781.25 | 0.990x | -0.99% | 1798.97 | 0.980x | 0.990x | 1.001x | -0.09% | 1.238x | -23.78% | 1.589x | -58.91% | 0 |

- Aggregate proxysss ops/s: `37498.70`
- Aggregate nginx ops/s: `38164.60`
- Aggregate proxysss/nginx ratio: `0.983x`
- Aggregate throughput improvement: `-1.74%`
