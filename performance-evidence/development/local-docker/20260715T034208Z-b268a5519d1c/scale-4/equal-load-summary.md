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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6263.33 | 6265.00 | 1.000x | -0.03% | 6229.33 | 1.005x | 1.006x | 0.827x | +17.34% | 1.087x | -8.71% | 1.860x | -85.97% | 0 |
| game-long-connection | 949.33 | 949.33 | 1.000x | +0.00% | 949.33 | 1.000x | 1.000x | 1.280x | -28.00% | 1.044x | -4.41% | 0.961x | +3.93% | 0 |
| generic-sse | 137.33 | 137.33 | 1.000x | +0.00% | 136.00 | 1.010x | 1.010x | 1.021x | -2.14% | 0.805x | +19.51% | 0.423x | +57.71% | 0 |
| https-static-small | 1424.33 | 1423.67 | 1.000x | +0.05% | 1418.67 | 1.004x | 1.004x | 1.012x | -1.24% | 1.152x | -15.22% | 0.823x | +17.73% | 0 |
| qcp-transparent | 906.67 | 906.67 | 1.000x | +0.00% | 906.67 | 1.000x | 1.000x | 0.847x | +15.35% | 0.842x | +15.84% | 0.363x | +63.73% | 0 |
| reverse-proxy | 3043.33 | 3044.33 | 1.000x | -0.03% | 3029.33 | 1.005x | 1.005x | 1.039x | -3.91% | 1.207x | -20.70% | 0.981x | +1.94% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 21.33 | 1.031x | 1.031x | 1.260x | -26.00% | 1.363x | -36.33% | 1.734x | -73.36% | 0 |
| static-small | 6186.33 | 6187.33 | 1.000x | -0.02% | 6186.67 | 1.000x | 1.000x | 0.832x | +16.76% | 0.772x | +22.84% | 0.836x | +16.40% | 0 |
| tcp-stream | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 1.329x | -32.89% | 1.709x | -70.91% | 0.473x | +52.75% | 0 |
| udp-stream | 906.67 | 906.67 | 1.000x | +0.00% | 906.67 | 1.000x | 1.000x | 0.778x | +22.20% | 1.546x | -54.55% | 0.372x | +62.79% | 0 |
| websocket-long-connection | 896.00 | 896.00 | 1.000x | +0.00% | 896.00 | 1.000x | 1.000x | 1.193x | -19.28% | 1.159x | -15.87% | 0.305x | +69.46% | 0 |

- Aggregate proxysss ops/s: `21652.65`
- Aggregate nginx ops/s: `21655.66`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
