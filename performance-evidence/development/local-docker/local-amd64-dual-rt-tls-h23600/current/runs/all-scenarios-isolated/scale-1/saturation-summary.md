# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-dual-rt-tls-h23600/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 14262.65 | 19179.40 | 0.744x | -25.64% | - | - | - | 1.623x | -62.27% | 1.189x | -18.88% | 0.941x | +5.89% | 0 |
| game-long-connection | 3983.30 | 3232.70 | 1.232x | +23.22% | - | - | - | 0.863x | +13.67% | 0.749x | +25.06% | 0.665x | +33.50% | 0 |
| generic-sse | 346.70 | 423.10 | 0.819x | -18.06% | - | - | - | 1.005x | -0.54% | 1.509x | -50.87% | 1.243x | -24.30% | 0 |
| https-static-small | 4973.25 | 4845.80 | 1.026x | +2.63% | - | - | - | 1.092x | -9.22% | 0.833x | +16.74% | 0.710x | +29.00% | 0 |
| qcp-transparent | 2684.80 | 3168.60 | 0.847x | -15.27% | - | - | - | 1.095x | -9.50% | 1.282x | -28.16% | 1.041x | -4.06% | 0 |
| reverse-proxy | 8007.15 | 9322.30 | 0.859x | -14.11% | - | - | - | 1.248x | -24.81% | 1.133x | -13.34% | 0.917x | +8.32% | 0 |
| static-large | 108.15 | 78.80 | 1.372x | +37.25% | - | - | - | 0.729x | +27.13% | 0.702x | +29.85% | 0.644x | +35.56% | 0 |
| static-small | 14357.95 | 18924.05 | 0.759x | -24.13% | - | - | - | 1.641x | -64.09% | 1.146x | -14.63% | 0.929x | +7.14% | 0 |
| tcp-stream | 3994.25 | 3206.05 | 1.246x | +24.58% | - | - | - | 0.847x | +15.29% | 0.751x | +24.89% | 0.657x | +34.28% | 0 |
| udp-stream | 2671.30 | 3152.15 | 0.847x | -15.25% | - | - | - | 1.041x | -4.08% | 1.315x | -31.52% | 1.075x | -7.46% | 0 |
| websocket-long-connection | 3756.90 | 3206.35 | 1.172x | +17.17% | - | - | - | 0.909x | +9.06% | 0.789x | +21.13% | 0.688x | +31.24% | 0 |

- Aggregate proxysss ops/s: `59146.40`
- Aggregate nginx ops/s: `68739.30`
- Aggregate proxysss/nginx ratio: `0.860x`
- Aggregate throughput improvement: `-13.96%`
