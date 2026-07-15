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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T020038Z-068de4cadd02/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6076.00 | 6069.00 | 1.001x | +0.12% | 6081.34 | 0.999x | 0.998x | 0.864x | +13.56% | 1.346x | -34.58% | 0.892x | +10.82% | 0 |
| game-long-connection | 944.00 | 944.00 | 1.000x | +0.00% | 948.43 | 0.995x | 0.995x | 1.156x | -15.62% | 5.276x | -427.64% | 1.972x | -97.19% | 0 |
| generic-sse | 113.00 | 114.00 | 0.991x | -0.88% | 114.25 | 0.989x | 0.998x | 0.986x | +1.39% | 1.493x | -49.33% | 1.333x | -33.28% | 0 |
| https-static-small | 779.00 | 778.00 | 1.001x | +0.13% | 779.96 | 0.999x | 0.997x | 1.008x | -0.75% | 1.430x | -43.02% | 0.593x | +40.72% | 0 |
| qcp-transparent | 920.00 | 920.00 | 1.000x | +0.00% | 921.98 | 0.998x | 0.998x | 1.000x | +0.00% | 3.254x | -225.36% | 0.898x | +10.22% | 0 |
| reverse-proxy | 2113.00 | 2118.00 | 0.998x | -0.24% | 2119.91 | 0.997x | 0.999x | 1.093x | -9.35% | 2.161x | -116.12% | 1.705x | -70.50% | 0 |
| static-large | 25.00 | 25.00 | 1.000x | +0.00% | 25.50 | 0.980x | 0.980x | 0.948x | +5.21% | 2.152x | -115.21% | 2.423x | -142.28% | 0 |
| static-small | 5232.00 | 5233.00 | 1.000x | -0.02% | 5240.75 | 0.998x | 0.999x | 0.879x | +12.12% | 3.247x | -224.72% | 0.956x | +4.43% | 0 |
| tcp-stream | 1016.00 | 1016.00 | 1.000x | +0.00% | 1019.89 | 0.996x | 0.996x | 1.042x | -4.15% | 4.492x | -349.15% | 1.984x | -98.40% | 0 |
| udp-stream | 872.00 | 872.00 | 1.000x | +0.00% | 880.18 | 0.991x | 0.991x | 0.900x | +9.97% | 4.287x | -328.75% | 2.696x | -169.59% | 0 |
| websocket-long-connection | 1048.00 | 1048.00 | 1.000x | +0.00% | 1054.44 | 0.994x | 0.994x | 1.039x | -3.92% | 4.012x | -301.19% | 1.142x | -14.15% | 0 |

- Aggregate proxysss ops/s: `19138.00`
- Aggregate nginx ops/s: `19137.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
