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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T031618Z-5793fa3b46de/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30004.50 | 24861.00 | 1.207x | +20.69% | - | - | - | 0.729x | +27.10% | 1.133x | -13.28% | 1.180x | -18.04% | 0 |
| game-long-connection | 7978.00 | 3925.00 | 2.033x | +103.26% | - | - | - | 0.385x | +61.54% | 0.672x | +32.81% | 0.700x | +30.00% | 0 |
| generic-sse | 992.00 | 558.00 | 1.778x | +77.78% | - | - | - | 0.510x | +48.97% | 0.775x | +22.49% | 0.801x | +19.94% | 0 |
| https-static-small | 5459.50 | 5384.00 | 1.014x | +1.40% | - | - | - | 0.990x | +1.02% | 1.451x | -45.08% | 0.719x | +28.14% | 0 |
| qcp-transparent | 7386.50 | 3968.00 | 1.862x | +86.15% | - | - | - | 0.411x | +58.94% | 0.772x | +22.78% | 0.822x | +17.82% | 0 |
| reverse-proxy | 14077.50 | 11644.00 | 1.209x | +20.90% | - | - | - | 0.816x | +18.36% | 0.985x | +1.50% | 0.878x | +12.23% | 0 |
| static-large | 84.00 | 100.00 | 0.840x | -16.00% | - | - | - | 1.278x | -27.81% | 1.641x | -64.11% | 0.403x | +59.70% | 0 |
| static-small | 31888.00 | 23053.50 | 1.383x | +38.32% | - | - | - | 0.613x | +38.67% | 0.930x | +6.97% | 1.026x | -2.57% | 0 |
| tcp-stream | 7434.50 | 3876.00 | 1.918x | +91.81% | - | - | - | 0.429x | +57.14% | 0.705x | +29.54% | 0.748x | +25.18% | 0 |
| udp-stream | 7205.50 | 3736.00 | 1.929x | +92.87% | - | - | - | 0.416x | +58.35% | 0.780x | +22.02% | 0.954x | +4.64% | 0 |
| websocket-long-connection | 7997.00 | 3621.00 | 2.209x | +120.85% | - | - | - | 0.391x | +60.90% | 0.617x | +38.29% | 0.741x | +25.89% | 0 |

- Aggregate proxysss ops/s: `120507.00`
- Aggregate nginx ops/s: `84726.50`
- Aggregate proxysss/nginx ratio: `1.422x`
- Aggregate throughput improvement: `+42.23%`
