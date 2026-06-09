#!/usr/bin/env python3
"""Generate combined nginx feature parity + throughput comparison reports (MD + HTML)."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    import yaml
except ImportError:
    yaml = None

# Extended nginx surface beyond the built-in parity matrix (honest gaps).
EXTENDED_NGINX_FEATURES = [
    ("HTTP reverse proxy", "supported", "services.reverse_proxy + domain_routes"),
    ("HTTPS / HTTP/2 termination", "supported", "http.tls_bind + rustls"),
    ("HTTP/3 (QUIC)", "supported", "http.h3_bind"),
    ("WebSocket / WSS", "supported", "upgrade-aware reverse proxy"),
    ("gRPC over HTTP/2", "supported", "transparent h2 proxy"),
    ("Static files", "supported", "services.static_sites"),
    ("WebDAV", "supported", "built-in WebDAV runtime"),
    ("TCP stream proxy", "supported", "tcp.listeners"),
    ("UDP stream proxy", "supported", "udp.listeners"),
    ("TLS SNI multi-cert", "supported", "http.tls.certificates + domain ssl"),
    ("Managed ACME (HTTP-01 / TLS-ALPN-01)", "supported", "http.tls.auto_https"),
    ("Wildcard ACME (DNS-01)", "partial", "explicit acme_dns_external + acme.sh only"),
    ("On-demand TLS", "supported", "http.tls.on_demand"),
    ("Proxy cache", "supported", "Cloudflare-style cache behaviors"),
    ("Compression (gzip/br/zstd)", "supported", "response_policy + route overrides"),
    ("Rate limiting", "supported", "fixed-window / token-bucket / leaky-bucket"),
    ("IP allow/deny", "supported", "services.access_control"),
    ("Active health checks", "supported", "load_balance.active_health"),
    ("Load balancing algorithms", "supported", "round-robin / weighted / rendezvous / least-conn"),
    ("FTP proxy", "partial", "control + data rewrite; not full nginx ftp module parity"),
    ("Mail proxy (SMTP/IMAP/POP3)", "missing", "not in scope"),
    ("njs / Lua scripting", "partial", "embedded TypeScript/JS plugins instead of njs"),
    ("nginx config language", "missing", "single YAML model by design"),
    ("geo / map / split_clients modules", "partial", "access_control + script plugins cover subsets"),
    ("upstream keepalive tuning knobs", "partial", "connection pooling via async runtime; fewer directives"),
    ("stub_status module", "partial", "Prometheus / JSON /admin stats instead"),
    ("Linux sysctl auto-tuning", "supported", "proxysss tune linux --distro ubuntu-24.04|debian-12 ..."),
]


def load_parity_from_binary(binary: Path) -> list[dict]:
    if yaml is None:
        return []
    output = subprocess.check_output(
        [str(binary), "config", "nginx-parity", "--format", "yaml"],
        text=True,
        encoding="utf-8",
    )
    data = yaml.safe_load(output)
    return data if isinstance(data, list) else []


def load_benchmark(path: Path | None) -> list[dict]:
    if not path or not path.exists():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, list) else []


def status_label(status: str) -> str:
    mapping = {
        "supported": "✅ 有",
        "partial": "⚠️ 部分",
        "missing": "❌ 无",
    }
    return mapping.get(status.lower(), status)


def build_markdown(parity: list[dict], bench: list[dict], meta: dict) -> str:
    lines = [
        "# proxysss vs nginx — 功能与性能对比",
        "",
        f"- 生成时间: {meta['generated_at']}",
        f"- proxysss 版本: {meta.get('version', 'unknown')}",
        "",
        "## 一、核心 nginx parity（代码内置矩阵）",
        "",
        "| nginx 能力 | proxysss 状态 | 证据 / 说明 | 剩余差距 |",
        "| --- | --- | --- | --- |",
    ]
    for item in parity:
        cap = item.get("capability", "?")
        status = item.get("status", "?")
        evidence = (item.get("evidence") or "").replace("|", "/")
        gap = (item.get("next_gap") or "—").replace("|", "/")
        lines.append(f"| {cap} | {status_label(str(status))} | {evidence} | {gap or '—'} |")

    lines.extend(
        [
            "",
            "## 二、扩展 nginx 模块对照（nginx 有 / proxysss 有没有）",
            "",
            "| nginx 常见能力 | proxysss | 说明 |",
            "| --- | --- | --- |",
        ]
    )
    for cap, status, note in EXTENDED_NGINX_FEATURES:
        lines.append(f"| {cap} | {status_label(status)} | {note} |")

    if bench:
        bench = sorted(bench, key=lambda r: float(r.get("ops_per_sec", 0) or 0), reverse=True)
        nginx = next((r for r in bench if r.get("name") == "nginx"), None)
        proxysss_row = next((r for r in bench if r.get("name") == "proxysss"), None)
        lines.extend(["", "## 三、吞吐性能对比（同 payload 静态文件）", ""])
        lines.append(
            f"- 并发: {bench[0].get('concurrency', '—')} · 时长: {bench[0].get('duration_secs', '—')}s"
        )
        if proxysss_row and nginx:
            ratio = float(proxysss_row.get("ops_per_sec", 0)) / float(nginx.get("ops_per_sec", 1))
            lines.append(
                f"- **proxysss / nginx ops/sec 比值: {ratio:.3f}x** ({proxysss_row.get('ops_per_sec'):.1f} vs {nginx.get('ops_per_sec'):.1f})"
            )
        lines.extend(
            [
                "",
                "| 排名 | 网关 | ops/sec | vs nginx | MiB/s | p50 ms | p95 ms | 错误 |",
                "| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for i, row in enumerate(bench, 1):
            ratio = "—"
            if nginx and float(nginx.get("ops_per_sec", 0)) > 0:
                ratio = f"{float(row.get('ops_per_sec', 0)) / float(nginx.get('ops_per_sec')):.2f}x"
            lines.append(
                f"| {i} | {row.get('name')} | {row.get('ops_per_sec', 0):.1f} | {ratio} | "
                f"{row.get('throughput_mib_s', 0):.2f} | {row.get('latency_p50_ms', 0):.2f} | "
                f"{row.get('latency_p95_ms', 0):.2f} | {row.get('errors', 0)} |"
            )
    else:
        lines.extend(["", "## 三、吞吐性能对比", "", "_尚未运行 benchmark；执行 `scripts/benchmark-gateways.ps1` 后重新生成。_"])

    lines.extend(
        [
            "",
            "## 四、Linux 平台可选优化（Ubuntu / Debian）",
            "",
            "```bash",
            "# 非交互：Ubuntu 24.04 edge 网关 sysctl 配置",
            "proxysss tune linux --distro ubuntu-2404 --profile edge --output ./proxysss-tcp.sysctl.conf",
            "",
            "# 在 Linux 主机上应用（需 root）",
            "sudo proxysss tune linux --distro debian-12 --profile edge --apply",
            "```",
            "",
            "支持 `--distro auto|ubuntu-2204|ubuntu-2404|debian-12|debian-13`，`--profile edge|bulk|latency`。",
            "",
        ]
    )
    return "\n".join(lines)


def build_html(md_body: str, parity: list[dict], bench: list[dict], meta: dict) -> str:
    parity_rows = "".join(
        f"<tr><td>{i.get('capability','')}</td><td>{status_label(str(i.get('status','')))}</td>"
        f"<td>{i.get('evidence','')}</td><td>{i.get('next_gap') or '—'}</td></tr>"
        for i in parity
    )
    extended_rows = "".join(
        f"<tr><td>{cap}</td><td>{status_label(st)}</td><td>{note}</td></tr>"
        for cap, st, note in EXTENDED_NGINX_FEATURES
    )
    bench_section = "<p><em>未运行 benchmark</em></p>"
    if bench:
        bench = sorted(bench, key=lambda r: float(r.get("ops_per_sec", 0) or 0), reverse=True)
        nginx = next((r for r in bench if r.get("name") == "nginx"), None)
        rows = []
        for i, row in enumerate(bench, 1):
            ratio = "—"
            if nginx and float(nginx.get("ops_per_sec", 0)) > 0:
                ratio = f"{float(row.get('ops_per_sec', 0)) / float(nginx.get('ops_per_sec')):.2f}x"
            rows.append(
                f"<tr><td>{i}</td><td><strong>{row.get('name')}</strong></td>"
                f"<td>{row.get('ops_per_sec', 0):.1f}</td><td>{ratio}</td>"
                f"<td>{row.get('throughput_mib_s', 0):.2f}</td>"
                f"<td>{row.get('latency_p50_ms', 0):.2f}</td>"
                f"<td>{row.get('latency_p95_ms', 0):.2f}</td>"
                f"<td>{row.get('errors', 0)}</td></tr>"
            )
        proxysss_row = next((r for r in bench if r.get("name") == "proxysss"), None)
        hero = ""
        if proxysss_row and nginx:
            ratio = float(proxysss_row.get("ops_per_sec", 0)) / float(nginx.get("ops_per_sec", 1))
            hero = f"<p class='hero'>proxysss / nginx = <strong>{ratio:.3f}x</strong> ops/sec</p>"
        bench_section = hero + (
            "<table><thead><tr><th>#</th><th>Gateway</th><th>ops/sec</th><th>vs nginx</th>"
            "<th>MiB/s</th><th>p50</th><th>p95</th><th>errors</th></tr></thead><tbody>"
            + "".join(rows)
            + "</tbody></table>"
        )

    return f"""<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<title>proxysss vs nginx</title>
