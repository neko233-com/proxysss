# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `qcp-transparent nginx target achievement 0.952 < 0.980 (actual=576.00 target=604.99); static-large nginx target achievement 0.929 < 0.980 (actual=13.00 target=14.00); tcp-stream nginx target achievement 0.978 < 0.980 (actual=1152.00 target=1177.99)`
| cdn-hot-update | 2130.00 | 2129.00 | 1.000x | +0.05% | 2132.98 | 0.999x | 0.998x | 0.479x | +52.14% | 0.337x | +66.29% | 1.648x | -64.85% | 0 |
| game-long-connection | 1056.00 | 1056.00 | 1.000x | +0.00% | 1061.22 | 0.995x | 0.995x | 0.672x | +32.78% | 0.102x | +89.82% | 0.234x | +76.63% | 0 |
| generic-sse | 60.00 | 60.00 | 1.000x | +0.00% | 60.50 | 0.992x | 0.992x | 0.668x | +33.17% | 0.229x | +77.05% | 0.393x | +60.67% | 0 |
| https-static-small | 223.00 | 223.00 | 1.000x | +0.00% | 223.75 | 0.997x | 0.997x | 0.842x | +15.80% | 1.206x | -20.61% | 1.179x | -17.95% | 0 |
| qcp-transparent | 576.00 | 576.00 | 1.000x | +0.00% | 604.99 | 0.952x | 0.952x | 0.469x | +53.14% | 0.996x | +0.42% | 0.570x | +43.00% | 0 |
| reverse-proxy | 1283.00 | 1282.00 | 1.001x | +0.08% | 1284.24 | 0.999x | 0.998x | 0.740x | +26.00% | 0.265x | +73.52% | 0.970x | +2.97% | 0 |
| static-large | 13.00 | 13.00 | 1.000x | +0.00% | 14.00 | 0.929x | 0.929x | 1.013x | -1.32% | 0.929x | +7.07% | 1.832x | -83.25% | 0 |
| static-small | 2406.00 | 2404.00 | 1.001x | +0.08% | 2408.46 | 0.999x | 0.998x | 0.509x | +49.07% | 0.616x | +38.38% | 0.858x | +14.23% | 0 |
| tcp-stream | 1152.00 | 1152.00 | 1.000x | +0.00% | 1177.99 | 0.978x | 0.978x | 0.678x | +32.23% | 0.236x | +76.43% | 0.187x | +81.29% | 0 |
| udp-stream | 864.00 | 864.00 | 1.000x | +0.00% | 868.74 | 0.995x | 0.995x | 0.470x | +52.96% | 0.297x | +70.29% | 0.291x | +70.92% | 0 |
| websocket-long-connection | 608.00 | 608.00 | 1.000x | +0.00% | 612.00 | 0.993x | 0.993x | 0.614x | +38.60% | 0.327x | +67.33% | 0.234x | +76.62% | 0 |

- Aggregate proxysss ops/s: `10371.00`
- Aggregate nginx ops/s: `10367.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.04%`
