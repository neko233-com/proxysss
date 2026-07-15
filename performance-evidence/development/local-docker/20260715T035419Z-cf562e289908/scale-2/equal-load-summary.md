# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `64`, HTTPS `16`, static-large `8`, SSE `4`, TCP/UDP/WebSocket `16`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6399.00 | 6400.33 | 1.000x | -0.02% | 6400.00 | 1.000x | 1.000x | 0.834x | +16.57% | 1.394x | -39.37% | 1.598x | -59.84% | 0 |
| game-long-connection | 965.33 | 965.33 | 1.000x | +0.00% | 965.33 | 1.000x | 1.000x | 1.283x | -28.26% | 1.717x | -71.72% | 1.953x | -95.25% | 0 |
| generic-sse | 117.33 | 117.33 | 1.000x | +0.00% | 117.33 | 1.000x | 1.000x | 1.010x | -1.05% | 1.721x | -72.07% | 1.634x | -63.43% | 0 |
| https-static-small | 1065.67 | 1065.67 | 1.000x | +0.00% | 1061.33 | 1.004x | 1.004x | 1.000x | +0.00% | 1.247x | -24.67% | 1.735x | -73.51% | 0 |
| qcp-transparent | 1002.67 | 1002.67 | 1.000x | +0.00% | 1008.00 | 0.995x | 0.995x | 0.896x | +10.43% | 1.229x | -22.86% | 0.673x | +32.66% | 0 |
| reverse-proxy | 3126.67 | 3126.33 | 1.000x | +0.01% | 3114.67 | 1.004x | 1.004x | 1.094x | -9.39% | 1.488x | -48.80% | 2.443x | -144.29% | 0 |
| static-large | 23.00 | 23.00 | 1.000x | +0.00% | 21.33 | 1.078x | 1.078x | 0.985x | +1.52% | 1.121x | -12.14% | 1.154x | -15.40% | 0 |
| static-small | 6366.00 | 6366.67 | 1.000x | -0.01% | 6357.33 | 1.001x | 1.001x | 0.833x | +16.67% | 1.292x | -29.21% | 1.075x | -7.49% | 0 |
| tcp-stream | 970.67 | 970.67 | 1.000x | +0.00% | 970.67 | 1.000x | 1.000x | 1.388x | -38.78% | 1.468x | -46.76% | 1.460x | -45.99% | 0 |
| udp-stream | 965.33 | 965.33 | 1.000x | +0.00% | 965.33 | 1.000x | 1.000x | 0.862x | +13.81% | 1.409x | -40.85% | 2.323x | -132.29% | 0 |
| websocket-long-connection | 976.00 | 976.00 | 1.000x | +0.00% | 976.00 | 1.000x | 1.000x | 1.188x | -18.77% | 1.656x | -65.56% | 2.043x | -104.34% | 0 |

- Aggregate proxysss ops/s: `21977.67`
- Aggregate nginx ops/s: `21979.33`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.01%`
