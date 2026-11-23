# 2C2G Docker two-host HTTP benchmark

- Date: 2026-11-23 19:31:50
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
| static-1k | 90,933 | 94,962 | 1.044x | 0.752 / 2.111 / 2.825 | 0.589 / 1.739 / 2.365 | 0 | 0 |
| static-64k | 46,195 | 57,548 | 1.246x | 1.304 / 3.460 / 4.310 | 1.014 / 2.772 / 3.641 | 0 | 0 |
| proxy-1k | 48,009 | 35,342 | 0.736x | 1.082 / 1.818 / 19.519 | 1.311 / 2.547 / 30.612 | 0 | 0 |
