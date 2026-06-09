#!/usr/bin/env python3
"""Generate Markdown + HTML benchmark comparison tables from results.json."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def load_results(path: Path) -> list[dict]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise SystemExit("results.json must be a JSON array")
    return data


def nginx_row(rows: list[dict]) -> dict | None:
    for row in rows:
        if row.get("name") == "nginx":
            return row
    return None


def ratio_vs_nginx(row: dict, nginx: dict | None) -> float | None:
    if not nginx:
        return None
    nginx_ops = float(nginx.get("ops_per_sec", 0) or 0)
    if nginx_ops <= 0:
        return None
    return float(row.get("ops_per_sec", 0) or 0) / nginx_ops


def fmt_num(value: float | int | None, digits: int = 2) -> str:
    if value is None:
        return "—"
    if isinstance(value, int):
        return f"{value:,}"
    return f"{value:,.{digits}f}"


def build_markdown(rows: list[dict], meta: dict) -> str:
    rows = sorted(rows, key=lambda r: float(r.get("ops_per_sec", 0) or 0), reverse=True)
    nginx = nginx_row(rows)
    lines = [
        "# Gateway throughput benchmark",
        "",
        f"- Generated: {meta['generated_at']}",
        f"- Concurrency: {meta.get('concurrency', '—')}",
        f"- Duration: {meta.get('duration_secs', '—')}s",
        f"- Workload: static `index.html` over HTTP/1.1",
        "",
        "## Summary ranking (ops/sec)",
        "",
        "| Rank | Gateway | ops/sec | vs nginx | MiB/s | p50 ms | p95 ms | p99 ms | success | errors |",
        "| ---: | ------- | ------: | -------: | ----: | -----: | -----: | -----: | ------: | -----: |",
    ]
    for index, row in enumerate(rows, start=1):
        ratio = ratio_vs_nginx(row, nginx)
        ratio_text = f"{ratio:.2f}x" if ratio is not None else "—"
        lines.append(
            "| {rank} | {name} | {ops} | {ratio} | {mib} | {p50} | {p95} | {p99} | {success} | {errors} |".format(
                rank=index,
                name=row.get("name", "?"),
                ops=fmt_num(row.get("ops_per_sec")),
                ratio=ratio_text,
                mib=fmt_num(row.get("throughput_mib_s")),
                p50=fmt_num(row.get("latency_p50_ms")),
                p95=fmt_num(row.get("latency_p95_ms")),
                p99=fmt_num(row.get("latency_p99_ms")),
                success=fmt_num(row.get("success"), 0),
                errors=fmt_num(row.get("errors"), 0),
            )
        )

    if nginx:
        proxysss = next((r for r in rows if r.get("name") == "proxysss"), None)
        if proxysss:
            ratio = ratio_vs_nginx(proxysss, nginx)
            lines.extend(
                [
                    "",
                    "## proxysss vs nginx",
                    "",
                    f"- proxysss ops/sec: **{fmt_num(proxysss.get('ops_per_sec'))}**",
                    f"- nginx ops/sec: **{fmt_num(nginx.get('ops_per_sec'))}**",
                    f"- ratio: **{ratio:.3f}x**" if ratio else "- ratio: unavailable",
                ]
            )

    lines.extend(["", "## Raw targets", ""])
    for row in rows:
        lines.append(f"- `{row.get('name')}` → `{row.get('url', '')}`")

    lines.append("")
    return "\n".join(lines)


def build_html(rows: list[dict], meta: dict) -> str:
    rows = sorted(rows, key=lambda r: float(r.get("ops_per_sec", 0) or 0), reverse=True)
    nginx = nginx_row(rows)
    proxysss = next((r for r in rows if r.get("name") == "proxysss"), None)
    ratio = ratio_vs_nginx(proxysss, nginx) if proxysss and nginx else None

    table_rows = []
    for index, row in enumerate(rows, start=1):
        row_ratio = ratio_vs_nginx(row, nginx)
        ratio_text = f"{row_ratio:.2f}x" if row_ratio is not None else "—"
        winner = index == 1
        table_rows.append(
            f"""
      <tr class="{'winner' if winner else ''}">
        <td>{index}</td>
        <td><strong>{row.get('name', '?')}</strong></td>
        <td>{fmt_num(row.get('ops_per_sec'))}</td>
        <td>{ratio_text}</td>
        <td>{fmt_num(row.get('throughput_mib_s'))}</td>
        <td>{fmt_num(row.get('latency_p50_ms'))}</td>
        <td>{fmt_num(row.get('latency_p95_ms'))}</td>
        <td>{fmt_num(row.get('latency_p99_ms'))}</td>
        <td>{fmt_num(row.get('success'), 0)}</td>
        <td>{fmt_num(row.get('errors'), 0)}</td>
      </tr>"""
        )

    ratio_banner = (
        f"<p class='hero'>proxysss / nginx = <strong>{ratio:.3f}x</strong></p>"
        if ratio is not None
        else ""
    )

    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>proxysss gateway benchmark</title>
  <style>
    :root {{
      color-scheme: light dark;
      --bg: #0b1020;
      --panel: #121a2f;
      --text: #e8eefc;
      --muted: #9fb0d9;
      --line: #24304f;
      --accent: #5eead4;
      --winner: #163d35;
    }}
    body {{
      margin: 0;
      font-family: ui-sans-serif, system-ui, Segoe UI, sans-serif;
      background: linear-gradient(180deg, #0b1020 0%, #111827 100%);
      color: var(--text);
      line-height: 1.5;
    }}
    main {{ max-width: 1100px; margin: 0 auto; padding: 2rem 1.25rem 3rem; }}
    h1 {{ margin: 0 0 0.5rem; font-size: 2rem; }}
    .meta {{ color: var(--muted); margin-bottom: 1rem; }}
    .hero {{
      display: inline-block;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 999px;
      padding: 0.35rem 0.9rem;
      margin: 0 0 1.25rem;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 12px;
      overflow: hidden;
    }}
    th, td {{ padding: 0.75rem 0.9rem; border-bottom: 1px solid var(--line); text-align: right; }}
    th:first-child, td:first-child, th:nth-child(2), td:nth-child(2) {{ text-align: left; }}
    th {{ color: var(--muted); font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.04em; }}
    tr.winner td {{ background: var(--winner); }}
    tr:last-child td {{ border-bottom: 0; }}
    .footer {{ margin-top: 1rem; color: var(--muted); font-size: 0.9rem; }}
    strong {{ color: var(--accent); }}
  </style>
</head>
<body>
  <main>
    <h1>Gateway throughput benchmark</h1>
    <p class="meta">Generated {meta['generated_at']} · concurrency {meta.get('concurrency', '—')} · duration {meta.get('duration_secs', '—')}s · static index.html over HTTP/1.1</p>
    {ratio_banner}
    <table>
      <thead>
        <tr>
          <th>#</th>
          <th>Gateway</th>
          <th>ops/sec</th>
          <th>vs nginx</th>
          <th>MiB/s</th>
          <th>p50</th>
          <th>p95</th>
          <th>p99</th>
          <th>success</th>
          <th>errors</th>
        </tr>
      </thead>
      <tbody>
        {''.join(table_rows)}
      </tbody>
    </table>
    <p class="footer">Vendor binaries and run artifacts are stored under <code>.benchmark/</code> (gitignored).</p>
  </main>
</body>
</html>
"""


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", required=True, help="Path to results.json")
    parser.add_argument("--out-dir", required=True, help="Directory for report.md and report.html")
    parser.add_argument("--concurrency", type=int, default=0)
    parser.add_argument("--duration-secs", type=int, default=0)
    args = parser.parse_args()

    results_path = Path(args.results)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    rows = load_results(results_path)
    if not rows:
        raise SystemExit("results.json is empty")

    concurrency = args.concurrency or int(rows[0].get("concurrency", 0) or 0)
    duration = args.duration_secs or int(rows[0].get("duration_secs", 0) or 0)
    meta = {
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
        "concurrency": concurrency,
        "duration_secs": duration,
    }

    md_path = out_dir / "report.md"
    html_path = out_dir / "report.html"
    md_path.write_text(build_markdown(rows, meta), encoding="utf-8")
    html_path.write_text(build_html(rows, meta), encoding="utf-8")

    print(f"benchmark report markdown: {md_path}")
    print(f"benchmark report html:     {html_path}")


if __name__ == "__main__":
    main()
