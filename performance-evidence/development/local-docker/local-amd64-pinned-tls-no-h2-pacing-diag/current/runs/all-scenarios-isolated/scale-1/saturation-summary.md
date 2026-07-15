# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-pinned-tls-no-h2-pacing-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 15492.75 | 20232.25 | 0.766x | -23.43% | - | - | - | 0.880x | +12.03% | 1.637x | -63.71% | 2.104x | -110.38% | 0 |
| game-long-connection | 2365.25 | 3692.00 | 0.641x | -35.94% | - | - | - | 1.328x | -32.80% | 1.855x | -85.50% | 2.156x | -115.56% | 0 |
| generic-sse | 372.75 | 453.00 | 0.823x | -17.72% | - | - | - | 0.989x | +1.10% | 1.656x | -65.60% | 1.749x | -74.89% | 0 |
| https-static-small | 3814.00 | 5146.50 | 0.741x | -25.89% | - | - | - | 0.881x | +11.95% | 1.653x | -65.33% | 1.368x | -36.76% | 0 |
| qcp-transparent | 2680.00 | 3483.50 | 0.769x | -23.07% | - | - | - | 0.946x | +5.37% | 1.783x | -78.32% | 2.253x | -125.28% | 0 |
| reverse-proxy | 8369.25 | 10382.50 | 0.806x | -19.39% | - | - | - | 1.062x | -6.19% | 1.695x | -69.52% | 2.105x | -110.55% | 0 |
| static-large | 100.00 | 81.50 | 1.227x | +22.70% | - | - | - | 0.791x | +20.93% | 0.846x | +15.37% | 0.911x | +8.88% | 0 |
| static-small | 16276.50 | 20538.25 | 0.792x | -20.75% | - | - | - | 0.806x | +19.39% | 1.735x | -73.47% | 2.067x | -106.69% | 0 |
| tcp-stream | 2342.00 | 3745.00 | 0.625x | -37.46% | - | - | - | 1.366x | -36.62% | 1.995x | -99.46% | 2.051x | -105.11% | 0 |
| udp-stream | 2615.75 | 3494.00 | 0.749x | -25.14% | - | - | - | 1.017x | -1.70% | 1.848x | -84.76% | 2.245x | -124.50% | 0 |
| websocket-long-connection | 2194.50 | 3514.00 | 0.625x | -37.55% | - | - | - | 1.352x | -35.22% | 2.046x | -104.62% | 2.158x | -115.78% | 0 |

- Aggregate proxysss ops/s: `56622.75`
- Aggregate nginx ops/s: `74762.50`
- Aggregate proxysss/nginx ratio: `0.757x`
- Aggregate throughput improvement: `-24.26%`
