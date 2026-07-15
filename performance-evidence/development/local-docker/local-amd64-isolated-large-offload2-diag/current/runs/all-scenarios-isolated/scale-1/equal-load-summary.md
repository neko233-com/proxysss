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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-large-offload2-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.979 < 0.980 (actual=2215.00 target=2263.24); game-long-connection nginx target achievement 0.979 < 0.980 (actual=1070.50 target=1093.49); https-static-small nginx target achievement 0.927 < 0.980 (actual=797.75 target=860.22); static-small nginx target achievement 0.977 < 0.980 (actual=2189.25 target=2241.84); tcp-stream nginx target achievement 0.979 < 0.980 (actual=1072.00 target=1095.14)`
| cdn-hot-update | 2232.25 | 2215.00 | 1.008x | +0.78% | 2263.24 | 0.986x | 0.979x | 0.798x | +20.18% | 0.291x | +70.91% | 0.167x | +83.30% | 0 |
| game-long-connection | 1092.00 | 1070.50 | 1.020x | +2.01% | 1093.49 | 0.999x | 0.979x | 0.942x | +5.76% | 0.362x | +63.79% | 0.350x | +64.97% | 0 |
| generic-sse | 46.50 | 46.50 | 1.000x | +0.00% | 46.90 | 0.991x | 0.991x | 0.998x | +0.24% | 0.284x | +71.60% | 0.227x | +77.29% | 1 |
| https-static-small | 817.50 | 797.75 | 1.025x | +2.48% | 860.22 | 0.950x | 0.927x | 0.972x | +2.75% | 0.491x | +50.86% | 0.236x | +76.37% | 0 |
| qcp-transparent | 484.00 | 480.00 | 1.008x | +0.83% | 484.73 | 0.998x | 0.990x | 0.985x | +1.54% | 0.407x | +59.26% | 0.709x | +29.13% | 0 |
| reverse-proxy | 1104.25 | 1102.00 | 1.002x | +0.20% | 1121.19 | 0.985x | 0.983x | 0.883x | +11.66% | 0.228x | +77.15% | 0.085x | +91.51% | 0 |
| static-large | 37.75 | 37.75 | 1.000x | +0.00% | 37.80 | 0.999x | 0.999x | 0.975x | +2.48% | 0.351x | +64.86% | 1.033x | -3.32% | 0 |
| static-small | 2193.00 | 2189.25 | 1.002x | +0.17% | 2241.84 | 0.978x | 0.977x | 0.787x | +21.33% | 0.260x | +74.02% | 0.076x | +92.44% | 0 |
| tcp-stream | 1094.00 | 1072.00 | 1.021x | +2.05% | 1095.14 | 0.999x | 0.979x | 0.898x | +10.23% | 0.381x | +61.92% | 0.255x | +74.51% | 0 |
| udp-stream | 488.00 | 484.00 | 1.008x | +0.83% | 488.58 | 0.999x | 0.991x | 0.977x | +2.34% | 0.548x | +45.16% | 0.257x | +74.30% | 0 |
| websocket-long-connection | 1056.00 | 1038.00 | 1.017x | +1.73% | 1056.52 | 1.000x | 0.982x | 0.961x | +3.90% | 0.384x | +61.58% | 0.295x | +70.50% | 0 |

- Aggregate proxysss ops/s: `10645.25`
- Aggregate nginx ops/s: `10532.75`
- Aggregate proxysss/nginx ratio: `1.011x`
- Aggregate throughput improvement: `+1.07%`
