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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-dual-lanes-event4-h23600/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 9253.10 | 20593.05 | 0.449x | -55.07% | - | - | - | 1.209x | -20.92% | 3.091x | -209.06% | 2.684x | -168.35% | 0 |
| game-long-connection | 4631.70 | 3658.30 | 1.266x | +26.61% | - | - | - | 0.719x | +28.06% | 0.889x | +11.10% | 0.928x | +7.17% | 0 |
| generic-sse | 199.90 | 464.05 | 0.431x | -56.92% | - | - | - | 1.500x | -50.01% | 3.880x | -288.03% | 4.302x | -330.19% | 0 |
| https-static-small | 5823.00 | 5698.40 | 1.022x | +2.19% | - | - | - | 0.953x | +4.74% | 0.987x | +1.33% | 0.970x | +2.97% | 0 |
| qcp-transparent | 1425.00 | 3402.15 | 0.419x | -58.11% | - | - | - | 1.419x | -41.93% | 3.926x | -292.60% | 3.783x | -278.29% | 0 |
| reverse-proxy | 4700.00 | 10657.80 | 0.441x | -55.90% | - | - | - | 1.686x | -68.57% | 3.468x | -246.76% | 3.410x | -240.98% | 0 |
| static-large | 113.25 | 81.30 | 1.393x | +39.30% | - | - | - | 0.717x | +28.33% | 0.756x | +24.37% | 0.742x | +25.78% | 0 |
| static-small | 9082.70 | 20667.80 | 0.439x | -56.05% | - | - | - | 1.260x | -26.01% | 3.143x | -214.32% | 2.670x | -166.95% | 0 |
| tcp-stream | 4600.05 | 3528.95 | 1.304x | +30.35% | - | - | - | 0.688x | +31.22% | 0.892x | +10.76% | 0.942x | +5.83% | 0 |
| udp-stream | 1504.35 | 3365.30 | 0.447x | -55.30% | - | - | - | 1.285x | -28.49% | 3.713x | -271.33% | 3.609x | -260.93% | 0 |
| websocket-long-connection | 4389.10 | 3519.90 | 1.247x | +24.69% | - | - | - | 0.727x | +27.29% | 0.900x | +9.95% | 0.943x | +5.73% | 0 |

- Aggregate proxysss ops/s: `45722.15`
- Aggregate nginx ops/s: `75637.00`
- Aggregate proxysss/nginx ratio: `0.604x`
- Aggregate throughput improvement: `-39.55%`
