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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 8532.00 | 11823.00 | 0.722x | -27.84% | - | - | - | 1.188x | -18.83% | 1.602x | -60.23% | 0.988x | +1.18% | 0 |
| game-long-connection | 4245.00 | 4831.00 | 0.879x | -12.13% | - | - | - | 0.961x | +3.91% | 1.257x | -25.65% | 0.843x | +15.73% | 0 |
| generic-sse | 242.00 | 398.00 | 0.608x | -39.20% | - | - | - | 1.418x | -41.76% | 2.313x | -131.27% | 1.377x | -37.71% | 0 |
| https-static-small | 1673.00 | 895.00 | 1.869x | +86.93% | - | - | - | 1.047x | -4.69% | 0.956x | +4.40% | 0.717x | +28.30% | 0 |
| qcp-transparent | 2420.00 | 3339.00 | 0.725x | -27.52% | - | - | - | 1.061x | -6.10% | 2.082x | -108.15% | 2.611x | -161.14% | 0 |
| reverse-proxy | 5137.00 | 7204.00 | 0.713x | -28.69% | - | - | - | 1.293x | -29.33% | 2.042x | -104.21% | 1.107x | -10.65% | 0 |
| static-large | 72.00 | 56.00 | 1.286x | +28.57% | - | - | - | 0.754x | +24.57% | 0.842x | +15.80% | 0.473x | +52.66% | 0 |
| static-small | 9634.00 | 11434.00 | 0.843x | -15.74% | - | - | - | 1.153x | -15.26% | 1.338x | -33.75% | 1.030x | -3.01% | 0 |
| tcp-stream | 4813.00 | 4712.00 | 1.021x | +2.14% | - | - | - | 0.802x | +19.79% | 1.167x | -16.70% | 0.893x | +10.72% | 0 |
| udp-stream | 3475.00 | 3504.00 | 0.992x | -0.83% | - | - | - | 0.486x | +51.38% | 1.930x | -93.01% | 1.659x | -65.88% | 0 |
| websocket-long-connection | 2448.00 | 3082.00 | 0.794x | -20.57% | - | - | - | 1.219x | -21.87% | 1.089x | -8.93% | 1.198x | -19.81% | 0 |

- Aggregate proxysss ops/s: `42691.00`
- Aggregate nginx ops/s: `51278.00`
- Aggregate proxysss/nginx ratio: `0.833x`
- Aggregate throughput improvement: `-16.75%`
