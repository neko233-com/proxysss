# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 11277.00 | 10571.00 | 1.067x | +6.68% | - | - | - | 0.816x | +18.39% | 1.142x | -14.23% | 0.771x | +22.90% | 0 |
| game-long-connection | 2929.00 | 5549.00 | 0.528x | -47.22% | - | - | - | 5.603x | -460.32% | 1.264x | -26.44% | 1.118x | -11.83% | 0 |
| generic-sse | 317.00 | 314.00 | 1.010x | +0.96% | - | - | - | 0.918x | +8.22% | 1.170x | -16.97% | 0.940x | +5.98% | 0 |
| https-static-small | 1384.00 | 1396.00 | 0.991x | -0.86% | - | - | - | 0.744x | +25.58% | 0.813x | +18.73% | 1.381x | -38.09% | 0 |
| qcp-transparent | 3969.00 | 2926.00 | 1.356x | +35.65% | - | - | - | 0.491x | +50.94% | 0.966x | +3.39% | 1.291x | -29.12% | 0 |
| reverse-proxy | 4787.00 | 5818.00 | 0.823x | -17.72% | - | - | - | 0.998x | +0.22% | 1.249x | -24.85% | 2.064x | -106.38% | 0 |
| static-large | 66.00 | 66.00 | 1.000x | +0.00% | - | - | - | 1.078x | -7.79% | 0.707x | +29.28% | 0.746x | +25.45% | 0 |
| static-small | 10396.00 | 9923.00 | 1.048x | +4.77% | - | - | - | 0.874x | +12.63% | 1.102x | -10.15% | 0.953x | +4.71% | 0 |
| tcp-stream | 2806.00 | 2727.00 | 1.029x | +2.90% | - | - | - | 1.150x | -15.00% | 0.911x | +8.86% | 0.629x | +37.08% | 0 |
| udp-stream | 2429.00 | 2974.00 | 0.817x | -18.33% | - | - | - | 1.256x | -25.58% | 1.338x | -33.82% | 1.360x | -35.99% | 0 |
| websocket-long-connection | 3061.00 | 2393.00 | 1.279x | +27.91% | - | - | - | 0.769x | +23.10% | 0.861x | +13.90% | 1.009x | -0.90% | 0 |

- Aggregate proxysss ops/s: `43421.00`
- Aggregate nginx ops/s: `44657.00`
- Aggregate proxysss/nginx ratio: `0.972x`
- Aggregate throughput improvement: `-2.77%`
