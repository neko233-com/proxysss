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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-tls-pool-fastlane-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 17759.75 | 20129.75 | 0.882x | -11.77% | - | - | - | 0.860x | +14.04% | 1.447x | -44.69% | 1.489x | -48.88% | 0 |
| game-long-connection | 3111.00 | 3575.50 | 0.870x | -12.99% | - | - | - | 0.923x | +7.74% | 1.434x | -43.43% | 1.496x | -49.56% | 0 |
| generic-sse | 432.75 | 436.00 | 0.993x | -0.75% | - | - | - | 0.846x | +15.41% | 1.429x | -42.85% | 1.410x | -41.01% | 0 |
| https-static-small | 3591.50 | 4911.25 | 0.731x | -26.87% | - | - | - | 1.417x | -41.70% | 1.289x | -28.93% | 1.177x | -17.74% | 0 |
| qcp-transparent | 3205.50 | 3408.75 | 0.940x | -5.96% | - | - | - | 0.813x | +18.67% | 1.559x | -55.91% | 1.577x | -57.74% | 0 |
| reverse-proxy | 9190.25 | 10287.00 | 0.893x | -10.66% | - | - | - | 0.976x | +2.43% | 1.543x | -54.32% | 1.561x | -56.12% | 0 |
| static-large | 111.00 | 81.75 | 1.358x | +35.78% | - | - | - | 0.766x | +23.37% | 0.731x | +26.88% | 0.375x | +62.54% | 0 |
| static-small | 18021.75 | 19947.00 | 0.903x | -9.65% | - | - | - | 0.824x | +17.65% | 1.427x | -42.75% | 1.460x | -46.00% | 0 |
| tcp-stream | 3143.25 | 3547.25 | 0.886x | -11.39% | - | - | - | 0.940x | +5.99% | 1.469x | -46.95% | 1.338x | -33.84% | 0 |
| udp-stream | 3224.00 | 3392.00 | 0.950x | -4.95% | - | - | - | 0.806x | +19.42% | 1.503x | -50.31% | 1.627x | -62.73% | 0 |
| websocket-long-connection | 2948.75 | 3446.25 | 0.856x | -14.44% | - | - | - | 0.947x | +5.33% | 1.483x | -48.30% | 1.444x | -44.35% | 0 |

- Aggregate proxysss ops/s: `64739.50`
- Aggregate nginx ops/s: `73162.50`
- Aggregate proxysss/nginx ratio: `0.885x`
- Aggregate throughput improvement: `-11.51%`
