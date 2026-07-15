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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6868.50 | 6872.00 | 0.999x | -0.05% | 6878.76 | 0.999x | 0.999x | 0.760x | +23.99% | 0.720x | +27.99% | 1.463x | -46.34% | 0 |
| game-long-connection | 1264.00 | 1264.00 | 1.000x | +0.00% | 1270.45 | 0.995x | 0.995x | 1.039x | -3.87% | 0.817x | +18.26% | 0.704x | +29.57% | 0 |
| generic-sse | 111.00 | 111.00 | 1.000x | +0.00% | 111.50 | 0.996x | 0.996x | 0.954x | +4.57% | 0.962x | +3.76% | 0.887x | +11.30% | 0 |
| https-static-small | 1195.00 | 1195.50 | 1.000x | -0.04% | 1197.07 | 0.998x | 0.999x | 1.041x | -4.11% | 1.850x | -84.97% | 6.150x | -514.95% | 0 |
| qcp-transparent | 984.00 | 984.00 | 1.000x | +0.00% | 991.94 | 0.992x | 0.992x | 0.837x | +16.30% | 0.951x | +4.90% | 0.733x | +26.70% | 0 |
| reverse-proxy | 2695.00 | 2693.00 | 1.001x | +0.07% | 2697.92 | 0.999x | 0.998x | 0.943x | +5.65% | 1.512x | -51.22% | 1.068x | -6.76% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.12 | 0.995x | 0.995x | 0.954x | +4.58% | 0.843x | +15.72% | 0.949x | +5.08% | 0 |
| static-small | 6732.50 | 6733.00 | 1.000x | -0.01% | 6742.52 | 0.999x | 0.999x | 0.779x | +22.08% | 0.728x | +27.17% | 0.850x | +15.01% | 0 |
| tcp-stream | 968.00 | 964.00 | 1.004x | +0.41% | 970.64 | 0.997x | 0.993x | 0.966x | +3.36% | 0.938x | +6.18% | 0.599x | +40.07% | 0 |
| udp-stream | 956.00 | 960.00 | 0.996x | -0.42% | 962.35 | 0.993x | 0.998x | 0.908x | +9.20% | 1.631x | -63.08% | 1.312x | -31.24% | 0 |
| websocket-long-connection | 1120.00 | 1116.00 | 1.004x | +0.36% | 1121.86 | 0.998x | 0.995x | 1.018x | -1.80% | 1.224x | -22.39% | 1.834x | -83.40% | 0 |

- Aggregate proxysss ops/s: `22916.00`
- Aggregate nginx ops/s: `22914.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
