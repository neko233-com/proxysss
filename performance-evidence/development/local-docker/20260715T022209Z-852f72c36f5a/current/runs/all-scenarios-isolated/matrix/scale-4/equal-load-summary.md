# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.960 < 0.980 (actual=18.00 target=18.75)`
| cdn-hot-update | 5353.50 | 5357.00 | 0.999x | -0.07% | 5360.81 | 0.999x | 0.999x | 0.844x | +15.58% | 1.159x | -15.87% | 1.129x | -12.86% | 0 |
| game-long-connection | 1152.00 | 1152.00 | 1.000x | +0.00% | 1155.11 | 0.997x | 0.997x | 1.247x | -24.69% | 1.048x | -4.84% | 0.868x | +13.23% | 0 |
| generic-sse | 140.00 | 140.00 | 1.000x | +0.00% | 140.62 | 0.996x | 0.996x | 1.028x | -2.80% | 1.303x | -30.32% | 1.097x | -9.71% | 0 |
| https-static-small | 1108.00 | 1108.00 | 1.000x | +0.00% | 1110.11 | 0.998x | 0.998x | 1.108x | -10.80% | 1.225x | -22.49% | 1.046x | -4.63% | 0 |
| qcp-transparent | 1152.00 | 1152.00 | 1.000x | +0.00% | 1173.84 | 0.981x | 0.981x | 0.965x | +3.45% | 1.836x | -83.60% | 0.653x | +34.68% | 0 |
| reverse-proxy | 2753.50 | 2755.50 | 0.999x | -0.07% | 2757.37 | 0.999x | 0.999x | 1.068x | -6.84% | 1.156x | -15.63% | 1.036x | -3.58% | 0 |
| static-large | 18.00 | 18.00 | 1.000x | +0.00% | 18.75 | 0.960x | 0.960x | 0.981x | +1.91% | 0.613x | +38.72% | 0.988x | +1.21% | 0 |
| static-small | 5134.00 | 5134.00 | 1.000x | +0.00% | 5142.63 | 0.998x | 0.998x | 0.879x | +12.14% | 0.819x | +18.14% | 0.899x | +10.12% | 0 |
| tcp-stream | 1600.00 | 1600.00 | 1.000x | +0.00% | 1606.75 | 0.996x | 0.996x | 1.143x | -14.29% | 1.335x | -33.46% | 0.389x | +61.11% | 0 |
| udp-stream | 1280.00 | 1280.00 | 1.000x | +0.00% | 1298.60 | 0.986x | 0.986x | 0.947x | +5.28% | 2.296x | -129.61% | 0.634x | +36.57% | 0 |
| websocket-long-connection | 960.00 | 960.00 | 1.000x | +0.00% | 977.73 | 0.982x | 0.982x | 1.082x | -8.20% | 0.662x | +33.84% | 0.601x | +39.91% | 0 |

- Aggregate proxysss ops/s: `20651.00`
- Aggregate nginx ops/s: `20656.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.03%`
