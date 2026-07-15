# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6380.67 | 6378.67 | 1.000x | +0.03% | 6357.33 | 1.004x | 1.003x | 0.692x | +30.85% | 0.114x | +88.62% | 0.318x | +68.17% | 0 |
| game-long-connection | 928.00 | 928.00 | 1.000x | +0.00% | 928.00 | 1.000x | 1.000x | 1.368x | -36.84% | 0.403x | +59.74% | 0.243x | +75.67% | 0 |
| generic-sse | 144.00 | 144.00 | 1.000x | +0.00% | 144.00 | 1.000x | 1.000x | 0.876x | +12.43% | 0.115x | +88.48% | 0.201x | +79.87% | 0 |
| https-static-small | 971.67 | 972.00 | 1.000x | -0.03% | 970.67 | 1.001x | 1.001x | 0.900x | +10.04% | 0.070x | +92.96% | 0.865x | +13.45% | 0 |
| qcp-transparent | 938.67 | 938.67 | 1.000x | +0.00% | 938.67 | 1.000x | 1.000x | 0.799x | +20.09% | 0.085x | +91.51% | 0.069x | +93.09% | 0 |
| reverse-proxy | 3082.67 | 3082.67 | 1.000x | +0.00% | 3072.00 | 1.003x | 1.003x | 0.869x | +13.06% | 0.081x | +91.94% | 0.792x | +20.78% | 0 |
| static-large | 23.00 | 23.00 | 1.000x | +0.00% | 21.33 | 1.078x | 1.078x | 0.998x | +0.18% | 0.332x | +66.78% | 0.235x | +76.53% | 0 |
| static-small | 6242.33 | 6243.67 | 1.000x | -0.02% | 6229.33 | 1.002x | 1.002x | 0.726x | +27.42% | 0.122x | +87.84% | 0.378x | +62.18% | 0 |
| tcp-stream | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 1.178x | -17.81% | 0.096x | +90.39% | 0.083x | +91.71% | 0 |
| udp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.749x | +25.12% | 0.106x | +89.35% | 0.086x | +91.37% | 0 |
| websocket-long-connection | 874.67 | 874.67 | 1.000x | +0.00% | 874.67 | 1.000x | 1.000x | 1.050x | -4.98% | 0.340x | +66.04% | 0.300x | +70.00% | 0 |

- Aggregate proxysss ops/s: `21463.01`
- Aggregate nginx ops/s: `21462.68`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.00%`
