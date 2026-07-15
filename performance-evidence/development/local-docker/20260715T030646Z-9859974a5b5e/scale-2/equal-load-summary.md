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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T030646Z-9859974a5b5e/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6653.00 | 6648.00 | 1.001x | +0.08% | 6624.00 | 1.004x | 1.004x | 0.768x | +23.16% | 1.230x | -23.05% | 1.390x | -38.97% | 0 |
| game-long-connection | 992.00 | 992.00 | 1.000x | +0.00% | 992.00 | 1.000x | 1.000x | 1.102x | -10.20% | 1.387x | -38.65% | 2.053x | -105.33% | 0 |
| generic-sse | 123.00 | 123.00 | 1.000x | +0.00% | 122.00 | 1.008x | 1.008x | 0.930x | +7.04% | 0.868x | +13.16% | 1.042x | -4.20% | 0 |
| https-static-small | 1531.00 | 1531.00 | 1.000x | +0.00% | 1528.00 | 1.002x | 1.002x | 0.935x | +6.51% | 1.129x | -12.90% | 0.758x | +24.24% | 0 |
| qcp-transparent | 1008.00 | 1008.00 | 1.000x | +0.00% | 1008.00 | 1.000x | 1.000x | 0.832x | +16.85% | 1.151x | -15.10% | 2.289x | -128.91% | 0 |
| reverse-proxy | 3172.50 | 3171.50 | 1.000x | +0.03% | 3168.00 | 1.001x | 1.001x | 0.906x | +9.38% | 1.069x | -6.87% | 0.946x | +5.42% | 0 |
| static-large | 22.50 | 22.50 | 1.000x | +0.00% | 20.00 | 1.125x | 1.125x | 1.002x | -0.18% | 1.138x | -13.85% | 0.928x | +7.19% | 0 |
| static-small | 6782.00 | 6783.50 | 1.000x | -0.02% | 6784.00 | 1.000x | 1.000x | 0.773x | +22.65% | 0.824x | +17.65% | 1.172x | -17.18% | 0 |
| tcp-stream | 960.00 | 960.00 | 1.000x | +0.00% | 960.00 | 1.000x | 1.000x | 0.996x | +0.36% | 1.555x | -55.48% | 2.013x | -101.30% | 0 |
| udp-stream | 1000.00 | 1000.00 | 1.000x | +0.00% | 1000.00 | 1.000x | 1.000x | 0.837x | +16.34% | 0.870x | +13.00% | 1.254x | -25.42% | 0 |
| websocket-long-connection | 936.00 | 936.00 | 1.000x | +0.00% | 944.00 | 0.992x | 0.992x | 0.952x | +4.79% | 1.136x | -13.63% | 1.871x | -87.06% | 0 |

- Aggregate proxysss ops/s: `23180.00`
- Aggregate nginx ops/s: `23175.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.02%`
