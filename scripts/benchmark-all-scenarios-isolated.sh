#!/usr/bin/env bash
# Role-isolated Linux mixed gateway benchmark. The 4c gateway under test,
# protocol backends, and load generators use disjoint cgroups/CPU sets and
# network namespaces. This removes the closed-loop client CPU confounder from
# benchmark-all-scenarios.sh while preserving its native Go fixtures/parser.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Preserve Linux paths passed to containers under Git Bash/MSYS. Host paths
# rooted in the checkout are converted explicitly for Docker Desktop.
DOCKER_CLI_BIN="$(type -P docker || true)"
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    docker() {
      local arg
      local -a docker_args=()
      for arg in "$@"; do
        if [[ "$arg" == "$ROOT"* ]]; then
          arg="$(cygpath -m "$ROOT")${arg#"$ROOT"}"
        fi
        docker_args+=("$arg")
      done
      MSYS2_ARG_CONV_EXCL='*' "$DOCKER_CLI_BIN" "${docker_args[@]}"
    }
    ;;
esac

case "$(uname -s)" in
  Linux | Darwin | MINGW* | MSYS* | CYGWIN*) ;;
  *)
    echo "benchmark-all-scenarios-isolated.sh requires Linux, macOS, or Windows Git Bash with local Docker" >&2
    exit 1
    ;;
esac
for command in docker go openssl; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "missing required command: $command" >&2
    exit 1
  }
done

# The defaults reserve 4 CPUs for the gateway, 4 for the backend fixtures,
# and 8 shared by the concurrently-running load clients.  Keep this within a
# 16-core host by default; callers with a larger generator can override it.
GATEWAY_CPUSET="${GATEWAY_CPUSET:-0-3}"
BACKEND_CPUSET="${BACKEND_CPUSET:-4-7}"
CLIENT_CPUSET="${CLIENT_CPUSET:-8-15}"
# Each scenario already has its own client process. During fixed-rate equal
# load, letting every process inherit the whole cpuset as Tokio's worker count
# creates 11 * N runnable threads and makes generator timers skip ticks. One
# I/O owner is enough there; static-large keeps two for response-body copying.
# Saturation continues using the whole client cpuset so it cannot cap a faster
# gateway before the gateway's own CPU is full.
EQUAL_LOAD_CLIENT_TOKIO_WORKERS="${EQUAL_LOAD_CLIENT_TOKIO_WORKERS:-1}"
EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS="${EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS:-2}"
VALIDATION_TIMING_FILE="${VALIDATION_TIMING_FILE:-}"
MAX_VALIDATION_SECS="${MAX_VALIDATION_SECS:-0}"
# CPU isolation is mandatory for fair throughput attribution. Memory is
# measured in every run, but left uncapped by default so a synthetic cgroup
# ceiling does not turn a safe memory-for-performance trade into a false fail.
# Set any of these explicitly when a release has a declared memory envelope.
GATEWAY_MEMORY="${GATEWAY_MEMORY:-}"
BACKEND_MEMORY="${BACKEND_MEMORY:-}"
CLIENT_MEMORY="${CLIENT_MEMORY:-}"
NOFILE_LIMIT="${NOFILE_LIMIT:-300000}"
NGINX_WORKERS="${NGINX_WORKERS:-4}"
GATEWAY_SOMAXCONN="${GATEWAY_SOMAXCONN:-65535}"
GATEWAY_SYSCTL_ARGS=(--sysctl "net.core.somaxconn=${GATEWAY_SOMAXCONN}")
TRAFFIC_PROFILE="${TRAFFIC_PROFILE:-balanced}"
BENCH_PLATFORM="${BENCH_PLATFORM:-linux/amd64}"
HTTP_CONCURRENCY="${HTTP_CONCURRENCY:-64}"
HTTPS_CONCURRENCY="${HTTPS_CONCURRENCY:-16}"
STATIC_LARGE_CONCURRENCY="${STATIC_LARGE_CONCURRENCY:-4}"
SSE_CONCURRENCY="${SSE_CONCURRENCY:-4}"
STREAM_CONNECTIONS="${STREAM_CONNECTIONS:-16}"
LOAD_SCALES="${LOAD_SCALES:-1}"
DURATION_SECS="${DURATION_SECS:-3}"
SAMPLE_AFTER_SECS="${SAMPLE_AFTER_SECS:-1}"
CAPTURE_DOCKER_STATS="${CAPTURE_DOCKER_STATS:-0}"
# The persistent controller execs eleven client processes per wave. A 100 ms
# absolute lead keeps their measurement windows aligned without consuming the
# one-minute validation budget as idle time.
CLIENT_START_LEAD_MS="${CLIENT_START_LEAD_MS:-100}"
UDP_CLIENT_TIMEOUT_MS="${UDP_CLIENT_TIMEOUT_MS:-1000}"
BENCHMARK_REPETITIONS="${BENCHMARK_REPETITIONS:-1}"
ALLOW_UNBALANCED_REPETITIONS="${ALLOW_UNBALANCED_REPETITIONS:-1}"
ISOLATED_REPETITIONS="${ISOLATED_REPETITIONS:-1}"
RUN_ORDER="${RUN_ORDER:-nginx proxysss}"
LATENCY_RUN_ORDER="${LATENCY_RUN_ORDER:-proxysss nginx}"
STRICT_SUPERIORITY="${STRICT_SUPERIORITY:-1}"
EQUAL_LOAD_FRACTION="${EQUAL_LOAD_FRACTION:-0.25}"
MIN_TARGET_ACHIEVEMENT="${MIN_TARGET_ACHIEVEMENT:-0.98}"
RUN_ISOLATED_SATURATION="${RUN_ISOLATED_SATURATION:-0}"
RUN_MIXED_MATRIX="${RUN_MIXED_MATRIX:-1}"
MIXED_SCENARIOS="${MIXED_SCENARIOS:-}"
ISOLATED_SCENARIOS="${ISOLATED_SCENARIOS:-}"
RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
RUN_DIR="$BENCH_ROOT/runs/all-scenarios-isolated/$RUN_ID"
BASE_RUN_DIR="$RUN_DIR"
PROXY_BIN="${PROXY_BIN:-$ROOT/target/release/proxysss}"
IMAGE="${IMAGE:-proxysss-isolated-all-bench:local}"
NETWORK="proxysss-all-isolated-$RUN_ID"
PREFIX="proxysss-all-isolated-$RUN_ID"
CLIENT_CONTAINER="$PREFIX-client"
SUBNET="${BENCH_SUBNET:-172.31.0.0/16}"
BACKEND_IP="${BACKEND_IP:-172.31.10.10}"
GATEWAY_IP="${GATEWAY_IP:-172.31.20.20}"
PROXYSSS_GATEWAY_IP="${PROXYSSS_GATEWAY_IP:-172.31.20.21}"
HELPER="$BASE_RUN_DIR/benchmark-helper"
LINUX_HELPER="$BASE_RUN_DIR/benchmark-helper-linux-amd64"
CONTEXT_DIR="$BASE_RUN_DIR/image-context"
WWW_DIR="$BASE_RUN_DIR/www"
ROLE_MACHINE_ID_HASH="$(printf '%s' "$(docker info --format '{{.ID}}')" | sha256sum | awk '{print $1}')"

