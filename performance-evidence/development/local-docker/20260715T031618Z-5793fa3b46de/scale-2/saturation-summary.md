# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32420.00 | 26613.50 | 1.218x | +21.82% | - | - | - | 0.621x | +37.86% | 1.501x | -50.09% | 1.520x | -51.99% | 0 |
| game-long-connection | 7251.50 | 4022.00 | 1.803x | +80.30% | - | - | - | 0.387x | +61.31% | 0.965x | +3.52% | 0.992x | +0.84% | 0 |
| generic-sse | 812.00 | 478.50 | 1.697x | +69.70% | - | - | - | 0.495x | +50.48% | 0.967x | +3.33% | 1.291x | -29.09% | 0 |
| https-static-small | 5534.00 | 5256.00 | 1.053x | +5.29% | - | - | - | 0.772x | +22.80% | 1.764x | -76.42% | 1.010x | -1.04% | 0 |
| qcp-transparent | 6116.50 | 4117.50 | 1.485x | +48.55% | - | - | - | 0.484x | +51.55% | 1.154x | -15.36% | 1.240x | -23.96% | 0 |
| reverse-proxy | 14289.00 | 12847.50 | 1.112x | +11.22% | - | - | - | 0.857x | +14.29% | 1.378x | -37.79% | 1.440x | -44.02% | 0 |
| static-large | 91.50 | 95.50 | 0.958x | -4.19% | - | - | - | 1.382x | -38.22% | 1.347x | -34.73% | 1.096x | -9.65% | 0 |
| static-small | 31754.50 | 26458.00 | 1.200x | +20.02% | - | - | - | 0.599x | +40.06% | 1.575x | -57.52% | 1.728x | -72.84% | 0 |
| tcp-stream | 7493.00 | 4157.50 | 1.802x | +80.23% | - | - | - | 0.381x | +61.85% | 0.946x | +5.41% | 1.047x | -4.73% | 0 |
| udp-stream | 6783.00 | 4066.50 | 1.668x | +66.80% | - | - | - | 0.420x | +58.02% | 1.093x | -9.34% | 1.306x | -30.65% | 0 |
| websocket-long-connection | 6807.00 | 4217.00 | 1.614x | +61.42% | - | - | - | 0.396x | +60.41% | 1.093x | -9.32% | 1.071x | -7.07% | 0 |

- Aggregate proxysss ops/s: `119352.00`
- Aggregate nginx ops/s: `92329.50`
- Aggregate proxysss/nginx ratio: `1.293x`
- Aggregate throughput improvement: `+29.27%`
