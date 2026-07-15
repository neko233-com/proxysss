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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-one-minute-gate-r4/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4984.00 | 4986.00 | 1.000x | -0.04% | 4987.53 | 0.999x | 1.000x | 0.287x | +71.31% | 0.194x | +80.62% | 0.465x | +53.48% | 0 |
| game-long-connection | 744.00 | 744.00 | 1.000x | +0.00% | 748.22 | 0.994x | 0.994x | 0.293x | +70.74% | 0.196x | +80.40% | 0.293x | +70.71% | 0 |
| generic-sse | 99.00 | 99.00 | 1.000x | +0.00% | 99.25 | 0.997x | 0.997x | 0.501x | +49.93% | 0.242x | +75.79% | 0.618x | +38.23% | 0 |
| https-static-small | 833.50 | 833.50 | 1.000x | +0.00% | 834.55 | 0.999x | 0.999x | 0.446x | +55.38% | 0.180x | +82.02% | 0.263x | +73.70% | 0 |
| qcp-transparent | 708.00 | 708.00 | 1.000x | +0.00% | 710.10 | 0.997x | 0.997x | 0.350x | +65.02% | 0.164x | +83.60% | 0.377x | +62.28% | 0 |
| reverse-proxy | 1944.00 | 1942.50 | 1.001x | +0.08% | 1944.70 | 1.000x | 0.999x | 0.407x | +59.26% | 0.249x | +75.10% | 0.602x | +39.79% | 0 |
| static-large | 25.50 | 25.50 | 1.000x | +0.00% | 26.00 | 0.981x | 0.981x | 0.959x | +4.12% | 1.069x | -6.90% | 0.625x | +37.54% | 0 |
| static-small | 5164.00 | 5167.00 | 0.999x | -0.06% | 5168.79 | 0.999x | 1.000x | 0.289x | +71.15% | 0.197x | +80.34% | 0.699x | +30.13% | 0 |
| tcp-stream | 812.00 | 812.00 | 1.000x | +0.00% | 815.08 | 0.996x | 0.996x | 0.407x | +59.32% | 0.232x | +76.83% | 0.259x | +74.06% | 0 |
| udp-stream | 688.00 | 684.00 | 1.006x | +0.58% | 688.59 | 0.999x | 0.993x | 0.336x | +66.37% | 0.151x | +84.89% | 0.264x | +73.58% | 0 |
| websocket-long-connection | 728.00 | 728.00 | 1.000x | +0.00% | 731.86 | 0.995x | 0.995x | 0.420x | +58.05% | 0.209x | +79.09% | 0.401x | +59.94% | 0 |

- Aggregate proxysss ops/s: `16730.00`
- Aggregate nginx ops/s: `16729.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
