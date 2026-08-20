#!/usr/bin/env bash
# Keep benchmark output out of normal working trees after a run.
#
# Call init_benchmark_artifacts after ROOT is set. BENCH_ROOT or
# KEEP_BENCH_ARTIFACTS=1 opts into retaining artifacts.

init_benchmark_artifacts() {
  BENCHMARK_DOCKER_IMAGES=()
  BENCHMARK_DOCKER_KEEP_IMAGES=()
  if [[ -n "${BENCH_ROOT:-}" ]]; then
    BENCH_ROOT_EXPLICIT=1
  else
    BENCH_ROOT="$ROOT/.benchmark"
    BENCH_ROOT_EXPLICIT=0
  fi
  KEEP_BENCH_ARTIFACTS="${KEEP_BENCH_ARTIFACTS:-0}"
}

register_benchmark_docker_image() {
  local image="${1:-}"
  [[ "$image" == proxysss-*:* || "$image" == proxysss-* ]] || return 0
  if [[ "${2:-0}" == "1" ]]; then
    BENCHMARK_DOCKER_KEEP_IMAGES+=("$image")
  else
    BENCHMARK_DOCKER_IMAGES+=("$image")
  fi
}

cleanup_benchmark_docker_images() {
  [[ "${BENCH_ROOT_EXPLICIT:-1}" == "0" ]] || return 0
  [[ "${KEEP_BENCH_ARTIFACTS:-0}" != "1" ]] || return 0
  command -v docker >/dev/null 2>&1 || return 0
  local image
  for image in "${BENCHMARK_DOCKER_IMAGES[@]:-}"; do
    [[ -n "$image" ]] || continue
    docker image rm -f -- "$image" >/dev/null 2>&1 || true
  done
}

cleanup_benchmark_artifacts() {
  [[ "${BENCH_ROOT_EXPLICIT:-1}" == "0" ]] || return 0
  [[ "$KEEP_BENCH_ARTIFACTS" == "1" ]] && return 0
  [[ -n "${BENCH_ROOT:-}" && "${ROOT:-}" == /* ]] || return 0
  [[ "$BENCH_ROOT" == "$ROOT/.benchmark" ]] || return 0
  rm -rf -- "$BENCH_ROOT"
}
