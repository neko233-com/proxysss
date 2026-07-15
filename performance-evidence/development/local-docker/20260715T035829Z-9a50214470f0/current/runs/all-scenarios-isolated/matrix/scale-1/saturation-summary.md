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
- Result file: `/Users/solarisneko/Desktop/Code/neko233-Projects/proxysss/.benchmark/direct-ubuntu24-amd64/20260715T035829Z-9a50214470f0/current/runs/all-scenarios-isolated/matrix/scale-1/saturation-results.json`

| Scenario | proxysss ops/s | nginx ops/s | Ops ratio | Ops improvement | Target ops/s | proxy completion | nginx completion | p50 ratio | p50 improvement | p95 ratio | p95 improvement | p99 ratio | p99 improvement | Errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cdn-hot-update | 31992.00 | 28312.33 | 1.130x | +13.00% | - | - | - | 0.755x | +24.50% | 1.140x | -13.95% | 1.008x | -0.84% | 0 |
| game-long-connection | 4380.33 | 3832.67 | 1.143x | +14.29% | - | - | - | 0.666x | +33.37% | 1.290x | -29.02% | 1.102x | -10.16% | 0 |
| generic-sse | 696.00 | 445.67 | 1.562x | +56.17% | - | - | - | 0.541x | +45.88% | 0.883x | +11.70% | 0.788x | +21.20% | 0 |
| https-static-small | 5654.00 | 5946.67 | 0.951x | -4.92% | - | - | - | 0.809x | +19.10% | 1.494x | -49.43% | 1.117x | -11.73% | 0 |
| qcp-transparent | 5333.67 | 3424.33 | 1.558x | +55.76% | - | - | - | 0.524x | +47.60% | 0.897x | +10.29% | 0.974x | +2.60% | 0 |
| reverse-proxy | 15112.33 | 11819.67 | 1.279x | +27.86% | - | - | - | 0.734x | +26.62% | 1.008x | -0.82% | 0.953x | +4.67% | 0 |
| static-large | 94.00 | 94.67 | 0.993x | -0.71% | - | - | - | 0.981x | +1.95% | 1.104x | -10.37% | 1.264x | -26.42% | 0 |
| static-small | 33700.33 | 27693.33 | 1.217x | +21.69% | - | - | - | 0.687x | +31.33% | 1.042x | -4.25% | 1.005x | -0.49% | 0 |
| tcp-stream | 4447.00 | 3599.67 | 1.235x | +23.54% | - | - | - | 0.617x | +38.29% | 1.214x | -21.37% | 1.081x | -8.09% | 0 |
| udp-stream | 5231.00 | 3763.67 | 1.390x | +38.99% | - | - | - | 0.576x | +42.41% | 0.990x | +1.05% | 0.931x | +6.93% | 0 |
| websocket-long-connection | 4158.00 | 3682.67 | 1.129x | +12.91% | - | - | - | 0.669x | +33.09% | 1.302x | -30.21% | 1.208x | -20.84% | 0 |

- Aggregate proxysss ops/s: `110798.66`
- Aggregate nginx ops/s: `92615.35`
- Aggregate proxysss/nginx ratio: `1.196x`
- Aggregate throughput improvement: `+19.63%`
