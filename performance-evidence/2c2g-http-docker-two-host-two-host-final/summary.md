# 2C2G Docker two-host HTTP benchmark

- Date: 2026-11-22 17:41:10
- Candidate: C:\Users\14170\AppData\Local\Temp\proxysss-two-host-2q\proxysss
- Server: Docker container `nginx:1.28`, 2 CPU / 2 GiB cgroup; backend nginx runs in the same server container
- Client: separate Docker container `debian:bookworm-slim`, 2 CPU / 2 GiB cgroup, Go HTTP/1.1 keep-alive, 64 preconnected connections
- Network: Docker bridge `proxysss-bench-two-host`; client connects to the server container DNS name, no published port and no Windows host client
- Server cgroup: cpu.max=200000 100000, memory.max=2147483648
- Client cgroup: cpu.max=200000 100000, memory.max=2147483648
- Samples: 3 rounds x 2s warm-up + 5s measurement; median RPS, worst error count reported
- Scope: static 1 KiB, static 64 KiB, 1 KiB reverse proxy only. This is a two-container Docker bridge comparison, not a physical two-host or cross-host production claim.

| Scenario | nginx RPS | proxysss RPS | Ratio | nginx p50/p95/p99 ms | proxysss p50/p95/p99 ms | nginx errors | proxysss errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| static-1k | 73,955 | 88,555 | 1.197x | 0.784 / 2.282 / 3.206 | 0.620 / 1.891 / 2.621 | 0 | 0 |
| static-64k | 44,072 | 54,208 | 1.230x | 1.397 / 3.774 / 4.773 | 1.071 / 2.960 / 3.909 | 0 | 0 |
| proxy-1k | 43,933 | 88,490 | 2.014x | 1.216 / 2.171 / 20.549 | 0.641 / 2.022 / 2.921 | 0 | 0 |
