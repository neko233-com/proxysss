# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 32326.50 | 26268.00 | 1.231x | +23.06% | - | - | - | 0.640x | +36.02% | 1.185x | -18.50% | 1.078x | -7.76% | 0 |
| game-long-connection | 6213.50 | 3827.00 | 1.624x | +62.36% | - | - | - | 0.400x | +59.97% | 1.164x | -16.37% | 1.039x | -3.92% | 0 |
| generic-sse | 661.50 | 439.50 | 1.505x | +50.51% | - | - | - | 0.565x | +43.50% | 0.929x | +7.06% | 0.849x | +15.05% | 0 |
| https-static-small | 7522.00 | 5301.00 | 1.419x | +41.90% | - | - | - | 0.513x | +48.67% | 1.159x | -15.92% | 0.966x | +3.37% | 0 |
| qcp-transparent | 5077.50 | 3903.00 | 1.301x | +30.09% | - | - | - | 0.569x | +43.13% | 1.141x | -14.09% | 0.956x | +4.37% | 0 |
| reverse-proxy | 13371.00 | 12480.00 | 1.071x | +7.14% | - | - | - | 0.840x | +15.99% | 1.223x | -22.30% | 0.916x | +8.42% | 0 |
| static-large | 86.00 | 89.50 | 0.961x | -3.91% | - | - | - | 1.027x | -2.65% | 1.118x | -11.83% | 1.166x | -16.63% | 0 |
| static-small | 29865.50 | 26285.00 | 1.136x | +13.62% | - | - | - | 0.682x | +31.81% | 1.324x | -32.39% | 1.203x | -20.27% | 0 |
| tcp-stream | 6044.50 | 3809.00 | 1.587x | +58.69% | - | - | - | 0.417x | +58.33% | 1.184x | -18.36% | 0.977x | +2.33% | 0 |
| udp-stream | 4795.00 | 3848.00 | 1.246x | +24.61% | - | - | - | 0.604x | +39.59% | 1.097x | -9.68% | 1.040x | -3.96% | 0 |
| websocket-long-connection | 5806.50 | 3606.50 | 1.610x | +61.00% | - | - | - | 0.421x | +57.94% | 1.122x | -12.15% | 0.881x | +11.93% | 0 |

- Aggregate proxysss ops/s: `111769.50`
- Aggregate nginx ops/s: `89856.50`
- Aggregate proxysss/nginx ratio: `1.244x`
- Aggregate throughput improvement: `+24.39%`
