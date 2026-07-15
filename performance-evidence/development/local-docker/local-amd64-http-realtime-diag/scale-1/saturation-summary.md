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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-realtime-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 34588.50 | 34827.50 | 0.993x | -0.69% | - | - | - | 0.921x | +7.89% | 1.369x | -36.87% | 1.352x | -35.22% | 0 |
| game-long-connection | 3623.00 | 5373.50 | 0.674x | -32.58% | - | - | - | 0.865x | +13.46% | 2.410x | -141.00% | 2.952x | -195.16% | 0 |
| generic-sse | 835.25 | 668.75 | 1.249x | +24.90% | - | - | - | 0.690x | +31.00% | 1.164x | -16.35% | 1.254x | -25.42% | 1 |
| reverse-proxy | 20021.50 | 17138.50 | 1.168x | +16.82% | - | - | - | 0.741x | +25.88% | 1.260x | -25.97% | 1.336x | -33.61% | 0 |
| static-small | 34837.50 | 35073.25 | 0.993x | -0.67% | - | - | - | 0.918x | +8.24% | 1.409x | -40.93% | 1.516x | -51.64% | 0 |
| tcp-stream | 3654.75 | 5363.75 | 0.681x | -31.86% | - | - | - | 0.844x | +15.56% | 2.484x | -148.40% | 2.884x | -188.43% | 0 |
| websocket-long-connection | 3417.00 | 5121.00 | 0.667x | -33.27% | - | - | - | 0.977x | +2.35% | 2.436x | -143.56% | 2.884x | -188.40% | 0 |

- Aggregate proxysss ops/s: `100977.50`
- Aggregate nginx ops/s: `103566.25`
- Aggregate proxysss/nginx ratio: `0.975x`
- Aggregate throughput improvement: `-2.50%`
