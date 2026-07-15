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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-udp-shared-realtime-per-core-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.968 < 0.980 (actual=6153.00 target=6359.30); https-static-small nginx target achievement 0.951 < 0.980 (actual=2337.75 target=2458.51); reverse-proxy nginx target achievement 0.974 < 0.980 (actual=3525.50 target=3617.86); static-small nginx target achievement 0.968 < 0.980 (actual=6452.75 target=6669.45)`
| cdn-hot-update | 6010.25 | 6153.00 | 0.977x | -2.32% | 6359.30 | 0.945x | 0.968x | 1.000x | +0.00% | 2.572x | -157.18% | 3.394x | -239.36% | 0 |
| game-long-connection | 2509.50 | 2556.50 | 0.982x | -1.84% | 2573.17 | 0.975x | 0.994x | 1.403x | -40.27% | 1.908x | -90.84% | 1.711x | -71.13% | 0 |
| generic-sse | 135.00 | 138.00 | 0.978x | -2.17% | 140.52 | 0.961x | 0.982x | 1.093x | -9.25% | 2.719x | -171.85% | 2.958x | -195.78% | 1 |
| https-static-small | 2178.25 | 2337.75 | 0.932x | -6.82% | 2458.51 | 0.886x | 0.951x | 1.168x | -16.79% | 1.500x | -50.00% | 1.923x | -92.32% | 0 |
| qcp-transparent | 1106.25 | 1134.00 | 0.976x | -2.45% | 1138.63 | 0.972x | 0.996x | 1.101x | -10.09% | 2.805x | -180.46% | 2.504x | -150.36% | 0 |
| reverse-proxy | 3457.75 | 3525.50 | 0.981x | -1.92% | 3617.86 | 0.956x | 0.974x | 1.046x | -4.55% | 2.678x | -167.82% | 3.120x | -212.02% | 0 |
| static-large | 62.25 | 62.25 | 1.000x | +0.00% | 62.82 | 0.991x | 0.991x | 1.044x | -4.40% | 1.321x | -32.15% | 1.066x | -6.57% | 0 |
| static-small | 6339.50 | 6452.75 | 0.982x | -1.76% | 6669.45 | 0.951x | 0.968x | 0.927x | +7.26% | 2.556x | -155.64% | 3.952x | -295.21% | 0 |
| tcp-stream | 2567.50 | 2620.00 | 0.980x | -2.00% | 2635.91 | 0.974x | 0.994x | 1.489x | -48.87% | 1.734x | -73.44% | 1.513x | -51.29% | 0 |
| udp-stream | 1067.00 | 1094.00 | 0.975x | -2.47% | 1099.96 | 0.970x | 0.995x | 1.123x | -12.31% | 3.000x | -200.00% | 3.229x | -222.86% | 0 |
| websocket-long-connection | 2311.00 | 2357.50 | 0.980x | -1.97% | 2378.83 | 0.971x | 0.991x | 1.370x | -36.98% | 1.683x | -68.25% | 1.894x | -89.41% | 0 |

- Aggregate proxysss ops/s: `27744.25`
- Aggregate nginx ops/s: `28431.25`
- Aggregate proxysss/nginx ratio: `0.976x`
- Aggregate throughput improvement: `-2.42%`
