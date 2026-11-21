#!/usr/bin/env bash
set -euo pipefail

# Strict project-local test/build cache cleanup for Linux agents. Mirrors
# scripts/clean-test-cache.ps1 and never touches global Cargo/Docker caches.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
keep_tmp="${KEEP_TMP:-0}"
keep_dist="${KEEP_DIST:-0}"

rm -rf target .benchmark .cache
if [[ "$keep_dist" != "1" ]]; then
  rm -rf dist
fi
if [[ "$keep_tmp" == "1" ]]; then
  find .tmp -mindepth 1 -maxdepth 1 ! -name verification -exec rm -rf {} + 2>/dev/null || true
  find .tmp/verification -mindepth 1 -maxdepth 1 ! -name latest -exec rm -rf {} + 2>/dev/null || true
else
  rm -rf .tmp
fi
rm -rf .tmp-bench-logs .tmp-gh-benchmark .tmp-gh-benchmark-latest .tmp-gh-benchmark-latest2 logs

# Remove only ignored one-off root artifacts; tracked source is untouched.
while IFS= read -r -d '' file; do
  rel="${file#./}"
  if git check-ignore -q -- "$rel"; then
    rm -f -- "$file"
  fi
done < <(find . -maxdepth 1 -type f \( -name 'tmp-*' -o -name '.tmp-*' -o -name 'benchmark-helper*' \) -print0)

if command -v docker >/dev/null 2>&1; then
  if [[ "$(docker inspect -f '{{index .Config.Labels "com.proxysss.owner"}}' proxysss-verify 2>/dev/null || true)" == "proxysss-project-verification" ]]; then
    docker rm -f proxysss-verify >/dev/null 2>&1 || true
  fi
  for volume in proxysss-perf proxysss-release-verify proxysss-build-cache proxysss-test-tmp proxysss-arm-target proxysss-musl-test-tmp; do
    if [[ -n "$(docker volume ls --format '{{.Name}}' 2>/dev/null | grep -x "$volume" || true)" ]]; then
      docker volume rm "$volume" >/dev/null 2>&1 || true
    fi
  done
fi

target_size=$(du -sb target 2>/dev/null | awk '{print $1}' || true)
bench_size=$(du -sb .benchmark 2>/dev/null | awk '{print $1}' || true)
if [[ "${target_size:-0}" -gt 1073741824 || "${bench_size:-0}" -gt 1073741824 ]]; then
  echo "cache cleanup did not reduce target/.benchmark below 1 GB" >&2
  exit 1
fi
echo "cache cleanup complete: target=${target_size:-0} bytes benchmark=${bench_size:-0} bytes"
