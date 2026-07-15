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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-2/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 35267.50 | 26622.00 | 1.325x | +32.48% | - | - | - | 0.622x | +37.81% | 0.938x | +6.19% | 0.973x | +2.69% | 0 |
| game-long-connection | 7676.50 | 3988.50 | 1.925x | +92.47% | - | - | - | 0.379x | +62.08% | 0.778x | +22.21% | 0.790x | +21.00% | 0 |
| generic-sse | 962.50 | 493.00 | 1.952x | +95.23% | - | - | - | 0.445x | +55.45% | 0.750x | +25.03% | 0.914x | +8.62% | 0 |
| https-static-small | 9104.50 | 6129.50 | 1.485x | +48.54% | - | - | - | 0.545x | +45.54% | 0.978x | +2.24% | 0.854x | +14.59% | 0 |
| qcp-transparent | 8133.50 | 4043.00 | 2.012x | +101.17% | - | - | - | 0.380x | +62.05% | 0.750x | +24.97% | 0.833x | +16.66% | 0 |
| reverse-proxy | 14668.50 | 12700.00 | 1.155x | +15.50% | - | - | - | 0.869x | +13.07% | 1.030x | -2.98% | 1.087x | -8.69% | 0 |
| static-large | 90.50 | 97.00 | 0.933x | -6.70% | - | - | - | 1.074x | -7.43% | 1.106x | -10.61% | 2.681x | -168.10% | 0 |
| static-small | 35394.50 | 27155.50 | 1.303x | +30.34% | - | - | - | 0.639x | +36.09% | 1.002x | -0.25% | 1.026x | -2.64% | 0 |
| tcp-stream | 7841.50 | 3857.00 | 2.033x | +103.31% | - | - | - | 0.364x | +63.62% | 0.742x | +25.84% | 0.718x | +28.20% | 0 |
| udp-stream | 7661.50 | 4007.00 | 1.912x | +91.20% | - | - | - | 0.391x | +60.89% | 0.826x | +17.45% | 0.972x | +2.78% | 0 |
| websocket-long-connection | 7579.00 | 3777.00 | 2.007x | +100.66% | - | - | - | 0.361x | +63.88% | 0.811x | +18.94% | 0.871x | +12.92% | 0 |

- Aggregate proxysss ops/s: `134380.00`
- Aggregate nginx ops/s: `92869.50`
- Aggregate proxysss/nginx ratio: `1.447x`
- Aggregate throughput improvement: `+44.70%`
