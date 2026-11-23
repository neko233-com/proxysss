# Docker two-host HTTP benchmark (2 CPU per container)

- Date: 2026-11-23 22:08:45
- Candidate: D:\Code\neko233-Projects\proxysss\.bench-tmp-bin\proxysss
- Server: Docker container `nginx:1.28`, 2 CPU / 2 GiB cgroup; backend nginx runs in the same server container
- Client: separate Docker container `debian:bookworm-slim`, 2 CPU / 2 GiB cgroup, Go HTTP/1.1 keep-alive, 64 preconnected connections
- Network: Docker bridge `proxysss-bench-two-host`; client connects to the server container DNS name, no published port and no Windows host client
- Server cgroup: cpu.max=200000 100000, memory.max=2147483648
- Client cgroup: cpu.max=200000 100000, memory.max=2147483648
- Samples: 3 rounds x 2s warm-up + 5s measurement; median RPS, worst error count reported
- Scope: static 1 KiB, static 64 KiB, 1 KiB reverse proxy only. This is a two-container Docker bridge comparison, not a physical two-host or cross-host production claim.

| Scenario | nginx RPS | proxysss RPS | Ratio | nginx p50/p95/p99 ms | proxysss p50/p95/p99 ms | nginx errors | proxysss errors |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| static-1k | 68,626 | 85,206 | 1.242x | 0.935 / 2.835 / 3.997 | 0.652 / 1.979 / 2.715 | 0 | 0 |
| static-64k | 38,635 | 53,776 | 1.392x | 1.583 / 4.350 / 5.784 | 1.141 / 3.225 / 4.215 | 0 | 0 |
| proxy-1k | 48,272 | 49,475 | 1.025x | 1.153 / 2.295 / 21.573 | 1.013 / 2.159 / 22.886 | 0 | 0 |
