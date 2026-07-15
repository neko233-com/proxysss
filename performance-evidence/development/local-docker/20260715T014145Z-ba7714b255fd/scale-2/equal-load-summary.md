# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `qcp-transparent nginx target achievement 0.977 < 0.980 (actual=688.00 target=704.47); static-large nginx target achievement 0.950 < 0.980 (actual=19.00 target=20.00)`
| cdn-hot-update | 3286.00 | 3289.00 | 0.999x | -0.09% | 3291.00 | 0.998x | 0.999x | 0.712x | +28.77% | 0.314x | +68.57% | 0.820x | +17.95% | 0 |
| game-long-connection | 640.00 | 640.00 | 1.000x | +0.00% | 641.98 | 0.997x | 0.997x | 0.957x | +4.27% | 0.384x | +61.63% | 0.160x | +83.95% | 0 |
| generic-sse | 74.00 | 74.00 | 1.000x | +0.00% | 74.25 | 0.997x | 0.997x | 0.870x | +12.99% | 0.392x | +60.83% | 0.859x | +14.14% | 0 |
| https-static-small | 418.00 | 418.00 | 1.000x | +0.00% | 419.24 | 0.997x | 0.997x | 0.894x | +10.63% | 0.211x | +78.93% | 0.973x | +2.73% | 0 |
| qcp-transparent | 704.00 | 688.00 | 1.023x | +2.33% | 704.47 | 0.999x | 0.977x | 0.742x | +25.79% | 0.215x | +78.52% | 0.113x | +88.75% | 0 |
| reverse-proxy | 1443.00 | 1443.00 | 1.000x | +0.00% | 1445.48 | 0.998x | 0.998x | 0.912x | +8.80% | 0.376x | +62.39% | 0.550x | +45.01% | 0 |
| static-large | 19.00 | 19.00 | 1.000x | +0.00% | 20.00 | 0.950x | 0.950x | 0.933x | +6.68% | 0.769x | +23.14% | 0.978x | +2.17% | 0 |
| static-small | 3618.00 | 3617.00 | 1.000x | +0.03% | 3622.57 | 0.999x | 0.998x | 0.707x | +29.33% | 0.232x | +76.76% | 1.078x | -7.82% | 0 |
| tcp-stream | 768.00 | 768.00 | 1.000x | +0.00% | 779.99 | 0.985x | 0.985x | 0.795x | +20.49% | 0.488x | +51.21% | 1.355x | -35.52% | 0 |
| udp-stream | 704.00 | 704.00 | 1.000x | +0.00% | 715.98 | 0.983x | 0.983x | 0.763x | +23.75% | 0.246x | +75.44% | 0.295x | +70.49% | 0 |
| websocket-long-connection | 640.00 | 640.00 | 1.000x | +0.00% | 652.00 | 0.982x | 0.982x | 0.770x | +23.03% | 0.400x | +59.96% | 0.129x | +87.15% | 0 |

- Aggregate proxysss ops/s: `12314.00`
- Aggregate nginx ops/s: `12300.00`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.11%`
