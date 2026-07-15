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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032904Z-02d3bd2d97f7/current/runs/all-scenarios-isolated/matrix/scale-4/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6324.50 | 6321.50 | 1.000x | +0.05% | 6272.00 | 1.008x | 1.008x | 0.754x | +24.60% | 1.582x | -58.18% | 0.709x | +29.07% | 0 |
| game-long-connection | 936.00 | 928.00 | 1.009x | +0.86% | 944.00 | 0.992x | 0.983x | 1.563x | -56.29% | 1.397x | -39.66% | 0.887x | +11.35% | 0 |
| generic-sse | 135.00 | 135.00 | 1.000x | +0.00% | 132.00 | 1.023x | 1.023x | 0.976x | +2.36% | 1.191x | -19.10% | 0.651x | +34.86% | 0 |
| https-static-small | 1248.00 | 1248.00 | 1.000x | +0.00% | 1248.00 | 1.000x | 1.000x | 0.969x | +3.10% | 1.741x | -74.12% | 1.056x | -5.62% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.822x | +17.76% | 1.154x | -15.38% | 0.495x | +50.47% | 0 |
| reverse-proxy | 3112.50 | 3111.00 | 1.000x | +0.05% | 3072.00 | 1.013x | 1.013x | 1.022x | -2.21% | 2.097x | -109.73% | 0.623x | +37.71% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 16.00 | 1.375x | 1.375x | 0.992x | +0.84% | 0.902x | +9.79% | 0.563x | +43.69% | 0 |
| static-small | 6155.50 | 6156.00 | 1.000x | -0.01% | 6144.00 | 1.002x | 1.002x | 0.758x | +24.19% | 1.218x | -21.82% | 0.274x | +72.60% | 0 |
| tcp-stream | 1056.00 | 1056.00 | 1.000x | +0.00% | 1056.00 | 1.000x | 1.000x | 1.402x | -40.18% | 2.606x | -160.58% | 0.764x | +23.63% | 0 |
| udp-stream | 1040.00 | 1040.00 | 1.000x | +0.00% | 1040.00 | 1.000x | 1.000x | 0.828x | +17.25% | 2.087x | -108.72% | 0.345x | +65.47% | 0 |
| websocket-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 912.00 | 1.000x | 1.000x | 1.336x | -33.58% | 4.123x | -312.29% | 1.271x | -27.06% | 0 |

- Aggregate proxysss ops/s: `21901.50`
- Aggregate nginx ops/s: `21889.50`
- Aggregate proxysss/nginx ratio: `1.001x`
- Aggregate throughput improvement: `+0.05%`
