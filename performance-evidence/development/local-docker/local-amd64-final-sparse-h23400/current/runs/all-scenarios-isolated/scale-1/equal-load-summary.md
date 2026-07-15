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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-final-sparse-h23400/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.970 < 0.980 (actual=6921.40 target=7136.49); https-static-small nginx target achievement 0.961 < 0.980 (actual=1834.60 target=1908.40); reverse-proxy nginx target achievement 0.976 < 0.980 (actual=3882.15 target=3976.14); static-small nginx target achievement 0.974 < 0.980 (actual=7049.10 target=7238.18)`
| cdn-hot-update | 7044.70 | 6921.40 | 1.018x | +1.78% | 7136.49 | 0.987x | 0.970x | 0.645x | +35.48% | 0.292x | +70.82% | 0.199x | +80.06% | 0 |
| game-long-connection | 1283.65 | 1263.95 | 1.016x | +1.56% | 1284.93 | 0.999x | 0.984x | 0.875x | +12.53% | 0.346x | +65.44% | 0.369x | +63.06% | 0 |
| generic-sse | 159.45 | 158.50 | 1.006x | +0.60% | 160.31 | 0.995x | 0.989x | 0.827x | +17.25% | 0.309x | +69.06% | 0.271x | +72.88% | 0 |
| https-static-small | 1851.75 | 1834.60 | 1.009x | +0.93% | 1908.40 | 0.970x | 0.961x | 0.880x | +11.99% | 0.406x | +59.37% | 0.270x | +72.95% | 0 |
| qcp-transparent | 1280.55 | 1261.55 | 1.015x | +1.51% | 1283.90 | 0.997x | 0.983x | 0.733x | +26.69% | 0.303x | +69.72% | 0.275x | +72.46% | 0 |
| reverse-proxy | 3931.75 | 3882.15 | 1.013x | +1.28% | 3976.14 | 0.989x | 0.976x | 0.738x | +26.19% | 0.273x | +72.71% | 0.209x | +79.11% | 0 |
| static-large | 31.00 | 31.00 | 1.000x | +0.00% | 31.01 | 1.000x | 1.000x | 0.974x | +2.59% | 0.661x | +33.90% | 0.399x | +60.15% | 0 |
| static-small | 7154.70 | 7049.10 | 1.015x | +1.50% | 7238.18 | 0.988x | 0.974x | 0.662x | +33.78% | 0.293x | +70.67% | 0.216x | +78.40% | 0 |
| tcp-stream | 1254.80 | 1235.80 | 1.015x | +1.54% | 1255.89 | 0.999x | 0.984x | 0.866x | +13.37% | 0.364x | +63.61% | 0.277x | +72.31% | 0 |
| udp-stream | 1301.80 | 1282.55 | 1.015x | +1.50% | 1304.84 | 0.998x | 0.983x | 0.702x | +29.83% | 0.288x | +71.21% | 0.292x | +70.79% | 0 |
| websocket-long-connection | 1251.20 | 1232.90 | 1.015x | +1.48% | 1252.35 | 0.999x | 0.984x | 0.858x | +14.24% | 0.387x | +61.29% | 0.370x | +63.03% | 0 |

- Aggregate proxysss ops/s: `26545.35`
- Aggregate nginx ops/s: `26153.50`
- Aggregate proxysss/nginx ratio: `1.015x`
- Aggregate throughput improvement: `+1.50%`
