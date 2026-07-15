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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T042520Z-52b3e01ee3f2/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 18921.00 | 26453.67 | 0.715x | -28.47% | - | - | - | 1.033x | -3.35% | 2.130x | -113.04% | 2.498x | -149.83% | 0 |
| game-long-connection | 4713.33 | 3637.67 | 1.296x | +29.57% | - | - | - | 0.609x | +39.12% | 1.237x | -23.73% | 1.203x | -20.26% | 0 |
| generic-sse | 353.00 | 442.00 | 0.799x | -20.14% | - | - | - | 1.058x | -5.79% | 2.040x | -104.04% | 2.073x | -107.28% | 0 |
| https-static-small | 10871.67 | 5818.33 | 1.869x | +86.85% | - | - | - | 0.259x | +74.10% | 1.016x | -1.63% | 0.964x | +3.59% | 0 |
| qcp-transparent | 7953.33 | 3846.33 | 2.068x | +106.78% | - | - | - | 0.207x | +79.26% | 1.041x | -4.09% | 1.081x | -8.11% | 0 |
| reverse-proxy | 9354.67 | 11889.33 | 0.787x | -21.32% | - | - | - | 1.108x | -10.76% | 1.840x | -83.99% | 1.927x | -92.67% | 0 |
| static-large | 99.00 | 94.00 | 1.053x | +5.32% | - | - | - | 0.921x | +7.86% | 0.981x | +1.89% | 1.391x | -39.13% | 0 |
| static-small | 18439.33 | 26944.00 | 0.684x | -31.56% | - | - | - | 1.159x | -15.90% | 2.192x | -119.21% | 2.352x | -135.18% | 0 |
| tcp-stream | 4665.33 | 3620.67 | 1.289x | +28.85% | - | - | - | 0.616x | +38.42% | 1.230x | -22.97% | 1.168x | -16.81% | 0 |
| udp-stream | 7582.67 | 3739.67 | 2.028x | +102.76% | - | - | - | 0.227x | +77.35% | 1.011x | -1.13% | 0.994x | +0.64% | 0 |
| websocket-long-connection | 4415.67 | 3445.00 | 1.282x | +28.18% | - | - | - | 0.653x | +34.70% | 1.175x | -17.46% | 1.253x | -25.33% | 0 |

- Aggregate proxysss ops/s: `87369.00`
- Aggregate nginx ops/s: `89930.67`
- Aggregate proxysss/nginx ratio: `0.972x`
- Aggregate throughput improvement: `-2.85%`
