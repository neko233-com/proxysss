#!/usr/bin/env bash
# Local-or-native Ubuntu 24 x86_64 Docker superiority matrix.
#
# This entry point intentionally has no GitHub Actions integration. It accepts
# a native amd64 Docker daemon or a local arm64 daemon with linux/amd64
# emulation, but always builds and benchmarks inside an Ubuntu 24 x86_64
# controller container. Gateway, backend, and load-client containers receive
# disjoint CPU sets so a faster closed-loop protocol cannot steal CPU from a
# sibling gateway path. Every comparable path scales together at 1x/2x/4x,
# including transparent QCP forwarding.
set -euo pipefail
FEEDBACK_START_SECS="$(date +%s)"

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
require_cmd tee

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "strict benchmark requires a clean checkout so every artifact maps to one commit" >&2
  exit 1
fi

docker version >/dev/null
DOCKER_SOCKET="${DOCKER_SOCKET:-$(docker context inspect --format '{{.Endpoints.docker.Host}}')}"
DOCKER_SOCKET="${DOCKER_SOCKET#unix://}"
DOCKER_DAEMON_SOCKET="${DOCKER_DAEMON_SOCKET:-/var/run/docker.sock}"
if [[ ! -S "$DOCKER_SOCKET" ]]; then
  echo "role-isolated controller requires a local Unix Docker socket, found: $DOCKER_SOCKET" >&2
  exit 1
fi

