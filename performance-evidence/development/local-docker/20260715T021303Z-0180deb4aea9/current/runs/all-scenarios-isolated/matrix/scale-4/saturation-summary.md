# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `balanced`
- Auto concurrency: HTTP `128`, HTTPS `32`, static-large `16`, SSE `8`, TCP/UDP/WebSocket `32`
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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T021303Z-0180deb4aea9/current/runs/all-scenarios-isolated/matrix/scale-4/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 17546.50 | 21043.50 | 0.834x | -16.62% | - | - | - | 0.866x | +13.45% | 1.771x | -77.15% | 1.865x | -86.50% | 0 |
| game-long-connection | 6005.00 | 4805.50 | 1.250x | +24.96% | - | - | - | 0.639x | +36.12% | 0.977x | +2.30% | 1.116x | -11.55% | 0 |
| generic-sse | 753.00 | 625.50 | 1.204x | +20.38% | - | - | - | 0.588x | +41.24% | 1.403x | -40.30% | 0.844x | +15.62% | 0 |
| https-static-small | 3038.00 | 3387.00 | 0.897x | -10.30% | - | - | - | 0.842x | +15.77% | 1.214x | -21.38% | 1.792x | -79.16% | 0 |
| qcp-transparent | 7795.00 | 4997.50 | 1.560x | +55.98% | - | - | - | 0.404x | +59.61% | 0.917x | +8.27% | 1.445x | -44.54% | 0 |
| reverse-proxy | 10050.50 | 10765.50 | 0.934x | -6.64% | - | - | - | 0.879x | +12.15% | 1.514x | -51.45% | 1.268x | -26.78% | 0 |
| static-large | 88.50 | 110.50 | 0.801x | -19.91% | - | - | - | 1.664x | -66.44% | 0.978x | +2.17% | 0.380x | +62.01% | 0 |
| static-small | 22418.50 | 17816.00 | 1.258x | +25.83% | - | - | - | 0.676x | +32.44% | 1.327x | -32.69% | 0.980x | +2.00% | 0 |
| tcp-stream | 6219.00 | 6468.50 | 0.961x | -3.86% | - | - | - | 1.050x | -4.99% | 1.037x | -3.66% | 0.999x | +0.13% | 0 |
| udp-stream | 7066.50 | 4810.00 | 1.469x | +46.91% | - | - | - | 0.404x | +59.60% | 1.068x | -6.85% | 1.429x | -42.87% | 0 |
| websocket-long-connection | 6251.50 | 5008.50 | 1.248x | +24.82% | - | - | - | 0.714x | +28.58% | 0.896x | +10.38% | 0.990x | +0.99% | 0 |

- Aggregate proxysss ops/s: `87232.00`
- Aggregate nginx ops/s: `79838.00`
- Aggregate proxysss/nginx ratio: `1.093x`
- Aggregate throughput improvement: `+9.26%`
