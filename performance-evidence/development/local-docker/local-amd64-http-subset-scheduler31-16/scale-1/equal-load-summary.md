# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `2` (median metrics, maximum observed errors)
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-subset-scheduler31-16/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.945 < 0.980 (actual=14877.25 target=15748.03); generic-sse nginx target achievement 0.958 < 0.980 (actual=279.70 target=291.84); https-static-small nginx target achievement 0.925 < 0.980 (actual=2942.25 target=3182.18); reverse-proxy nginx target achievement 0.945 < 0.980 (actual=7045.55 target=7452.26); static-small nginx target achievement 0.943 < 0.980 (actual=14729.55 target=15617.37)`
| cdn-hot-update | 15605.15 | 14877.25 | 1.049x | +4.89% | 15748.03 | 0.991x | 0.945x | 0.814x | +18.62% | 0.456x | +54.44% | 0.208x | +79.25% | 0 |
| generic-sse | 289.75 | 279.70 | 1.036x | +3.59% | 291.84 | 0.993x | 0.958x | 0.850x | +15.01% | 0.315x | +68.52% | 0.153x | +84.69% | 0 |
| https-static-small | 3102.30 | 2942.25 | 1.054x | +5.44% | 3182.18 | 0.975x | 0.925x | 1.001x | -0.14% | 0.574x | +42.63% | 0.246x | +75.41% | 0 |
| reverse-proxy | 7361.00 | 7045.55 | 1.045x | +4.48% | 7452.26 | 0.988x | 0.945x | 0.847x | +15.26% | 0.349x | +65.07% | 0.152x | +84.84% | 0 |
| static-small | 15427.20 | 14729.55 | 1.047x | +4.74% | 15617.37 | 0.988x | 0.943x | 0.802x | +19.75% | 0.424x | +57.59% | 0.196x | +80.44% | 0 |

- Aggregate proxysss ops/s: `41785.40`
- Aggregate nginx ops/s: `39874.30`
- Aggregate proxysss/nginx ratio: `1.048x`
- Aggregate throughput improvement: `+4.79%`