[[ -f "$PROXY_BIN" ]] || {
  echo "missing Linux proxysss binary: $PROXY_BIN" >&2
  exit 1
}
[[ "$RUN_ORDER" == "nginx proxysss" || "$RUN_ORDER" == "proxysss nginx" ]] || {
  echo "RUN_ORDER must be 'nginx proxysss' or 'proxysss nginx'" >&2
  exit 1
}
[[ "$LATENCY_RUN_ORDER" == "nginx proxysss" || "$LATENCY_RUN_ORDER" == "proxysss nginx" ]] || {
  echo "LATENCY_RUN_ORDER must be 'nginx proxysss' or 'proxysss nginx'" >&2
  exit 1
}
[[ "$NGINX_WORKERS" =~ ^[1-9][0-9]*$ ]] || {
  echo "NGINX_WORKERS must be a positive integer" >&2
  exit 1
}
for value_name in BENCHMARK_REPETITIONS ISOLATED_REPETITIONS; do
  value="${!value_name}"
  [[ "$value" =~ ^[1-9][0-9]*$ ]] || {
    echo "$value_name must be a positive integer" >&2
    exit 1
  }
done
for value_name in EQUAL_LOAD_CLIENT_TOKIO_WORKERS EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS; do
  value="${!value_name}"
  [[ "$value" =~ ^[1-9][0-9]*$ ]] || {
    echo "$value_name must be a positive integer" >&2
    exit 1
  }
done
[[ "$CLIENT_START_LEAD_MS" =~ ^[1-9][0-9]*$ ]] || {
  echo "CLIENT_START_LEAD_MS must be a positive integer" >&2
  exit 1
}
[[ "$UDP_CLIENT_TIMEOUT_MS" =~ ^[1-9][0-9]*$ ]] || {
  echo "UDP_CLIENT_TIMEOUT_MS must be a positive integer" >&2
  exit 1
}
[[ "$MAX_VALIDATION_SECS" =~ ^[0-9]+$ ]] || {
  echo "MAX_VALIDATION_SECS must be a non-negative integer" >&2
  exit 1
}
command -v timeout >/dev/null 2>&1 || {
  echo "GNU timeout is required for the hard validation deadline" >&2
  exit 1
}
for scale in $LOAD_SCALES; do
  [[ "$scale" =~ ^[1-9][0-9]*$ ]] || {
    echo "LOAD_SCALES entries must be positive integers: $scale" >&2
    exit 1
  }
done
[[ "$CAPTURE_DOCKER_STATS" == "0" || "$CAPTURE_DOCKER_STATS" == "1" ]] || {
  echo "CAPTURE_DOCKER_STATS must be 0 or 1" >&2
  exit 1
}

cpuset_cpu_ids() {
  local cpuset="$1" item first last cpu
  local items=()
  IFS=',' read -r -a items <<<"$cpuset"
  for item in "${items[@]}"; do
    item="${item//[[:space:]]/}"
    if [[ "$item" =~ ^([0-9]+)-([0-9]+)$ ]]; then
      first="${BASH_REMATCH[1]}"
      last="${BASH_REMATCH[2]}"
      (( last >= first )) || return 1
      for ((cpu = first; cpu <= last; cpu++)); do printf '%s\n' "$cpu"; done
    elif [[ "$item" =~ ^[0-9]+$ ]]; then
      printf '%s\n' "$item"
    else
      return 1
    fi
  done
}

validate_role_cpusets() {
  local online max_cpu set role cpu ids
  online="${AVAILABLE_CPUS:-$(getconf _NPROCESSORS_ONLN)}"
  [[ "$online" =~ ^[1-9][0-9]*$ ]] || {
    echo "cannot determine online CPU count" >&2
    return 1
  }
  max_cpu=$((online - 1))
  local owner_map=" "
  for role in gateway backend client; do
    case "$role" in
      gateway) set="$GATEWAY_CPUSET" ;;
      backend) set="$BACKEND_CPUSET" ;;
      client) set="$CLIENT_CPUSET" ;;
    esac
    ids="$(cpuset_cpu_ids "$set")" || {
      echo "$role cpuset '$set' is invalid" >&2
      return 1
    }
    [[ -n "$ids" ]] || { echo "$role cpuset must not be empty" >&2; return 1; }
    while IFS= read -r cpu; do
      [[ "$cpu" =~ ^[0-9]+$ && "$cpu" -le "$max_cpu" ]] || {
        echo "$role cpuset '$set' needs CPU $cpu, but this host exposes 0-$max_cpu" >&2
        return 1
      }
      if [[ "$owner_map" == *" $cpu:"* ]]; then
        echo "$role cpuset '$set' overlaps another role on CPU $cpu; benchmark roles must be disjoint" >&2
        return 1
      fi
      owner_map+="$cpu:$role "
    done <<<"$ids"
  done
}

validate_role_cpusets
GATEWAY_CPU_CORES="$(cpuset_cpu_ids "$GATEWAY_CPUSET" | wc -l | tr -d '[:space:]')"
[[ "$GATEWAY_CPU_CORES" =~ ^[1-9][0-9]*$ ]] || {
  echo "cannot determine gateway CPU count from $GATEWAY_CPUSET" >&2
  exit 1
}
CLIENT_CPU_CORES="$(cpuset_cpu_ids "$CLIENT_CPUSET" | wc -l | tr -d '[:space:]')"
[[ "$CLIENT_CPU_CORES" =~ ^[1-9][0-9]*$ ]] || {
  echo "cannot determine client CPU count from $CLIENT_CPUSET" >&2
  exit 1
}
case "$TRAFFIC_PROFILE" in
  small|balanced|bulk) ;;
  *) echo "TRAFFIC_PROFILE must be small, balanced, or bulk" >&2; exit 1 ;;
esac
if [[ "$ALLOW_UNBALANCED_REPETITIONS" != "1" ]] \
  && (( BENCHMARK_REPETITIONS < 4 || BENCHMARK_REPETITIONS % 2 != 0 )); then
  echo "BENCHMARK_REPETITIONS must be an even number >= 4 for balanced gateway order" >&2
  exit 1
fi

