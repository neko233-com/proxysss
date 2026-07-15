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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023010Z-42117b5eff18/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5212.50 | 5212.50 | 1.000x | +0.00% | 5220.23 | 0.999x | 0.999x | 0.798x | +20.23% | 1.159x | -15.92% | 0.875x | +12.47% | 0 |
| game-long-connection | 1048.00 | 1048.00 | 1.000x | +0.00% | 1056.25 | 0.992x | 0.992x | 1.127x | -12.67% | 1.557x | -55.67% | 2.579x | -157.92% | 0 |
| generic-sse | 105.00 | 105.00 | 1.000x | +0.00% | 106.12 | 0.989x | 0.989x | 0.968x | +3.20% | 1.283x | -28.33% | 1.194x | -19.39% | 0 |
| https-static-small | 1228.50 | 1228.50 | 1.000x | +0.00% | 1230.01 | 0.999x | 0.999x | 1.088x | -8.82% | 1.072x | -7.15% | 0.815x | +18.46% | 0 |
| qcp-transparent | 864.00 | 864.00 | 1.000x | +0.00% | 872.31 | 0.990x | 0.990x | 0.865x | +13.52% | 1.523x | -52.28% | 1.245x | -24.48% | 0 |
| reverse-proxy | 2145.50 | 2143.50 | 1.001x | +0.09% | 2147.07 | 0.999x | 0.998x | 0.970x | +3.02% | 1.336x | -33.62% | 1.281x | -28.13% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 21.37 | 0.983x | 0.983x | 0.965x | +3.50% | 1.163x | -16.33% | 0.845x | +15.48% | 0 |
| static-small | 5416.00 | 5417.50 | 1.000x | -0.03% | 5424.65 | 0.998x | 0.999x | 0.783x | +21.69% | 0.888x | +11.23% | 0.426x | +57.45% | 0 |
| tcp-stream | 952.00 | 952.00 | 1.000x | +0.00% | 955.00 | 0.997x | 0.997x | 1.140x | -14.00% | 1.550x | -54.96% | 1.207x | -20.65% | 0 |
| udp-stream | 880.00 | 880.00 | 1.000x | +0.00% | 883.20 | 0.996x | 0.996x | 0.921x | +7.88% | 1.329x | -32.90% | 0.708x | +29.24% | 0 |
| websocket-long-connection | 1032.00 | 1032.00 | 1.000x | +0.00% | 1036.14 | 0.996x | 0.996x | 1.124x | -12.39% | 1.544x | -54.38% | 1.223x | -22.35% | 0 |

- Aggregate proxysss ops/s: `18904.50`
- Aggregate nginx ops/s: `18904.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
