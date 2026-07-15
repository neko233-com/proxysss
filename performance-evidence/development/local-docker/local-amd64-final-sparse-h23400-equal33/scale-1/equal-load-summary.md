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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-final-sparse-h23400-equal33/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.975 < 0.980 (actual=1297.15 target=1330.23)`
| cdn-hot-update | 5485.55 | 5467.65 | 1.003x | +0.33% | 5548.81 | 0.989x | 0.985x | 0.758x | +24.21% | 0.800x | +20.02% | 0.744x | +25.61% | 0 |
| game-long-connection | 977.60 | 976.80 | 1.001x | +0.08% | 980.51 | 0.997x | 0.996x | 0.995x | +0.49% | 0.956x | +4.41% | 0.943x | +5.74% | 0 |
| generic-sse | 126.00 | 126.00 | 1.000x | +0.00% | 126.91 | 0.993x | 0.993x | 0.974x | +2.56% | 0.908x | +9.21% | 0.757x | +24.35% | 0 |
| https-static-small | 1294.25 | 1297.15 | 0.998x | -0.22% | 1330.23 | 0.973x | 0.975x | 0.986x | +1.39% | 0.951x | +4.86% | 0.751x | +24.85% | 0 |
| qcp-transparent | 951.40 | 951.55 | 1.000x | -0.02% | 955.91 | 0.995x | 0.995x | 0.873x | +12.73% | 0.852x | +14.81% | 1.119x | -11.87% | 0 |
| reverse-proxy | 2849.70 | 2820.30 | 1.010x | +1.04% | 2874.60 | 0.991x | 0.981x | 0.890x | +11.04% | 0.882x | +11.84% | 0.877x | +12.33% | 0 |
| static-large | 24.50 | 24.55 | 0.998x | -0.20% | 24.60 | 0.996x | 0.998x | 0.983x | +1.74% | 0.994x | +0.62% | 0.767x | +23.28% | 0 |
| static-small | 5397.95 | 5403.80 | 0.999x | -0.11% | 5469.15 | 0.987x | 0.988x | 0.779x | +22.09% | 0.797x | +20.26% | 0.753x | +24.73% | 0 |
| tcp-stream | 991.60 | 990.00 | 1.002x | +0.16% | 994.53 | 0.997x | 0.995x | 1.002x | -0.17% | 1.012x | -1.18% | 1.050x | -4.99% | 0 |
| udp-stream | 940.90 | 940.80 | 1.000x | +0.01% | 944.29 | 0.996x | 0.996x | 0.892x | +10.79% | 0.864x | +13.58% | 1.190x | -18.96% | 0 |
| websocket-long-connection | 991.75 | 989.60 | 1.002x | +0.22% | 995.15 | 0.997x | 0.994x | 0.997x | +0.29% | 1.067x | -6.70% | 0.848x | +15.19% | 0 |

- Aggregate proxysss ops/s: `20031.20`
- Aggregate nginx ops/s: `19988.20`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.22%`
