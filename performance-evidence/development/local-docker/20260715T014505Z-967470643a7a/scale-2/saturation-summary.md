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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15770.00 | 17559.50 | 0.898x | -10.19% | - | - | - | 0.848x | +15.17% | 1.317x | -31.69% | 1.417x | -41.72% | 0 |
| game-long-connection | 4362.00 | 3593.00 | 1.214x | +21.40% | - | - | - | 0.764x | +23.58% | 0.897x | +10.32% | 0.951x | +4.87% | 0 |
| generic-sse | 450.00 | 476.00 | 0.945x | -5.46% | - | - | - | 0.869x | +13.13% | 1.306x | -30.61% | 1.759x | -75.86% | 0 |
| https-static-small | 4066.00 | 2689.00 | 1.512x | +51.21% | - | - | - | 0.698x | +30.18% | 0.701x | +29.94% | 0.600x | +40.05% | 0 |
| qcp-transparent | 3419.50 | 3090.00 | 1.107x | +10.66% | - | - | - | 0.784x | +21.61% | 1.157x | -15.67% | 1.380x | -38.01% | 0 |
| reverse-proxy | 8467.00 | 8291.00 | 1.021x | +2.12% | - | - | - | 0.844x | +15.63% | 1.379x | -37.94% | 1.703x | -70.27% | 0 |
| static-large | 113.50 | 88.50 | 1.282x | +28.25% | - | - | - | 0.804x | +19.60% | 0.700x | +30.04% | 0.956x | +4.43% | 0 |
| static-small | 16651.00 | 17542.00 | 0.949x | -5.08% | - | - | - | 0.810x | +19.02% | 1.354x | -35.45% | 1.367x | -36.69% | 0 |
| tcp-stream | 4746.50 | 3290.00 | 1.443x | +44.27% | - | - | - | 0.661x | +33.93% | 0.713x | +28.73% | 1.027x | -2.69% | 0 |
| udp-stream | 4443.50 | 3119.50 | 1.424x | +42.44% | - | - | - | 0.443x | +55.66% | 1.081x | -8.06% | 1.348x | -34.76% | 0 |
| websocket-long-connection | 4479.00 | 2871.50 | 1.560x | +55.98% | - | - | - | 0.618x | +38.25% | 0.680x | +31.99% | 0.725x | +27.48% | 0 |

- Aggregate proxysss ops/s: `66968.00`
- Aggregate nginx ops/s: `62610.00`
- Aggregate proxysss/nginx ratio: `1.070x`
- Aggregate throughput improvement: `+6.96%`
