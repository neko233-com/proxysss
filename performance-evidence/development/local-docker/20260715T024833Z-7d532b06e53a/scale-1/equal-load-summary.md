# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T024833Z-7d532b06e53a/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.958 < 0.980 (actual=20.00 target=20.87)`
| cdn-hot-update | 6227.00 | 6229.50 | 1.000x | -0.04% | 6239.03 | 0.998x | 0.998x | 0.802x | +19.84% | 0.645x | +35.47% | 0.328x | +67.24% | 0 |
| game-long-connection | 1032.00 | 1032.00 | 1.000x | +0.00% | 1037.21 | 0.995x | 0.995x | 1.044x | -4.37% | 0.730x | +27.03% | 0.663x | +33.74% | 0 |
| generic-sse | 109.50 | 109.50 | 1.000x | +0.00% | 110.12 | 0.994x | 0.994x | 0.914x | +8.64% | 0.661x | +33.91% | 0.832x | +16.77% | 0 |
| https-static-small | 1174.50 | 1175.50 | 0.999x | -0.09% | 1176.47 | 0.998x | 0.999x | 0.902x | +9.79% | 0.641x | +35.87% | 0.256x | +74.39% | 0 |
| qcp-transparent | 1016.00 | 1016.00 | 1.000x | +0.00% | 1017.68 | 0.998x | 0.998x | 0.832x | +16.76% | 0.541x | +45.90% | 0.335x | +66.48% | 0 |
| reverse-proxy | 2840.50 | 2842.00 | 0.999x | -0.05% | 2845.46 | 0.998x | 0.999x | 0.930x | +7.00% | 0.406x | +59.41% | 0.788x | +21.18% | 0 |
| static-large | 20.00 | 20.00 | 1.000x | +0.00% | 20.87 | 0.958x | 0.958x | 0.948x | +5.17% | 0.501x | +49.93% | 0.888x | +11.23% | 0 |
| static-small | 6450.00 | 6450.50 | 1.000x | -0.01% | 6458.12 | 0.999x | 0.999x | 0.794x | +20.57% | 0.728x | +27.18% | 0.550x | +45.00% | 0 |
| tcp-stream | 992.00 | 992.00 | 1.000x | +0.00% | 995.40 | 0.997x | 0.997x | 1.004x | -0.36% | 0.680x | +32.05% | 0.297x | +70.27% | 0 |
| udp-stream | 944.00 | 944.00 | 1.000x | +0.00% | 947.19 | 0.997x | 0.997x | 0.838x | +16.22% | 0.628x | +37.23% | 0.306x | +69.39% | 0 |
| websocket-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 868.62 | 0.995x | 0.995x | 0.998x | +0.15% | 0.631x | +36.85% | 0.808x | +19.22% | 0 |

- Aggregate proxysss ops/s: `21669.50`
- Aggregate nginx ops/s: `21675.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.03%`
