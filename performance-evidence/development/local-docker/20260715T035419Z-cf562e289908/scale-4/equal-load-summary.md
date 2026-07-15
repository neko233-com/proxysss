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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6008.33 | 6005.33 | 1.000x | +0.05% | 5973.33 | 1.006x | 1.005x | 0.731x | +26.94% | 0.250x | +75.05% | 0.770x | +22.99% | 0 |
| game-long-connection | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 1.427x | -42.72% | 0.876x | +12.42% | 0.489x | +51.09% | 0 |
| generic-sse | 134.67 | 134.67 | 1.000x | +0.00% | 133.33 | 1.010x | 1.010x | 0.885x | +11.46% | 0.281x | +71.92% | 0.280x | +72.00% | 0 |
| https-static-small | 1394.00 | 1393.67 | 1.000x | +0.02% | 1386.67 | 1.005x | 1.005x | 0.914x | +8.63% | 0.522x | +47.78% | 0.676x | +32.37% | 0 |
| qcp-transparent | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 0.795x | +20.51% | 0.788x | +21.21% | 0.180x | +81.96% | 0 |
| reverse-proxy | 3044.67 | 3044.67 | 1.000x | +0.00% | 3029.33 | 1.005x | 1.005x | 0.855x | +14.54% | 0.191x | +80.90% | 0.141x | +85.91% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 16.00 | 1.312x | 1.312x | 0.974x | +2.55% | 0.811x | +18.93% | 0.368x | +63.17% | 0 |
| static-small | 6175.00 | 6172.67 | 1.000x | +0.04% | 6144.00 | 1.005x | 1.005x | 0.710x | +29.02% | 0.301x | +69.86% | 1.049x | -4.87% | 0 |
| tcp-stream | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 1.210x | -21.04% | 0.816x | +18.44% | 0.973x | +2.73% | 0 |
| udp-stream | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 0.764x | +23.64% | 0.230x | +76.96% | 0.253x | +74.68% | 0 |
| websocket-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.137x | -13.69% | 0.855x | +14.49% | 0.314x | +68.62% | 0 |

- Aggregate proxysss ops/s: `21215.01`
- Aggregate nginx ops/s: `21209.35`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.03%`
