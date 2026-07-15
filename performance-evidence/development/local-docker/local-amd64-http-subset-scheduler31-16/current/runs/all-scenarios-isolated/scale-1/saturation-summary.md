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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-subset-scheduler31-16/current/runs/all-scenarios-isolated/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 42987.45 | 31509.65 | 1.364x | +36.43% | - | - | - | 0.811x | +18.88% | 0.683x | +31.75% | 0.617x | +38.32% | 0 |
| generic-sse | 1237.05 | 583.75 | 2.119x | +111.91% | - | - | - | 0.435x | +56.54% | 0.562x | +43.79% | 0.388x | +61.24% | 0 |
| https-static-small | 8900.75 | 6365.70 | 1.398x | +39.82% | - | - | - | 0.536x | +46.42% | 0.937x | +6.28% | 0.681x | +31.93% | 0 |
| reverse-proxy | 27350.65 | 14904.85 | 1.835x | +83.50% | - | - | - | 0.477x | +52.27% | 0.582x | +41.85% | 0.482x | +51.81% | 0 |
| static-small | 42946.45 | 31235.90 | 1.375x | +37.49% | - | - | - | 0.807x | +19.27% | 0.678x | +32.20% | 0.615x | +38.52% | 0 |

- Aggregate proxysss ops/s: `123422.35`
- Aggregate nginx ops/s: `84599.85`
- Aggregate proxysss/nginx ratio: `1.459x`
- Aggregate throughput improvement: `+45.89%`
