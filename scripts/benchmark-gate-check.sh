#!/usr/bin/env bash
set -euo pipefail

RESULTS_FILE="${1:?results json path required}"
BASELINE_FILE="${2:-$(cd "$(dirname "$0")/.." && pwd)/scripts/benchmark-baseline.json}"

if [[ ! -f "$RESULTS_FILE" ]]; then
  echo "benchmark results not found: $RESULTS_FILE" >&2
  exit 1
fi
if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "benchmark baseline not found: $BASELINE_FILE" >&2
  exit 1
fi

python3 - "$RESULTS_FILE" "$BASELINE_FILE" <<'PY'
import json
import sys

results_path, baseline_path = sys.argv[1], sys.argv[2]
results = json.load(open(results_path, encoding="utf-8"))
baseline = json.load(open(baseline_path, encoding="utf-8"))

by_name = {row["name"]: row for row in results}
proxysss = by_name.get("proxysss")
nginx = by_name.get("nginx")
if not proxysss or not nginx:
    raise SystemExit("results missing proxysss or nginx entry")

max_errors = int(baseline.get("max_error_count", 0))
min_ratio = float(baseline.get("min_proxysss_vs_nginx_ops_ratio", 0.65))

for row in results:
    if int(row.get("errors", 0)) > max_errors:
        raise SystemExit(f"benchmark gate failed: {row['name']} reported {row['errors']} errors")

nginx_ops = float(nginx.get("ops_per_sec", 0))
proxysss_ops = float(proxysss.get("ops_per_sec", 0))
if nginx_ops <= 0:
    raise SystemExit("benchmark gate failed: nginx ops_per_sec is zero")

ratio = proxysss_ops / nginx_ops
print(f"benchmark gate: proxysss={proxysss_ops:.2f} ops/s nginx={nginx_ops:.2f} ops/s ratio={ratio:.3f} min={min_ratio:.3f}")
if ratio < min_ratio:
    raise SystemExit(f"benchmark gate failed: proxysss/nginx ops ratio {ratio:.3f} < required {min_ratio:.3f}")
print("benchmark gate passed")
PY
