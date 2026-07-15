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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-soft-h23200-equal35/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `https-static-small nginx target achievement 0.976 < 0.980 (actual=1564.05 target=1602.24)`
| cdn-hot-update | 6281.10 | 6339.70 | 0.991x | -0.92% | 6398.72 | 0.982x | 0.991x | 0.778x | +22.20% | 0.899x | +10.07% | 1.388x | -38.76% | 0 |
| game-long-connection | 1085.20 | 1090.85 | 0.995x | -0.52% | 1093.19 | 0.993x | 0.998x | 1.065x | -6.53% | 1.105x | -10.54% | 2.142x | -114.15% | 0 |
| generic-sse | 141.95 | 142.85 | 0.994x | -0.63% | 143.56 | 0.989x | 0.995x | 1.004x | -0.42% | 1.040x | -4.03% | 2.291x | -129.06% | 0 |
| https-static-small | 1543.05 | 1564.05 | 0.987x | -1.34% | 1602.24 | 0.963x | 0.976x | 1.053x | -5.29% | 1.132x | -13.24% | 1.683x | -68.35% | 0 |
| qcp-transparent | 1044.70 | 1054.25 | 0.991x | -0.91% | 1056.25 | 0.989x | 0.998x | 0.885x | +11.47% | 0.950x | +4.99% | 1.716x | -71.57% | 0 |
| reverse-proxy | 3142.85 | 3153.45 | 0.997x | -0.34% | 3190.75 | 0.985x | 0.988x | 0.917x | +8.30% | 0.984x | +1.61% | 1.848x | -84.81% | 0 |
| static-large | 26.50 | 26.50 | 1.000x | +0.00% | 26.60 | 0.996x | 0.996x | 1.003x | -0.33% | 1.097x | -9.70% | 2.695x | -169.45% | 0 |
| static-small | 6234.85 | 6268.15 | 0.995x | -0.53% | 6342.91 | 0.983x | 0.988x | 0.782x | +21.84% | 0.909x | +9.14% | 1.383x | -38.32% | 0 |
| tcp-stream | 1097.40 | 1103.25 | 0.995x | -0.53% | 1105.43 | 0.993x | 0.998x | 1.043x | -4.31% | 1.324x | -32.38% | 1.811x | -81.10% | 0 |
| udp-stream | 1056.50 | 1066.80 | 0.990x | -0.97% | 1068.80 | 0.988x | 0.998x | 0.893x | +10.69% | 1.008x | -0.76% | 1.624x | -62.39% | 0 |
| websocket-long-connection | 1040.50 | 1047.00 | 0.994x | -0.62% | 1049.32 | 0.992x | 0.998x | 1.043x | -4.29% | 1.177x | -17.73% | 2.304x | -130.38% | 0 |

- Aggregate proxysss ops/s: `22694.60`
- Aggregate nginx ops/s: `22856.85`
- Aggregate proxysss/nginx ratio: `0.993x`
- Aggregate throughput improvement: `-0.71%`