<style>
body {{ font-family: Segoe UI, system-ui, sans-serif; margin: 0; background: #0b1020; color: #e8eefc; }}
main {{ max-width: 1200px; margin: 0 auto; padding: 2rem 1rem 3rem; }}
h1,h2 {{ color: #5eead4; }}
table {{ width: 100%; border-collapse: collapse; margin: 1rem 0 2rem; background: #121a2f; }}
th,td {{ border: 1px solid #24304f; padding: 0.55rem 0.7rem; text-align: left; }}
th {{ color: #9fb0d9; font-size: 0.85rem; }}
.hero {{ background: #163d35; display: inline-block; padding: 0.4rem 0.9rem; border-radius: 999px; }}
code {{ background: #1e293b; padding: 0.1rem 0.35rem; border-radius: 4px; }}
</style>
</head>
<body>
<main>
<h1>proxysss vs nginx — 功能与性能对比</h1>
<p>Generated {meta['generated_at']} · proxysss {meta.get('version','')}</p>
<h2>核心 nginx parity</h2>
<table><thead><tr><th>能力</th><th>proxysss</th><th>证据</th><th>差距</th></tr></thead><tbody>{parity_rows}</tbody></table>
<h2>扩展 nginx 模块对照</h2>
<table><thead><tr><th>nginx 能力</th><th>proxysss</th><th>说明</th></tr></thead><tbody>{extended_rows}</tbody></table>
<h2>吞吐性能</h2>
{bench_section}
<h2>Linux 优化</h2>
<p>可选：<code>proxysss tune linux --distro ubuntu-2404 --profile edge --apply</code></p>
</main>
</body>
</html>"""


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="", help="path to proxysss binary")
    parser.add_argument("--benchmark", default="", help="path to results.json")
    parser.add_argument("--out-dir", required=True)
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[1]
    binary = Path(args.binary) if args.binary else repo / "target" / "release" / ("proxysss.exe" if sys.platform == "win32" else "proxysss")
    bench_path = Path(args.benchmark) if args.benchmark else repo / ".benchmark" / "runs" / "latest" / "results.json"
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    version = "unknown"
    if binary.exists():
        version = subprocess.check_output([str(binary), "--version"], text=True).strip()

    parity = load_parity_from_binary(binary) if binary.exists() and yaml else []
    bench = load_benchmark(bench_path if bench_path.exists() else None)
    meta = {"generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"), "version": version}

    md = build_markdown(parity, bench, meta)
    html = build_html(md, parity, bench, meta)
    md_path = out_dir / "nginx-compare.md"
    html_path = out_dir / "nginx-compare.html"
    md_path.write_text(md, encoding="utf-8")
    html_path.write_text(html, encoding="utf-8")
    print(f"nginx compare markdown: {md_path}")
    print(f"nginx compare html:     {html_path}")


if __name__ == "__main__":
    main()
