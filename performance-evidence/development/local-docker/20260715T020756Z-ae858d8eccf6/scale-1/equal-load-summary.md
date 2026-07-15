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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020756Z-ae858d8eccf6/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5707.50 | 5709.50 | 1.000x | -0.04% | 5713.27 | 0.999x | 0.999x | 0.822x | +17.75% | 1.007x | -0.72% | 1.373x | -37.32% | 0 |
| game-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 899.18 | 0.996x | 0.996x | 1.047x | -4.67% | 1.017x | -1.66% | 1.027x | -2.71% | 0 |
| generic-sse | 109.00 | 109.00 | 1.000x | +0.00% | 109.50 | 0.995x | 0.995x | 1.014x | -1.40% | 1.489x | -48.92% | 1.195x | -19.54% | 0 |
| https-static-small | 1077.00 | 1077.50 | 1.000x | -0.05% | 1078.46 | 0.999x | 0.999x | 1.085x | -8.47% | 1.478x | -47.79% | 1.518x | -51.79% | 0 |
| qcp-transparent | 824.00 | 824.00 | 1.000x | +0.00% | 825.68 | 0.998x | 0.998x | 0.909x | +9.12% | 1.407x | -40.74% | 1.927x | -92.68% | 0 |
| reverse-proxy | 2189.00 | 2190.00 | 1.000x | -0.05% | 2190.73 | 0.999x | 1.000x | 1.000x | +0.00% | 1.912x | -91.20% | 1.541x | -54.10% | 0 |
| static-large | 29.50 | 29.50 | 1.000x | +0.00% | 29.62 | 0.996x | 0.996x | 0.968x | +3.19% | 0.943x | +5.66% | 1.081x | -8.14% | 0 |
| static-small | 5777.50 | 5778.50 | 1.000x | -0.02% | 5781.39 | 0.999x | 1.000x | 0.838x | +16.18% | 0.963x | +3.70% | 1.642x | -64.23% | 0 |
| tcp-stream | 876.00 | 876.00 | 1.000x | +0.00% | 877.19 | 0.999x | 0.999x | 1.027x | -2.66% | 1.406x | -40.55% | 0.790x | +21.01% | 0 |
| udp-stream | 840.00 | 844.00 | 0.995x | -0.47% | 844.42 | 0.995x | 1.000x | 0.917x | +8.33% | 1.233x | -23.34% | 2.928x | -192.77% | 0 |
| websocket-long-connection | 836.00 | 836.00 | 1.000x | +0.00% | 837.35 | 0.998x | 0.998x | 1.043x | -4.30% | 1.242x | -24.25% | 1.669x | -66.89% | 0 |

- Aggregate proxysss ops/s: `19161.50`
- Aggregate nginx ops/s: `19170.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.04%`
