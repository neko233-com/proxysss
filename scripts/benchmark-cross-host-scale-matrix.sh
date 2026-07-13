#!/usr/bin/env bash
# Replays the strict three-host WSS gate at increasing active-session scales.
# Capacity remains a realistic 20k idle hold by default; it is deliberately not
# multiplied into a synthetic 100k connection requirement.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

[[ "$(uname -s)" == "Linux" ]] || {
  echo "benchmark-cross-host-scale-matrix.sh must run from a Linux client host" >&2
  exit 1
}

SCALE_FACTORS="${SCALE_FACTORS:-1 2 4}"
BASE_CONNECTIONS="${BASE_CONNECTIONS:-4096}"
CAPACITY_CONNECTIONS="${CAPACITY_CONNECTIONS:-20000}"
RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
RUN_ROOT="$BENCH_ROOT/runs/cross-host-wss-scale-matrix/$RUN_ID"

[[ "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]] || {
  echo "RUN_ID contains unsafe characters" >&2
  exit 1
}
[[ "$BASE_CONNECTIONS" =~ ^[1-9][0-9]*$ ]] || {
  echo "BASE_CONNECTIONS must be positive" >&2
  exit 1
}
[[ "$CAPACITY_CONNECTIONS" =~ ^[1-9][0-9]*$ ]] || {
  echo "CAPACITY_CONNECTIONS must be positive" >&2
  exit 1
}

mkdir -p "$RUN_ROOT"
for scale in $SCALE_FACTORS; do
  [[ "$scale" =~ ^[1-9][0-9]*$ ]] || {
    echo "invalid scale factor: $scale" >&2
    exit 1
  }
  child_id="${RUN_ID}-scale-${scale}"
  child_connections=$((BASE_CONNECTIONS * scale))
  echo "==> strict cross-host WSS scale ${scale}x (${child_connections} active, ${CAPACITY_CONNECTIONS} idle)"
  BENCH_ROOT="$RUN_ROOT" \
  RUN_ID="$child_id" \
  CONNECTIONS="$child_connections" \
  CAPACITY_CONNECTIONS="$CAPACITY_CONNECTIONS" \
  bash "$ROOT/scripts/benchmark-cross-host-wss.sh"

  child_metadata="$RUN_ROOT/runs/cross-host-wss/$child_id/run-metadata.txt"
  test -f "$child_metadata"
  printf 'scale=%s\nactive_connections=%s\ncapacity_connections=%s\n' \
    "$scale" "$child_connections" "$CAPACITY_CONNECTIONS" >>"$child_metadata"
done

echo "strict cross-host scale matrix passed for factors: $SCALE_FACTORS"
echo "results: $RUN_ROOT"
