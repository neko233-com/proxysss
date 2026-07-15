# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7022.00 | 7020.00 | 1.000x | +0.03% | 7018.67 | 1.000x | 1.000x | 0.845x | +15.52% | 0.810x | +18.99% | 0.415x | +58.46% | 0 |
| game-long-connection | 965.33 | 965.33 | 1.000x | +0.00% | 965.33 | 1.000x | 1.000x | 1.226x | -22.63% | 1.177x | -17.69% | 0.744x | +25.61% | 0 |
| generic-sse | 111.00 | 111.00 | 1.000x | +0.00% | 110.67 | 1.003x | 1.003x | 0.988x | +1.20% | 1.122x | -12.17% | 0.674x | +32.60% | 0 |
| https-static-small | 1340.67 | 1340.33 | 1.000x | +0.03% | 1338.67 | 1.001x | 1.001x | 0.979x | +2.07% | 1.085x | -8.47% | 1.071x | -7.13% | 0 |
| qcp-transparent | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.906x | +9.42% | 0.982x | +1.85% | 0.859x | +14.11% | 0 |
| reverse-proxy | 2976.00 | 2977.00 | 1.000x | -0.03% | 2976.00 | 1.000x | 1.000x | 1.052x | -5.21% | 1.129x | -12.88% | 0.736x | +26.36% | 0 |
| static-large | 18.00 | 18.00 | 1.000x | +0.00% | 17.33 | 1.038x | 1.038x | 1.257x | -25.71% | 1.611x | -61.06% | 1.569x | -56.92% | 0 |
| static-small | 6789.67 | 6790.33 | 1.000x | -0.01% | 6784.00 | 1.001x | 1.001x | 0.837x | +16.28% | 0.870x | +13.03% | 1.065x | -6.49% | 0 |
| tcp-stream | 946.67 | 944.00 | 1.003x | +0.28% | 946.67 | 1.000x | 0.997x | 1.224x | -22.41% | 1.268x | -26.80% | 0.812x | +18.78% | 0 |
| udp-stream | 965.33 | 965.33 | 1.000x | +0.00% | 965.33 | 1.000x | 1.000x | 0.931x | +6.86% | 1.181x | -18.10% | 1.110x | -10.97% | 0 |
| websocket-long-connection | 912.00 | 912.00 | 1.000x | +0.00% | 912.00 | 1.000x | 1.000x | 1.139x | -13.88% | 1.378x | -37.80% | 0.744x | +25.56% | 0 |

- Aggregate proxysss ops/s: `23006.67`
- Aggregate nginx ops/s: `23003.32`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
