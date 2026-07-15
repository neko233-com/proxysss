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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T045100Z-3005fdea28ca/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5843.33 | 5843.67 | 1.000x | -0.01% | 5834.67 | 1.001x | 1.002x | 0.778x | +22.22% | 0.808x | +19.16% | 0.391x | +60.91% | 0 |
| game-long-connection | 866.67 | 869.33 | 0.997x | -0.31% | 869.33 | 0.997x | 1.000x | 0.894x | +10.58% | 1.438x | -43.78% | 0.418x | +58.18% | 0 |
| generic-sse | 109.67 | 109.67 | 1.000x | +0.00% | 109.33 | 1.003x | 1.003x | 0.946x | +5.41% | 0.821x | +17.86% | 0.273x | +72.67% | 0 |
| https-static-small | 694.33 | 694.33 | 1.000x | +0.00% | 693.33 | 1.001x | 1.001x | 0.991x | +0.85% | 0.809x | +19.08% | 0.348x | +65.17% | 0 |
| qcp-transparent | 800.00 | 800.00 | 1.000x | +0.00% | 800.00 | 1.000x | 1.000x | 0.823x | +17.69% | 0.743x | +25.68% | 0.442x | +55.81% | 0 |
| reverse-proxy | 2535.33 | 2536.00 | 1.000x | -0.03% | 2528.00 | 1.003x | 1.003x | 0.915x | +8.48% | 0.804x | +19.60% | 0.343x | +65.73% | 0 |
| static-large | 21.00 | 21.00 | 1.000x | +0.00% | 20.00 | 1.050x | 1.050x | 0.875x | +12.46% | 1.016x | -1.58% | 0.264x | +73.63% | 0 |
| static-small | 6261.67 | 6262.00 | 1.000x | -0.01% | 6261.33 | 1.000x | 1.000x | 0.744x | +25.58% | 0.900x | +10.04% | 0.284x | +71.62% | 0 |
| tcp-stream | 853.33 | 853.33 | 1.000x | +0.00% | 856.00 | 0.997x | 0.997x | 0.911x | +8.90% | 1.097x | -9.65% | 0.386x | +61.39% | 0 |
| udp-stream | 829.33 | 829.33 | 1.000x | +0.00% | 829.33 | 1.000x | 1.000x | 0.816x | +18.37% | 0.844x | +15.61% | 0.500x | +49.98% | 0 |
| websocket-long-connection | 834.67 | 834.67 | 1.000x | +0.00% | 834.67 | 1.000x | 1.000x | 0.867x | +13.28% | 0.632x | +36.76% | 0.371x | +62.93% | 0 |

- Aggregate proxysss ops/s: `19649.33`
- Aggregate nginx ops/s: `19653.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.02%`
