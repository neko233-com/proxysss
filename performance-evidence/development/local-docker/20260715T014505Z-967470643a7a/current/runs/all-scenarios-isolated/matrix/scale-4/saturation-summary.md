# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 12846.50 | 17384.50 | 0.739x | -26.10% | - | - | - | 1.006x | -0.57% | 2.395x | -139.54% | 2.209x | -120.92% | 0 |
| game-long-connection | 5084.00 | 3353.00 | 1.516x | +51.63% | - | - | - | 0.580x | +42.05% | 0.721x | +27.94% | 0.874x | +12.60% | 0 |
| generic-sse | 405.50 | 427.00 | 0.950x | -5.04% | - | - | - | 0.669x | +33.14% | 2.086x | -108.55% | 3.096x | -209.63% | 0 |
| https-static-small | 4837.50 | 3016.50 | 1.604x | +60.37% | - | - | - | 0.636x | +36.38% | 0.687x | +31.33% | 0.574x | +42.55% | 0 |
| qcp-transparent | 3338.50 | 2945.00 | 1.134x | +13.36% | - | - | - | 0.519x | +48.09% | 1.833x | -83.29% | 2.325x | -132.49% | 0 |
| reverse-proxy | 6936.50 | 8839.00 | 0.785x | -21.52% | - | - | - | 0.932x | +6.84% | 2.327x | -132.75% | 2.898x | -189.84% | 0 |
| static-large | 108.00 | 100.00 | 1.080x | +8.00% | - | - | - | 1.007x | -0.70% | 1.158x | -15.85% | 0.584x | +41.60% | 0 |
| static-small | 13006.50 | 16560.00 | 0.785x | -21.46% | - | - | - | 0.903x | +9.71% | 2.104x | -110.45% | 2.406x | -140.58% | 0 |
| tcp-stream | 4548.50 | 2941.00 | 1.547x | +54.66% | - | - | - | 0.559x | +44.09% | 0.841x | +15.89% | 0.828x | +17.22% | 0 |
| udp-stream | 2959.00 | 2904.00 | 1.019x | +1.89% | - | - | - | 0.613x | +38.69% | 2.001x | -100.06% | 2.332x | -133.17% | 0 |
| websocket-long-connection | 3905.00 | 2565.00 | 1.522x | +52.24% | - | - | - | 0.583x | +41.69% | 0.853x | +14.66% | 0.967x | +3.25% | 0 |

- Aggregate proxysss ops/s: `57975.50`
- Aggregate nginx ops/s: `61035.00`
- Aggregate proxysss/nginx ratio: `0.950x`
- Aggregate throughput improvement: `-5.01%`
