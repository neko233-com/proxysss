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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-phase-clients-r2-1x2x4x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 3511.05 | 3500.50 | 1.003x | +0.30% | 3528.11 | 0.995x | 0.992x | 0.727x | +27.34% | 0.623x | +37.74% | 0.350x | +64.97% | 0 |
| game-long-connection | 652.00 | 649.80 | 1.003x | +0.34% | 652.37 | 0.999x | 0.996x | 0.953x | +4.75% | 0.755x | +24.46% | 0.464x | +53.61% | 0 |
| generic-sse | 84.80 | 84.85 | 0.999x | -0.06% | 85.20 | 0.995x | 0.996x | 0.985x | +1.52% | 0.731x | +26.86% | 0.462x | +53.76% | 0 |
| https-static-small | 831.30 | 840.85 | 0.989x | -1.14% | 854.97 | 0.972x | 0.983x | 0.978x | +2.24% | 0.798x | +20.21% | 0.477x | +52.34% | 0 |
| qcp-transparent | 631.95 | 630.80 | 1.002x | +0.18% | 633.26 | 0.998x | 0.996x | 0.853x | +14.67% | 0.606x | +39.45% | 0.833x | +16.68% | 0 |
| reverse-proxy | 1819.75 | 1817.90 | 1.001x | +0.10% | 1830.24 | 0.994x | 0.993x | 0.871x | +12.90% | 0.636x | +36.45% | 0.415x | +58.47% | 0 |
| static-large | 15.50 | 15.50 | 1.000x | +0.00% | 15.57 | 0.996x | 0.996x | 0.976x | +2.42% | 0.912x | +8.82% | 0.680x | +32.03% | 0 |
| static-small | 3471.65 | 3460.10 | 1.003x | +0.33% | 3486.98 | 0.996x | 0.992x | 0.716x | +28.42% | 0.608x | +39.19% | 0.365x | +63.54% | 0 |
| tcp-stream | 638.40 | 635.90 | 1.004x | +0.39% | 639.23 | 0.999x | 0.995x | 0.936x | +6.38% | 0.743x | +25.73% | 0.402x | +59.82% | 0 |
| udp-stream | 626.35 | 625.20 | 1.002x | +0.18% | 627.84 | 0.998x | 0.996x | 0.840x | +15.97% | 0.607x | +39.31% | 0.504x | +49.60% | 0 |
| websocket-long-connection | 620.80 | 618.90 | 1.003x | +0.31% | 621.21 | 0.999x | 0.996x | 0.951x | +4.91% | 0.754x | +24.57% | 0.591x | +40.94% | 0 |

- Aggregate proxysss ops/s: `12903.55`
- Aggregate nginx ops/s: `12880.30`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.18%`
