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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 30769.00 | 24036.67 | 1.280x | +28.01% | - | - | - | 0.709x | +29.12% | 0.994x | +0.62% | 1.042x | -4.22% | 0 |
| game-long-connection | 8760.33 | 3508.00 | 2.497x | +149.72% | - | - | - | 0.330x | +66.99% | 0.619x | +38.09% | 0.669x | +33.11% | 0 |
| generic-sse | 1005.00 | 540.00 | 1.861x | +86.11% | - | - | - | 0.444x | +55.56% | 0.884x | +11.55% | 1.098x | -9.78% | 0 |
| https-static-small | 8855.00 | 5578.67 | 1.587x | +58.73% | - | - | - | 0.516x | +48.44% | 0.781x | +21.85% | 0.804x | +19.63% | 0 |
| qcp-transparent | 6832.67 | 3525.67 | 1.938x | +93.80% | - | - | - | 0.418x | +58.22% | 0.815x | +18.54% | 0.956x | +4.45% | 0 |
| reverse-proxy | 14459.33 | 12183.33 | 1.187x | +18.68% | - | - | - | 0.802x | +19.80% | 1.074x | -7.44% | 1.146x | -14.62% | 0 |
| static-large | 85.33 | 98.00 | 0.871x | -12.93% | - | - | - | 1.208x | -20.81% | 1.552x | -55.16% | 0.421x | +57.87% | 0 |
| static-small | 29617.00 | 24707.67 | 1.199x | +19.87% | - | - | - | 0.727x | +27.33% | 1.103x | -10.26% | 1.221x | -22.10% | 0 |
| tcp-stream | 8679.33 | 3535.00 | 2.455x | +145.53% | - | - | - | 0.332x | +66.82% | 0.620x | +38.05% | 0.655x | +34.53% | 0 |
| udp-stream | 7664.00 | 3671.33 | 2.088x | +108.75% | - | - | - | 0.370x | +62.98% | 0.745x | +25.55% | 0.957x | +4.29% | 0 |
| websocket-long-connection | 7822.33 | 3592.67 | 2.177x | +117.73% | - | - | - | 0.377x | +62.31% | 0.643x | +35.74% | 0.699x | +30.07% | 0 |

- Aggregate proxysss ops/s: `124549.32`
- Aggregate nginx ops/s: `84977.01`
- Aggregate proxysss/nginx ratio: `1.466x`
- Aggregate throughput improvement: `+46.57%`
