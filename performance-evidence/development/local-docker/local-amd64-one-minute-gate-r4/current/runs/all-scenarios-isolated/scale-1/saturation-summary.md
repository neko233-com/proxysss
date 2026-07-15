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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-one-minute-gate-r4/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 19951.50 | 22769.50 | 0.876x | -12.38% | - | - | - | 0.938x | +6.23% | 1.147x | -14.70% | 1.008x | -0.80% | 0 |
| game-long-connection | 3108.50 | 2993.00 | 1.039x | +3.86% | - | - | - | 1.057x | -5.73% | 1.015x | -1.48% | 0.892x | +10.84% | 0 |
| generic-sse | 397.00 | 412.00 | 0.964x | -3.64% | - | - | - | 0.949x | +5.14% | 1.296x | -29.61% | 1.279x | -27.92% | 0 |
| https-static-small | 3338.50 | 3902.50 | 0.855x | -14.45% | - | - | - | 1.159x | -15.92% | 1.131x | -13.15% | 1.039x | -3.87% | 0 |
| qcp-transparent | 2840.50 | 3029.00 | 0.938x | -6.22% | - | - | - | 1.073x | -7.31% | 1.154x | -15.37% | 0.946x | +5.43% | 0 |
| reverse-proxy | 7779.00 | 8618.00 | 0.903x | -9.74% | - | - | - | 1.168x | -16.76% | 1.158x | -15.84% | 1.267x | -26.67% | 0 |
| static-large | 109.50 | 104.00 | 1.053x | +5.29% | - | - | - | 1.014x | -1.35% | 0.960x | +3.95% | 0.712x | +28.82% | 0 |
| static-small | 20677.50 | 24166.50 | 0.856x | -14.44% | - | - | - | 0.976x | +2.40% | 1.195x | -19.54% | 0.999x | +0.09% | 0 |
| tcp-stream | 3260.50 | 3337.50 | 0.977x | -2.31% | - | - | - | 1.089x | -8.87% | 1.079x | -7.90% | 1.029x | -2.87% | 0 |
| udp-stream | 2754.50 | 2916.00 | 0.945x | -5.54% | - | - | - | 1.063x | -6.27% | 1.093x | -9.31% | 1.188x | -18.81% | 0 |
| websocket-long-connection | 3137.00 | 2927.50 | 1.072x | +7.16% | - | - | - | 0.977x | +2.32% | 0.979x | +2.08% | 0.859x | +14.14% | 0 |

- Aggregate proxysss ops/s: `67354.00`
- Aggregate nginx ops/s: `75175.50`
- Aggregate proxysss/nginx ratio: `0.896x`
- Aggregate throughput improvement: `-10.40%`
