# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T022209Z-852f72c36f5a/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5992.50 | 5995.00 | 1.000x | -0.04% | 6004.32 | 0.998x | 0.998x | 0.832x | +16.76% | 1.297x | -29.68% | 1.251x | -25.12% | 0 |
| game-long-connection | 1040.00 | 1040.00 | 1.000x | +0.00% | 1050.49 | 0.990x | 0.990x | 1.000x | +0.00% | 1.998x | -99.81% | 3.100x | -209.95% | 0 |
| generic-sse | 129.00 | 129.00 | 1.000x | +0.00% | 129.37 | 0.997x | 0.997x | 1.035x | -3.48% | 1.913x | -91.30% | 1.668x | -66.83% | 0 |
| https-static-small | 1195.00 | 1233.00 | 0.969x | -3.08% | 1234.57 | 0.968x | 0.999x | 1.051x | -5.06% | 2.670x | -166.96% | 2.347x | -134.68% | 0 |
| qcp-transparent | 1024.00 | 1024.00 | 1.000x | +0.00% | 1027.62 | 0.996x | 0.996x | 0.888x | +11.22% | 2.089x | -108.95% | 2.966x | -196.63% | 0 |
| reverse-proxy | 2932.50 | 2932.00 | 1.000x | +0.02% | 2936.32 | 0.999x | 0.999x | 0.983x | +1.66% | 1.599x | -59.85% | 1.430x | -42.99% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.37 | 0.983x | 0.983x | 0.939x | +6.07% | 1.003x | -0.34% | 1.168x | -16.77% | 0 |
| static-small | 6178.00 | 6181.50 | 0.999x | -0.06% | 6187.76 | 0.998x | 0.999x | 0.836x | +16.44% | 1.925x | -92.49% | 1.266x | -26.59% | 0 |
| tcp-stream | 1024.00 | 1024.00 | 1.000x | +0.00% | 1035.93 | 0.988x | 0.988x | 1.005x | -0.54% | 1.538x | -53.79% | 1.724x | -72.38% | 0 |
| udp-stream | 976.00 | 976.00 | 1.000x | +0.00% | 982.50 | 0.993x | 0.993x | 0.905x | +9.53% | 1.118x | -11.85% | 1.818x | -81.85% | 0 |
| websocket-long-connection | 1200.00 | 1200.00 | 1.000x | +0.00% | 1208.37 | 0.993x | 0.993x | 0.963x | +3.65% | 2.194x | -119.43% | 2.151x | -115.05% | 0 |

- Aggregate proxysss ops/s: `21713.00`
- Aggregate nginx ops/s: `21756.50`
- Aggregate proxysss/nginx ratio: `0.998x`
- Aggregate throughput improvement: `-0.20%`
