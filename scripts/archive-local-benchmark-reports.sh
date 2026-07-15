#!/usr/bin/env bash
# Archive small, reviewable benchmark evidence while leaving multi-GiB build,
# container, binary, and client scratch artifacts under ignored .benchmark/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="${1:-$ROOT/.benchmark/direct-ubuntu24-amd64}"
DESTINATION="${2:-$ROOT/performance-evidence/development/local-docker}"

if [[ ! -d "$SOURCE" ]]; then
  echo "benchmark report source does not exist: $SOURCE" >&2
  exit 1
fi
command -v rsync >/dev/null 2>&1 || {
  echo "rsync is required to archive benchmark reports" >&2
  exit 1
}

mkdir -p "$DESTINATION"
rsync -a --delete --prune-empty-dirs \
  --include='*/' \
  --include='host-fingerprint.txt' \
  --include='validation-timing.txt' \
  --include='matrix.log' \
  --include='run-metadata.txt' \
  --include='equal-load-plan.txt' \
  --include='*-summary.md' \
  --include='*-summary.html' \
  --include='*-results.json' \
  --include='*-results.jsonl' \
  --include='*-gateway-memory-*.txt' \
  --exclude='*' \
  "$SOURCE/" "$DESTINATION/"

index="$DESTINATION/INDEX.tsv"
printf 'run\tcommit\texecution_mode\tvalidation_secs\tscales\tstatus\n' >"$index"
for run_dir in "$DESTINATION"/*; do
  [[ -d "$run_dir" ]] || continue
  run="${run_dir##*/}"
  fingerprint="$run_dir/host-fingerprint.txt"
  commit=""
  execution_mode="unknown"
  validation_secs=""
  if [[ -f "$fingerprint" ]]; then
    commit="$(sed -n 's/^commit=//p' "$fingerprint" | tail -1)"
    execution_mode="$(sed -n 's/^execution_mode=//p' "$fingerprint" | tail -1)"
    validation_secs="$(sed -n 's/^validation_elapsed_secs=//p' "$fingerprint" | tail -1)"
  fi
  if [[ -z "$validation_secs" && -f "$run_dir/validation-timing.txt" ]]; then
    validation_secs="$(sed -n 's/^validation_elapsed_secs=//p' "$run_dir/validation-timing.txt" | tail -1)"
  fi
  [[ -n "$commit" ]] || commit="unknown"
  [[ -n "$execution_mode" ]] || execution_mode="unknown"
  [[ -n "$validation_secs" ]] || validation_secs="unknown"

  scales=""
  for scale_dir in "$run_dir"/scale-*; do
    [[ -d "$scale_dir" ]] || continue
    scale="${scale_dir##*/scale-}"
    if [[ -f "$scale_dir/saturation-summary.md" || -f "$scale_dir/equal-load-summary.md" ]]; then
      scales="${scales}${scales:+,}${scale}"
    fi
  done
  [[ -n "$scales" ]] || scales="none"

  status="incomplete"
  if [[ -f "$run_dir/matrix.log" ]]; then
    if grep -q 'all strict Ubuntu 24 x86_64 Docker scales passed' "$run_dir/matrix.log"; then
      status="passed"
    elif grep -Eq 'strict matrix failed|benchmark gate failed|scale [0-9]+ failed' "$run_dir/matrix.log"; then
      status="failed"
    fi
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$run" "$commit" "$execution_mode" "$validation_secs" "$scales" "$status" >>"$index"
done

report_count="$(find "$DESTINATION" -type f | wc -l | tr -d ' ')"
report_bytes="$(du -sk "$DESTINATION" | awk '{print $1 * 1024}')"
echo "archived_reports=$report_count"
echo "archived_bytes=$report_bytes"
echo "destination=$DESTINATION"
