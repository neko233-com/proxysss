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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031102Z-fc8ba6547cf8/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22681.50 | 26302.00 | 0.862x | -13.77% | - | - | - | 0.680x | +32.05% | 2.063x | -106.34% | 1.843x | -84.27% | 0 |
| game-long-connection | 7444.00 | 3540.50 | 2.103x | +110.25% | - | - | - | 0.332x | +66.76% | 0.899x | +10.14% | 0.887x | +11.34% | 0 |
| generic-sse | 359.00 | 440.00 | 0.816x | -18.41% | - | - | - | 0.841x | +15.88% | 1.927x | -92.72% | 2.266x | -126.59% | 0 |
| https-static-small | 1708.50 | 5690.50 | 0.300x | -69.98% | - | - | - | 2.836x | -183.56% | 4.171x | -317.12% | 2.464x | -146.40% | 0 |
| qcp-transparent | 2491.50 | 3658.00 | 0.681x | -31.89% | - | - | - | 0.959x | +4.11% | 2.058x | -105.84% | 2.025x | -102.47% | 0 |
| reverse-proxy | 10254.00 | 11510.50 | 0.891x | -10.92% | - | - | - | 0.798x | +20.20% | 1.574x | -57.40% | 1.401x | -40.11% | 0 |
| static-large | 85.00 | 92.00 | 0.924x | -7.61% | - | - | - | 0.988x | +1.23% | 1.299x | -29.95% | 1.923x | -92.27% | 0 |
| static-small | 24226.00 | 28284.00 | 0.857x | -14.35% | - | - | - | 0.684x | +31.61% | 2.152x | -115.17% | 2.203x | -120.34% | 0 |
| tcp-stream | 7304.00 | 3480.50 | 2.099x | +109.85% | - | - | - | 0.329x | +67.14% | 1.023x | -2.26% | 0.833x | +16.73% | 0 |
| udp-stream | 3937.50 | 3596.50 | 1.095x | +9.48% | - | - | - | 0.620x | +38.03% | 1.371x | -37.12% | 1.456x | -45.59% | 0 |
| websocket-long-connection | 7294.00 | 3699.00 | 1.972x | +97.19% | - | - | - | 0.350x | +65.02% | 0.943x | +5.70% | 0.934x | +6.56% | 0 |

- Aggregate proxysss ops/s: `87785.00`
- Aggregate nginx ops/s: `90293.50`
- Aggregate proxysss/nginx ratio: `0.972x`
- Aggregate throughput improvement: `-2.78%`
