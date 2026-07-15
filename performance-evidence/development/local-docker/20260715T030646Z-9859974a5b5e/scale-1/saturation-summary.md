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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 33555.50 | 28404.50 | 1.181x | +18.13% | - | - | - | 0.689x | +31.07% | 1.079x | -7.93% | 0.990x | +0.97% | 0 |
| game-long-connection | 6370.50 | 5206.00 | 1.224x | +22.37% | - | - | - | 0.497x | +50.26% | 1.070x | -7.00% | 0.859x | +14.07% | 0 |
| generic-sse | 646.50 | 444.50 | 1.454x | +45.44% | - | - | - | 0.590x | +40.98% | 0.889x | +11.13% | 0.782x | +21.84% | 0 |
| https-static-small | 4549.00 | 5149.50 | 0.883x | -11.66% | - | - | - | 0.938x | +6.18% | 1.430x | -43.04% | 1.114x | -11.42% | 0 |
| qcp-transparent | 4522.50 | 3730.50 | 1.212x | +21.23% | - | - | - | 0.640x | +35.96% | 0.998x | +0.16% | 1.018x | -1.76% | 0 |
| reverse-proxy | 15099.50 | 11340.50 | 1.331x | +33.15% | - | - | - | 0.692x | +30.81% | 0.874x | +12.56% | 0.846x | +15.36% | 0 |
| static-large | 86.50 | 91.50 | 0.945x | -5.46% | - | - | - | 1.040x | -4.04% | 0.993x | +0.73% | 1.934x | -93.42% | 0 |
| static-small | 32636.00 | 27164.50 | 1.201x | +20.14% | - | - | - | 0.691x | +30.90% | 0.997x | +0.26% | 0.930x | +7.04% | 0 |
| tcp-stream | 6447.50 | 3918.50 | 1.645x | +64.54% | - | - | - | 0.400x | +59.97% | 0.936x | +6.45% | 0.792x | +20.82% | 0 |
| udp-stream | 4133.00 | 4008.50 | 1.031x | +3.11% | - | - | - | 0.755x | +24.51% | 1.064x | -6.44% | 1.071x | -7.09% | 0 |
| websocket-long-connection | 6072.00 | 3591.50 | 1.691x | +69.07% | - | - | - | 0.402x | +59.79% | 0.954x | +4.64% | 0.995x | +0.45% | 0 |

- Aggregate proxysss ops/s: `114118.50`
- Aggregate nginx ops/s: `93050.00`
- Aggregate proxysss/nginx ratio: `1.226x`
- Aggregate throughput improvement: `+22.64%`
