# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for `game-long-connection, qcp-transparent, tcp-stream, udp-stream, websocket-long-connection`
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=true, strict=true)
- Saturation ops gate: `false`
- Equal-load latency gate: `true`
- Minimum fixed-load completion: `0.980`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7093.00 | 7097.00 | 0.999x | -0.06% | 7088.00 | 1.001x | 1.001x | 0.792x | +20.77% | 0.622x | +37.82% | 2.277x | -127.71% | 0 |
| game-long-connection | 1300.00 | 1300.00 | 1.000x | +0.00% | 1300.00 | 1.000x | 1.000x | 0.949x | +5.10% | 0.971x | +2.90% | 1.086x | -8.65% | 0 |
| generic-sse | 111.00 | 111.00 | 1.000x | +0.00% | 111.00 | 1.000x | 1.000x | 0.997x | +0.29% | 1.266x | -26.55% | 5.369x | -436.93% | 0 |
| https-static-small | 1136.50 | 1136.50 | 1.000x | +0.00% | 1136.00 | 1.000x | 1.000x | 0.916x | +8.37% | 0.758x | +24.16% | 0.713x | +28.69% | 0 |
| qcp-transparent | 932.00 | 932.00 | 1.000x | +0.00% | 932.00 | 1.000x | 1.000x | 0.864x | +13.62% | 0.844x | +15.60% | 2.382x | -138.23% | 0 |
| reverse-proxy | 2832.00 | 2831.00 | 1.000x | +0.04% | 2832.00 | 1.000x | 1.000x | 0.985x | +1.54% | 1.214x | -21.44% | 3.058x | -205.78% | 0 |
| static-large | 21.50 | 21.50 | 1.000x | +0.00% | 20.00 | 1.075x | 1.075x | 1.005x | -0.48% | 0.985x | +1.50% | 1.448x | -44.84% | 0 |
| static-small | 6786.00 | 6788.00 | 1.000x | -0.03% | 6784.00 | 1.000x | 1.001x | 0.780x | +21.98% | 0.952x | +4.85% | 1.803x | -80.29% | 0 |
| tcp-stream | 976.00 | 976.00 | 1.000x | +0.00% | 976.00 | 1.000x | 1.000x | 1.004x | -0.39% | 1.044x | -4.38% | 2.268x | -126.81% | 0 |
| udp-stream | 1000.00 | 1000.00 | 1.000x | +0.00% | 1000.00 | 1.000x | 1.000x | 0.833x | +16.67% | 0.854x | +14.57% | 2.139x | -113.90% | 0 |
| websocket-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.942x | +5.84% | 0.888x | +11.20% | 1.976x | -97.65% | 0 |

- Aggregate proxysss ops/s: `23084.00`
- Aggregate nginx ops/s: `23089.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
