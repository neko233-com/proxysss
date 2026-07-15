# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T040534Z-d6e9f2b9206e/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 28602.67 | 25066.67 | 1.141x | +14.11% | - | - | - | 0.743x | +25.74% | 1.320x | -32.00% | 1.332x | -33.15% | 0 |
| game-long-connection | 8722.00 | 3479.33 | 2.507x | +150.68% | - | - | - | 0.338x | +66.21% | 0.642x | +35.77% | 0.696x | +30.43% | 0 |
| generic-sse | 982.67 | 557.33 | 1.763x | +76.32% | - | - | - | 0.507x | +49.30% | 0.878x | +12.25% | 1.152x | -15.20% | 0 |
| https-static-small | 5668.33 | 5698.33 | 0.995x | -0.53% | - | - | - | 0.970x | +3.02% | 1.451x | -45.14% | 0.752x | +24.82% | 0 |
| qcp-transparent | 7134.00 | 3575.67 | 1.995x | +99.52% | - | - | - | 0.420x | +58.00% | 0.790x | +21.03% | 0.971x | +2.87% | 0 |
| reverse-proxy | 14988.33 | 12700.33 | 1.180x | +18.02% | - | - | - | 0.826x | +17.39% | 1.094x | -9.42% | 1.202x | -20.23% | 0 |
| static-large | 91.33 | 100.33 | 0.910x | -8.97% | - | - | - | 1.165x | -16.53% | 1.202x | -20.22% | 1.412x | -41.22% | 0 |
| static-small | 31736.33 | 25763.00 | 1.232x | +23.19% | - | - | - | 0.705x | +29.47% | 1.200x | -19.98% | 1.399x | -39.92% | 0 |
| tcp-stream | 8642.00 | 3509.67 | 2.462x | +146.23% | - | - | - | 0.350x | +65.00% | 0.639x | +36.09% | 0.714x | +28.60% | 0 |
| udp-stream | 7273.00 | 3589.00 | 2.026x | +102.65% | - | - | - | 0.404x | +59.56% | 0.836x | +16.39% | 0.996x | +0.37% | 0 |
| websocket-long-connection | 7962.00 | 3493.67 | 2.279x | +127.90% | - | - | - | 0.405x | +59.52% | 0.677x | +32.31% | 0.711x | +28.89% | 0 |

- Aggregate proxysss ops/s: `121802.66`
- Aggregate nginx ops/s: `87533.33`
- Aggregate proxysss/nginx ratio: `1.392x`
- Aggregate throughput improvement: `+39.15%`
