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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T043126Z-a45301aad820/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 5519.33 | 5517.00 | 1.000x | +0.04% | 5514.67 | 1.001x | 1.000x | 0.821x | +17.87% | 1.352x | -35.16% | 1.593x | -59.35% | 0 |
| game-long-connection | 752.00 | 752.00 | 1.000x | +0.00% | 752.00 | 1.000x | 1.000x | 1.177x | -17.70% | 1.234x | -23.36% | 0.779x | +22.15% | 0 |
| generic-sse | 99.67 | 99.67 | 1.000x | +0.00% | 99.33 | 1.003x | 1.003x | 1.014x | -1.39% | 1.236x | -23.59% | 1.811x | -81.11% | 0 |
| https-static-small | 1078.67 | 1079.00 | 1.000x | -0.03% | 1077.33 | 1.001x | 1.002x | 0.927x | +7.27% | 1.450x | -45.00% | 1.423x | -42.31% | 0 |
| qcp-transparent | 736.00 | 736.00 | 1.000x | +0.00% | 736.00 | 1.000x | 1.000x | 0.859x | +14.07% | 1.292x | -29.17% | 1.741x | -74.06% | 0 |
| reverse-proxy | 2275.67 | 2275.67 | 1.000x | +0.00% | 2272.00 | 1.002x | 1.002x | 1.010x | -1.00% | 1.687x | -68.68% | 1.564x | -56.37% | 0 |
| static-large | 19.67 | 19.67 | 1.000x | +0.00% | 18.67 | 1.054x | 1.054x | 0.962x | +3.77% | 0.881x | +11.89% | 1.577x | -57.71% | 0 |
| static-small | 5742.33 | 5743.33 | 1.000x | -0.02% | 5738.67 | 1.001x | 1.001x | 0.788x | +21.17% | 1.542x | -54.15% | 1.894x | -89.43% | 0 |
| tcp-stream | 728.00 | 728.00 | 1.000x | +0.00% | 728.00 | 1.000x | 1.000x | 1.206x | -20.61% | 1.477x | -47.70% | 1.979x | -97.95% | 0 |
| udp-stream | 720.00 | 720.00 | 1.000x | +0.00% | 720.00 | 1.000x | 1.000x | 0.877x | +12.25% | 0.914x | +8.60% | 1.558x | -55.78% | 0 |
| websocket-long-connection | 712.00 | 712.00 | 1.000x | +0.00% | 712.00 | 1.000x | 1.000x | 1.027x | -2.70% | 1.069x | -6.91% | 1.025x | -2.53% | 0 |

- Aggregate proxysss ops/s: `18383.34`
- Aggregate nginx ops/s: `18382.34`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
