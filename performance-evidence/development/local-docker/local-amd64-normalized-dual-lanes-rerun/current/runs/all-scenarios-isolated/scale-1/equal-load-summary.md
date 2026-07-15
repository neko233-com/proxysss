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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-normalized-dual-lanes-rerun/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.974 < 0.980 (actual=1759.20 target=1805.46)`
| cdn-hot-update | 5624.20 | 5652.40 | 0.995x | -0.50% | 5692.94 | 0.988x | 0.993x | 0.844x | +15.65% | 1.504x | -50.42% | 3.434x | -243.43% | 0 |
| game-long-connection | 1316.20 | 1318.80 | 0.998x | -0.20% | 1321.66 | 0.996x | 0.998x | 1.201x | -20.07% | 1.965x | -96.49% | 2.883x | -188.27% | 0 |
| generic-sse | 140.35 | 140.35 | 1.000x | +0.00% | 141.11 | 0.995x | 0.995x | 1.040x | -3.99% | 1.842x | -84.23% | 3.040x | -204.03% | 0 |
| https-static-small | 1753.80 | 1759.20 | 0.997x | -0.31% | 1805.46 | 0.971x | 0.974x | 1.143x | -14.26% | 1.708x | -70.76% | 2.543x | -154.33% | 0 |
| qcp-transparent | 1151.30 | 1156.50 | 0.996x | -0.45% | 1158.75 | 0.994x | 0.998x | 0.955x | +4.45% | 1.744x | -74.40% | 2.488x | -148.81% | 0 |
| reverse-proxy | 3182.25 | 3181.65 | 1.000x | +0.02% | 3207.06 | 0.992x | 0.992x | 0.963x | +3.70% | 1.708x | -70.76% | 3.741x | -274.11% | 0 |
| static-large | 32.40 | 32.40 | 1.000x | +0.00% | 32.50 | 0.997x | 0.997x | 1.036x | -3.61% | 1.195x | -19.49% | 1.516x | -51.58% | 0 |
| static-small | 5479.90 | 5493.95 | 0.997x | -0.26% | 5544.97 | 0.988x | 0.991x | 0.827x | +17.32% | 1.503x | -50.26% | 3.592x | -259.19% | 0 |
| tcp-stream | 1319.00 | 1320.65 | 0.999x | -0.12% | 1323.85 | 0.996x | 0.998x | 1.192x | -19.23% | 1.980x | -97.96% | 3.227x | -222.67% | 0 |
| udp-stream | 1146.15 | 1151.90 | 0.995x | -0.50% | 1154.40 | 0.993x | 0.998x | 0.966x | +3.41% | 1.744x | -74.38% | 3.466x | -246.64% | 0 |
| websocket-long-connection | 1253.60 | 1255.25 | 0.999x | -0.13% | 1258.46 | 0.996x | 0.997x | 1.152x | -15.23% | 1.972x | -97.19% | 2.718x | -171.81% | 0 |

- Aggregate proxysss ops/s: `22399.15`
- Aggregate nginx ops/s: `22463.05`
- Aggregate proxysss/nginx ratio: `0.997x`
- Aggregate throughput improvement: `-0.28%`
