# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `32`, HTTPS `8`, static-large `4`, SSE `2`, TCP/UDP/WebSocket `8`
- Non-critical minimum proxysss/nginx ops ratio: `1.00` except diagnostic scenarios ``
- SSE stream error tolerance: `proxysss <= nginx + 0`
- WebSocket reconnect/error tolerance: `proxysss <= nginx + 0`
- UDP datagram error tolerance: `proxysss <= nginx + 0`
- Critical long-connection fair ratio gate: `1.00` for ``
- Aggregate mixed-load fair ratio gate: `1.00`
- Maximum proxysss/nginx p50/p95/p99 latency ratio: `1.00` (required=false, strict=true)
- Saturation ops gate: `true`
- Equal-load latency gate: `false`
- Minimum fixed-load completion: `0.000`
- Reference under-target policy: `report warning; candidate must still meet target and win latency`
- Zero-error gate: `true`
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T034208Z-b268a5519d1c/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 34901.33 | 28100.33 | 1.242x | +24.20% | - | - | - | 0.685x | +31.49% | 1.022x | -2.25% | 1.028x | -2.78% | 0 |
| game-long-connection | 4386.67 | 3872.00 | 1.133x | +13.29% | - | - | - | 0.598x | +40.23% | 1.347x | -34.69% | 1.179x | -17.95% | 0 |
| generic-sse | 772.33 | 444.33 | 1.738x | +73.82% | - | - | - | 0.491x | +50.94% | 0.811x | +18.94% | 0.720x | +28.03% | 0 |
| https-static-small | 8613.33 | 5365.33 | 1.605x | +60.54% | - | - | - | 0.455x | +54.46% | 0.969x | +3.09% | 0.767x | +23.31% | 0 |
| qcp-transparent | 5812.67 | 3847.67 | 1.511x | +51.07% | - | - | - | 0.499x | +50.13% | 0.944x | +5.56% | 0.896x | +10.39% | 0 |
| reverse-proxy | 15411.67 | 11912.67 | 1.294x | +29.37% | - | - | - | 0.742x | +25.85% | 0.941x | +5.86% | 0.780x | +21.97% | 0 |
| static-large | 73.33 | 94.33 | 0.777x | -22.26% | - | - | - | 1.411x | -41.07% | 1.280x | -27.99% | 0.504x | +49.64% | 0 |
| static-small | 36227.67 | 27175.00 | 1.333x | +33.31% | - | - | - | 0.655x | +34.48% | 0.931x | +6.94% | 0.894x | +10.64% | 0 |
| tcp-stream | 4408.33 | 3788.67 | 1.164x | +16.36% | - | - | - | 0.608x | +39.17% | 1.332x | -33.16% | 1.051x | -5.13% | 0 |
| udp-stream | 5597.00 | 3868.67 | 1.447x | +44.68% | - | - | - | 0.533x | +46.74% | 0.990x | +1.02% | 0.855x | +14.45% | 0 |
| websocket-long-connection | 4360.67 | 3656.67 | 1.193x | +19.25% | - | - | - | 0.597x | +40.31% | 1.264x | -26.38% | 1.217x | -21.73% | 0 |

- Aggregate proxysss ops/s: `120565.00`
- Aggregate nginx ops/s: `92125.67`
- Aggregate proxysss/nginx ratio: `1.309x`
- Aggregate throughput improvement: `+30.87%`
