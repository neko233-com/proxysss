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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-paced-pool-1x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.966 < 0.980 (actual=1820.00 target=1883.68)`
| cdn-hot-update | 7585.20 | 7556.90 | 1.004x | +0.37% | 7705.27 | 0.984x | 0.981x | 0.844x | +15.57% | 1.202x | -20.20% | 1.480x | -48.03% | 0 |
| game-long-connection | 1837.10 | 1834.15 | 1.002x | +0.16% | 1852.71 | 0.992x | 0.990x | 1.136x | -13.61% | 1.355x | -35.49% | 1.323x | -32.27% | 0 |
| generic-sse | 181.90 | 182.00 | 0.999x | -0.05% | 184.04 | 0.988x | 0.989x | 1.002x | -0.17% | 1.212x | -21.22% | 1.217x | -21.66% | 0 |
| https-static-small | 1807.65 | 1820.00 | 0.993x | -0.68% | 1883.68 | 0.960x | 0.966x | 1.095x | -9.47% | 1.187x | -18.68% | 0.978x | +2.22% | 0 |
| qcp-transparent | 1375.15 | 1377.05 | 0.999x | -0.14% | 1389.13 | 0.990x | 0.991x | 0.943x | +5.69% | 1.334x | -33.36% | 1.507x | -50.65% | 0 |
| reverse-proxy | 3976.65 | 3970.80 | 1.001x | +0.15% | 4040.40 | 0.984x | 0.983x | 0.950x | +5.04% | 1.166x | -16.64% | 1.195x | -19.48% | 0 |
| static-large | 42.50 | 42.50 | 1.000x | +0.00% | 42.60 | 0.998x | 0.998x | 0.985x | +1.47% | 0.967x | +3.32% | 0.705x | +29.50% | 0 |
| static-small | 7704.00 | 7701.85 | 1.000x | +0.03% | 7843.14 | 0.982x | 0.982x | 0.851x | +14.93% | 1.225x | -22.55% | 1.517x | -51.73% | 0 |
| tcp-stream | 1825.15 | 1821.90 | 1.002x | +0.18% | 1842.04 | 0.991x | 0.989x | 1.131x | -13.10% | 1.295x | -29.48% | 1.329x | -32.91% | 0 |
| udp-stream | 1374.60 | 1375.80 | 0.999x | -0.09% | 1388.17 | 0.990x | 0.991x | 0.943x | +5.74% | 1.205x | -20.47% | 1.325x | -32.46% | 0 |
| websocket-long-connection | 1729.80 | 1726.20 | 1.002x | +0.21% | 1747.11 | 0.990x | 0.988x | 1.107x | -10.71% | 1.344x | -34.35% | 1.330x | -32.96% | 0 |

- Aggregate proxysss ops/s: `29439.70`
- Aggregate nginx ops/s: `29409.15`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.10%`
