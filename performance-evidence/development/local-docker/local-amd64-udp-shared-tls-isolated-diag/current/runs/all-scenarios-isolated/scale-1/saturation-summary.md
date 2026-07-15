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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-udp-shared-tls-isolated-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 16070.25 | 20556.50 | 0.782x | -21.82% | - | - | - | 0.814x | +18.56% | 1.913x | -91.32% | 1.983x | -98.29% | 0 |
| game-long-connection | 3534.25 | 3737.50 | 0.946x | -5.44% | - | - | - | 0.731x | +26.91% | 1.422x | -42.25% | 1.531x | -53.12% | 0 |
| generic-sse | 377.75 | 467.00 | 0.809x | -19.11% | - | - | - | 0.923x | +7.70% | 1.855x | -85.54% | 2.172x | -117.25% | 1 |
| https-static-small | 3623.75 | 5043.00 | 0.719x | -28.14% | - | - | - | 1.443x | -44.30% | 1.429x | -42.91% | 1.138x | -13.80% | 0 |
| qcp-transparent | 2812.50 | 3491.75 | 0.805x | -19.45% | - | - | - | 0.828x | +17.23% | 1.988x | -98.85% | 2.268x | -126.82% | 0 |
| reverse-proxy | 8253.00 | 10596.00 | 0.779x | -22.11% | - | - | - | 0.975x | +2.52% | 1.933x | -93.33% | 1.928x | -92.81% | 0 |
| static-large | 113.25 | 86.25 | 1.313x | +31.30% | - | - | - | 0.775x | +22.54% | 0.731x | +26.94% | 0.456x | +54.39% | 0 |
| static-small | 15924.75 | 20356.00 | 0.782x | -21.77% | - | - | - | 0.845x | +15.52% | 1.811x | -81.11% | 1.949x | -94.92% | 0 |
| tcp-stream | 3531.75 | 3859.75 | 0.915x | -8.50% | - | - | - | 0.779x | +22.07% | 1.504x | -50.37% | 1.453x | -45.28% | 0 |
| udp-stream | 2806.75 | 3657.25 | 0.767x | -23.26% | - | - | - | 0.869x | +13.08% | 2.041x | -104.13% | 2.345x | -134.52% | 0 |
| websocket-long-connection | 3265.25 | 3669.75 | 0.890x | -11.02% | - | - | - | 0.793x | +20.73% | 1.453x | -45.25% | 1.510x | -51.02% | 0 |

- Aggregate proxysss ops/s: `60313.25`
- Aggregate nginx ops/s: `75520.75`
- Aggregate proxysss/nginx ratio: `0.799x`
- Aggregate throughput improvement: `-20.14%`
