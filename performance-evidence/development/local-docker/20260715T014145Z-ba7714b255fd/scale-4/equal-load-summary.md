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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014145Z-ba7714b255fd/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.875 < 0.980 (actual=7.00 target=8.00); qcp-transparent nginx target achievement 0.968 < 0.980 (actual=864.00 target=892.23); websocket-long-connection nginx target achievement 0.979 < 0.980 (actual=800.00 target=817.24)`
| cdn-hot-update | 3788.00 | 3792.00 | 0.999x | -0.11% | 3793.72 | 0.998x | 1.000x | 0.741x | +25.88% | 0.526x | +47.40% | 0.722x | +27.84% | 0 |
| game-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 879.99 | 0.982x | 0.982x | 0.630x | +36.96% | 0.928x | +7.24% | 0.744x | +25.63% | 0 |
| generic-sse | 105.00 | 105.00 | 1.000x | +0.00% | 106.00 | 0.991x | 0.991x | 0.943x | +5.72% | 0.340x | +66.02% | 0.582x | +41.80% | 0 |
| https-static-small | 7.00 | 7.00 | 1.000x | +0.00% | 8.00 | 0.875x | 0.875x | 0.883x | +11.73% | 0.767x | +23.32% | 0.767x | +23.32% | 0 |
| qcp-transparent | 864.00 | 864.00 | 1.000x | +0.00% | 892.23 | 0.968x | 0.968x | 0.720x | +27.98% | 0.497x | +50.26% | 0.400x | +60.02% | 0 |
| reverse-proxy | 2454.00 | 2452.00 | 1.001x | +0.08% | 2457.71 | 0.998x | 0.998x | 0.930x | +7.02% | 0.655x | +34.55% | 0.683x | +31.70% | 0 |
| static-large | 14.00 | 14.00 | 1.000x | +0.00% | 14.25 | 0.982x | 0.982x | 1.059x | -5.87% | 0.520x | +47.96% | 0.733x | +26.70% | 0 |
| static-small | 3436.00 | 3441.00 | 0.999x | -0.15% | 3443.73 | 0.998x | 0.999x | 0.769x | +23.11% | 0.379x | +62.09% | 0.708x | +29.24% | 0 |
| tcp-stream | 1248.00 | 1248.00 | 1.000x | +0.00% | 1252.50 | 0.996x | 0.996x | 0.805x | +19.45% | 0.281x | +71.91% | 0.106x | +89.42% | 0 |
| udp-stream | 864.00 | 864.00 | 1.000x | +0.00% | 880.50 | 0.981x | 0.981x | 0.944x | +5.64% | 0.403x | +59.69% | 0.379x | +62.06% | 0 |
| websocket-long-connection | 800.00 | 800.00 | 1.000x | +0.00% | 817.24 | 0.979x | 0.979x | 0.838x | +16.17% | 0.804x | +19.57% | 0.134x | +86.58% | 0 |

- Aggregate proxysss ops/s: `14444.00`
- Aggregate nginx ops/s: `14451.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.05%`
