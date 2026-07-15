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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T023643Z-76b1f39b6b3e/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 29627.50 | 26453.00 | 1.120x | +12.00% | - | - | - | 0.733x | +26.72% | 1.146x | -14.59% | 1.092x | -9.16% | 0 |
| game-long-connection | 5292.50 | 4278.00 | 1.237x | +23.71% | - | - | - | 0.584x | +41.61% | 1.025x | -2.53% | 0.970x | +3.04% | 0 |
| generic-sse | 731.00 | 451.00 | 1.621x | +62.08% | - | - | - | 0.523x | +47.73% | 0.743x | +25.71% | 1.435x | -43.55% | 0 |
| https-static-small | 7155.00 | 4788.50 | 1.494x | +49.42% | - | - | - | 0.468x | +53.20% | 1.014x | -1.36% | 0.800x | +20.03% | 0 |
| qcp-transparent | 5467.00 | 3690.00 | 1.482x | +48.16% | - | - | - | 0.565x | +43.49% | 0.783x | +21.73% | 0.955x | +4.54% | 0 |
| reverse-proxy | 13572.50 | 11114.00 | 1.221x | +22.12% | - | - | - | 0.814x | +18.64% | 0.859x | +14.11% | 0.824x | +17.63% | 0 |
| static-large | 84.00 | 87.50 | 0.960x | -4.00% | - | - | - | 1.127x | -12.70% | 1.102x | -10.21% | 0.806x | +19.43% | 0 |
| static-small | 30919.50 | 25003.50 | 1.237x | +23.66% | - | - | - | 0.707x | +29.25% | 0.891x | +10.92% | 1.024x | -2.44% | 0 |
| tcp-stream | 5380.50 | 4325.50 | 1.244x | +24.39% | - | - | - | 0.580x | +41.96% | 0.986x | +1.37% | 0.830x | +16.98% | 0 |
| udp-stream | 5835.50 | 3861.50 | 1.511x | +51.12% | - | - | - | 0.492x | +50.83% | 0.842x | +15.78% | 0.879x | +12.07% | 0 |
| websocket-long-connection | 5324.50 | 3628.00 | 1.468x | +46.76% | - | - | - | 0.507x | +49.33% | 0.941x | +5.91% | 0.846x | +15.37% | 0 |

- Aggregate proxysss ops/s: `109389.50`
- Aggregate nginx ops/s: `87680.50`
- Aggregate proxysss/nginx ratio: `1.248x`
- Aggregate throughput improvement: `+24.76%`
