#!/usr/bin/env bash
# Strict multi-scale Linux release experiment. Each scale runs the complete
# nginx-comparable mixed matrix and requires proxysss to win ops/s and every
# p50/p95/p99 percentile with zero errors. Gateway, backend, and client roles
# stay on disjoint cpusets; each scale also uses the repeated-median method.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "benchmark-production-scale-matrix.sh is Linux-only" >&2
  exit 1
fi

SCALE_FACTORS="${SCALE_FACTORS:-1 2 4}"
BASE_HTTP_CONCURRENCY="${BASE_HTTP_CONCURRENCY:-64}"
BASE_HTTPS_CONCURRENCY="${BASE_HTTPS_CONCURRENCY:-16}"
BASE_STATIC_LARGE_CONCURRENCY="${BASE_STATIC_LARGE_CONCURRENCY:-4}"
BASE_SSE_CONCURRENCY="${BASE_SSE_CONCURRENCY:-4}"
BASE_STREAM_CONNECTIONS="${BASE_STREAM_CONNECTIONS:-16}"
DURATION_SECS="${DURATION_SECS:-30}"
BENCHMARK_REPETITIONS="${BENCHMARK_REPETITIONS:-4}"
ISOLATED_REPETITIONS="${ISOLATED_REPETITIONS:-3}"
RUN_ROOT="${BENCH_ROOT:-$ROOT/.benchmark/runs/production-scale-matrix/$(date +%Y%m%d-%H%M%S)}"

mkdir -p "$RUN_ROOT"

for scale in $SCALE_FACTORS; do
  if ! [[ "$scale" =~ ^[1-9][0-9]*$ ]]; then
    echo "invalid scale factor: $scale" >&2
    exit 1
  fi

  scale_root="$RUN_ROOT/scale-$scale"
  echo "==> strict production scale ${scale}x"
  BENCH_ROOT="$scale_root" \
  HTTP_CONCURRENCY="$((BASE_HTTP_CONCURRENCY * scale))" \
  HTTPS_CONCURRENCY="$((BASE_HTTPS_CONCURRENCY * scale))" \
  STATIC_LARGE_CONCURRENCY="$((BASE_STATIC_LARGE_CONCURRENCY * scale))" \
  SSE_CONCURRENCY="$((BASE_SSE_CONCURRENCY * scale))" \
  STREAM_CONNECTIONS="$((BASE_STREAM_CONNECTIONS * scale))" \
  DURATION_SECS="$DURATION_SECS" \
  RUN_MIXED_MATRIX=1 \
  RUN_ISOLATED_SATURATION=1 \
  BENCHMARK_REPETITIONS="$BENCHMARK_REPETITIONS" \
  ISOLATED_REPETITIONS="$ISOLATED_REPETITIONS" \
  STRICT_SUPERIORITY=1 \
  bash "$ROOT/scripts/benchmark-all-scenarios-isolated.sh"
done

echo "strict production scale matrix passed for factors: $SCALE_FACTORS"
echo "results: $RUN_ROOT"
