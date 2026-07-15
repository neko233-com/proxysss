# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-final-h23800-1x2x4x/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.974 < 0.980 (actual=1063.10 target=1092.00)`
| cdn-hot-update | 4212.50 | 4207.75 | 1.001x | +0.11% | 4259.85 | 0.989x | 0.988x | 0.743x | +25.67% | 0.794x | +20.56% | 0.805x | +19.49% | 0 |
| game-long-connection | 822.95 | 824.40 | 0.998x | -0.18% | 825.93 | 0.996x | 0.998x | 1.015x | -1.48% | 1.020x | -1.96% | 0.997x | +0.30% | 0 |
| generic-sse | 104.45 | 104.70 | 0.998x | -0.24% | 105.31 | 0.992x | 0.994x | 0.978x | +2.22% | 0.915x | +8.53% | 1.001x | -0.06% | 0 |
| https-static-small | 1061.15 | 1063.10 | 0.998x | -0.18% | 1092.00 | 0.972x | 0.974x | 0.986x | +1.43% | 0.980x | +2.02% | 0.925x | +7.48% | 0 |
| qcp-transparent | 824.60 | 827.15 | 0.997x | -0.31% | 828.59 | 0.995x | 0.998x | 0.868x | +13.17% | 0.885x | +11.53% | 0.976x | +2.41% | 0 |
| reverse-proxy | 2316.90 | 2315.35 | 1.001x | +0.07% | 2341.41 | 0.990x | 0.989x | 0.908x | +9.20% | 0.881x | +11.88% | 0.901x | +9.94% | 0 |
| static-large | 18.50 | 18.50 | 1.000x | +0.00% | 18.52 | 0.999x | 0.999x | 1.001x | -0.12% | 0.982x | +1.84% | 1.370x | -37.02% | 0 |
| static-small | 4241.50 | 4248.75 | 0.998x | -0.17% | 4290.69 | 0.989x | 0.990x | 0.760x | +23.96% | 0.797x | +20.32% | 0.822x | +17.81% | 0 |
| tcp-stream | 795.20 | 796.80 | 0.998x | -0.20% | 798.32 | 0.996x | 0.998x | 0.980x | +2.04% | 1.059x | -5.91% | 1.297x | -29.68% | 0 |
| udp-stream | 827.65 | 830.20 | 0.997x | -0.31% | 832.03 | 0.995x | 0.998x | 0.892x | +10.80% | 0.845x | +15.46% | 0.908x | +9.18% | 0 |
| websocket-long-connection | 761.60 | 762.95 | 0.998x | -0.18% | 764.16 | 0.997x | 0.998x | 0.994x | +0.64% | 1.069x | -6.88% | 1.267x | -26.74% | 0 |

- Aggregate proxysss ops/s: `15987.00`
- Aggregate nginx ops/s: `15999.65`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.08%`
