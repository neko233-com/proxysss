# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-qos-rebalance-1x2x4x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.967 < 0.980 (actual=3847.80 target=3978.61); https-static-small nginx target achievement 0.950 < 0.980 (actual=936.65 target=986.19); static-small nginx target achievement 0.969 < 0.980 (actual=3765.85 target=3884.91)`
| cdn-hot-update | 3929.60 | 3847.80 | 1.021x | +2.13% | 3978.61 | 0.988x | 0.967x | 0.606x | +39.42% | 0.191x | +80.93% | 0.153x | +84.75% | 0 |
| game-long-connection | 730.40 | 719.85 | 1.015x | +1.47% | 730.66 | 1.000x | 0.985x | 0.748x | +25.19% | 0.257x | +74.32% | 0.276x | +72.35% | 0 |
| generic-sse | 94.45 | 94.15 | 1.003x | +0.32% | 95.11 | 0.993x | 0.990x | 0.806x | +19.44% | 0.228x | +77.18% | 0.304x | +69.57% | 0 |
| https-static-small | 954.00 | 936.65 | 1.019x | +1.85% | 986.19 | 0.967x | 0.950x | 0.824x | +17.55% | 0.264x | +73.59% | 0.216x | +78.45% | 0 |
| qcp-transparent | 708.45 | 700.40 | 1.011x | +1.15% | 710.86 | 0.997x | 0.985x | 0.674x | +32.56% | 0.232x | +76.76% | 0.298x | +70.18% | 0 |
| reverse-proxy | 2044.10 | 2023.60 | 1.010x | +1.01% | 2062.25 | 0.991x | 0.981x | 0.705x | +29.55% | 0.229x | +77.15% | 0.217x | +78.26% | 0 |
| static-large | 17.50 | 17.50 | 1.000x | +0.00% | 17.51 | 0.999x | 0.999x | 0.938x | +6.15% | 0.402x | +59.76% | 0.398x | +60.18% | 0 |
| static-small | 3841.45 | 3765.85 | 1.020x | +2.01% | 3884.91 | 0.989x | 0.969x | 0.622x | +37.83% | 0.202x | +79.83% | 0.178x | +82.23% | 0 |
| tcp-stream | 745.60 | 734.65 | 1.015x | +1.49% | 745.78 | 1.000x | 0.985x | 0.751x | +24.90% | 0.236x | +76.43% | 0.236x | +76.36% | 0 |
| udp-stream | 706.90 | 698.95 | 1.011x | +1.14% | 709.35 | 0.997x | 0.985x | 0.667x | +33.31% | 0.233x | +76.66% | 0.290x | +70.97% | 0 |
| websocket-long-connection | 717.80 | 708.25 | 1.013x | +1.35% | 719.23 | 0.998x | 0.985x | 0.789x | +21.13% | 0.246x | +75.44% | 0.338x | +66.19% | 0 |

- Aggregate proxysss ops/s: `14490.25`
- Aggregate nginx ops/s: `14247.65`
- Aggregate proxysss/nginx ratio: `1.017x`
- Aggregate throughput improvement: `+1.70%`
