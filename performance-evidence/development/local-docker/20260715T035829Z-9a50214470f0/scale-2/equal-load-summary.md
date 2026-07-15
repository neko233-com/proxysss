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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-2/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 6569.67 | 6570.67 | 1.000x | -0.02% | 6570.67 | 1.000x | 1.000x | 0.778x | +22.16% | 0.809x | +19.06% | 1.900x | -89.99% | 0 |
| game-long-connection | 938.67 | 938.67 | 1.000x | +0.00% | 938.67 | 1.000x | 1.000x | 1.147x | -14.65% | 1.076x | -7.60% | 3.250x | -225.05% | 0 |
| generic-sse | 121.00 | 121.00 | 1.000x | +0.00% | 120.00 | 1.008x | 1.008x | 0.959x | +4.11% | 0.877x | +12.34% | 1.011x | -1.12% | 0 |
| https-static-small | 1334.00 | 1333.33 | 1.001x | +0.05% | 1333.33 | 1.001x | 1.000x | 0.937x | +6.27% | 0.910x | +8.99% | 3.343x | -234.32% | 0 |
| qcp-transparent | 912.00 | 912.00 | 1.000x | +0.00% | 917.33 | 0.994x | 0.994x | 0.913x | +8.71% | 0.938x | +6.18% | 1.586x | -58.65% | 0 |
| reverse-proxy | 3108.33 | 3108.67 | 1.000x | -0.01% | 3093.33 | 1.005x | 1.005x | 0.981x | +1.92% | 0.923x | +7.73% | 0.876x | +12.37% | 0 |
| static-large | 23.00 | 23.00 | 1.000x | +0.00% | 21.33 | 1.078x | 1.078x | 0.962x | +3.76% | 0.945x | +5.51% | 1.422x | -42.22% | 0 |
| static-small | 6444.33 | 6442.33 | 1.000x | +0.03% | 6442.67 | 1.000x | 1.000x | 0.798x | +20.22% | 0.775x | +22.49% | 1.537x | -53.67% | 0 |
| tcp-stream | 901.33 | 901.33 | 1.000x | +0.00% | 901.33 | 1.000x | 1.000x | 1.166x | -16.60% | 1.748x | -74.83% | 1.044x | -4.36% | 0 |
| udp-stream | 917.33 | 917.33 | 1.000x | +0.00% | 917.33 | 1.000x | 1.000x | 0.856x | +14.41% | 0.892x | +10.84% | 2.025x | -102.51% | 0 |
| websocket-long-connection | 906.67 | 906.67 | 1.000x | +0.00% | 906.67 | 1.000x | 1.000x | 1.000x | +0.00% | 1.534x | -53.41% | 2.286x | -128.57% | 0 |

- Aggregate proxysss ops/s: `22176.33`
- Aggregate nginx ops/s: `22175.00`
- Aggregate proxysss/nginx ratio: `1.000x`
- Aggregate throughput improvement: `+0.01%`
