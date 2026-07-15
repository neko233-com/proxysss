# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/20260715T013625Z-a47f70324ce4/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `static-large nginx target achievement 0.970 < 0.980 (actual=16.00 target=16.50); udp-stream nginx target achievement 0.975 < 0.980 (actual=592.00 target=607.23)`
| cdn-hot-update | 2638.00 | 2642.00 | 0.998x | -0.15% | 2642.66 | 0.998x | 1.000x | 0.766x | +23.44% | 1.465x | -46.49% | 0.804x | +19.57% | 0 |
| game-long-connection | 720.00 | 720.00 | 1.000x | +0.00% | 732.23 | 0.983x | 0.983x | 1.092x | -9.23% | 2.402x | -140.20% | 2.399x | -139.93% | 0 |
| generic-sse | 78.00 | 78.00 | 1.000x | +0.00% | 78.50 | 0.994x | 0.994x | 0.967x | +3.31% | 1.892x | -89.24% | 1.026x | -2.60% | 0 |
| https-static-small | 345.00 | 345.00 | 1.000x | +0.00% | 346.00 | 0.997x | 0.997x | 1.017x | -1.72% | 2.027x | -102.69% | 1.113x | -11.28% | 0 |
| qcp-transparent | 720.00 | 720.00 | 1.000x | +0.00% | 731.50 | 0.984x | 0.984x | 0.825x | +17.50% | 1.708x | -70.76% | 0.685x | +31.54% | 0 |
| reverse-proxy | 1195.00 | 1195.00 | 1.000x | +0.00% | 1196.73 | 0.999x | 0.999x | 0.984x | +1.65% | 2.519x | -151.95% | 0.996x | +0.38% | 0 |
| static-large | 16.00 | 16.00 | 1.000x | +0.00% | 16.50 | 0.970x | 0.970x | 0.989x | +1.07% | 0.930x | +6.98% | 1.047x | -4.73% | 0 |
| static-small | 2475.00 | 2478.00 | 0.999x | -0.12% | 2480.72 | 0.998x | 0.999x | 0.736x | +26.37% | 2.171x | -117.13% | 1.215x | -21.45% | 0 |
| tcp-stream | 672.00 | 672.00 | 1.000x | +0.00% | 681.72 | 0.986x | 0.986x | 0.749x | +25.10% | 2.003x | -100.28% | 0.925x | +7.51% | 0 |
| udp-stream | 592.00 | 592.00 | 1.000x | +0.00% | 607.23 | 0.975x | 0.975x | 0.916x | +8.39% | 3.107x | -210.72% | 4.536x | -353.58% | 0 |
| websocket-long-connection | 592.00 | 592.00 | 1.000x | +0.00% | 598.24 | 0.990x | 0.990x | 0.901x | +9.86% | 2.022x | -102.24% | 1.505x | -50.53% | 0 |

- Aggregate proxysss ops/s: `10043.00`
- Aggregate nginx ops/s: `10050.00`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.07%`
