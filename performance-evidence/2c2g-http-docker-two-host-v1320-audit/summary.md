# 2C2G Docker two-host HTTP benchmark

- Date: 2026-11-23 19:27:03
- Candidate: D:\Code\neko233-Projects\proxysss\rel-bin\proxysss
- Server: Docker container `nginx:1.28`, 2 CPU / 2 GiB cgroup; backend nginx runs in the same server container
- Client: separate Docker container `debian:bookworm-slim`, 2 CPU / 2 GiB cgroup, Go HTTP/1.1 keep-alive, 64 preconnected connections
- Network: Docker bridge `proxysss-bench-two-host`; client connects to the server container DNS name, no published port and no Windows host client
- Server cgroup: cpu.max=200000 100000, memory.max=2147483648
- Client cgroup: cpu.max=200000 100000, memory.max=2147483648
- Samples: 3 rounds x 2s warm-up + 5s measurement; median RPS, worst error count reported
- Scope: static 1 KiB, static 64 KiB, 1 KiB reverse proxy only. This is a two-container Docker bridge comparison, not a physical two-host or cross-host production claim.

| Scenario | nginx RPS | proxysss RPS | Ratio | nginx p50/p95/p99 ms | proxysss p50/p95/p99 ms | nginx errors | proxysss errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| static-1k | 73,922 | 93,708 | 1.268x | 0.779 / 2.182 / 2.874 | 0.580 / 1.741 / 2.403 | 0 | 0 |
| static-64k | 47,126 | 57,074 | 1.211x | 1.294 / 3.438 / 4.281 | 0.996 / 2.691 / 3.433 | 0 | 0 |
| proxy-1k | 52,785 | 35,076 | 0.665x | 1.066 / 1.860 / 18.820 | 1.377 / 2.623 / 30.348 | 0 | 0 |
