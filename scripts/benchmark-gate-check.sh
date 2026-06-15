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

command -v go >/dev/null 2>&1 || {
  echo "missing required command: go" >&2
  exit 1
}

go run "$(cd "$(dirname "$0")" && pwd)/benchmark-helper.go" check-simple-gate \
  --results "$RESULTS_FILE" \
  --baseline "$BASELINE_FILE"
