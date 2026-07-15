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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-sync-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.967 < 0.980 (actual=7404.50 target=7659.17); https-static-small nginx target achievement 0.943 < 0.980 (actual=2403.75 target=2547.77); reverse-proxy nginx target achievement 0.971 < 0.980 (actual=4041.50 target=4162.87); static-small nginx target achievement 0.969 < 0.980 (actual=7786.50 target=8034.15)`
| cdn-hot-update | 7285.50 | 7404.50 | 0.984x | -1.61% | 7659.17 | 0.951x | 0.967x | 1.039x | -3.92% | 3.776x | -277.61% | 6.063x | -506.30% | 0 |
| game-long-connection | 2760.50 | 2807.50 | 0.983x | -1.67% | 2837.89 | 0.973x | 0.989x | 1.518x | -51.77% | 1.881x | -88.08% | 1.793x | -79.31% | 0 |
| generic-sse | 166.75 | 171.75 | 0.971x | -2.91% | 174.64 | 0.955x | 0.983x | 1.210x | -21.00% | 3.996x | -299.63% | 3.794x | -279.36% | 1 |
| https-static-small | 2280.25 | 2403.75 | 0.949x | -5.14% | 2547.77 | 0.895x | 0.943x | 1.373x | -37.28% | 1.549x | -54.85% | 1.672x | -67.17% | 0 |
| qcp-transparent | 1294.25 | 1320.50 | 0.980x | -1.99% | 1326.48 | 0.976x | 0.995x | 1.517x | -51.67% | 2.875x | -187.48% | 2.242x | -124.17% | 0 |
| reverse-proxy | 3949.00 | 4041.50 | 0.977x | -2.29% | 4162.87 | 0.949x | 0.971x | 1.175x | -17.52% | 3.628x | -262.80% | 4.668x | -366.80% | 0 |
| static-large | 63.75 | 64.00 | 0.996x | -0.39% | 64.05 | 0.995x | 0.999x | 1.043x | -4.33% | 1.454x | -45.40% | 1.117x | -11.73% | 0 |
| static-small | 7553.00 | 7786.50 | 0.970x | -3.00% | 8034.15 | 0.940x | 0.969x | 1.071x | -7.09% | 3.747x | -274.72% | 5.563x | -456.29% | 0 |
| tcp-stream | 2614.00 | 2676.00 | 0.977x | -2.32% | 2696.33 | 0.969x | 0.992x | 1.458x | -45.79% | 1.930x | -93.01% | 1.717x | -71.66% | 0 |
| udp-stream | 1312.25 | 1332.00 | 0.985x | -1.48% | 1337.12 | 0.981x | 0.996x | 1.644x | -64.36% | 3.425x | -242.46% | 2.663x | -166.33% | 0 |
| websocket-long-connection | 2714.25 | 2762.50 | 0.983x | -1.75% | 2802.10 | 0.969x | 0.986x | 1.483x | -48.30% | 1.883x | -88.32% | 1.365x | -36.50% | 0 |

- Aggregate proxysss ops/s: `31993.50`
- Aggregate nginx ops/s: `32770.50`
- Aggregate proxysss/nginx ratio: `0.976x`
- Aggregate throughput improvement: `-2.37%`
