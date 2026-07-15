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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T044325Z-3bf5b546df92/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 655.00 | 2033.33 | 0.322x | -67.79% | - | - | - | 1.371x | -37.07% | 5.199x | -419.87% | 5.243x | -424.31% | 0 |
| game-long-connection | 52.33 | 318.00 | 0.165x | -83.54% | - | - | - | 4.548x | -354.80% | 7.856x | -685.62% | 6.021x | -502.14% | 0 |
| generic-sse | 10.67 | 49.33 | 0.216x | -78.37% | - | - | - | 2.969x | -196.88% | 7.257x | -625.75% | 4.141x | -314.10% | 0 |
| https-static-small | 13386.00 | 719.33 | 18.609x | +1760.90% | - | - | - | 0.040x | +95.96% | 0.044x | +95.59% | 0.089x | +91.10% | 0 |
| qcp-transparent | 53.00 | 243.00 | 0.218x | -78.19% | - | - | - | 3.282x | -228.23% | 5.402x | -440.25% | 4.015x | -301.46% | 0 |
| reverse-proxy | 200.33 | 901.00 | 0.222x | -77.77% | - | - | - | 3.270x | -227.01% | 6.676x | -567.57% | 5.180x | -418.03% | 0 |
| static-large | 8.00 | 19.33 | 0.414x | -58.61% | - | - | - | 1.347x | -34.74% | 5.619x | -461.89% | 4.801x | -380.10% | 0 |
| static-small | 451.67 | 1771.67 | 0.255x | -74.51% | - | - | - | 3.123x | -212.32% | 5.370x | -436.98% | 4.599x | -359.92% | 0 |
| tcp-stream | 62.67 | 310.00 | 0.202x | -79.78% | - | - | - | 4.369x | -336.88% | 4.892x | -389.21% | 3.944x | -294.41% | 0 |
| udp-stream | 58.67 | 279.67 | 0.210x | -79.02% | - | - | - | 3.292x | -229.22% | 6.459x | -545.85% | 12.094x | -1109.38% | 0 |
| websocket-long-connection | 67.67 | 373.00 | 0.181x | -81.86% | - | - | - | 4.129x | -312.93% | 7.537x | -653.71% | 4.986x | -398.56% | 0 |

- Aggregate proxysss ops/s: `15006.01`
- Aggregate nginx ops/s: `7017.66`
- Aggregate proxysss/nginx ratio: `2.138x`
- Aggregate throughput improvement: `+113.83%`
