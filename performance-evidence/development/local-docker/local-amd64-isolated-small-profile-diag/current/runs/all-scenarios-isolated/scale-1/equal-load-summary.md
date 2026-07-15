# proxysss all-scenarios benchmark

- Matrix mode: `mixed concurrent`
- Measurement phase: `equal-offered-load`
- Interleaved samples per gateway: `1` (median metrics, maximum observed errors)
- Detected CPU cores: `2`
- Runtime traffic profile: `small`
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
- Result file: `/work/.benchmark/direct-ubuntu24-amd64/local-amd64-isolated-small-profile-diag/current/runs/all-scenarios-isolated/scale-1/equal-load-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
- Reference under-target warnings: `cdn-hot-update nginx target achievement 0.966 < 0.980 (actual=5100.00 target=5281.40); https-static-small nginx target achievement 0.924 < 0.980 (actual=2075.25 target=2245.30); reverse-proxy nginx target achievement 0.977 < 0.980 (actual=2652.75 target=2714.39); static-small nginx target achievement 0.975 < 0.980 (actual=5264.00 target=5397.20)`
| cdn-hot-update | 5135.00 | 5100.00 | 1.007x | +0.69% | 5281.40 | 0.972x | 0.966x | 0.952x | +4.80% | 1.993x | -99.26% | 2.253x | -125.35% | 0 |
| game-long-connection | 2862.75 | 2857.25 | 1.002x | +0.19% | 2877.70 | 0.995x | 0.993x | 1.123x | -12.29% | 1.647x | -64.74% | 1.293x | -29.26% | 0 |
| generic-sse | 102.25 | 103.00 | 0.993x | -0.73% | 104.30 | 0.980x | 0.988x | 1.091x | -9.15% | 2.081x | -108.07% | 2.025x | -102.50% | 1 |
| https-static-small | 2079.75 | 2075.25 | 1.002x | +0.22% | 2245.30 | 0.926x | 0.924x | 1.133x | -13.27% | 1.072x | -7.22% | 0.991x | +0.93% | 0 |
| qcp-transparent | 2672.00 | 2669.00 | 1.001x | +0.11% | 2693.60 | 0.992x | 0.991x | 1.097x | -9.68% | 1.309x | -30.91% | 1.072x | -7.16% | 0 |
| reverse-proxy | 2624.75 | 2652.75 | 0.989x | -1.06% | 2714.39 | 0.967x | 0.977x | 1.036x | -3.59% | 1.727x | -72.71% | 1.631x | -63.14% | 0 |
| static-large | 59.75 | 59.75 | 1.000x | +0.00% | 60.02 | 0.996x | 0.996x | 1.134x | -13.41% | 1.331x | -33.07% | 1.151x | -15.13% | 0 |
| static-small | 5298.00 | 5264.00 | 1.006x | +0.65% | 5397.20 | 0.982x | 0.975x | 1.025x | -2.49% | 2.110x | -111.04% | 2.326x | -132.55% | 0 |
| tcp-stream | 2826.00 | 2819.75 | 1.002x | +0.22% | 2843.94 | 0.994x | 0.991x | 1.139x | -13.93% | 1.406x | -40.55% | 1.261x | -26.10% | 0 |
| udp-stream | 2677.25 | 2684.50 | 0.997x | -0.27% | 2705.44 | 0.990x | 0.992x | 1.116x | -11.56% | 1.328x | -32.80% | 1.149x | -14.92% | 0 |
| websocket-long-connection | 2508.00 | 2506.00 | 1.001x | +0.08% | 2530.84 | 0.991x | 0.990x | 1.109x | -10.92% | 1.361x | -36.13% | 1.111x | -11.14% | 0 |

- Aggregate proxysss ops/s: `28845.50`
- Aggregate nginx ops/s: `28791.25`
- Aggregate proxysss/nginx ratio: `1.002x`
- Aggregate throughput improvement: `+0.19%`
