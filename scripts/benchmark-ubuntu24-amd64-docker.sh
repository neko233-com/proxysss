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
DURATION_SECS="${DURATION_SECS:-2}"
BENCHMARK_REPETITIONS="${BENCHMARK_REPETITIONS:-1}"
LOAD_SCALES="${LOAD_SCALES:-1 2 4}"
ALLOW_UNBALANCED_REPETITIONS="${ALLOW_UNBALANCED_REPETITIONS:-1}"
RUN_SERIAL_ISOLATED="${RUN_SERIAL_ISOLATED:-0}"
SAMPLE_AFTER_SECS="${SAMPLE_AFTER_SECS:-1}"
CLIENT_START_LEAD_SECS="${CLIENT_START_LEAD_SECS:-}"
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
CURRENT_BENCH_ROOT="/work/$OUTPUT_REL/current"
TARGET_DIR="/work/.benchmark/ubuntu24-amd64-target"
CARGO_HOME_DIR="/work/.benchmark/ubuntu24-amd64-cargo-home"

require_positive_integer DURATION_SECS "$DURATION_SECS"
require_positive_integer BENCHMARK_REPETITIONS "$BENCHMARK_REPETITIONS"
require_positive_integer EQUAL_LOAD_CLIENT_TOKIO_WORKERS "$EQUAL_LOAD_CLIENT_TOKIO_WORKERS"
require_positive_integer EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS "$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS"
if [[ "$ALLOW_UNBALANCED_REPETITIONS" != "0" && "$ALLOW_UNBALANCED_REPETITIONS" != "1" ]]; then
  echo "ALLOW_UNBALANCED_REPETITIONS must be 0 or 1" >&2
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
docker run --rm --platform linux/amd64 "$IMAGE" bash -lc '
  set -euo pipefail
  test "$(uname -m)" = x86_64
  . /etc/os-release
  test "$ID" = ubuntu
  test "$VERSION_ID" = 24.04
'

if [[ -z "$TOTAL_CPUS" ]]; then
  TOTAL_CPUS="$(docker run --rm --platform linux/amd64 "$IMAGE" nproc)"
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
if [[ -z "$CLIENT_START_LEAD_SECS" ]]; then
  CLIENT_START_LEAD_SECS=1
fi
require_positive_integer CLIENT_START_LEAD_SECS "$CLIENT_START_LEAD_SECS"
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
  echo "client_start_lead_secs=$CLIENT_START_LEAD_SECS"
  echo "run_order=$RUN_ORDER"
  echo "equal_load_fraction=$EQUAL_LOAD_FRACTION"
  echo "equal_load_client_tokio_workers=$EQUAL_LOAD_CLIENT_TOKIO_WORKERS"
  echo "equal_load_static_large_client_tokio_workers=$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS"
  docker version --format 'docker_client={{.Client.Version}} docker_server={{.Server.Version}}'
  for key in net.core.somaxconn net.ipv4.ip_local_port_range net.ipv4.tcp_max_syn_backlog fs.file-max; do
    docker run --rm --platform linux/amd64 "$IMAGE" sysctl "$key" 2>/dev/null || true
  done
} >"$OUTPUT_ROOT/host-fingerprint.txt"

echo "==> building current checkout as optimized x86_64 Linux release"
docker run --rm --platform linux/amd64 \
  -v "$ROOT:/work" \
  -w /work \
  -e CARGO_HOME="$CARGO_HOME_DIR" \
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

  echo "==> strict role-isolated matrix scale=${scale}x gateway-cpus=$CPU_CORES http=$http_concurrency https=$https_concurrency static-large=$static_large_concurrency sse=$sse_concurrency realtime=$stream_connections"
  set +e
  docker run --rm --platform linux/amd64 \
    --ulimit nofile=1048576:1048576 \
    -v "$ROOT:/work" \
    -v "$DOCKER_DAEMON_SOCKET:/var/run/docker.sock" \
    -w /work \
    -e BENCH_ROOT="$CURRENT_BENCH_ROOT" \
    -e RUN_ID="scale-$scale" \
    -e PROXY_BIN="$TARGET_DIR/release/proxysss" \
    -e IMAGE="proxysss-isolated-ubuntu24-amd64:$RUN_ID-$scale" \
    -e BENCH_PLATFORM=linux/amd64 \
    -e GATEWAY_CPUSET="$GATEWAY_CPUSET" \
    -e BACKEND_CPUSET="$BACKEND_CPUSET" \
    -e CLIENT_CPUSET="$CLIENT_CPUSET" \
    -e NGINX_WORKERS="$CPU_CORES" \
    -e TRAFFIC_PROFILE=balanced \
    -e BENCHMARK_REPETITIONS="$BENCHMARK_REPETITIONS" \
    -e ALLOW_UNBALANCED_REPETITIONS="$ALLOW_UNBALANCED_REPETITIONS" \
    -e DURATION_SECS="$DURATION_SECS" \
    -e SAMPLE_AFTER_SECS="$SAMPLE_AFTER_SECS" \
    -e CLIENT_START_LEAD_SECS="$CLIENT_START_LEAD_SECS" \
    -e HTTP_CONCURRENCY="$http_concurrency" \
    -e HTTPS_CONCURRENCY="$https_concurrency" \
    -e STATIC_LARGE_CONCURRENCY="$static_large_concurrency" \
    -e SSE_CONCURRENCY="$sse_concurrency" \
    -e STREAM_CONNECTIONS="$stream_connections" \
    -e MIXED_SCENARIOS="$MIXED_SCENARIOS" \
    -e RUN_ORDER="$RUN_ORDER" \
    -e EQUAL_LOAD_FRACTION="$EQUAL_LOAD_FRACTION" \
    -e EQUAL_LOAD_CLIENT_TOKIO_WORKERS="$EQUAL_LOAD_CLIENT_TOKIO_WORKERS" \
    -e EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS="$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS" \
    -e RUN_MIXED_MATRIX=1 \
    -e RUN_ISOLATED_SATURATION="$RUN_SERIAL_ISOLATED" \
    -e STRICT_SUPERIORITY=1 \
    "$IMAGE" \
    bash -lc 'test "$(uname -m)" = x86_64 && bash scripts/benchmark-all-scenarios-isolated.sh' \
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
      source_dir="/work/$OUTPUT_REL/current/runs/all-scenarios-isolated/scale-$SCALE"
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
  echo "$OUTPUT_ROOT/scale-$scale/saturation-summary.md"
  echo "$OUTPUT_ROOT/scale-$scale/equal-load-summary.md"
done
