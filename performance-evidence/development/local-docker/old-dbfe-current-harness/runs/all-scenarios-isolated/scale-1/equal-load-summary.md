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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/old-dbfe-current-harness/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.964 < 0.980 (actual=6730.25 target=6983.85); https-static-small nginx target achievement 0.940 < 0.980 (actual=1649.00 target=1754.39); reverse-proxy nginx target achievement 0.968 < 0.980 (actual=3559.75 target=3676.47); static-small nginx target achievement 0.971 < 0.980 (actual=7021.50 target=7228.37)`
| cdn-hot-update | 6595.25 | 6730.25 | 0.980x | -2.01% | 6983.85 | 0.944x | 0.964x | 0.865x | +13.54% | 1.421x | -42.07% | 2.318x | -131.81% | 0 |
| game-long-connection | 1710.25 | 1767.75 | 0.967x | -3.25% | 1776.99 | 0.962x | 0.995x | 1.121x | -12.14% | 1.469x | -46.88% | 1.880x | -88.02% | 0 |
| generic-sse | 159.50 | 161.25 | 0.989x | -1.09% | 164.37 | 0.970x | 0.981x | 1.022x | -2.23% | 1.611x | -61.13% | 3.344x | -234.38% | 1 |
| https-static-small | 1540.50 | 1649.00 | 0.934x | -6.58% | 1754.39 | 0.878x | 0.940x | 1.107x | -10.74% | 1.262x | -26.21% | 1.984x | -98.36% | 0 |
| qcp-transparent | 1152.00 | 1188.00 | 0.970x | -3.03% | 1195.46 | 0.964x | 0.994x | 1.019x | -1.94% | 1.721x | -72.11% | 3.506x | -250.64% | 0 |
| reverse-proxy | 3478.50 | 3559.75 | 0.977x | -2.28% | 3676.47 | 0.946x | 0.968x | 0.986x | +1.39% | 1.405x | -40.45% | 4.806x | -380.63% | 0 |
| static-large | 40.00 | 40.00 | 1.000x | +0.00% | 40.12 | 0.997x | 0.997x | 0.985x | +1.49% | 2.199x | -119.87% | 1.114x | -11.43% | 0 |
| static-small | 6857.50 | 7021.50 | 0.977x | -2.34% | 7228.37 | 0.949x | 0.971x | 0.855x | +14.54% | 1.356x | -35.61% | 3.121x | -212.11% | 0 |
| tcp-stream | 1708.25 | 1756.00 | 0.973x | -2.72% | 1766.78 | 0.967x | 0.994x | 1.144x | -14.41% | 1.409x | -40.91% | 2.201x | -120.09% | 0 |
| udp-stream | 1191.00 | 1228.00 | 0.970x | -3.01% | 1233.05 | 0.966x | 0.996x | 0.990x | +0.96% | 1.767x | -76.67% | 2.550x | -155.00% | 0 |
| websocket-long-connection | 1576.00 | 1625.00 | 0.970x | -3.02% | 1637.33 | 0.963x | 0.992x | 1.140x | -13.95% | 1.225x | -22.50% | 2.206x | -120.60% | 0 |

- Aggregate proxysss ops/s: `26008.75`
- Aggregate nginx ops/s: `26726.50`
- Aggregate proxysss/nginx ratio: `0.973x`
- Aggregate throughput improvement: `-2.69%`