mkdir -p "$RUN_DIR" "$CONTEXT_DIR" "$WWW_DIR"
go build -o "$HELPER" "$ROOT/scripts/benchmark-helper.go"
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o "$LINUX_HELPER" "$ROOT/scripts/benchmark-helper.go"
cp "$PROXY_BIN" "$CONTEXT_DIR/proxysss"
cp "$ROOT/docker/isolated-websocket-bench.Dockerfile" "$CONTEXT_DIR/Dockerfile"
docker build --platform "$BENCH_PLATFORM" --build-arg NGINX_VERSION=1.31.2 -t "$IMAGE" "$CONTEXT_DIR" >/dev/null

cat >"$WWW_DIR/small.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>small bench</title></head><body><h1>proxysss isolated mixed benchmark</h1><p>same payload for both gateways.</p></body></html>
HTML
printf 'hot-update-v2\n' >"$WWW_DIR/hot.dat"
"$HELPER" write-large-file --path "$WWW_DIR/large.bin"

cleanup() {
  set +e
  docker ps -aq --filter "name=^/${PREFIX}" | xargs -r docker rm -f >/dev/null 2>&1
  docker network rm "$NETWORK" >/dev/null 2>&1
}
trap cleanup EXIT
docker network create --driver bridge --subnet "$SUBNET" "$NETWORK" >/dev/null

write_proxy_config() {
  cat >"$RUN_DIR/proxysss.yaml" <<YAML
config_version: 1
logging:
  access_log: false
http:
  plain_bind: 0.0.0.0:18080
  tls_bind: 0.0.0.0:18443
  h3_bind: ''
  tls:
    mode: manual
    cert_path: /run/proxysss/bench.crt
    key_path: /run/proxysss/bench.key
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  performance:
    enabled: true
    profile: edge
    traffic_profile: ${TRAFFIC_PROFILE}
    adaptive_system: true
    socket_extreme: true
  hot_reload:
    enabled: false
affinity:
  enabled: false
load_balance:
  retries: { enabled: false }
  passive_health: { enabled: false }
  active_health: { enabled: false }
services:
  reverse_proxy:
    routes:
      - name: http-echo
        path_prefix: /proxy
        upstream: http://${BACKEND_IP}:18190
        strip_prefix: true
        forward_headers: false
      - name: websocket-echo
        path_prefix: /ws
        upstream: ws://${BACKEND_IP}:18192
        forward_headers: false
      - name: generic-sse
        path_prefix: /sse
        upstream: http://${BACKEND_IP}:18191
        forward_headers: false
  static_sites:
    - name: bench
      path_prefix: /bench
      root: /work/www
      index_files: [small.html]
      autoindex: false
tcp:
  listeners:
    - name: tcp-echo
      bind: 0.0.0.0:18200
      upstream: ${BACKEND_IP}:18201
      nodelay: true
      connect_timeout_ms: 1000
udp:
  listeners:
    - name: udp-echo
      bind: 0.0.0.0:18300
      upstream: ${BACKEND_IP}:18301
      session_ttl_secs: 30
      max_associations: 65536
    - name: qcp-transparent
      bind: 0.0.0.0:18310
      upstream: ${BACKEND_IP}:18301
      protocol: qcp
      session_ttl_secs: 30
      max_associations: 65536
YAML
}

