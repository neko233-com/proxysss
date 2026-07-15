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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-dual-rt-tls-h23600/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.915 < 0.980 (actual=4568.90 target=4991.42); game-long-connection nginx target achievement 0.923 < 0.980 (actual=1044.45 target=1131.38); generic-sse nginx target achievement 0.945 < 0.980 (actual=114.65 target=121.34); https-static-small nginx target achievement 0.883 < 0.980 (actual=1498.30 target=1695.99); qcp-transparent nginx target achievement 0.931 < 0.980 (actual=874.70 target=939.63); reverse-proxy nginx target achievement 0.928 < 0.980 (actual=2600.85 target=2802.35); static-large nginx target achievement 0.977 < 0.980 (actual=26.95 target=27.58); static-small nginx target achievement 0.922 < 0.980 (actual=4631.55 target=5025.13); tcp-stream nginx target achievement 0.927 < 0.980 (actual=1039.65 target=1122.02); udp-stream nginx target achievement 0.931 < 0.980 (actual=870.35 target=934.91); websocket-long-connection nginx target achievement 0.922 < 0.980 (actual=1034.30 target=1122.18)`
| cdn-hot-update | 4778.75 | 4568.90 | 1.046x | +4.59% | 4991.42 | 0.957x | 0.915x | 0.566x | +43.42% | 0.506x | +49.37% | 0.552x | +44.81% | 0 |
| game-long-connection | 1101.40 | 1044.45 | 1.055x | +5.45% | 1131.38 | 0.974x | 0.923x | 0.791x | +20.94% | 0.473x | +52.68% | 0.551x | +44.93% | 0 |
| generic-sse | 118.40 | 114.65 | 1.033x | +3.27% | 121.34 | 0.976x | 0.945x | 0.697x | +30.28% | 0.483x | +51.74% | 0.874x | +12.58% | 0 |
| https-static-small | 1594.70 | 1498.30 | 1.064x | +6.43% | 1695.99 | 0.940x | 0.883x | 0.764x | +23.63% | 0.490x | +50.97% | 0.487x | +51.27% | 0 |
| qcp-transparent | 911.00 | 874.70 | 1.041x | +4.15% | 939.63 | 0.970x | 0.931x | 0.643x | +35.74% | 0.563x | +43.74% | 0.644x | +35.61% | 0 |
| reverse-proxy | 2707.30 | 2600.85 | 1.041x | +4.09% | 2802.35 | 0.966x | 0.928x | 0.625x | +37.49% | 0.470x | +53.02% | 0.724x | +27.55% | 0 |
| static-large | 27.50 | 26.95 | 1.020x | +2.04% | 27.58 | 0.997x | 0.977x | 0.841x | +15.91% | 0.381x | +61.91% | 0.611x | +38.89% | 0 |
| static-small | 4834.50 | 4631.55 | 1.044x | +4.38% | 5025.13 | 0.962x | 0.922x | 0.578x | +42.15% | 0.488x | +51.17% | 0.617x | +38.33% | 0 |
| tcp-stream | 1093.15 | 1039.65 | 1.051x | +5.15% | 1122.02 | 0.974x | 0.927x | 0.810x | +19.05% | 0.513x | +48.73% | 0.655x | +34.51% | 0 |
| udp-stream | 907.30 | 870.35 | 1.042x | +4.25% | 934.91 | 0.970x | 0.931x | 0.604x | +39.55% | 0.503x | +49.71% | 0.678x | +32.24% | 0 |
| websocket-long-connection | 1094.10 | 1034.30 | 1.058x | +5.78% | 1122.18 | 0.975x | 0.922x | 0.768x | +23.18% | 0.476x | +52.37% | 0.558x | +44.24% | 0 |

- Aggregate proxysss ops/s: `19168.10`
- Aggregate nginx ops/s: `18304.65`
- Aggregate proxysss/nginx ratio: `1.047x`
- Aggregate throughput improvement: `+4.72%`
