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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29143.00 | 27014.00 | 1.079x | +7.88% | - | - | - | 0.721x | +27.93% | 1.347x | -34.74% | 1.240x | -23.98% | 0 |
| game-long-connection | 4600.50 | 3516.00 | 1.308x | +30.84% | - | - | - | 0.611x | +38.94% | 1.216x | -21.61% | 1.013x | -1.29% | 0 |
| generic-sse | 602.50 | 434.50 | 1.387x | +38.67% | - | - | - | 0.619x | +38.07% | 1.005x | -0.50% | 0.971x | +2.92% | 0 |
| https-static-small | 5521.50 | 5164.00 | 1.069x | +6.92% | - | - | - | 0.661x | +33.88% | 1.540x | -54.04% | 1.143x | -14.25% | 0 |
| qcp-transparent | 4569.50 | 3565.50 | 1.282x | +28.16% | - | - | - | 0.604x | +39.65% | 1.103x | -10.26% | 0.975x | +2.47% | 0 |
| reverse-proxy | 12737.00 | 11703.00 | 1.088x | +8.84% | - | - | - | 0.838x | +16.20% | 1.206x | -20.57% | 1.105x | -10.52% | 0 |
| static-large | 90.00 | 93.50 | 0.963x | -3.74% | - | - | - | 0.988x | +1.16% | 1.068x | -6.81% | 1.879x | -87.90% | 0 |
| static-small | 29540.50 | 27190.00 | 1.086x | +8.64% | - | - | - | 0.711x | +28.90% | 1.261x | -26.11% | 1.123x | -12.34% | 0 |
| tcp-stream | 4635.00 | 3477.00 | 1.333x | +33.30% | - | - | - | 0.597x | +40.25% | 1.233x | -23.30% | 0.938x | +6.21% | 0 |
| udp-stream | 4551.00 | 3606.50 | 1.262x | +26.19% | - | - | - | 0.604x | +39.55% | 1.079x | -7.86% | 0.974x | +2.63% | 0 |
| websocket-long-connection | 4283.00 | 3343.50 | 1.281x | +28.10% | - | - | - | 0.651x | +34.93% | 1.274x | -27.43% | 0.941x | +5.91% | 0 |

- Aggregate proxysss ops/s: `100273.50`
- Aggregate nginx ops/s: `89107.50`
- Aggregate proxysss/nginx ratio: `1.125x`
- Aggregate throughput improvement: `+12.53%`
