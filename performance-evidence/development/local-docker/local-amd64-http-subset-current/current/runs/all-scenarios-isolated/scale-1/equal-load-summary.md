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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-http-subset-current/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.974 < 0.980 (actual=12941.50 target=13289.04); https-static-small nginx target achievement 0.963 < 0.980 (actual=3071.00 target=3188.52); reverse-proxy nginx target achievement 0.976 < 0.980 (actual=6940.80 target=7112.69); static-small nginx target achievement 0.976 < 0.980 (actual=13094.80 target=13422.82)`
| cdn-hot-update | 12942.35 | 12941.50 | 1.000x | +0.01% | 13289.04 | 0.974x | 0.974x | 0.850x | +15.01% | 0.902x | +9.81% | 0.916x | +8.40% | 0 |
| generic-sse | 276.45 | 276.80 | 0.999x | -0.13% | 281.65 | 0.982x | 0.983x | 0.904x | +9.61% | 0.798x | +20.20% | 0.736x | +26.39% | 0 |
| https-static-small | 3054.70 | 3071.00 | 0.995x | -0.53% | 3188.52 | 0.958x | 0.963x | 1.091x | -9.08% | 1.006x | -0.57% | 0.874x | +12.64% | 0 |
| reverse-proxy | 6955.50 | 6940.80 | 1.002x | +0.21% | 7112.69 | 0.978x | 0.976x | 0.889x | +11.10% | 0.875x | +12.54% | 0.843x | +15.65% | 0 |
| static-small | 13055.00 | 13094.80 | 0.997x | -0.30% | 13422.82 | 0.973x | 0.976x | 0.837x | +16.27% | 0.912x | +8.80% | 0.910x | +8.99% | 0 |

- Aggregate proxysss ops/s: `36284.00`
- Aggregate nginx ops/s: `36324.90`
- Aggregate proxysss/nginx ratio: `0.999x`
- Aggregate throughput improvement: `-0.11%`
