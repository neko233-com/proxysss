# proxysss

**English** · [简体中文](README-CN.md) · [Documentation](https://neko233-com.github.io/proxysss/) · [Downloads](https://github.com/neko233-com/proxysss/releases/latest)

One Rust binary and one YAML file for websites, APIs, and realtime connections. proxysss is a general-purpose gateway designed to replace nginx. Optional embedded TypeScript scripts and plugins handle application-specific extensions.

## Install and run in the background

Windows PowerShell:

```powershell
& ([ScriptBlock]::Create((irm https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.ps1))) -Action install -Version latest
```

Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/proxysss/main/scripts/install.sh | bash
```

Check the installation and startup registration:

```bash
proxysss --version
proxysss check-config
proxysss service status
```

For a manual first installation, run `proxysss init`, then `proxysss service install`. Windows uses a hidden launcher at user logon; Linux uses systemd, and macOS uses a LaunchAgent. Linux listeners on ports 80/443 require the appropriate port permissions.

| Address | Purpose |
| --- | --- |
| `http://127.0.0.1/` | Minimal welcome page and documentation links on port 80 |
| `http://127.0.0.1:7777/` | Admin console, bound to loopback by default |
| `http://127.0.0.1:7777/docs.html` | Built-in documentation from the admin listener |
| `http://127.0.0.1/docs.html` | Built-in documentation from the public listener |

Fresh installations use `root` / `root` for the console. Change these credentials before production use. Configuration exposure and write operations are disabled by default; viewing runtime status does not require enabling either. The console retains login and session verification.

## Configure your first website

The default configuration file is `proxysss.yaml`. Select another path with `-c`, `--config`, or `-config`. Merge this example into the corresponding block of your existing configuration:

```yaml
services:
  reverse_proxy:
    routes:
      - name: my-app
        hosts: [app.example.com]
        path_prefix: /
        upstream: http://127.0.0.1:3000
```

Point the domain to your server and make sure the upstream is running. Validate the configuration, inspect reload boundaries, and restart the background process:

```bash
proxysss check-config
proxysss config reload-plan
proxysss restart
```

To serve a static website:

```yaml
services:
  static_sites:
    - name: my-static-site
      path_prefix: /
      root: ./public
      index_files: [index.html]
      autoindex: false
```

Static delivery supports HTML, images, fonts, audio/video, streaming large files, HEAD, byte Range downloads, and cache validators. Directory listings are opt-in. Dotfiles are hidden by default, and resolved paths remain within the site root. See the [static and CDN origin guide](https://neko233-com.github.io/proxysss/cdn-origin.html).

## Capabilities

| Area | Support |
| --- | --- |
| Websites and APIs | HTTP/1.1, HTTPS, HTTP/2, HTTP/3, gRPC-over-HTTP/2, WebSocket, reverse proxying, compression, cache/proxy cache |
| Traffic policy | IP/CIDR access control, fixed-window / token-bucket / leaky-bucket rate limiting, health checks, weighted upstreams, retries and passive quarantine |
| Files and CDN | Static sites, authenticated CDN origin requests, signed URLs, WebDAV, FTP and FileCloud |
| Realtime protocols | TCP/UDP, game connections, MQTT TCP/TLS/WebSocket, CoAP-style UDP, transparent KCP/QCP forwarding |
| AI and discovery | New API / sub2api / OpenAI-compatible forwarding; Consul / etcd / Nacos registry integration |
| Certificates and extensions | Built-in ACME, wildcard DNS-01, configuration reload, in-process TypeScript and optional plugins |

MQTT application behavior remains in the upstream broker; proxysss forwards edge traffic. TypeScript runs through embedded QuickJS and in-process type stripping, without Node, Deno, or an external compiler.

Use `http.tls.auto_https.domains` for automatic certificates on ordinary domains. Wildcards use `http.tls.acme.challenge: dns01` and a configured DNS provider. See the [configuration guide](https://neko233-com.github.io/proxysss/configuration.html).

## Operate and inspect

```bash
proxysss config explain
proxysss config routes
proxysss config security
proxysss config performance
proxysss config watched-scripts
proxysss config nginx-parity --format yaml
proxysss service status
```

Access and error logs default to `logs/access.log` and `logs/error.log`, with level `info`. Security and performance settings include defaults and recommendations in the [security and platform guide](https://neko233-com.github.io/proxysss/security-performance.html).

Performance validation runs locally on Linux. GitHub Actions validates release materials and packages six platforms; it does not run tests or benchmarks. A functional release does not establish strict performance superiority over nginx.

## Documentation

The official detailed guides are Chinese first; both README editions cover installation and everyday operation.

- [Getting started and examples](https://neko233-com.github.io/proxysss/)
- [Configuration](https://neko233-com.github.io/proxysss/configuration.html) · [Architecture](https://neko233-com.github.io/proxysss/architecture.html)
- [TypeScript and plugins](https://neko233-com.github.io/proxysss/ts-how-to-use.html)
- [Migrate from nginx](https://neko233-com.github.io/proxysss/nginx-to-proxysss.html) · [Migrate from Caddy](https://neko233-com.github.io/proxysss/caddy-to-proxysss.html)

Local validation, caches and packages use fixed project-local `.tmp`, `.cache`, `target` and `dist` paths. On Windows, run `test.cmd` or `scripts/verify-docker-scenarios.ps1`. Scenario validation uses the fixed Docker container `proxysss-verify` and removes it when finished.
