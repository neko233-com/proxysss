# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `saturation`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-subset-current/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 26580.80 | 35162.35 | 0.756x | -24.41% | - | - | - | 0.955x | +4.50% | 2.090x | -109.00% | 2.413x | -141.33% | 0 |
| generic-sse | 563.35 | 665.45 | 0.847x | -15.34% | - | - | - | 0.933x | +6.66% | 1.774x | -77.36% | 2.539x | -153.90% | 0 |
| https-static-small | 6378.45 | 8129.80 | 0.785x | -21.54% | - | - | - | 0.824x | +17.56% | 2.159x | -115.93% | 1.855x | -85.54% | 0 |
| reverse-proxy | 14225.45 | 18406.20 | 0.773x | -22.71% | - | - | - | 0.983x | +1.70% | 1.951x | -95.08% | 2.604x | -160.44% | 0 |
| static-small | 26856.65 | 35817.45 | 0.750x | -25.02% | - | - | - | 0.955x | +4.50% | 2.149x | -114.93% | 2.539x | -153.91% | 0 |

- Aggregate proxysss ops/s: `74604.70`
- Aggregate nginx ops/s: `98181.25`
- Aggregate proxysss/nginx ratio: `0.760x`
- Aggregate throughput improvement: `-24.01%`
