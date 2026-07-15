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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T032528Z-6d7ef9acafbd/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6749.00 | 6747.00 | 1.000x | +0.03% | 6752.00 | 1.000x | 0.999x | 0.783x | +21.69% | 0.884x | +11.58% | 0.257x | +74.29% | 0 |
| game-long-connection | 876.00 | 876.00 | 1.000x | +0.00% | 876.00 | 1.000x | 1.000x | 1.257x | -25.71% | 1.268x | -26.81% | 0.749x | +25.07% | 0 |
| generic-sse | 108.50 | 108.50 | 1.000x | +0.00% | 108.00 | 1.005x | 1.005x | 0.952x | +4.76% | 0.651x | +34.89% | 0.209x | +79.07% | 0 |
| https-static-small | 1290.50 | 1290.50 | 1.000x | +0.00% | 1288.00 | 1.002x | 1.002x | 0.944x | +5.65% | 0.952x | +4.84% | 0.647x | +35.27% | 0 |
| qcp-transparent | 888.00 | 888.00 | 1.000x | +0.00% | 888.00 | 1.000x | 1.000x | 0.857x | +14.29% | 0.736x | +26.42% | 0.775x | +22.55% | 0 |
| reverse-proxy | 2923.50 | 2923.50 | 1.000x | +0.00% | 2912.00 | 1.004x | 1.004x | 0.910x | +8.98% | 0.732x | +26.83% | 0.563x | +43.72% | 0 |
| static-large | 22.00 | 22.00 | 1.000x | +0.00% | 22.00 | 1.000x | 1.000x | 0.997x | +0.25% | 0.944x | +5.55% | 1.067x | -6.74% | 0 |
| static-small | 6792.50 | 6793.00 | 1.000x | -0.01% | 6784.00 | 1.001x | 1.001x | 0.784x | +21.62% | 1.088x | -8.84% | 0.195x | +80.51% | 0 |
| tcp-stream | 868.00 | 868.00 | 1.000x | +0.00% | 868.00 | 1.000x | 1.000x | 1.221x | -22.10% | 1.102x | -10.19% | 0.382x | +61.77% | 0 |
| udp-stream | 900.00 | 900.00 | 1.000x | +0.00% | 900.00 | 1.000x | 1.000x | 0.881x | +11.86% | 0.771x | +22.91% | 0.599x | +40.05% | 0 |
| websocket-long-connection | 832.00 | 832.00 | 1.000x | +0.00% | 832.00 | 1.000x | 1.000x | 1.176x | -17.61% | 0.929x | +7.11% | 0.289x | +71.05% | 0 |

- Aggregate proxysss ops/s: `22250.00`
- Aggregate nginx ops/s: `22248.50`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
