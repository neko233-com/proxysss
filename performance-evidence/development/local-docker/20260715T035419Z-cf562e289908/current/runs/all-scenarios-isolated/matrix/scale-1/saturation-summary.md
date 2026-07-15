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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035419Z-cf562e289908/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 31159.33 | 25127.00 | 1.240x | +24.01% | - | - | - | 0.649x | +35.09% | 0.945x | +5.47% | 1.004x | -0.39% | 0 |
| game-long-connection | 4417.67 | 3519.33 | 1.255x | +25.53% | - | - | - | 0.611x | +38.93% | 1.167x | -16.66% | 0.973x | +2.72% | 0 |
| generic-sse | 611.00 | 437.67 | 1.396x | +39.60% | - | - | - | 0.589x | +41.06% | 0.962x | +3.82% | 1.072x | -7.17% | 0 |
| https-static-small | 4802.67 | 5252.00 | 0.914x | -8.56% | - | - | - | 0.827x | +17.30% | 1.330x | -33.05% | 1.194x | -19.38% | 0 |
| qcp-transparent | 4695.00 | 3387.00 | 1.386x | +38.62% | - | - | - | 0.553x | +44.72% | 0.955x | +4.50% | 0.964x | +3.60% | 0 |
| reverse-proxy | 13224.67 | 10790.67 | 1.226x | +22.56% | - | - | - | 0.777x | +22.25% | 0.922x | +7.83% | 0.997x | +0.29% | 0 |
| static-large | 92.00 | 93.00 | 0.989x | -1.08% | - | - | - | 0.971x | +2.89% | 1.047x | -4.66% | 1.941x | -94.09% | 0 |
| static-small | 28328.67 | 25995.00 | 1.090x | +8.98% | - | - | - | 0.742x | +25.78% | 1.122x | -12.20% | 1.048x | -4.80% | 0 |
| tcp-stream | 4456.67 | 3535.67 | 1.260x | +26.05% | - | - | - | 0.617x | +38.28% | 1.101x | -10.11% | 1.028x | -2.83% | 0 |
| udp-stream | 4752.67 | 3540.00 | 1.343x | +34.26% | - | - | - | 0.563x | +43.74% | 0.964x | +3.59% | 0.984x | +1.64% | 0 |
| websocket-long-connection | 4206.67 | 3278.67 | 1.283x | +28.30% | - | - | - | 0.607x | +39.31% | 1.057x | -5.65% | 1.057x | -5.75% | 0 |

- Aggregate proxysss ops/s: `100747.02`
- Aggregate nginx ops/s: `84956.01`
- Aggregate proxysss/nginx ratio: `1.186x`
- Aggregate throughput improvement: `+18.59%`
