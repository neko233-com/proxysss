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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013317Z-2518dca6f297/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 3795.50 | 3796.50 | 1.000x | -0.03% | 3798.67 | 0.999x | 0.999x | 0.750x | +25.00% | 0.736x | +26.44% | 0.625x | +37.46% | 0 |
| game-long-connection | 776.00 | 776.00 | 1.000x | +0.00% | 778.36 | 0.997x | 0.997x | 1.014x | -1.35% | 1.150x | -14.99% | 0.827x | +17.28% | 0 |
| generic-sse | 91.00 | 91.00 | 1.000x | +0.00% | 91.12 | 0.999x | 0.999x | 0.999x | +0.11% | 1.518x | -51.77% | 0.859x | +14.05% | 0 |
| https-static-small | 793.00 | 793.00 | 1.000x | +0.00% | 793.73 | 0.999x | 0.999x | 1.032x | -3.24% | 1.044x | -4.39% | 0.911x | +8.93% | 0 |
| qcp-transparent | 504.00 | 504.00 | 1.000x | +0.00% | 507.36 | 0.993x | 0.993x | 0.766x | +23.39% | 0.798x | +20.20% | 0.788x | +21.21% | 0 |
| reverse-proxy | 1368.50 | 1368.50 | 1.000x | +0.00% | 1369.57 | 0.999x | 0.999x | 0.955x | +4.55% | 0.674x | +32.61% | 0.417x | +58.34% | 0 |
| static-large | 19.50 | 19.50 | 1.000x | +0.00% | 19.62 | 0.994x | 0.994x | 0.987x | +1.33% | 1.090x | -8.98% | 1.005x | -0.52% | 0 |
| static-small | 3767.00 | 3766.50 | 1.000x | +0.01% | 3768.70 | 1.000x | 0.999x | 0.749x | +25.13% | 0.811x | +18.93% | 0.928x | +7.23% | 0 |
| tcp-stream | 780.00 | 780.00 | 1.000x | +0.00% | 783.09 | 0.996x | 0.996x | 1.020x | -2.01% | 1.187x | -18.70% | 1.024x | -2.38% | 0 |
| udp-stream | 672.00 | 672.00 | 1.000x | +0.00% | 675.62 | 0.995x | 0.995x | 0.892x | +10.79% | 0.796x | +20.43% | 0.723x | +27.71% | 0 |
| websocket-long-connection | 812.00 | 812.00 | 1.000x | +0.00% | 815.58 | 0.996x | 0.996x | 1.021x | -2.06% | 1.268x | -26.78% | 1.889x | -88.93% | 0 |

- Aggregate proxysss ops/s: `13378.50`
- Aggregate nginx ops/s: `13379.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.00%`
