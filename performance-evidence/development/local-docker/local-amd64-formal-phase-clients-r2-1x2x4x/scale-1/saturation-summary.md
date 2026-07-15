# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `4` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-formal-phase-clients-r2-1x2x4x/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 14823.80 | 14112.95 | 1.050x | +5.04% | - | - | - | 0.956x | +4.38% | 0.999x | +0.10% | 0.808x | +19.19% | 0 |
| game-long-connection | 3105.75 | 2609.60 | 1.190x | +19.01% | - | - | - | 0.918x | +8.21% | 0.840x | +15.96% | 0.680x | +31.98% | 0 |
| generic-sse | 366.45 | 340.80 | 1.075x | +7.53% | - | - | - | 0.850x | +14.97% | 1.191x | -19.11% | 0.955x | +4.46% | 0 |
| https-static-small | 3899.65 | 3420.20 | 1.140x | +14.02% | - | - | - | 0.926x | +7.40% | 0.875x | +12.50% | 0.711x | +28.89% | 0 |
| qcp-transparent | 2637.90 | 2533.20 | 1.041x | +4.13% | - | - | - | 0.906x | +9.41% | 1.129x | -12.93% | 0.948x | +5.18% | 0 |
| reverse-proxy | 7767.30 | 7321.35 | 1.061x | +6.09% | - | - | - | 0.958x | +4.19% | 1.041x | -4.11% | 0.851x | +14.93% | 0 |
| static-large | 82.20 | 62.30 | 1.319x | +31.94% | - | - | - | 0.770x | +23.04% | 0.737x | +26.30% | 0.650x | +34.96% | 0 |
| static-small | 14823.75 | 13948.80 | 1.063x | +6.27% | - | - | - | 0.910x | +8.97% | 1.003x | -0.33% | 0.794x | +20.64% | 0 |
| tcp-stream | 3118.90 | 2556.95 | 1.220x | +21.98% | - | - | - | 0.880x | +12.02% | 0.826x | +17.44% | 0.685x | +31.49% | 0 |
| udp-stream | 2599.05 | 2511.40 | 1.035x | +3.49% | - | - | - | 0.917x | +8.25% | 1.131x | -13.12% | 0.950x | +5.03% | 0 |
| websocket-long-connection | 2939.75 | 2485.00 | 1.183x | +18.30% | - | - | - | 0.931x | +6.91% | 0.843x | +15.74% | 0.658x | +34.22% | 0 |

- Aggregate proxysss ops/s: `56164.50`
- Aggregate nginx ops/s: `51902.55`
- Aggregate proxysss/nginx ratio: `1.082x`
- Aggregate throughput improvement: `+8.21%`
