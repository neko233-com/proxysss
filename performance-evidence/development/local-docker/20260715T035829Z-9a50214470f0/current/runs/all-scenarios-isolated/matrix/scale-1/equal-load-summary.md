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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7073.67 | 7075.33 | 1.000x | -0.02% | 7072.00 | 1.000x | 1.000x | 0.836x | +16.38% | 0.935x | +6.52% | 0.467x | +53.27% | 0 |
| game-long-connection | 957.33 | 957.33 | 1.000x | +0.00% | 957.33 | 1.000x | 1.000x | 1.123x | -12.35% | 1.227x | -22.66% | 0.737x | +26.26% | 0 |
| generic-sse | 111.33 | 111.33 | 1.000x | +0.00% | 111.33 | 1.000x | 1.000x | 1.018x | -1.81% | 1.121x | -12.11% | 0.836x | +16.37% | 0 |
| https-static-small | 1412.67 | 1412.67 | 1.000x | +0.00% | 1413.33 | 1.000x | 1.000x | 0.992x | +0.82% | 1.091x | -9.07% | 1.073x | -7.29% | 0 |
| qcp-transparent | 853.33 | 853.33 | 1.000x | +0.00% | 856.00 | 0.997x | 0.997x | 0.893x | +10.73% | 1.115x | -11.49% | 1.183x | -18.34% | 0 |
| reverse-proxy | 2953.33 | 2953.00 | 1.000x | +0.01% | 2954.67 | 1.000x | 0.999x | 1.003x | -0.33% | 1.187x | -18.71% | 0.896x | +10.43% | 0 |
| static-large | 23.33 | 23.33 | 1.000x | +0.00% | 22.67 | 1.029x | 1.029x | 0.986x | +1.36% | 0.958x | +4.20% | 0.837x | +16.31% | 0 |
| static-small | 6919.00 | 6917.33 | 1.000x | +0.02% | 6912.00 | 1.001x | 1.001x | 0.907x | +9.30% | 1.161x | -16.06% | 2.821x | -182.12% | 0 |
| tcp-stream | 898.67 | 898.67 | 1.000x | +0.00% | 898.67 | 1.000x | 1.000x | 1.101x | -10.12% | 1.142x | -14.21% | 0.809x | +19.14% | 0 |
| udp-stream | 938.67 | 938.67 | 1.000x | +0.00% | 938.67 | 1.000x | 1.000x | 0.903x | +9.69% | 1.248x | -24.82% | 2.121x | -112.11% | 0 |
| websocket-long-connection | 920.00 | 920.00 | 1.000x | +0.00% | 920.00 | 1.000x | 1.000x | 1.088x | -8.83% | 1.360x | -36.04% | 1.440x | -44.04% | 0 |

- Aggregate proxysss ops/s: `23061.33`
- Aggregate nginx ops/s: `23060.99`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
