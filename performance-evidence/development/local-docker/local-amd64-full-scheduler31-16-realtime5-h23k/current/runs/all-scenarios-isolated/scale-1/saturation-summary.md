# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-full-scheduler31-16-realtime5-h23k/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 22111.10 | 20931.50 | 1.056x | +5.64% | - | - | - | 0.783x | +21.68% | 1.062x | -6.19% | 1.064x | -6.37% | 0 |
| game-long-connection | 3892.90 | 3675.80 | 1.059x | +5.91% | - | - | - | 0.873x | +12.67% | 1.133x | -13.27% | 1.162x | -16.15% | 0 |
| generic-sse | 586.90 | 462.55 | 1.269x | +26.88% | - | - | - | 0.663x | +33.69% | 1.093x | -9.28% | 1.108x | -10.81% | 0 |
| https-static-small | 5227.75 | 5375.75 | 0.972x | -2.75% | - | - | - | 0.949x | +5.12% | 1.155x | -15.48% | 1.015x | -1.48% | 0 |
| qcp-transparent | 4761.10 | 3578.60 | 1.330x | +33.04% | - | - | - | 0.579x | +42.11% | 1.026x | -2.64% | 1.028x | -2.76% | 0 |
| reverse-proxy | 12162.35 | 10833.65 | 1.123x | +12.26% | - | - | - | 0.823x | +17.69% | 1.097x | -9.66% | 1.093x | -9.33% | 0 |
| static-large | 94.45 | 82.80 | 1.141x | +14.07% | - | - | - | 0.885x | +11.49% | 0.881x | +11.86% | 0.749x | +25.11% | 0 |
| static-small | 21724.05 | 21133.15 | 1.028x | +2.80% | - | - | - | 0.810x | +18.97% | 1.111x | -11.13% | 1.094x | -9.40% | 0 |
| tcp-stream | 3864.55 | 3728.65 | 1.036x | +3.64% | - | - | - | 0.893x | +10.69% | 1.140x | -14.02% | 1.183x | -18.33% | 0 |
| udp-stream | 4787.15 | 3559.10 | 1.345x | +34.50% | - | - | - | 0.558x | +44.16% | 1.041x | -4.14% | 1.037x | -3.72% | 0 |
| websocket-long-connection | 3636.80 | 3481.95 | 1.044x | +4.45% | - | - | - | 0.906x | +9.44% | 1.133x | -13.29% | 1.112x | -11.17% | 0 |

- Aggregate proxysss ops/s: `82849.10`
- Aggregate nginx ops/s: `76843.50`
- Aggregate proxysss/nginx ratio: `1.078x`
- Aggregate throughput improvement: `+7.82%`
