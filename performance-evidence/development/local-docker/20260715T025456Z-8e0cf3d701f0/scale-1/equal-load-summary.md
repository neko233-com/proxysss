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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T025456Z-8e0cf3d701f0/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.977 < 0.980 (actual=21.00 target=21.50)`
| cdn-hot-update | 6382.00 | 6378.00 | 1.001x | +0.06% | 6389.78 | 0.999x | 0.998x | 0.817x | +18.31% | 0.782x | +21.84% | 0.958x | +4.23% | 0 |
| game-long-connection | 968.00 | 968.00 | 1.000x | +0.00% | 970.87 | 0.997x | 0.997x | 1.042x | -4.23% | 2.001x | -100.08% | 0.726x | +27.42% | 0 |
| generic-sse | 109.50 | 110.00 | 0.995x | -0.45% | 110.12 | 0.994x | 0.999x | 1.014x | -1.45% | 1.181x | -18.10% | 1.028x | -2.76% | 0 |
| https-static-small | 1209.50 | 1209.50 | 1.000x | +0.00% | 1211.20 | 0.999x | 0.999x | 0.968x | +3.18% | 0.921x | +7.93% | 0.887x | +11.27% | 0 |
| qcp-transparent | 992.00 | 992.00 | 1.000x | +0.00% | 995.02 | 0.997x | 0.997x | 0.899x | +10.08% | 1.095x | -9.53% | 1.273x | -27.30% | 0 |
| reverse-proxy | 2626.00 | 2624.50 | 1.001x | +0.06% | 2629.42 | 0.999x | 0.998x | 1.008x | -0.77% | 1.020x | -1.96% | 1.023x | -2.31% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 21.50 | 0.977x | 0.977x | 1.049x | -4.86% | 1.132x | -13.23% | 1.018x | -1.79% | 0 |
| static-small | 6215.00 | 6218.00 | 1.000x | -0.05% | 6223.26 | 0.999x | 0.999x | 0.846x | +15.45% | 1.020x | -2.01% | 1.178x | -17.81% | 0 |
| tcp-stream | 928.00 | 928.00 | 1.000x | +0.00% | 935.23 | 0.992x | 0.992x | 1.022x | -2.16% | 1.980x | -98.04% | 0.901x | +9.87% | 0 |
| udp-stream | 948.00 | 948.00 | 1.000x | +0.00% | 953.18 | 0.995x | 0.995x | 0.908x | +9.16% | 1.475x | -47.51% | 1.685x | -68.48% | 0 |
| websocket-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 920.60 | 0.991x | 0.991x | 1.051x | -5.14% | 1.445x | -44.52% | 0.840x | +15.97% | 0 |

- Aggregate proxysss ops/s: `21311.00`
- Aggregate nginx ops/s: `21309.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
