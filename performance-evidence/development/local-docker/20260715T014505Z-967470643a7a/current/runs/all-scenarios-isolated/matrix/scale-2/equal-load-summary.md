# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T014505Z-967470643a7a/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 3939.00 | 3939.00 | 1.000x | +0.00% | 3942.34 | 0.999x | 0.999x | 0.793x | +20.71% | 1.228x | -22.79% | 2.681x | -168.08% | 0 |
| game-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 898.22 | 0.998x | 0.998x | 0.943x | +5.67% | 0.925x | +7.53% | 2.111x | -111.07% | 0 |
| generic-sse | 112.00 | 112.00 | 1.000x | +0.00% | 112.50 | 0.996x | 0.996x | 0.949x | +5.12% | 1.054x | -5.40% | 1.279x | -27.92% | 0 |
| https-static-small | 671.50 | 671.50 | 1.000x | +0.00% | 672.24 | 0.999x | 0.999x | 1.030x | -2.97% | 1.281x | -28.10% | 1.188x | -18.80% | 0 |
| qcp-transparent | 768.00 | 768.00 | 1.000x | +0.00% | 772.50 | 0.994x | 0.994x | 0.874x | +12.59% | 1.504x | -50.35% | 3.524x | -252.40% | 0 |
| reverse-proxy | 2071.50 | 2072.50 | 1.000x | -0.05% | 2072.74 | 0.999x | 1.000x | 0.925x | +7.47% | 2.685x | -168.51% | 1.804x | -80.45% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.12 | 0.995x | 0.995x | 0.979x | +2.09% | 1.839x | -83.87% | 1.499x | -49.91% | 0 |
| static-small | 4158.50 | 4160.50 | 1.000x | -0.05% | 4162.60 | 0.999x | 0.999x | 0.811x | +18.88% | 1.363x | -36.32% | 3.185x | -218.53% | 0 |
| tcp-stream | 816.00 | 816.00 | 1.000x | +0.00% | 822.50 | 0.992x | 0.992x | 0.958x | +4.24% | 1.709x | -70.92% | 2.011x | -101.08% | 0 |
| udp-stream | 776.00 | 776.00 | 1.000x | +0.00% | 779.84 | 0.995x | 0.995x | 0.887x | +11.27% | 1.226x | -22.63% | 3.242x | -224.23% | 0 |
| websocket-long-connection | 712.00 | 712.00 | 1.000x | +0.00% | 717.84 | 0.992x | 0.992x | 0.939x | +6.06% | 1.882x | -88.17% | 2.308x | -130.85% | 0 |

- Aggregate proxysss ops/s: `14942.50`
- Aggregate nginx ops/s: `14945.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
