#!/usr/bin/env bash
# Dedicated-host Ubuntu 24 x86_64 Docker superiority matrix.
#
# This entry point intentionally has no GitHub Actions integration. It builds
# the current checkout inside the same Ubuntu 24 image used by the benchmark,
# then scales every comparable gateway path together at 1x/2x/4x. Transparent
# QCP forwarding is enabled by default as extended realtime evidence.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_positive_integer() {
  local name="$1"
  local value="$2"
  if ! [[ "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "$name must be a positive integer: $value" >&2
    exit 1
  fi
}

require_cmd docker
require_cmd git
require_cmd nproc
require_cmd tee

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "strict benchmark requires a clean checkout so every artifact maps to one commit" >&2
  exit 1
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "this strict matrix requires an x86_64 Linux host" >&2
  exit 1
fi
if [[ ! -r /etc/os-release ]]; then
  echo "cannot verify Ubuntu 24.04: /etc/os-release is missing" >&2
  exit 1
fi
# shellcheck disable=SC1091
. /etc/os-release
if [[ "${ID:-}" != "ubuntu" || "${VERSION_ID:-}" != "24.04" ]]; then
  echo "this strict matrix requires an Ubuntu 24.04 host (found ${ID:-unknown} ${VERSION_ID:-unknown})" >&2
  exit 1
fi
docker version >/dev/null

CPU_CORES="${CPU_CORES:-$(nproc)}"
DURATION_SECS="${DURATION_SECS:-10}"
BENCHMARK_REPETITIONS="${BENCHMARK_REPETITIONS:-3}"
LOAD_SCALES="${LOAD_SCALES:-1 2 4}"
EXTENDED_REALTIME="${EXTENDED_REALTIME:-1}"
IMAGE="${PROXYSSS_BENCH_IMAGE:-proxysss-ubuntu24-amd64-bench:local}"
COMMIT="$(git rev-parse HEAD)"
RUN_ID="${BENCH_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-${COMMIT:0:12}}"
if ! [[ "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "BENCH_RUN_ID contains unsupported characters: $RUN_ID" >&2
  exit 1
fi
OUTPUT_REL=".benchmark/direct-ubuntu24-amd64/$RUN_ID"
OUTPUT_ROOT="$ROOT/$OUTPUT_REL"
CURRENT_BENCH_ROOT="/work/$OUTPUT_REL/current"
TARGET_DIR="/work/.benchmark/ubuntu24-amd64-target"

require_positive_integer CPU_CORES "$CPU_CORES"
require_positive_integer DURATION_SECS "$DURATION_SECS"
require_positive_integer BENCHMARK_REPETITIONS "$BENCHMARK_REPETITIONS"
if [[ "$EXTENDED_REALTIME" != "0" && "$EXTENDED_REALTIME" != "1" ]]; then
  echo "EXTENDED_REALTIME must be 0 or 1" >&2
  exit 1
fi
for scale in $LOAD_SCALES; do
  require_positive_integer LOAD_SCALE "$scale"
done

mkdir -p "$OUTPUT_ROOT"
{
  echo "commit=$COMMIT"
  echo "host_kernel=$(uname -sr)"
  echo "host_arch=$(uname -m)"
  echo "host_os=$ID $VERSION_ID"
  echo "cpu_cores=$CPU_CORES"
  echo "load_scales=$LOAD_SCALES"
  echo "duration_secs=$DURATION_SECS"
  echo "repetitions=$BENCHMARK_REPETITIONS"
  echo "extended_realtime=$EXTENDED_REALTIME"
  docker version --format 'docker_client={{.Client.Version}} docker_server={{.Server.Version}}'
  for key in net.core.somaxconn net.ipv4.ip_local_port_range net.ipv4.tcp_max_syn_backlog fs.file-max; do
    sysctl "$key" 2>/dev/null || true
  done
} >"$OUTPUT_ROOT/host-fingerprint.txt"

image_ready=0
restore_ownership() {
  if [[ "$image_ready" != "1" ]]; then
    return
  fi
  docker run --rm --platform linux/amd64 \
    -v "$ROOT:/work" \
    "$IMAGE" \
    chown -R "$(id -u):$(id -g)" "/work/$OUTPUT_REL" /work/.benchmark/ubuntu24-amd64-target \
    >/dev/null 2>&1 || true
}
trap restore_ownership EXIT

echo "==> building Ubuntu 24 benchmark image: $IMAGE"
docker build --platform linux/amd64 -f docker/ubuntu24-bench.Dockerfile -t "$IMAGE" .
image_ready=1

image_arch="$(docker image inspect "$IMAGE" --format '{{.Architecture}}')"
if [[ "$image_arch" != "amd64" ]]; then
  echo "benchmark image must be amd64, found $image_arch" >&2
  exit 1
fi
docker run --rm --platform linux/amd64 "$IMAGE" bash -lc '
  set -euo pipefail
  test "$(uname -m)" = x86_64
  . /etc/os-release
  test "$ID" = ubuntu
  test "$VERSION_ID" = 24.04
'

echo "==> building current checkout as optimized x86_64 Linux release"
docker run --rm --platform linux/amd64 \
  -v "$ROOT:/work" \
  -w /work \
  -e CARGO_TARGET_DIR="$TARGET_DIR" \
  "$IMAGE" \
  cargo build --locked --release

for scale in $LOAD_SCALES; do
  http_concurrency=$((CPU_CORES * 16 * scale))
  https_concurrency=$((CPU_CORES * 4 * scale))
  static_large_concurrency=$((CPU_CORES * 2 * scale))
  sse_concurrency=$((CPU_CORES * scale))
  stream_connections=$((CPU_CORES * 4 * scale))
  log_file="$OUTPUT_ROOT/scale-$scale.log"

  echo "==> strict mixed matrix scale=${scale}x http=$http_concurrency https=$https_concurrency static-large=$static_large_concurrency sse=$sse_concurrency realtime=$stream_connections"
  set +e
  docker run --rm --platform linux/amd64 \
    --network host \
    --ulimit nofile=1048576:1048576 \
    -v "$ROOT:/work" \
    -w /work \
    -e BENCH_ROOT="$CURRENT_BENCH_ROOT" \
    -e PROXY_BIN="$TARGET_DIR/release/proxysss" \
    -e FORCE_BUILD=0 \
    -e CPU_CORES="$CPU_CORES" \
    -e MIXED_MATRIX=1 \
    -e TRAFFIC_PROFILE=balanced \
    -e BENCHMARK_REPETITIONS="$BENCHMARK_REPETITIONS" \
    -e DURATION_SECS="$DURATION_SECS" \
    -e CONCURRENCY="$http_concurrency" \
    -e HTTPS_CONCURRENCY="$https_concurrency" \
    -e STATIC_LARGE_CONCURRENCY="$static_large_concurrency" \
    -e SSE_CONCURRENCY="$sse_concurrency" \
    -e STREAM_CONNECTIONS="$stream_connections" \
    -e EXTENDED_REALTIME="$EXTENDED_REALTIME" \
    -e STRICT_SUPERIORITY=1 \
    -e REQUIRE_ZERO_ERRORS=1 \
    -e REQUIRE_LATENCY_PERCENTILES=1 \
    -e GATE_LATENCY=0 \
    -e MAX_LATENCY_RATIO=1.0 \
    "$IMAGE" \
    bash -lc 'test "$(uname -m)" = x86_64 && bash scripts/benchmark-all-scenarios.sh' \
    2>&1 | tee "$log_file"
  benchmark_status="${PIPESTATUS[0]}"
  set -e

  docker run --rm --platform linux/amd64 \
    -v "$ROOT:/work" \
    -e OUTPUT_REL="$OUTPUT_REL" \
    -e SCALE="$scale" \
    "$IMAGE" \
    bash -lc '
      set -euo pipefail
      source_dir="/work/$OUTPUT_REL/current/runs/all-scenarios"
      archive_dir="/work/$OUTPUT_REL/scale-$SCALE"
      test -d "$source_dir"
      rm -rf "$archive_dir"
      cp -a "$source_dir" "$archive_dir"
    '

  if [[ "$benchmark_status" != "0" ]]; then
    echo "strict scale ${scale}x failed; evidence retained at $OUTPUT_ROOT/scale-$scale" >&2
    exit "$benchmark_status"
  fi
done

echo "==> all strict Ubuntu 24 x86_64 Docker scales passed"
for scale in $LOAD_SCALES; do
  echo "$OUTPUT_ROOT/scale-$scale/summary.md"
done
