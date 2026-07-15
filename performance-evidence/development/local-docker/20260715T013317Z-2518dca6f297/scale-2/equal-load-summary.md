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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 4686.50 | 4685.00 | 1.000x | +0.03% | 4688.30 | 1.000x | 0.999x | 0.747x | +25.33% | 0.391x | +60.91% | 0.256x | +74.39% | 0 |
| game-long-connection | 1040.00 | 1040.00 | 1.000x | +0.00% | 1041.73 | 0.998x | 0.998x | 0.949x | +5.10% | 0.496x | +50.39% | 0.223x | +77.72% | 0 |
| generic-sse | 117.50 | 117.50 | 1.000x | +0.00% | 117.62 | 0.999x | 0.999x | 1.014x | -1.35% | 0.337x | +66.28% | 0.403x | +59.67% | 0 |
| https-static-small | 911.00 | 910.00 | 1.001x | +0.11% | 911.37 | 1.000x | 0.998x | 1.030x | -2.97% | 1.010x | -1.01% | 0.437x | +56.28% | 0 |
| qcp-transparent | 816.00 | 816.00 | 1.000x | +0.00% | 823.09 | 0.991x | 0.991x | 0.765x | +23.45% | 0.663x | +33.73% | 0.315x | +68.52% | 0 |
| reverse-proxy | 2205.00 | 2205.50 | 1.000x | -0.02% | 2206.44 | 0.999x | 1.000x | 0.995x | +0.49% | 0.348x | +65.20% | 0.916x | +8.39% | 0 |
| static-large | 25.00 | 25.00 | 1.000x | +0.00% | 25.50 | 0.980x | 0.980x | 0.978x | +2.24% | 0.446x | +55.37% | 1.053x | -5.29% | 0 |
| static-small | 4890.00 | 4888.50 | 1.000x | +0.03% | 4892.22 | 1.000x | 0.999x | 0.798x | +20.18% | 0.212x | +78.76% | 0.255x | +74.55% | 0 |
| tcp-stream | 848.00 | 848.00 | 1.000x | +0.00% | 849.98 | 0.998x | 0.998x | 0.983x | +1.75% | 0.288x | +71.23% | 0.563x | +43.68% | 0 |
| udp-stream | 888.00 | 888.00 | 1.000x | +0.00% | 896.46 | 0.991x | 0.991x | 0.936x | +6.43% | 0.317x | +68.25% | 0.241x | +75.85% | 0 |
| websocket-long-connection | 1160.00 | 1160.00 | 1.000x | +0.00% | 1161.95 | 0.998x | 0.998x | 1.007x | -0.72% | 0.497x | +50.30% | 0.711x | +28.85% | 0 |

- Aggregate proxysss ops/s: `17587.00`
- Aggregate nginx ops/s: `17583.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
