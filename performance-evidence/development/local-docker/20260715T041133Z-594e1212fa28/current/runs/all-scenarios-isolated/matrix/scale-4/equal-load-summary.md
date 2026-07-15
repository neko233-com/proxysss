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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041133Z-594e1212fa28/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6360.33 | 6357.67 | 1.000x | +0.04% | 6357.33 | 1.000x | 1.000x | 0.778x | +22.22% | 1.307x | -30.67% | 2.007x | -100.65% | 0 |
| game-long-connection | 885.33 | 885.33 | 1.000x | +0.00% | 885.33 | 1.000x | 1.000x | 1.162x | -16.23% | 1.229x | -22.91% | 0.808x | +19.16% | 0 |
| generic-sse | 140.00 | 140.00 | 1.000x | +0.00% | 138.67 | 1.010x | 1.010x | 0.971x | +2.85% | 1.346x | -34.62% | 0.961x | +3.92% | 0 |
| https-static-small | 902.67 | 902.67 | 1.000x | +0.00% | 896.00 | 1.007x | 1.007x | 0.939x | +6.10% | 1.111x | -11.13% | 0.901x | +9.87% | 0 |
| qcp-transparent | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 0.793x | +20.75% | 1.496x | -49.57% | 2.479x | -147.92% | 0 |
| reverse-proxy | 3109.00 | 3109.33 | 1.000x | -0.01% | 3072.00 | 1.012x | 1.012x | 1.039x | -3.92% | 1.760x | -75.97% | 0.955x | +4.52% | 0 |
| static-large | 23.00 | 23.00 | 1.000x | +0.00% | 21.33 | 1.078x | 1.078x | 0.896x | +10.44% | 0.902x | +9.85% | 1.319x | -31.89% | 0 |
| static-small | 6325.67 | 6325.00 | 1.000x | +0.01% | 6314.67 | 1.002x | 1.002x | 0.816x | +18.39% | 1.492x | -49.20% | 2.512x | -151.17% | 0 |
| tcp-stream | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.017x | -1.69% | 1.039x | -3.92% | 2.832x | -183.23% | 0 |
| udp-stream | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 0.860x | +13.97% | 2.057x | -105.74% | 2.136x | -113.61% | 0 |
| websocket-long-connection | 864.00 | 864.00 | 1.000x | +0.00% | 864.00 | 1.000x | 1.000x | 1.002x | -0.25% | 0.924x | +7.61% | 0.614x | +38.62% | 0 |

- Aggregate proxysss ops/s: `21319.33`
- Aggregate nginx ops/s: `21316.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
