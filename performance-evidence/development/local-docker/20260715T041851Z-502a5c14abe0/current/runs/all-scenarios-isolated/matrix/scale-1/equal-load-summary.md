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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T041851Z-502a5c14abe0/current/runs/all-scenarios-isolated/matrix/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 7242.67 | 7242.67 | 1.000x | +0.00% | 7242.67 | 1.000x | 1.000x | 0.826x | +17.39% | 1.143x | -14.31% | 1.077x | -7.73% | 0 |
| game-long-connection | 810.67 | 810.67 | 1.000x | +0.00% | 810.67 | 1.000x | 1.000x | 0.988x | +1.15% | 2.242x | -124.22% | 0.715x | +28.54% | 0 |
| generic-sse | 113.33 | 113.33 | 1.000x | +0.00% | 113.33 | 1.000x | 1.000x | 0.956x | +4.39% | 1.069x | -6.94% | 0.962x | +3.76% | 0 |
| https-static-small | 1156.00 | 1155.67 | 1.000x | +0.03% | 1154.67 | 1.001x | 1.001x | 0.947x | +5.31% | 1.814x | -81.44% | 1.093x | -9.25% | 0 |
| qcp-transparent | 914.67 | 914.67 | 1.000x | +0.00% | 914.67 | 1.000x | 1.000x | 0.877x | +12.30% | 1.291x | -29.08% | 0.995x | +0.48% | 0 |
| reverse-proxy | 3180.33 | 3181.33 | 1.000x | -0.03% | 3178.67 | 1.001x | 1.001x | 0.987x | +1.25% | 1.363x | -36.25% | 0.451x | +54.91% | 0 |
| static-large | 23.67 | 23.67 | 1.000x | +0.00% | 22.67 | 1.044x | 1.044x | 0.949x | +5.14% | 0.766x | +23.38% | 1.065x | -6.48% | 0 |
| static-small | 6953.00 | 6952.67 | 1.000x | +0.00% | 6954.67 | 1.000x | 1.000x | 0.842x | +15.82% | 1.512x | -51.18% | 1.098x | -9.80% | 0 |
| tcp-stream | 792.00 | 792.00 | 1.000x | +0.00% | 792.00 | 1.000x | 1.000x | 1.048x | -4.82% | 1.756x | -75.65% | 1.186x | -18.58% | 0 |
| udp-stream | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 0.929x | +7.14% | 1.442x | -44.16% | 0.819x | +18.06% | 0 |
| websocket-long-connection | 741.33 | 741.33 | 1.000x | +0.00% | 741.33 | 1.000x | 1.000x | 1.003x | -0.33% | 2.615x | -161.49% | 1.809x | -80.89% | 0 |

- Aggregate proxysss ops/s: `22845.00`
- Aggregate nginx ops/s: `22845.34`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `-0.00%`
