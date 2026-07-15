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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 27100.00 | 25533.00 | 1.061x | +6.14% | - | - | - | 0.714x | +28.64% | 1.405x | -40.51% | 2.380x | -137.96% | 0 |
| game-long-connection | 7893.33 | 3724.00 | 2.120x | +111.96% | - | - | - | 0.385x | +61.50% | 0.702x | +29.75% | 0.795x | +20.52% | 0 |
| generic-sse | 760.67 | 576.67 | 1.319x | +31.91% | - | - | - | 0.590x | +41.00% | 1.171x | -17.11% | 1.606x | -60.59% | 0 |
| https-static-small | 3889.33 | 5450.67 | 0.714x | -28.64% | - | - | - | 1.314x | -31.38% | 2.326x | -132.64% | 1.678x | -67.79% | 0 |
| qcp-transparent | 6531.67 | 3777.00 | 1.729x | +72.93% | - | - | - | 0.403x | +59.73% | 1.023x | -2.35% | 1.401x | -40.08% | 0 |
| reverse-proxy | 13169.67 | 12335.67 | 1.068x | +6.76% | - | - | - | 0.810x | +18.98% | 1.348x | -34.75% | 2.318x | -131.76% | 0 |
| static-large | 93.00 | 98.00 | 0.949x | -5.10% | - | - | - | 0.976x | +2.43% | 1.303x | -30.33% | 1.365x | -36.51% | 0 |
| static-small | 28902.33 | 24987.33 | 1.157x | +15.67% | - | - | - | 0.673x | +32.72% | 1.305x | -30.51% | 2.083x | -108.27% | 0 |
| tcp-stream | 7904.67 | 3692.67 | 2.141x | +114.06% | - | - | - | 0.369x | +63.11% | 0.683x | +31.69% | 0.843x | +15.69% | 0 |
| udp-stream | 6198.33 | 3845.00 | 1.612x | +61.20% | - | - | - | 0.452x | +54.78% | 1.008x | -0.75% | 1.266x | -26.56% | 0 |
| websocket-long-connection | 7504.00 | 3533.00 | 2.124x | +112.40% | - | - | - | 0.407x | +59.30% | 0.687x | +31.34% | 0.721x | +27.90% | 0 |

- Aggregate proxysss ops/s: `109947.00`
- Aggregate nginx ops/s: `87553.01`
- Aggregate proxysss/nginx ratio: `1.256x`
- Aggregate throughput improvement: `+25.58%`
