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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-large-diag/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 33469.50 | 33699.25 | 0.993x | -0.68% | - | - | - | 0.950x | +4.98% | 1.334x | -33.42% | 1.154x | -15.41% | 0 |
| generic-sse | 925.00 | 702.25 | 1.317x | +31.72% | - | - | - | 0.682x | +31.84% | 0.880x | +11.97% | 0.942x | +5.76% | 1 |
| reverse-proxy | 18433.75 | 15661.50 | 1.177x | +17.70% | - | - | - | 0.795x | +20.49% | 0.978x | +2.24% | 1.031x | -3.11% | 0 |
| static-large | 102.75 | 102.00 | 1.007x | +0.74% | - | - | - | 0.983x | +1.66% | 1.039x | -3.92% | 1.147x | -14.74% | 0 |
| static-small | 35000.25 | 33749.25 | 1.037x | +3.71% | - | - | - | 0.933x | +6.68% | 1.138x | -13.77% | 1.166x | -16.58% | 0 |

- Aggregate proxysss ops/s: `87931.25`
- Aggregate nginx ops/s: `83914.25`
- Aggregate proxysss/nginx ratio: `1.048x`
- Aggregate throughput improvement: `+4.79%`
