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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 20079.00 | 22418.00 | 0.896x | -10.43% | - | - | - | 0.837x | +16.31% | 1.809x | -80.86% | 1.645x | -64.45% | 0 |
| game-long-connection | 7775.00 | 4341.50 | 1.791x | +79.09% | - | - | - | 0.442x | +55.77% | 0.823x | +17.66% | 0.973x | +2.71% | 0 |
| generic-sse | 429.00 | 517.00 | 0.830x | -17.02% | - | - | - | 0.915x | +8.53% | 2.466x | -146.61% | 2.905x | -190.46% | 0 |
| https-static-small | 5163.50 | 4454.50 | 1.159x | +15.92% | - | - | - | 0.873x | +12.73% | 0.966x | +3.36% | 1.112x | -11.16% | 0 |
| qcp-transparent | 3100.50 | 4939.00 | 0.628x | -37.22% | - | - | - | 1.069x | -6.88% | 2.210x | -121.05% | 2.528x | -152.81% | 0 |
| reverse-proxy | 8665.50 | 10115.50 | 0.857x | -14.33% | - | - | - | 0.924x | +7.57% | 1.843x | -84.31% | 1.908x | -90.80% | 0 |
| static-large | 138.50 | 137.00 | 1.011x | +1.09% | - | - | - | 0.998x | +0.19% | 1.152x | -15.25% | 0.787x | +21.30% | 0 |
| static-small | 17788.50 | 22407.50 | 0.794x | -20.61% | - | - | - | 0.827x | +17.33% | 1.929x | -92.90% | 2.247x | -124.66% | 0 |
| tcp-stream | 5540.00 | 4136.50 | 1.339x | +33.93% | - | - | - | 0.583x | +41.69% | 1.020x | -2.01% | 1.126x | -12.58% | 0 |
| udp-stream | 3473.00 | 3914.50 | 0.887x | -11.28% | - | - | - | 0.746x | +25.41% | 2.027x | -102.67% | 2.438x | -143.83% | 0 |
| websocket-long-connection | 5773.50 | 3847.50 | 1.501x | +50.06% | - | - | - | 0.533x | +46.72% | 0.894x | +10.60% | 0.916x | +8.43% | 0 |

- Aggregate proxysss ops/s: `77926.00`
- Aggregate nginx ops/s: `81228.50`
- Aggregate proxysss/nginx ratio: `0.959x`
- Aggregate throughput improvement: `-4.07%`
