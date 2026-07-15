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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-shared-data-runtime-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.945 < 0.980 (actual=13107.25 target=13876.84); generic-sse nginx target achievement 0.965 < 0.980 (actual=330.25 target=342.29); https-static-small nginx target achievement 0.919 < 0.980 (actual=2270.25 target=2471.42); qcp-transparent nginx target achievement 0.967 < 0.980 (actual=2776.25 target=2871.50); reverse-proxy nginx target achievement 0.943 < 0.980 (actual=7545.75 target=8004.00); static-small nginx target achievement 0.943 < 0.980 (actual=13121.25 target=13907.00); udp-stream nginx target achievement 0.973 < 0.980 (actual=2726.75 target=2802.10); websocket-long-connection nginx target achievement 0.978 < 0.980 (actual=1565.00 target=1600.32)`
| cdn-hot-update | 13101.00 | 13107.25 | 1.000x | -0.05% | 13876.84 | 0.944x | 0.945x | 0.746x | +25.41% | 0.708x | +29.24% | 0.792x | +20.75% | 0 |
| game-long-connection | 1652.00 | 1625.50 | 1.016x | +1.63% | 1655.97 | 0.998x | 0.982x | 1.290x | -29.02% | 1.030x | -3.01% | 0.813x | +18.67% | 0 |
| generic-sse | 329.25 | 330.25 | 0.997x | -0.30% | 342.29 | 0.962x | 0.965x | 0.858x | +14.16% | 0.749x | +25.10% | 0.820x | +17.97% | 1 |
| https-static-small | 2271.00 | 2270.25 | 1.000x | +0.03% | 2471.42 | 0.919x | 0.919x | 0.835x | +16.47% | 0.870x | +13.04% | 0.802x | +19.77% | 0 |
| qcp-transparent | 2767.25 | 2776.25 | 0.997x | -0.32% | 2871.50 | 0.964x | 0.967x | 0.822x | +17.78% | 0.804x | +19.61% | 0.894x | +10.61% | 0 |
| reverse-proxy | 7575.25 | 7545.75 | 1.004x | +0.39% | 8004.00 | 0.946x | 0.943x | 0.793x | +20.74% | 0.694x | +30.56% | 0.668x | +33.22% | 0 |
| static-large | 58.75 | 58.50 | 1.004x | +0.43% | 58.97 | 0.996x | 0.992x | 0.892x | +10.81% | 0.620x | +38.00% | 0.882x | +11.78% | 0 |
| static-small | 13130.75 | 13121.25 | 1.001x | +0.07% | 13907.00 | 0.944x | 0.943x | 0.748x | +25.18% | 0.712x | +28.83% | 0.814x | +18.61% | 0 |
| tcp-stream | 1735.25 | 1711.00 | 1.014x | +1.42% | 1743.30 | 0.995x | 0.981x | 1.311x | -31.12% | 1.058x | -5.75% | 0.837x | +16.25% | 0 |
| udp-stream | 2703.50 | 2726.75 | 0.991x | -0.85% | 2802.10 | 0.965x | 0.973x | 0.802x | +19.80% | 0.721x | +27.92% | 0.882x | +11.81% | 0 |
| websocket-long-connection | 1594.00 | 1565.00 | 1.019x | +1.85% | 1600.32 | 0.996x | 0.978x | 1.195x | -19.49% | 0.991x | +0.88% | 0.858x | +14.16% | 0 |

- Aggregate proxysss ops/s: `46918.00`
- Aggregate nginx ops/s: `46837.75`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.17%`
