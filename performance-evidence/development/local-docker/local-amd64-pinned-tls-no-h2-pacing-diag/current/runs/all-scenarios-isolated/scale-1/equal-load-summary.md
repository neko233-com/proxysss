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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-pinned-tls-no-h2-pacing-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.958 < 0.980 (actual=7423.25 target=7746.31); generic-sse nginx target achievement 0.974 < 0.980 (actual=181.50 target=186.36); https-static-small nginx target achievement 0.924 < 0.980 (actual=1761.00 target=1906.58); qcp-transparent nginx target achievement 0.978 < 0.980 (actual=1310.75 target=1339.81); reverse-proxy nginx target achievement 0.964 < 0.980 (actual=4034.50 target=4184.10); static-small nginx target achievement 0.956 < 0.980 (actual=7781.00 target=8136.28)`
| cdn-hot-update | 7468.75 | 7423.25 | 1.006x | +0.61% | 7746.31 | 0.964x | 0.958x | 0.837x | +16.27% | 0.892x | +10.79% | 0.853x | +14.68% | 0 |
| game-long-connection | 1178.00 | 1159.75 | 1.016x | +1.57% | 1182.56 | 0.996x | 0.981x | 1.168x | -16.84% | 1.251x | -25.05% | 1.212x | -21.23% | 0 |
| generic-sse | 182.75 | 181.50 | 1.007x | +0.69% | 186.36 | 0.981x | 0.974x | 1.007x | -0.67% | 0.845x | +15.49% | 0.531x | +46.95% | 0 |
| https-static-small | 1688.75 | 1761.00 | 0.959x | -4.10% | 1906.58 | 0.886x | 0.924x | 1.164x | -16.40% | 0.911x | +8.89% | 0.687x | +31.31% | 0 |
| qcp-transparent | 1324.00 | 1310.75 | 1.010x | +1.01% | 1339.81 | 0.988x | 0.978x | 0.987x | +1.30% | 1.109x | -10.90% | 0.896x | +10.39% | 0 |
| reverse-proxy | 4055.50 | 4034.50 | 1.005x | +0.52% | 4184.10 | 0.969x | 0.964x | 0.976x | +2.41% | 0.866x | +13.37% | 0.604x | +39.60% | 0 |
| static-large | 40.50 | 40.50 | 1.000x | +0.00% | 40.75 | 0.994x | 0.994x | 0.973x | +2.73% | 0.561x | +43.85% | 0.955x | +4.53% | 0 |
| static-small | 7772.75 | 7781.00 | 0.999x | -0.11% | 8136.28 | 0.955x | 0.956x | 0.836x | +16.41% | 0.954x | +4.62% | 0.681x | +31.91% | 0 |
| tcp-stream | 1166.00 | 1149.00 | 1.015x | +1.48% | 1170.96 | 0.996x | 0.981x | 1.229x | -22.94% | 1.051x | -5.10% | 0.983x | +1.65% | 0 |
| udp-stream | 1292.00 | 1282.00 | 1.008x | +0.78% | 1307.83 | 0.988x | 0.980x | 0.932x | +6.80% | 1.134x | -13.43% | 0.812x | +18.85% | 0 |
| websocket-long-connection | 1092.75 | 1077.00 | 1.015x | +1.46% | 1097.24 | 0.996x | 0.982x | 1.174x | -17.35% | 1.007x | -0.73% | 0.904x | +9.63% | 0 |

- Aggregate proxysss ops/s: `27261.75`
- Aggregate nginx ops/s: `27200.25`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.23%`
