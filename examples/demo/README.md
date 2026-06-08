# proxysss demos

These commands help you exercise proxysss without external dependencies.

## Built-in echo backends

Start lightweight stand-ins for upstream services:

```bash
# HTTP echo on 127.0.0.1:8081
proxysss demo http-echo

# TCP echo on 127.0.0.1:6379
proxysss demo tcp-echo

# UDP echo on 127.0.0.1:5353
proxysss demo udp-echo
```

## Full gateway lab

For multi-service routing, TLS, and automation examples see:

- [../lab-proxysss/README.md](../lab-proxysss/README.md)
- [../lab/README.md](../lab/README.md)

Typical flow:

```bash
proxysss init
proxysss -config ./proxysss.yaml check-config
proxysss -config ./proxysss.yaml
```

Then open:

- `http://127.0.0.1/` — welcome page
- `http://127.0.0.1:7777/` — admin dashboard (default `root` / `root`)
- `http://127.0.0.1/metrics` — Prometheus metrics (default `monitoring.format: prometheus`)

## Weighted load-balancing smoke test

Add to `proxysss.yaml`:

```yaml
load_balance:
  algorithm: weighted
services:
  reverse_proxy:
    routes:
      - name: weighted-api
        path_prefix: /
        upstream: http://127.0.0.1:8081
        upstreams: [http://127.0.0.1:8082]
        upstream_weights:
          http://127.0.0.1:8081: 1
          http://127.0.0.1:8082: 4
```

Run two echo servers on `8081` and `8082`, reload proxysss, and watch access logs skew toward the heavier backend.
