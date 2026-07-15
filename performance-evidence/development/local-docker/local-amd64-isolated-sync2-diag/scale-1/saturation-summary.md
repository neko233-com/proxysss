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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-sync2-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 10286.25 | 21760.00 | 0.473x | -52.73% | - | - | - | 0.910x | +8.97% | 3.773x | -277.28% | 3.078x | -207.77% | 0 |
| game-long-connection | 4523.25 | 4004.25 | 1.130x | +12.96% | - | - | - | 0.736x | +26.42% | 1.161x | -16.13% | 1.124x | -12.43% | 0 |
| generic-sse | 225.25 | 494.25 | 0.456x | -54.43% | - | - | - | 1.240x | -24.03% | 4.137x | -313.67% | 4.708x | -370.84% | 1 |
| https-static-small | 3593.00 | 5248.75 | 0.685x | -31.55% | - | - | - | 1.521x | -52.10% | 1.473x | -47.26% | 1.213x | -21.27% | 0 |
| qcp-transparent | 1838.25 | 3900.75 | 0.471x | -52.87% | - | - | - | 2.298x | -129.83% | 2.126x | -112.59% | 1.947x | -94.74% | 0 |
| reverse-proxy | 5424.00 | 11158.25 | 0.486x | -51.39% | - | - | - | 1.243x | -24.35% | 3.736x | -273.56% | 3.823x | -282.26% | 0 |
| static-large | 116.25 | 84.50 | 1.376x | +37.57% | - | - | - | 0.744x | +25.60% | 0.734x | +26.61% | 0.353x | +64.70% | 0 |
| static-small | 10025.50 | 21287.25 | 0.471x | -52.90% | - | - | - | 0.902x | +9.80% | 3.418x | -241.79% | 2.992x | -199.19% | 0 |
| tcp-stream | 4478.00 | 3841.50 | 1.166x | +16.57% | - | - | - | 0.687x | +31.30% | 1.121x | -12.06% | 1.262x | -26.16% | 0 |
| udp-stream | 1828.00 | 3955.00 | 0.462x | -53.78% | - | - | - | 2.366x | -136.57% | 2.260x | -125.97% | 2.035x | -103.48% | 0 |
| websocket-long-connection | 4319.00 | 3631.25 | 1.189x | +18.94% | - | - | - | 0.675x | +32.50% | 1.127x | -12.68% | 1.272x | -27.17% | 0 |

- Aggregate proxysss ops/s: `46656.75`
- Aggregate nginx ops/s: `79365.75`
- Aggregate proxysss/nginx ratio: `0.588x`
- Aggregate throughput improvement: `-41.21%`