TOTAL_CPUS="${TOTAL_CPUS:-}"
CPU_CORES="${CPU_CORES:-}"
DURATION_SECS="${DURATION_SECS:-1}"
BENCHMARK_REPETITIONS="${BENCHMARK_REPETITIONS:-1}"
LOAD_SCALES="${LOAD_SCALES:-1 2 4}"
ALLOW_UNBALANCED_REPETITIONS="${ALLOW_UNBALANCED_REPETITIONS:-1}"
RUN_SERIAL_ISOLATED="${RUN_SERIAL_ISOLATED:-0}"
SAMPLE_AFTER_SECS="${SAMPLE_AFTER_SECS:-1}"
CAPTURE_DOCKER_STATS="${CAPTURE_DOCKER_STATS:-0}"
CLIENT_START_LEAD_MS="${CLIENT_START_LEAD_MS:-250}"
MAX_FEEDBACK_SECS="${MAX_FEEDBACK_SECS:-60}"
MIXED_SCENARIOS="${MIXED_SCENARIOS:-}"
RUN_ORDER="${RUN_ORDER:-nginx proxysss}"
EQUAL_LOAD_FRACTION="${EQUAL_LOAD_FRACTION:-0.25}"
EQUAL_LOAD_CLIENT_TOKIO_WORKERS="${EQUAL_LOAD_CLIENT_TOKIO_WORKERS:-1}"
EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS="${EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS:-2}"
IMAGE="${PROXYSSS_BENCH_IMAGE:-proxysss-ubuntu24-amd64-bench:local}"
COMMIT="$(git rev-parse HEAD)"
RUN_ID="${BENCH_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-${COMMIT:0:12}}"
if ! [[ "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "BENCH_RUN_ID contains unsupported characters: $RUN_ID" >&2
  exit 1
fi
OUTPUT_REL=".benchmark/direct-ubuntu24-amd64/$RUN_ID"
OUTPUT_ROOT="$ROOT/$OUTPUT_REL"
CURRENT_BENCH_ROOT="$OUTPUT_ROOT/current"
TARGET_DIR="/work/.benchmark/ubuntu24-amd64-target"
CROSS_TARGET_REL=".benchmark/ubuntu24-amd64-cross-target"
CROSS_TARGET_DIR="$ROOT/$CROSS_TARGET_REL"
CARGO_HOME_DIR="/work/.benchmark/ubuntu24-amd64-cargo-home"

require_positive_integer DURATION_SECS "$DURATION_SECS"
require_positive_integer BENCHMARK_REPETITIONS "$BENCHMARK_REPETITIONS"
require_positive_integer EQUAL_LOAD_CLIENT_TOKIO_WORKERS "$EQUAL_LOAD_CLIENT_TOKIO_WORKERS"
require_positive_integer EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS "$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS"
require_positive_integer CLIENT_START_LEAD_MS "$CLIENT_START_LEAD_MS"
require_positive_integer MAX_FEEDBACK_SECS "$MAX_FEEDBACK_SECS"
if [[ "$ALLOW_UNBALANCED_REPETITIONS" != "0" && "$ALLOW_UNBALANCED_REPETITIONS" != "1" ]]; then
  echo "ALLOW_UNBALANCED_REPETITIONS must be 0 or 1" >&2
  exit 1
fi
if [[ "$CAPTURE_DOCKER_STATS" != "0" && "$CAPTURE_DOCKER_STATS" != "1" ]]; then
  echo "CAPTURE_DOCKER_STATS must be 0 or 1" >&2
  exit 1
fi
if [[ "$RUN_SERIAL_ISOLATED" != "0" && "$RUN_SERIAL_ISOLATED" != "1" ]]; then
  echo "RUN_SERIAL_ISOLATED must be 0 or 1" >&2
  exit 1
fi
if [[ "$RUN_ORDER" != "nginx proxysss" && "$RUN_ORDER" != "proxysss nginx" ]]; then
  echo "RUN_ORDER must be 'nginx proxysss' or 'proxysss nginx'" >&2
  exit 1
fi
for scale in $LOAD_SCALES; do
  require_positive_integer LOAD_SCALE "$scale"
done

mkdir -p "$OUTPUT_ROOT"

image_ready=0
restore_ownership() {
  if [[ "$image_ready" != "1" ]]; then
    return
  fi
  docker run --rm --platform linux/amd64 \
    -v "$ROOT:/work" \
    "$IMAGE" \
    chown -R "$(id -u):$(id -g)" "/work/$OUTPUT_REL" \
      /work/.benchmark/ubuntu24-amd64-target \
      /work/.benchmark/ubuntu24-amd64-cargo-home >/dev/null 2>&1 || true
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
container_probe="$(docker run --rm --platform linux/amd64 "$IMAGE" bash -lc '
  set -euo pipefail
  test "$(uname -m)" = x86_64
  . /etc/os-release
  test "$ID" = ubuntu
  test "$VERSION_ID" = 24.04
  echo "detected_nproc=$(nproc)"
  for key in net.core.somaxconn net.ipv4.ip_local_port_range net.ipv4.tcp_max_syn_backlog fs.file-max; do
    sysctl "$key" 2>/dev/null || true
  done
')"

if [[ -z "$TOTAL_CPUS" ]]; then
  TOTAL_CPUS="$(printf '%s\n' "$container_probe" | sed -n 's/^detected_nproc=//p')"
fi
require_positive_integer TOTAL_CPUS "$TOTAL_CPUS"
if (( TOTAL_CPUS < 4 )); then
  echo "role-isolated benchmark requires at least 4 Docker CPUs, found $TOTAL_CPUS" >&2
  exit 1
fi
if [[ -z "$CPU_CORES" ]]; then
  CPU_CORES=$((TOTAL_CPUS / 4))
fi
require_positive_integer CPU_CORES "$CPU_CORES"
BACKEND_CORES="${BACKEND_CORES:-$CPU_CORES}"
require_positive_integer BACKEND_CORES "$BACKEND_CORES"
CLIENT_CORES=$((TOTAL_CPUS - CPU_CORES - BACKEND_CORES))
if (( CLIENT_CORES < 1 )); then
  echo "CPU allocation needs at least one client CPU: total=$TOTAL_CPUS gateway=$CPU_CORES backend=$BACKEND_CORES" >&2
  exit 1
fi
GATEWAY_CPUSET="0-$((CPU_CORES - 1))"
BACKEND_CPUSET="$CPU_CORES-$((CPU_CORES + BACKEND_CORES - 1))"
CLIENT_CPUSET="$((CPU_CORES + BACKEND_CORES))-$((TOTAL_CPUS - 1))"
docker_server_arch="$(docker version --format '{{.Server.Arch}}')"
execution_mode="native-amd64"
if [[ "$docker_server_arch" != "amd64" ]]; then
  execution_mode="emulated-amd64"
fi
build_mode="native-amd64-container"
if [[ "$execution_mode" == "emulated-amd64" ]]; then
  build_mode="native-host-zig-cross"
fi
{
  echo "commit=$COMMIT"
  echo "host_kernel=$(uname -sr)"
  echo "host_arch=$(uname -m)"
  echo "docker_server_arch=$docker_server_arch"
  echo "docker_socket=$DOCKER_SOCKET"
  echo "docker_daemon_socket=$DOCKER_DAEMON_SOCKET"
  echo "container_arch=x86_64"
  echo "container_os=ubuntu-24.04"
  echo "execution_mode=$execution_mode"
  echo "build_mode=$build_mode"
  echo "total_cpus=$TOTAL_CPUS"
  echo "gateway_cpu_cores=$CPU_CORES"
  echo "gateway_cpuset=$GATEWAY_CPUSET"
  echo "backend_cpuset=$BACKEND_CPUSET"
  echo "client_cpuset=$CLIENT_CPUSET"
  echo "load_scales=$LOAD_SCALES"
  echo "duration_secs=$DURATION_SECS"
  echo "repetitions=$BENCHMARK_REPETITIONS"
  echo "mixed_scenarios=${MIXED_SCENARIOS:-all}"
  echo "run_serial_isolated=$RUN_SERIAL_ISOLATED"
  echo "client_start_lead_ms=$CLIENT_START_LEAD_MS"
  echo "max_feedback_secs=$MAX_FEEDBACK_SECS"
  echo "run_order=$RUN_ORDER"
  echo "equal_load_fraction=$EQUAL_LOAD_FRACTION"
  echo "capture_docker_stats=$CAPTURE_DOCKER_STATS"
  echo "equal_load_client_tokio_workers=$EQUAL_LOAD_CLIENT_TOKIO_WORKERS"
  echo "equal_load_static_large_client_tokio_workers=$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS"
  docker version --format 'docker_client={{.Client.Version}} docker_server={{.Server.Version}}'
  printf '%s\n' "$container_probe" | grep -v '^detected_nproc='
} >"$OUTPUT_ROOT/host-fingerprint.txt"

echo "==> building current checkout as optimized x86_64 Linux release"
if [[ "$build_mode" == "native-host-zig-cross" ]]; then
  command -v zig >/dev/null 2>&1 || {
    echo "arm64 Docker feedback requires zig for a sub-minute native cross build" >&2
    exit 1
  }
  command -v cargo-zigbuild >/dev/null 2>&1 || {
    echo "arm64 Docker feedback requires cargo-zigbuild" >&2
    exit 1
  }
  rustup target list --installed | grep -qx 'x86_64-unknown-linux-gnu' || {
    echo "missing Rust target x86_64-unknown-linux-gnu" >&2
    exit 1
  }
  CARGO_TARGET_DIR="$CROSS_TARGET_DIR" \
    cargo zigbuild --locked --release --target x86_64-unknown-linux-gnu.2.17
  PROXY_BIN_HOST_PATH="$CROSS_TARGET_DIR/x86_64-unknown-linux-gnu/release/proxysss"
  PROXY_BIN_PATH="/work/$CROSS_TARGET_REL/x86_64-unknown-linux-gnu/release/proxysss"
else
  docker run --rm --platform linux/amd64 \
    -v "$ROOT:/work" \
    -w /work \
    -e CARGO_HOME="$CARGO_HOME_DIR" \
    -e CARGO_TARGET_DIR="$TARGET_DIR" \
    "$IMAGE" \
    cargo build --locked --release
  PROXY_BIN_HOST_PATH="$ROOT/.benchmark/ubuntu24-amd64-target/release/proxysss"
  PROXY_BIN_PATH="$TARGET_DIR/release/proxysss"
fi

# Cross compilation is only a build acceleration. The produced ELF still has
# to execute inside the same Ubuntu 24 amd64 image before any benchmark starts.
docker run --rm --platform linux/amd64 \
  -v "$ROOT:/work:ro" \
  "$IMAGE" "$PROXY_BIN_PATH" --version >/dev/null

http_concurrency=$((CPU_CORES * 16))
https_concurrency=$((CPU_CORES * 4))
static_large_concurrency=$((CPU_CORES * 2))
sse_concurrency=$CPU_CORES
stream_connections=$((CPU_CORES * 4))
log_file="$OUTPUT_ROOT/matrix.log"

echo "==> strict role-isolated persistent matrix scales=[$LOAD_SCALES] gateway-cpus=$CPU_CORES"
set +e
BENCH_ROOT="$CURRENT_BENCH_ROOT" \
RUN_ID=matrix \
PROXY_BIN="$PROXY_BIN_HOST_PATH" \
IMAGE="proxysss-isolated-ubuntu24-amd64:$RUN_ID" \
BENCH_PLATFORM=linux/amd64 \
AVAILABLE_CPUS="$TOTAL_CPUS" \
GATEWAY_CPUSET="$GATEWAY_CPUSET" \
BACKEND_CPUSET="$BACKEND_CPUSET" \
CLIENT_CPUSET="$CLIENT_CPUSET" \
NGINX_WORKERS="$CPU_CORES" \
TRAFFIC_PROFILE=balanced \
BENCHMARK_REPETITIONS="$BENCHMARK_REPETITIONS" \
ALLOW_UNBALANCED_REPETITIONS="$ALLOW_UNBALANCED_REPETITIONS" \
LOAD_SCALES="$LOAD_SCALES" \
DURATION_SECS="$DURATION_SECS" \
SAMPLE_AFTER_SECS="$SAMPLE_AFTER_SECS" \
CAPTURE_DOCKER_STATS="$CAPTURE_DOCKER_STATS" \
CLIENT_START_LEAD_MS="$CLIENT_START_LEAD_MS" \
HTTP_CONCURRENCY="$http_concurrency" \
HTTPS_CONCURRENCY="$https_concurrency" \
STATIC_LARGE_CONCURRENCY="$static_large_concurrency" \
SSE_CONCURRENCY="$sse_concurrency" \
STREAM_CONNECTIONS="$stream_connections" \
MIXED_SCENARIOS="$MIXED_SCENARIOS" \
RUN_ORDER="$RUN_ORDER" \
EQUAL_LOAD_FRACTION="$EQUAL_LOAD_FRACTION" \
EQUAL_LOAD_CLIENT_TOKIO_WORKERS="$EQUAL_LOAD_CLIENT_TOKIO_WORKERS" \
EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS="$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS" \
RUN_MIXED_MATRIX=1 \
RUN_ISOLATED_SATURATION="$RUN_SERIAL_ISOLATED" \
STRICT_SUPERIORITY=1 \
bash scripts/benchmark-all-scenarios-isolated.sh \
  2>&1 | tee "$log_file"
benchmark_status="${PIPESTATUS[0]}"
set -e

for scale in $LOAD_SCALES; do
  source_dir="$OUTPUT_ROOT/current/runs/all-scenarios-isolated/matrix/scale-$scale"
  archive_dir="$OUTPUT_ROOT/scale-$scale"
  test -d "$source_dir"
  rm -rf "$archive_dir"
  cp -a "$source_dir" "$archive_dir"
done

feedback_elapsed_secs=$(( $(date +%s) - FEEDBACK_START_SECS ))
echo "feedback_elapsed_secs=$feedback_elapsed_secs" | tee -a "$OUTPUT_ROOT/host-fingerprint.txt"
if (( feedback_elapsed_secs > MAX_FEEDBACK_SECS )); then
  echo "strict feedback exceeded ${MAX_FEEDBACK_SECS}s: ${feedback_elapsed_secs}s" >&2
  benchmark_status=1
fi
if [[ "$benchmark_status" != "0" ]]; then
  echo "strict matrix failed; all scale evidence retained at $OUTPUT_ROOT" >&2
  exit "$benchmark_status"
fi

echo "==> all strict Ubuntu 24 x86_64 Docker scales passed in ${feedback_elapsed_secs}s"
for scale in $LOAD_SCALES; do
  echo "$OUTPUT_ROOT/scale-$scale/saturation-summary.md"
  echo "$OUTPUT_ROOT/scale-$scale/equal-load-summary.md"
done
