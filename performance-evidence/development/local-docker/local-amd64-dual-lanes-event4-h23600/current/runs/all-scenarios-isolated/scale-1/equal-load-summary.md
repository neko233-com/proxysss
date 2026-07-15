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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-dual-lanes-event4-h23600/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.961 < 0.980 (actual=3112.75 target=3238.54); game-long-connection nginx target achievement 0.958 < 0.980 (actual=1226.20 target=1280.20); generic-sse nginx target achievement 0.977 < 0.980 (actual=68.35 target=69.96); https-static-small nginx target achievement 0.927 < 0.980 (actual=1848.55 target=1994.02); qcp-transparent nginx target achievement 0.971 < 0.980 (actual=484.15 target=498.72); reverse-proxy nginx target achievement 0.968 < 0.980 (actual=1593.10 target=1644.99); static-small nginx target achievement 0.964 < 0.980 (actual=3064.00 target=3178.70); tcp-stream nginx target achievement 0.955 < 0.980 (actual=1178.95 target=1234.95); udp-stream nginx target achievement 0.969 < 0.980 (actual=510.35 target=526.49); websocket-long-connection nginx target achievement 0.955 < 0.980 (actual=1176.20 target=1231.91)`
| cdn-hot-update | 3207.45 | 3112.75 | 1.030x | +3.04% | 3238.54 | 0.990x | 0.961x | 0.665x | +33.46% | 0.201x | +79.91% | 0.128x | +87.23% | 0 |
| game-long-connection | 1275.30 | 1226.20 | 1.040x | +4.00% | 1280.20 | 0.996x | 0.958x | 0.901x | +9.93% | 0.367x | +63.30% | 0.222x | +77.78% | 0 |
| generic-sse | 69.60 | 68.35 | 1.018x | +1.83% | 69.96 | 0.995x | 0.977x | 0.875x | +12.49% | 0.213x | +78.72% | 0.162x | +83.78% | 0 |
| https-static-small | 1907.95 | 1848.55 | 1.032x | +3.21% | 1994.02 | 0.957x | 0.927x | 0.884x | +11.64% | 0.455x | +54.47% | 0.198x | +80.23% | 0 |
| qcp-transparent | 497.20 | 484.15 | 1.027x | +2.70% | 498.72 | 0.997x | 0.971x | 0.745x | +25.50% | 0.222x | +77.82% | 0.234x | +76.59% | 0 |
| reverse-proxy | 1629.40 | 1593.10 | 1.023x | +2.28% | 1644.99 | 0.991x | 0.968x | 0.782x | +21.81% | 0.188x | +81.22% | 0.140x | +85.95% | 0 |
| static-large | 28.40 | 28.10 | 1.011x | +1.07% | 28.45 | 0.998x | 0.988x | 0.965x | +3.47% | 0.375x | +62.50% | 0.141x | +85.90% | 0 |
| static-small | 3147.20 | 3064.00 | 1.027x | +2.72% | 3178.70 | 0.990x | 0.964x | 0.656x | +34.43% | 0.247x | +75.32% | 0.134x | +86.65% | 0 |
| tcp-stream | 1230.65 | 1178.95 | 1.044x | +4.39% | 1234.95 | 0.997x | 0.955x | 0.918x | +8.16% | 0.357x | +64.32% | 0.169x | +83.06% | 0 |
| udp-stream | 525.20 | 510.35 | 1.029x | +2.91% | 526.49 | 0.998x | 0.969x | 0.781x | +21.92% | 0.243x | +75.72% | 0.221x | +77.90% | 0 |
| websocket-long-connection | 1227.20 | 1176.20 | 1.043x | +4.34% | 1231.91 | 0.996x | 0.955x | 0.877x | +12.26% | 0.408x | +59.18% | 0.255x | +74.54% | 0 |

- Aggregate proxysss ops/s: `14745.55`
- Aggregate nginx ops/s: `14290.70`
- Aggregate proxysss/nginx ratio: `1.032x`
- Aggregate throughput improvement: `+3.18%`
