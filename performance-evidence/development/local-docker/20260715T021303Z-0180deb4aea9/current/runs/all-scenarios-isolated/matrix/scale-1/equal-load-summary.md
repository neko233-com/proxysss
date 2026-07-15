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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.966 < 0.980 (actual=25.00 target=25.87)`
| cdn-hot-update | 5820.00 | 5817.50 | 1.000x | +0.04% | 5823.48 | 0.999x | 0.999x | 0.819x | +18.10% | 0.983x | +1.66% | 1.703x | -70.25% | 0 |
| game-long-connection | 996.00 | 1000.00 | 0.996x | -0.40% | 1002.38 | 0.994x | 0.998x | 1.074x | -7.36% | 1.169x | -16.85% | 1.697x | -69.69% | 0 |
| generic-sse | 97.00 | 97.00 | 1.000x | +0.00% | 97.50 | 0.995x | 0.995x | 1.018x | -1.82% | 0.849x | +15.12% | 1.602x | -60.20% | 0 |
| https-static-small | 869.00 | 869.00 | 1.000x | +0.00% | 870.32 | 0.998x | 0.998x | 1.075x | -7.54% | 1.298x | -29.82% | 1.902x | -90.15% | 0 |
| qcp-transparent | 852.00 | 852.00 | 1.000x | +0.00% | 857.82 | 0.993x | 0.993x | 0.901x | +9.89% | 1.670x | -67.02% | 0.875x | +12.46% | 0 |
| reverse-proxy | 1966.50 | 1967.00 | 1.000x | -0.03% | 1969.84 | 0.998x | 0.999x | 1.019x | -1.88% | 1.997x | -99.74% | 1.042x | -4.21% | 0 |
| static-large | 25.00 | 25.00 | 1.000x | +0.00% | 25.87 | 0.966x | 0.966x | 0.979x | +2.06% | 1.147x | -14.66% | 1.173x | -17.34% | 0 |
| static-small | 5385.00 | 5386.00 | 1.000x | -0.02% | 5394.47 | 0.998x | 0.998x | 0.836x | +16.42% | 1.097x | -9.71% | 0.744x | +25.60% | 0 |
| tcp-stream | 944.00 | 944.00 | 1.000x | +0.00% | 947.64 | 0.996x | 0.996x | 1.026x | -2.63% | 1.195x | -19.54% | 1.208x | -20.81% | 0 |
| udp-stream | 920.00 | 920.00 | 1.000x | +0.00% | 921.87 | 0.998x | 0.998x | 0.914x | +8.62% | 1.734x | -73.43% | 1.818x | -81.76% | 0 |
| websocket-long-connection | 776.00 | 772.00 | 1.005x | +0.52% | 777.23 | 0.998x | 0.993x | 1.059x | -5.91% | 0.889x | +11.11% | 1.518x | -51.82% | 0 |

- Aggregate proxysss ops/s: `18650.50`
- Aggregate nginx ops/s: `18649.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
