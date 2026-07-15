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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T015446Z-aa565081c62d/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.977 < 0.980 (actual=21.00 target=21.50)`
| cdn-hot-update | 4829.50 | 4826.50 | 1.001x | +0.06% | 4831.65 | 1.000x | 0.999x | 0.831x | +16.88% | 0.869x | +13.08% | 1.024x | -2.42% | 0 |
| game-long-connection | 644.00 | 644.00 | 1.000x | +0.00% | 648.25 | 0.993x | 0.993x | 1.012x | -1.22% | 1.155x | -15.53% | 1.612x | -61.17% | 0 |
| generic-sse | 82.50 | 82.50 | 1.000x | +0.00% | 82.87 | 0.996x | 0.996x | 0.999x | +0.14% | 1.024x | -2.44% | 2.156x | -115.62% | 0 |
| https-static-small | 764.50 | 765.00 | 0.999x | -0.07% | 765.62 | 0.999x | 0.999x | 1.037x | -3.73% | 1.214x | -21.43% | 1.483x | -48.28% | 0 |
| qcp-transparent | 676.00 | 676.00 | 1.000x | +0.00% | 678.48 | 0.996x | 0.996x | 0.897x | +10.29% | 1.122x | -12.19% | 2.123x | -112.32% | 0 |
| reverse-proxy | 1672.00 | 1672.00 | 1.000x | +0.00% | 1673.55 | 0.999x | 0.999x | 0.937x | +6.29% | 0.982x | +1.84% | 1.617x | -61.73% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 21.50 | 0.977x | 0.977x | 0.956x | +4.37% | 1.099x | -9.88% | 0.708x | +29.22% | 0 |
| static-small | 4040.00 | 4041.00 | 1.000x | -0.02% | 4041.94 | 1.000x | 1.000x | 0.828x | +17.22% | 1.000x | +0.00% | 0.734x | +26.59% | 0 |
| tcp-stream | 724.00 | 724.00 | 1.000x | +0.00% | 725.62 | 0.998x | 0.998x | 1.008x | -0.84% | 0.985x | +1.50% | 2.989x | -198.94% | 0 |
| udp-stream | 548.00 | 548.00 | 1.000x | +0.00% | 551.34 | 0.994x | 0.994x | 0.911x | +8.94% | 1.305x | -30.45% | 3.121x | -212.08% | 0 |
| websocket-long-connection | 592.00 | 592.00 | 1.000x | +0.00% | 592.59 | 0.999x | 0.999x | 0.987x | +1.33% | 1.391x | -39.13% | 0.926x | +7.40% | 0 |

- Aggregate proxysss ops/s: `14593.50`
- Aggregate nginx ops/s: `14592.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
