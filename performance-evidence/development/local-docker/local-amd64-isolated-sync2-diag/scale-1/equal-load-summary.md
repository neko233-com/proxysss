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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-sync2-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.829 < 0.980 (actual=5967.50 target=7199.10); game-long-connection nginx target achievement 0.830 < 0.980 (actual=2326.00 target=2802.10); generic-sse nginx target achievement 0.897 < 0.980 (actual=141.50 target=157.67); https-static-small nginx target achievement 0.777 < 0.980 (actual=1955.00 target=2514.93); qcp-transparent nginx target achievement 0.883 < 0.980 (actual=1136.00 target=1286.59); reverse-proxy nginx target achievement 0.870 < 0.980 (actual=3303.25 target=3796.42); static-large nginx target achievement 0.921 < 0.980 (actual=54.50 target=59.15); static-small nginx target achievement 0.824 < 0.980 (actual=5779.00 target=7017.54); tcp-stream nginx target achievement 0.828 < 0.980 (actual=2225.00 target=2688.17); udp-stream nginx target achievement 0.867 < 0.980 (actual=1109.00 target=1279.59); websocket-long-connection nginx target achievement 0.816 < 0.980 (actual=2073.25 target=2541.30)`
| cdn-hot-update | 6982.50 | 5967.50 | 1.170x | +17.01% | 7199.10 | 0.970x | 0.829x | 0.517x | +48.32% | 0.200x | +80.03% | 0.187x | +81.27% | 0 |
| game-long-connection | 2792.50 | 2326.00 | 1.201x | +20.06% | 2802.10 | 0.997x | 0.830x | 0.620x | +37.96% | 0.306x | +69.35% | 0.255x | +74.50% | 0 |
| generic-sse | 156.25 | 141.50 | 1.104x | +10.42% | 157.67 | 0.991x | 0.897x | 0.620x | +38.04% | 0.221x | +77.86% | 0.294x | +70.55% | 1 |
| https-static-small | 2236.25 | 1955.00 | 1.144x | +14.39% | 2514.93 | 0.889x | 0.777x | 0.730x | +27.03% | 0.275x | +72.49% | 0.100x | +90.00% | 0 |
| qcp-transparent | 1286.00 | 1136.00 | 1.132x | +13.20% | 1286.59 | 1.000x | 0.883x | 0.627x | +37.34% | 0.233x | +76.69% | 0.250x | +74.97% | 0 |
| reverse-proxy | 3697.00 | 3303.25 | 1.119x | +11.92% | 3796.42 | 0.974x | 0.870x | 0.492x | +50.83% | 0.202x | +79.78% | 0.173x | +82.69% | 0 |
| static-large | 59.00 | 54.50 | 1.083x | +8.26% | 59.15 | 0.997x | 0.921x | 0.859x | +14.13% | 0.178x | +82.17% | 0.508x | +49.24% | 0 |
| static-small | 6845.50 | 5779.00 | 1.185x | +18.45% | 7017.54 | 0.975x | 0.824x | 0.491x | +50.95% | 0.238x | +76.18% | 0.169x | +83.06% | 0 |
| tcp-stream | 2676.00 | 2225.00 | 1.203x | +20.27% | 2688.17 | 0.995x | 0.828x | 0.646x | +35.38% | 0.323x | +67.71% | 0.197x | +80.32% | 0 |
| udp-stream | 1274.00 | 1109.00 | 1.149x | +14.88% | 1279.59 | 0.996x | 0.867x | 0.676x | +32.38% | 0.254x | +74.59% | 0.159x | +84.13% | 0 |
| websocket-long-connection | 2535.50 | 2073.25 | 1.223x | +22.30% | 2541.30 | 0.998x | 0.816x | 0.678x | +32.19% | 0.290x | +70.96% | 0.196x | +80.42% | 0 |

- Aggregate proxysss ops/s: `30540.50`
- Aggregate nginx ops/s: `26070.00`
- Aggregate proxysss/nginx ratio: `1.171x`
- Aggregate throughput improvement: `+17.15%`
