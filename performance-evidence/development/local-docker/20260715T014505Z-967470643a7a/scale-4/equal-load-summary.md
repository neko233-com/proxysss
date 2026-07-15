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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `qcp-transparent nginx target achievement 0.978 < 0.980 (actual=720.00 target=736.24); tcp-stream nginx target achievement 0.979 < 0.980 (actual=720.00 target=735.24)`
| cdn-hot-update | 3211.00 | 3209.00 | 1.001x | +0.06% | 3211.56 | 1.000x | 0.999x | 0.765x | +23.50% | 0.487x | +51.35% | 0.678x | +32.24% | 0 |
| game-long-connection | 832.00 | 832.00 | 1.000x | +0.00% | 838.24 | 0.993x | 0.993x | 0.951x | +4.91% | 0.457x | +54.26% | 0.208x | +79.24% | 0 |
| generic-sse | 101.00 | 101.00 | 1.000x | +0.00% | 101.37 | 0.996x | 0.996x | 0.984x | +1.57% | 0.999x | +0.13% | 0.360x | +63.99% | 0 |
| https-static-small | 753.50 | 753.00 | 1.001x | +0.07% | 754.11 | 0.999x | 0.999x | 1.066x | -6.59% | 2.682x | -168.16% | 4.391x | -339.14% | 0 |
| qcp-transparent | 720.00 | 720.00 | 1.000x | +0.00% | 736.24 | 0.978x | 0.978x | 0.712x | +28.82% | 1.015x | -1.50% | 0.512x | +48.79% | 0 |
| reverse-proxy | 1733.00 | 1733.00 | 1.000x | +0.00% | 1734.11 | 0.999x | 0.999x | 0.959x | +4.12% | 1.230x | -23.02% | 1.053x | -5.29% | 0 |
| static-large | 24.50 | 24.50 | 1.000x | +0.00% | 25.00 | 0.980x | 0.980x | 0.990x | +0.96% | 0.895x | +10.51% | 0.972x | +2.78% | 0 |
| static-small | 3249.50 | 3250.00 | 1.000x | -0.02% | 3251.62 | 0.999x | 1.000x | 0.799x | +20.10% | 1.009x | -0.93% | 0.826x | +17.43% | 0 |
| tcp-stream | 720.00 | 720.00 | 1.000x | +0.00% | 735.24 | 0.979x | 0.979x | 1.235x | -23.47% | 0.969x | +3.12% | 0.169x | +83.12% | 0 |
| udp-stream | 720.00 | 720.00 | 1.000x | +0.00% | 725.99 | 0.992x | 0.992x | 0.755x | +24.49% | 1.653x | -65.27% | 0.387x | +61.33% | 0 |
| websocket-long-connection | 640.00 | 640.00 | 1.000x | +0.00% | 641.24 | 0.998x | 0.998x | 0.912x | +8.84% | 1.016x | -1.58% | 1.651x | -65.06% | 0 |

- Aggregate proxysss ops/s: `12704.50`
- Aggregate nginx ops/s: `12702.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