write_nginx_config() {
  cat >"$RUN_DIR/nginx.conf" <<NGINX
user www-data;
worker_processes ${NGINX_WORKERS};
worker_rlimit_nofile 1048576;
events { use epoll; worker_connections 65535; multi_accept on; }
http {
  access_log off;
  sendfile on;
  tcp_nopush on;
  tcp_nodelay on;
  keepalive_timeout 65;
  upstream http_echo { server ${BACKEND_IP}:18190; keepalive 128; }
  upstream generic_sse { server ${BACKEND_IP}:18191; keepalive 128; }
  upstream ws_echo { server ${BACKEND_IP}:18192; keepalive 128; }
  server {
    listen 0.0.0.0:18080 backlog=65536 reuseport;
    location /bench/ { alias /work/www/; index small.html; }
    location /proxy/ { proxy_http_version 1.1; proxy_set_header Connection ""; proxy_set_header Host \$host; proxy_pass http://http_echo/; }
    location /sse { proxy_http_version 1.1; proxy_set_header Connection ""; proxy_buffering off; proxy_pass http://generic_sse/sse; }
    location /ws/ { proxy_http_version 1.1; proxy_set_header Upgrade \$http_upgrade; proxy_set_header Connection "upgrade"; proxy_set_header Host \$host; proxy_pass http://ws_echo; }
  }
  server {
    listen 0.0.0.0:18443 ssl backlog=65536 reuseport;
    http2 on;
    ssl_certificate /run/nginx/bench.crt;
    ssl_certificate_key /run/nginx/bench.key;
    ssl_session_cache shared:SSL:20m;
    ssl_session_timeout 1h;
    location /bench/ { alias /work/www/; index small.html; }
  }
}
stream {
  upstream tcp_echo { server ${BACKEND_IP}:18201; }
  server { listen 0.0.0.0:18200 backlog=65536 reuseport; proxy_pass tcp_echo; proxy_connect_timeout 1s; proxy_timeout 30s; tcp_nodelay on; }
  upstream udp_echo { server ${BACKEND_IP}:18301; }
  server { listen 0.0.0.0:18300 udp reuseport; proxy_pass udp_echo; proxy_responses 1; proxy_timeout 30s; }
  server { listen 0.0.0.0:18310 udp reuseport; proxy_pass udp_echo; proxy_responses 1; proxy_timeout 30s; }
}
NGINX
}

write_proxy_config
write_nginx_config

write_fairness_manifest() {
  grep -Fq 'plain_bind: 0.0.0.0:18080' "$RUN_DIR/proxysss.yaml"
  grep -Fq 'tls_bind: 0.0.0.0:18443' "$RUN_DIR/proxysss.yaml"
  grep -Fq 'listen 0.0.0.0:18080 backlog=65536 reuseport;' "$RUN_DIR/nginx.conf"
  grep -Fq 'listen 0.0.0.0:18443 ssl backlog=65536 reuseport;' "$RUN_DIR/nginx.conf"
  grep -Fq 'http2 on;' "$RUN_DIR/nginx.conf"
  grep -Fq 'events { use epoll; worker_connections 65535; multi_accept on; }' "$RUN_DIR/nginx.conf"
  grep -Fq 'sendfile on;' "$RUN_DIR/nginx.conf"
  grep -Fq 'tcp_nodelay on;' "$RUN_DIR/nginx.conf"
  grep -Fq 'socket_extreme: true' "$RUN_DIR/proxysss.yaml"
  {
    echo 'comparison=equivalent-protocol-and-routing-surface'
    echo "gateway_cpuset=$GATEWAY_CPUSET"
    echo "gateway_nofile=$NOFILE_LIMIT"
    echo "gateway_somaxconn=$GATEWAY_SOMAXCONN"
    echo "nginx_workers=$NGINX_WORKERS"
    echo 'shared_kernel=true'
    echo 'shared_container_sysctls=true'
    echo 'shared_ports=http:18080,https-h2:18443,tcp:18200,udp:18300,qcp-transparent:18310'
    echo 'nginx_optimizations=epoll,multi_accept,reuseport,sendfile,tcp_nopush,tcp_nodelay,upstream_keepalive,tls_session_cache'
    echo 'proxysss_optimizations=adaptive_system,socket_extreme,reuseport,preload,pools,h2'
    echo "proxysss_config_sha256=$(sha256sum "$RUN_DIR/proxysss.yaml" | awk '{print $1}')"
    echo "nginx_config_sha256=$(sha256sum "$RUN_DIR/nginx.conf" | awk '{print $1}')"
  } >"$RUN_DIR/fairness-config.txt"
}

write_fairness_manifest

memory_limit_enabled() {
  [[ -n "$1" && "$1" != "0" && "$1" != "infinity" && "$1" != "unlimited" ]]
}

start_backend() {
  local name="$PREFIX-backend"
  local memory_arg=""
  memory_limit_enabled "$BACKEND_MEMORY" && memory_arg="--memory=$BACKEND_MEMORY"
  docker create --name "$name" --network "$NETWORK" --ip "$BACKEND_IP" \
    --platform "$BENCH_PLATFORM" \
    --cpuset-cpus "$BACKEND_CPUSET" ${memory_arg:+"$memory_arg"} \
    --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
    "$IMAGE" bash -ec '
      proxysss demo http-echo --listen 0.0.0.0:18190 &
      proxysss demo ws-echo --listen 0.0.0.0:18192 &
      proxysss demo tcp-echo --listen 0.0.0.0:18201 &
      proxysss demo udp-echo --listen 0.0.0.0:18301 &
      /usr/local/bin/benchmark-helper serve-sse --listen 0.0.0.0:18191 --chunks 1 &
      wait -n
    ' >/dev/null
  docker cp "$LINUX_HELPER" "$name:/usr/local/bin/benchmark-helper"
  docker start "$name" >/dev/null
}

gateway_ip_for() {
  if [[ "$1" == "proxysss" ]]; then
    printf '%s\n' "$PROXYSSS_GATEWAY_IP"
  else
    printf '%s\n' "$GATEWAY_IP"
  fi
}

start_gateway() {
  local kind="$1"
  local name="$PREFIX-gateway-$kind"
  local gateway_ip
  gateway_ip="$(gateway_ip_for "$kind")"
  local memory_arg=""
  memory_limit_enabled "$GATEWAY_MEMORY" && memory_arg="--memory=$GATEWAY_MEMORY"
  if [[ "$kind" == "proxysss" ]]; then
    docker create --name "$name" --network "$NETWORK" --ip "$gateway_ip" \
      --platform "$BENCH_PLATFORM" \
      --cpuset-cpus "$GATEWAY_CPUSET" ${memory_arg:+"$memory_arg"} \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      "${GATEWAY_SYSCTL_ARGS[@]}" \
      "$IMAGE" bash -ec '
        mkdir -p /run/proxysss
        openssl ecparam -name prime256v1 -genkey -noout -out /run/proxysss/bench.key
        openssl req -x509 -new -sha256 -days 1 -subj /CN=isolated-mixed \
          -key /run/proxysss/bench.key -out /run/proxysss/bench.crt >/dev/null 2>&1
        exec proxysss -config /work/proxysss.yaml
      ' >/dev/null
    docker cp "$RUN_DIR/proxysss.yaml" "$name:/work/proxysss.yaml"
  else
    docker create --name "$name" --network "$NETWORK" --ip "$gateway_ip" \
      --platform "$BENCH_PLATFORM" \
      --cpuset-cpus "$GATEWAY_CPUSET" ${memory_arg:+"$memory_arg"} \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      "${GATEWAY_SYSCTL_ARGS[@]}" \
      "$IMAGE" bash -ec '
        mkdir -p /run/nginx
        openssl ecparam -name prime256v1 -genkey -noout -out /run/nginx/bench.key
        openssl req -x509 -new -sha256 -days 1 -subj /CN=isolated-mixed \
          -key /run/nginx/bench.key -out /run/nginx/bench.crt >/dev/null 2>&1
        exec nginx -c /work/nginx.conf -g "daemon off;"
      ' >/dev/null
    docker cp "$RUN_DIR/nginx.conf" "$name:/work/nginx.conf"
  fi
  docker cp "$WWW_DIR/." "$name:/work/www"
  docker start "$name" >/dev/null
}

wait_gateway() {
  local kind="$1"
  local gateway_ip
  gateway_ip="$(gateway_ip_for "$kind")"
  for _ in $(seq 1 60); do
    if docker exec "$CLIENT_CONTAINER" proxysss bench http \
      --url "http://${gateway_ip}:18080/bench/small.html" --concurrency 1 \
      --duration-secs 1 2>/dev/null | grep -Eq '^success +: [1-9]'; then
      return 0
    fi
    sleep 0.5
  done
  docker logs "$PREFIX-gateway-$kind" >&2 || true
  return 1
}

activate_gateway() {
  local kind="$1" other
  if [[ "$kind" == "nginx" ]]; then
    other="proxysss"
  else
    other="nginx"
  fi
  # Keep the selected candidate warm while atomically removing its peer from
  # the shared gateway cpuset. A pause-both boundary cold-starts CFS and QEMU
  # scheduling on every one-second wave and distorts p95/p99.
  docker unpause "$PREFIX-gateway-$kind" >/dev/null 2>&1 || true
  docker pause "$PREFIX-gateway-$other" >/dev/null 2>&1 || true
}

declare -a SATURATION_ROWS=()
declare -a ISOLATED_ROWS=()
declare -a LATENCY_ROWS=()
ALL_SCENARIOS=(
  static-small static-large cdn-hot-update https-static-small reverse-proxy
  generic-sse websocket-long-connection game-long-connection tcp-stream udp-stream
  qcp-transparent
)
SCENARIOS=("${ALL_SCENARIOS[@]}")
if [[ -n "$ISOLATED_SCENARIOS" ]]; then
  read -r -a SCENARIOS <<<"$ISOLATED_SCENARIOS"
  for scenario in "${SCENARIOS[@]}"; do
    if [[ " ${ALL_SCENARIOS[*]} " != *" $scenario "* ]]; then
      echo "unsupported ISOLATED_SCENARIOS entry: $scenario" >&2
      exit 1
    fi
  done
fi

if [[ -n "$MIXED_SCENARIOS" ]]; then
  read -r -a MIXED_SCENARIO_LIST <<<"$MIXED_SCENARIOS"
  for scenario in "${MIXED_SCENARIO_LIST[@]}"; do
    if [[ " ${ALL_SCENARIOS[*]} " != *" $scenario "* ]]; then
      echo "unsupported MIXED_SCENARIOS entry: $scenario" >&2
      exit 1
    fi
  done
fi

scenario_requested() {
  local only_scenario="$1" scenario="$2"
  if [[ -n "$only_scenario" ]]; then
    [[ "$only_scenario" == "$scenario" ]]
  elif [[ -z "$MIXED_SCENARIOS" ]]; then
    return 0
  else
    [[ " $MIXED_SCENARIOS " == *" $scenario "* ]]
  fi
}

# Docker stats only reports a momentary value. The release evidence uses the
# cgroup-v2 peak as well, so capture both while the candidate container still
# exists. Ubuntu 24 production hosts use unified cgroups; failing here is
# intentional because a run without current/peak evidence is not releasable.
capture_gateway_memory() {
  local label="$1" kind="$2" suffix="$3"
  docker exec "$PREFIX-gateway-$kind" sh -ec '
    test -r /sys/fs/cgroup/memory.current
    test -r /sys/fs/cgroup/memory.peak
    printf "memory_current_bytes=%s\\n" "$(cat /sys/fs/cgroup/memory.current)"
    printf "memory_peak_bytes=%s\\n" "$(cat /sys/fs/cgroup/memory.peak)"
  ' >"$RUN_DIR/${label}-${kind}-gateway-memory-${suffix}.txt"
}

start_client_controller() {
  local memory_arg=""
  memory_limit_enabled "$CLIENT_MEMORY" && memory_arg="--memory=$CLIENT_MEMORY"
  docker create --name "$CLIENT_CONTAINER" --network "$NETWORK" \
    --platform "$BENCH_PLATFORM" \
    --cpuset-cpus "$CLIENT_CPUSET" ${memory_arg:+"$memory_arg"} \
    --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
    "$IMAGE" sleep infinity >/dev/null
  docker start "$CLIENT_CONTAINER" >/dev/null
}

launch_client() {
  local phase="$1" kind="$2" scenario="$3" protocol="$4" target="$5" concurrency="$6"
  shift 6
  # Saturation must be able to drive the faster gateway to full capacity, so
  # it may use the whole client cpuset. Fixed-rate latency needs fewer runnable
  # timer owners: one normally and two for static-large response copying.
  local runtime_workers="$CLIENT_CPU_CORES"
  if [[ "$phase" == "equal-load" ]]; then
    runtime_workers="$EQUAL_LOAD_CLIENT_TOKIO_WORKERS"
    if [[ "$scenario" == "static-large" ]]; then
      runtime_workers="$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS"
    fi
  fi
  if (( runtime_workers > CLIENT_CPU_CORES )); then
    runtime_workers="$CLIENT_CPU_CORES"
  fi
  if [[ "$phase" == "equal-load" ]]; then
    local interval
    interval="$(awk -F'|' -v scenario="$scenario" '$1 == scenario { print $2; exit }' "$EQUAL_LOAD_PLAN")"
    [[ "$interval" =~ ^[1-9][0-9]*$ ]] || {
      echo "missing equal-load interval for $scenario" >&2
      return 1
    }
    if [[ "$protocol" == "websocket" ]]; then
      set -- "$@" --message-interval-micros "$interval"
    else
      set -- "$@" --operation-interval-micros "$interval"
    fi
  fi
  {
    printf 'TOKIO_WORKER_THREADS=%q proxysss bench' "$runtime_workers"
    printf ' %q' "$@"
    # shellcheck disable=SC2016
    printf ' --start-at-unix-ms "$start_at" > %q 2>&1 &\n' "/tmp/proxysss-bench-results/$scenario.txt"
    printf 'pids+=("$!")\n'
  } >>"$WAVE_SCRIPT"
  printf '%s|%s|%s|%s|%s|%s|%s\n' "$WAVE_CLIENT_NAME" "$scenario" "$protocol" "$target" "$concurrency" "$kind" "$phase" >>"$RUN_DIR/clients.meta"
}

run_candidate() {
  local phase="$1" kind="$2" only_scenario="${3:-}"
  : >"$RUN_DIR/clients.meta"
  activate_gateway "$kind"

  local gateway_ip
  gateway_ip="$(gateway_ip_for "$kind")"
  local http="http://${gateway_ip}:18080" https="https://${gateway_ip}:18443"
  WAVE_CLIENT_NAME="$PREFIX-client-$phase-$kind"
  if [[ -n "$only_scenario" ]]; then
    WAVE_CLIENT_NAME="$WAVE_CLIENT_NAME-$only_scenario"
  fi
  WAVE_SCRIPT="$RUN_DIR/$WAVE_CLIENT_NAME.sh"
  WAVE_RESULTS_DIR="$RUN_DIR/$WAVE_CLIENT_NAME-results"
  rm -rf "$WAVE_RESULTS_DIR"
  mkdir -p "$WAVE_RESULTS_DIR"
  cat >"$WAVE_SCRIPT" <<'CLIENT_WAVE'
#!/usr/bin/env bash
set -euo pipefail
start_at="$1"
rm -rf /tmp/proxysss-bench-results
mkdir -p /tmp/proxysss-bench-results
pids=()
CLIENT_WAVE
  scenario_requested "$only_scenario" static-small && launch_client "$phase" "$kind" static-small http "$http/bench/small.html" "$HTTP_CONCURRENCY" http --url "$http/bench/small.html" --concurrency "$HTTP_CONCURRENCY" --duration-secs "$DURATION_SECS"
  scenario_requested "$only_scenario" static-large && launch_client "$phase" "$kind" static-large http "$http/bench/large.bin" "$STATIC_LARGE_CONCURRENCY" http --url "$http/bench/large.bin" --concurrency "$STATIC_LARGE_CONCURRENCY" --duration-secs "$DURATION_SECS"
  scenario_requested "$only_scenario" cdn-hot-update && launch_client "$phase" "$kind" cdn-hot-update http "$http/bench/hot.dat" "$HTTP_CONCURRENCY" http --url "$http/bench/hot.dat" --concurrency "$HTTP_CONCURRENCY" --duration-secs "$DURATION_SECS"
  scenario_requested "$only_scenario" https-static-small && launch_client "$phase" "$kind" https-static-small http "$https/bench/small.html" "$HTTPS_CONCURRENCY" http --url "$https/bench/small.html" --concurrency "$HTTPS_CONCURRENCY" --duration-secs "$DURATION_SECS" --insecure
  scenario_requested "$only_scenario" reverse-proxy && launch_client "$phase" "$kind" reverse-proxy http "$http/proxy/ping" "$HTTP_CONCURRENCY" http --url "$http/proxy/ping" --concurrency "$HTTP_CONCURRENCY" --duration-secs "$DURATION_SECS"
  scenario_requested "$only_scenario" generic-sse && launch_client "$phase" "$kind" generic-sse sse "$http/sse" "$SSE_CONCURRENCY" sse --url "$http/sse" --concurrency "$SSE_CONCURRENCY" --duration-secs "$DURATION_SECS" --max-chunks 1
  scenario_requested "$only_scenario" websocket-long-connection && launch_client "$phase" "$kind" websocket-long-connection websocket "ws://${gateway_ip}:18080/ws/" "$STREAM_CONNECTIONS" websocket --url "ws://${gateway_ip}:18080/ws/" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 256
  scenario_requested "$only_scenario" game-long-connection && launch_client "$phase" "$kind" game-long-connection tcp "${gateway_ip}:18200" "$STREAM_CONNECTIONS" tcp --addr "${gateway_ip}:18200" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 256
  scenario_requested "$only_scenario" tcp-stream && launch_client "$phase" "$kind" tcp-stream tcp "${gateway_ip}:18200" "$STREAM_CONNECTIONS" tcp --addr "${gateway_ip}:18200" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 1024
  scenario_requested "$only_scenario" udp-stream && launch_client "$phase" "$kind" udp-stream udp "${gateway_ip}:18300" "$STREAM_CONNECTIONS" udp --addr "${gateway_ip}:18300" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 512 --timeout-ms "$UDP_CLIENT_TIMEOUT_MS"
  scenario_requested "$only_scenario" qcp-transparent && launch_client "$phase" "$kind" qcp-transparent udp "${gateway_ip}:18310" "$STREAM_CONNECTIONS" udp --addr "${gateway_ip}:18310" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 1024 --timeout-ms "$UDP_CLIENT_TIMEOUT_MS"
  cat >>"$WAVE_SCRIPT" <<'CLIENT_WAVE'
status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done
exit "$status"
CLIENT_WAVE

  WAVE_START_AT_UNIX_MS=$(( $("$HELPER" now-unix-ms) + CLIENT_START_LEAD_MS ))
  local client_exec_log="$RUN_DIR/$phase-$kind-client-exec.log"
  local remaining_validation_secs=2147483647
  if (( MAX_VALIDATION_SECS > 0 )); then
    remaining_validation_secs=$((MATRIX_VALIDATION_DEADLINE_SECS - $(date +%s)))
    if (( remaining_validation_secs <= 0 )); then
      echo "hard validation deadline reached before $WAVE_CLIENT_NAME" >&2
      return 124
    fi
  fi
  timeout --foreground --signal=TERM --kill-after=0.1s "${remaining_validation_secs}s" \
    docker exec -i "$CLIENT_CONTAINER" bash -s -- "$WAVE_START_AT_UNIX_MS" \
    <"$WAVE_SCRIPT" >"$client_exec_log" 2>&1 &
  local client_exec_pid=$!
  local name row_scenario protocol target concurrency gateway result_phase
  local stats_name="$phase-$kind"
  if [[ -n "$only_scenario" ]]; then stats_name="$stats_name-$only_scenario"; fi
  if [[ "$CAPTURE_DOCKER_STATS" == "1" ]]; then
    local sample_at_ms now_ms sample_wait_secs
    sample_at_ms=$((WAVE_START_AT_UNIX_MS + SAMPLE_AFTER_SECS * 1000))
    now_ms="$("$HELPER" now-unix-ms)"
    if (( sample_at_ms > now_ms )); then
      sample_wait_secs=$(((sample_at_ms - now_ms + 999) / 1000))
      sleep "$sample_wait_secs"
    fi
    local stat_targets=("$PREFIX-gateway-$kind" "$PREFIX-backend" "$CLIENT_CONTAINER")
    docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}} {{.PIDs}}' \
      "${stat_targets[@]}" | tee "$RUN_DIR/$stats_name-stats.txt"
  else
    printf 'disabled_for_one_minute_feedback=true\n' >"$RUN_DIR/$stats_name-stats.txt"
  fi
  local exit_code
  set +e
  wait "$client_exec_pid"
  exit_code=$?
  set -e
  if [[ "$exit_code" == "124" || "$exit_code" == "137" ]]; then
    docker exec "$CLIENT_CONTAINER" sh -c 'pkill -TERM -x proxysss 2>/dev/null || true' >/dev/null 2>&1 || true
    echo "client wave $WAVE_CLIENT_NAME stopped at the ${MAX_VALIDATION_SECS}s hard deadline" >&2
    return 124
  fi
  docker cp "$CLIENT_CONTAINER:/tmp/proxysss-bench-results/." "$WAVE_RESULTS_DIR"
  if [[ "$exit_code" != "0" ]]; then
    cat "$client_exec_log" >&2 || true
    echo "client wave $WAVE_CLIENT_NAME failed with exit $exit_code" >&2
    return 1
  fi
  while IFS='|' read -r name row_scenario protocol target concurrency gateway result_phase; do
    local output
    output="$(<"$WAVE_RESULTS_DIR/$row_scenario.txt")"
    printf '%s\n' "$output" >"$RUN_DIR/$result_phase-$gateway-$row_scenario.txt"
    local row planned_target="-1"
    if [[ "$result_phase" == "equal-load" ]]; then
      planned_target="$(awk -F'|' -v scenario="$row_scenario" '$1 == scenario { print $3; exit }' "$EQUAL_LOAD_PLAN")"
      [[ "$planned_target" =~ ^[0-9]+([.][0-9]+)?$ ]] || {
        echo "missing executable equal-load target for $row_scenario" >&2
        return 1
      }
    fi
    row="$(printf '%s\n' "$output" | "$HELPER" parse-bench \
      --scenario "$row_scenario" --gateway "$gateway" --protocol "$protocol" \
      --target "$target" --concurrency "$concurrency" --duration "$DURATION_SECS" \
      --target-ops-per-sec "$planned_target")"
    if [[ "$result_phase" == "saturation" ]]; then
      SATURATION_ROWS+=("$row")
    elif [[ "$result_phase" == "isolated-saturation" ]]; then
      ISOLATED_ROWS+=("$row")
    else
      LATENCY_ROWS+=("$row")
    fi
  done <"$RUN_DIR/clients.meta"

}

order_for_repetition() {
  local base_order="$1" repetition="$2"
  if (( repetition % 2 == 1 )); then
    printf '%s\n' "$base_order"
  elif [[ "$base_order" == "nginx proxysss" ]]; then
    printf '%s\n' "proxysss nginx"
  else
    printf '%s\n' "nginx proxysss"
  fi
}

BASE_HTTP_CONCURRENCY="$HTTP_CONCURRENCY"
BASE_HTTPS_CONCURRENCY="$HTTPS_CONCURRENCY"
BASE_STATIC_LARGE_CONCURRENCY="$STATIC_LARGE_CONCURRENCY"
BASE_SSE_CONCURRENCY="$SSE_CONCURRENCY"
BASE_STREAM_CONNECTIONS="$STREAM_CONNECTIONS"

configure_scale_paths() {
  local scale="$1"
  RUN_DIR="$BASE_RUN_DIR/scale-$scale"
  mkdir -p "$RUN_DIR"
  SATURATION_RESULTS_JSONL="$RUN_DIR/saturation-results.jsonl"
  SATURATION_RESULTS_JSON="$RUN_DIR/saturation-results.json"
  ISOLATED_RESULTS_JSONL="$RUN_DIR/isolated-saturation-results.jsonl"
  ISOLATED_RESULTS_JSON="$RUN_DIR/isolated-saturation-results.json"
  LATENCY_RESULTS_JSONL="$RUN_DIR/equal-load-results.jsonl"
  LATENCY_RESULTS_JSON="$RUN_DIR/equal-load-results.json"
  EQUAL_LOAD_PLAN="$RUN_DIR/equal-load-plan.txt"
  SATURATION_SUMMARY_MD="$RUN_DIR/saturation-summary.md"
  SATURATION_SUMMARY_HTML="$RUN_DIR/saturation-summary.html"
  ISOLATED_SUMMARY_MD="$RUN_DIR/isolated-saturation-summary.md"
  ISOLATED_SUMMARY_HTML="$RUN_DIR/isolated-saturation-summary.html"
  LATENCY_SUMMARY_MD="$RUN_DIR/equal-load-summary.md"
  LATENCY_SUMMARY_HTML="$RUN_DIR/equal-load-summary.html"
  HTTP_CONCURRENCY=$((BASE_HTTP_CONCURRENCY * scale))
  HTTPS_CONCURRENCY=$((BASE_HTTPS_CONCURRENCY * scale))
  STATIC_LARGE_CONCURRENCY=$((BASE_STATIC_LARGE_CONCURRENCY * scale))
  SSE_CONCURRENCY=$((BASE_SSE_CONCURRENCY * scale))
  STREAM_CONNECTIONS=$((BASE_STREAM_CONNECTIONS * scale))
}

write_scale_reports() {
  local scale="$1" strict=false mixed_min_ratio=0
  if [[ "$STRICT_SUPERIORITY" == "1" ]]; then
    strict=true
    mixed_min_ratio=1.0
  fi
  local saturation_status=0 isolated_status=0 latency_status=0
  set +e
  if [[ "$RUN_MIXED_MATRIX" == "1" ]]; then
    "$HELPER" write-all-scenarios-summary \
      --results "$SATURATION_RESULTS_JSON" --md "$SATURATION_SUMMARY_MD" --html "$SATURATION_SUMMARY_HTML" \
      --min-ratio "$mixed_min_ratio" --critical-ratio 1.0 --critical-scenarios "" \
      --diagnostic-scenarios "" --sse-error-tolerance 0 --websocket-error-tolerance 0 \
      --udp-error-tolerance 0 --aggregate-ratio 1.0 --max-latency-ratio 1.0 \
      --require-latency-percentiles=false --require-zero-errors=true \
      --gate-ops=true --gate-latency=false --min-target-achievement=0 --phase=saturation \
      --strict-superiority="$strict" --mixed-matrix=true --cpu-cores "$GATEWAY_CPU_CORES" \
      --traffic-profile "$TRAFFIC_PROFILE" --samples-per-gateway "$BENCHMARK_REPETITIONS" \
      --http-concurrency "$HTTP_CONCURRENCY" --https-concurrency "$HTTPS_CONCURRENCY" \
      --static-large-concurrency "$STATIC_LARGE_CONCURRENCY" \
      --sse-concurrency "$SSE_CONCURRENCY" --stream-connections "$STREAM_CONNECTIONS"
    saturation_status=$?
  fi
  if [[ "$RUN_ISOLATED_SATURATION" == "1" ]]; then
    "$HELPER" write-all-scenarios-summary \
      --results "$ISOLATED_RESULTS_JSON" --md "$ISOLATED_SUMMARY_MD" --html "$ISOLATED_SUMMARY_HTML" \
      --min-ratio 1.0 --critical-ratio 1.0 \
      --critical-scenarios "websocket-long-connection game-long-connection tcp-stream udp-stream qcp-transparent" \
      --diagnostic-scenarios "" --sse-error-tolerance 0 --websocket-error-tolerance 0 \
      --udp-error-tolerance 0 --aggregate-ratio 1.0 --max-latency-ratio 1.0 \
      --require-latency-percentiles=false --require-zero-errors=true \
      --gate-ops=true --gate-latency=false --min-target-achievement=0 --phase=isolated-saturation \
      --strict-superiority="$strict" --mixed-matrix=false --cpu-cores "$GATEWAY_CPU_CORES" \
      --traffic-profile "$TRAFFIC_PROFILE" --samples-per-gateway "$ISOLATED_REPETITIONS" \
      --http-concurrency "$HTTP_CONCURRENCY" --https-concurrency "$HTTPS_CONCURRENCY" \
      --static-large-concurrency "$STATIC_LARGE_CONCURRENCY" \
      --sse-concurrency "$SSE_CONCURRENCY" --stream-connections "$STREAM_CONNECTIONS"
    isolated_status=$?
  fi
  if [[ "$RUN_MIXED_MATRIX" == "1" ]]; then
    "$HELPER" write-all-scenarios-summary \
      --results "$LATENCY_RESULTS_JSON" --md "$LATENCY_SUMMARY_MD" --html "$LATENCY_SUMMARY_HTML" \
      --min-ratio 1.0 --critical-ratio 1.0 \
      --critical-scenarios "websocket-long-connection game-long-connection tcp-stream udp-stream qcp-transparent" \
      --diagnostic-scenarios "" --sse-error-tolerance 0 --websocket-error-tolerance 0 \
      --udp-error-tolerance 0 --aggregate-ratio 1.0 --max-latency-ratio 1.0 \
      --require-latency-percentiles=true --require-zero-errors=true \
      --gate-ops=false --gate-latency=true --min-target-achievement="$MIN_TARGET_ACHIEVEMENT" --phase=equal-offered-load \
      --strict-superiority="$strict" --mixed-matrix=true --cpu-cores "$GATEWAY_CPU_CORES" \
      --traffic-profile "$TRAFFIC_PROFILE" --samples-per-gateway "$BENCHMARK_REPETITIONS" \
      --http-concurrency "$HTTP_CONCURRENCY" --https-concurrency "$HTTPS_CONCURRENCY" \
      --static-large-concurrency "$STATIC_LARGE_CONCURRENCY" \
      --sse-concurrency "$SSE_CONCURRENCY" --stream-connections "$STREAM_CONNECTIONS"
    latency_status=$?
  fi
  set -e

  cat >"$RUN_DIR/run-metadata.txt" <<EOF
run_id=$RUN_ID
load_scale=$scale
run_order=$RUN_ORDER
latency_run_order=$LATENCY_RUN_ORDER
benchmark_repetitions=$BENCHMARK_REPETITIONS
isolated_repetitions=$ISOLATED_REPETITIONS
equal_load_fraction=$EQUAL_LOAD_FRACTION
min_target_achievement=$MIN_TARGET_ACHIEVEMENT
capture_docker_stats=$CAPTURE_DOCKER_STATS
run_isolated_saturation=$RUN_ISOLATED_SATURATION
run_mixed_matrix=$RUN_MIXED_MATRIX
mixed_scenarios=${MIXED_SCENARIOS:-all}
isolated_scenarios=${ISOLATED_SCENARIOS:-all}
gateway_cpuset=$GATEWAY_CPUSET
gateway_cpu_cores=$GATEWAY_CPU_CORES
nginx_gateway_ip=$GATEWAY_IP
proxysss_gateway_ip=$PROXYSSS_GATEWAY_IP
gateway_memory=${GATEWAY_MEMORY:-unlimited}
backend_cpuset=$BACKEND_CPUSET
client_cpuset=$CLIENT_CPUSET
saturation_client_tokio_workers=$CLIENT_CPU_CORES
equal_load_client_tokio_workers=$EQUAL_LOAD_CLIENT_TOKIO_WORKERS
equal_load_static_large_client_tokio_workers=$EQUAL_LOAD_STATIC_LARGE_CLIENT_TOKIO_WORKERS
nginx_workers=$NGINX_WORKERS
traffic_profile=$TRAFFIC_PROFILE
bench_platform=$BENCH_PLATFORM
client_start_lead_ms=$CLIENT_START_LEAD_MS
udp_client_timeout_ms=$UDP_CLIENT_TIMEOUT_MS
max_validation_secs=$MAX_VALIDATION_SECS
role_isolation=docker-cgroup-cpuset-network-namespace
role_machine_id_hashes=client:$ROLE_MACHINE_ID_HASH,gateway:$ROLE_MACHINE_ID_HASH,backend:$ROLE_MACHINE_ID_HASH
gateway_memory_samples=cgroup-v2-current-and-peak
nginx_version=1.31.2-mainline-O3-fno-plt
proxy_binary_sha256=$(sha256sum "$PROXY_BIN" | awk '{print $1}')
EOF

  if [[ "$saturation_status" != "0" || "$isolated_status" != "0" || "$latency_status" != "0" ]]; then
    echo "scale $scale failed: mixed_saturation=$saturation_status isolated_saturation=$isolated_status latency=$latency_status" >&2
    return 1
  fi
  return 0
}

run_scale() {
  local scale="$1"
  configure_scale_paths "$scale"
  SATURATION_ROWS=()
  ISOLATED_ROWS=()
  LATENCY_ROWS=()
  echo "==> persistent strict matrix scale=${scale}x http=$HTTP_CONCURRENCY https=$HTTPS_CONCURRENCY static-large=$STATIC_LARGE_CONCURRENCY sse=$SSE_CONCURRENCY realtime=$STREAM_CONNECTIONS"

  if [[ "$RUN_MIXED_MATRIX" == "1" ]]; then
    for repetition in $(seq 1 "$BENCHMARK_REPETITIONS"); do
      repetition_order="$(order_for_repetition "$RUN_ORDER" "$repetition")"
      for kind in $repetition_order; do run_candidate saturation "$kind" || return $?; done
    done
    printf '%s\n' "${SATURATION_ROWS[@]}" >"$SATURATION_RESULTS_JSONL"
    "$HELPER" aggregate-bench-medians --in "$SATURATION_RESULTS_JSONL" --out "$SATURATION_RESULTS_JSON"
    "$HELPER" write-equal-load-plan --results "$SATURATION_RESULTS_JSON" --out "$EQUAL_LOAD_PLAN" \
      --fraction "$EQUAL_LOAD_FRACTION" --duration-secs "$DURATION_SECS"
    for repetition in $(seq 1 "$BENCHMARK_REPETITIONS"); do
      repetition_order="$(order_for_repetition "$LATENCY_RUN_ORDER" "$repetition")"
      for kind in $repetition_order; do run_candidate equal-load "$kind" || return $?; done
    done
    printf '%s\n' "${LATENCY_ROWS[@]}" >"$LATENCY_RESULTS_JSONL"
    "$HELPER" aggregate-bench-medians --in "$LATENCY_RESULTS_JSONL" --out "$LATENCY_RESULTS_JSON"
  fi

  if [[ "$RUN_ISOLATED_SATURATION" == "1" ]]; then
    local scenario_index=0
    for scenario in "${SCENARIOS[@]}"; do
      for repetition in $(seq 1 "$ISOLATED_REPETITIONS"); do
        order_repetition=$((scenario_index + repetition))
        scenario_order="$(order_for_repetition "$RUN_ORDER" "$order_repetition")"
        for kind in $scenario_order; do run_candidate isolated-saturation "$kind" "$scenario" || return $?; done
      done
      scenario_index=$((scenario_index + 1))
    done
    printf '%s\n' "${ISOLATED_ROWS[@]}" >"$ISOLATED_RESULTS_JSONL"
    "$HELPER" aggregate-bench-medians --in "$ISOLATED_RESULTS_JSONL" --out "$ISOLATED_RESULTS_JSON"
  fi

  docker unpause "$PREFIX-gateway-nginx" "$PREFIX-gateway-proxysss" >/dev/null 2>&1 || true
  capture_gateway_memory "scale-$scale" nginx final
  capture_gateway_memory "scale-$scale" proxysss final

  write_scale_reports "$scale"
}

# Backend, both gateways, and the client controller stay warm across every
# scale. The inactive gateway is paused on the shared gateway cpuset.
start_backend
start_client_controller
start_gateway nginx
wait_gateway nginx
start_gateway proxysss
wait_gateway proxysss

matrix_validation_start_secs="$(date +%s)"
MATRIX_VALIDATION_DEADLINE_SECS=$((matrix_validation_start_secs + MAX_VALIDATION_SECS - 1))
overall_status=0
for scale in $LOAD_SCALES; do
  if (( MAX_VALIDATION_SECS > 0 && $(date +%s) >= MATRIX_VALIDATION_DEADLINE_SECS )); then
    echo "hard validation deadline reached before scale $scale" >&2
    overall_status=124
    break
  fi
  if ! run_scale "$scale"; then overall_status=1; fi
done
matrix_validation_elapsed_secs=$(( $(date +%s) - matrix_validation_start_secs ))
if [[ -n "$VALIDATION_TIMING_FILE" ]]; then
  {
    echo "validation_start_secs=$matrix_validation_start_secs"
    echo "validation_elapsed_secs=$matrix_validation_elapsed_secs"
  } >"$VALIDATION_TIMING_FILE"
fi

if [[ "$overall_status" != "0" ]]; then
  echo "isolated benchmark matrix failed; all scale reports retained under $BASE_RUN_DIR" >&2
  exit 1
fi
echo "isolated benchmark passed: scales=$LOAD_SCALES run_dir=$BASE_RUN_DIR"
