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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-tls-pool-fastlane-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.959 < 0.980 (actual=8518.25 target=8879.02); https-static-small nginx target achievement 0.935 < 0.980 (actual=1678.50 target=1795.74); reverse-proxy nginx target achievement 0.964 < 0.980 (actual=4429.00 target=4595.06); static-small nginx target achievement 0.966 < 0.980 (actual=8700.25 target=9009.01)`
| cdn-hot-update | 8567.75 | 8518.25 | 1.006x | +0.58% | 8879.02 | 0.965x | 0.959x | 0.820x | +18.01% | 1.029x | -2.95% | 1.129x | -12.90% | 0 |
| game-long-connection | 1546.00 | 1545.75 | 1.000x | +0.02% | 1555.21 | 0.994x | 0.994x | 1.290x | -29.00% | 1.231x | -23.10% | 1.291x | -29.14% | 0 |
| generic-sse | 209.75 | 212.50 | 0.987x | -1.29% | 216.36 | 0.969x | 0.982x | 1.013x | -1.25% | 0.986x | +1.38% | 1.234x | -23.38% | 0 |
| https-static-small | 1663.75 | 1678.50 | 0.991x | -0.88% | 1795.74 | 0.926x | 0.935x | 1.088x | -8.82% | 1.079x | -7.85% | 1.042x | -4.20% | 0 |
| qcp-transparent | 1588.25 | 1592.00 | 0.998x | -0.24% | 1602.56 | 0.991x | 0.993x | 0.912x | +8.79% | 0.958x | +4.24% | 1.250x | -24.97% | 0 |
| reverse-proxy | 4493.50 | 4429.00 | 1.015x | +1.46% | 4595.06 | 0.978x | 0.964x | 0.953x | +4.69% | 1.049x | -4.85% | 1.055x | -5.50% | 0 |
| static-large | 40.75 | 40.75 | 1.000x | +0.00% | 40.87 | 0.997x | 0.997x | 1.048x | -4.81% | 1.046x | -4.60% | 1.101x | -10.06% | 0 |
| static-small | 8774.50 | 8700.25 | 1.009x | +0.85% | 9009.01 | 0.974x | 0.966x | 0.808x | +19.16% | 1.060x | -5.96% | 1.165x | -16.45% | 0 |
| tcp-stream | 1564.00 | 1564.00 | 1.000x | +0.00% | 1571.40 | 0.995x | 0.995x | 1.152x | -15.21% | 1.399x | -39.90% | 1.220x | -22.05% | 0 |
| udp-stream | 1598.25 | 1599.50 | 0.999x | -0.08% | 1611.93 | 0.992x | 0.992x | 0.980x | +2.03% | 0.994x | +0.57% | 0.957x | +4.35% | 0 |
| websocket-long-connection | 1466.00 | 1462.00 | 1.003x | +0.27% | 1474.11 | 0.994x | 0.992x | 1.237x | -23.70% | 1.302x | -30.16% | 1.391x | -39.09% | 0 |

- Aggregate proxysss ops/s: `31512.50`
- Aggregate nginx ops/s: `31342.50`
- Aggregate proxysss/nginx ratio: `1.005x`
- Aggregate throughput improvement: `+0.54%`
