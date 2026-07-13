#!/usr/bin/env bash
# Role-isolated Docker benchmark for a single 4c/8GiB WebSocket/WSS gateway.
#
# This is intentionally separate from benchmark-all-scenarios.sh: the latter is
# the broad mixed nginx-parity release matrix, while this harness verifies the
# production game-gateway question directly: active WSS echo latency/throughput
# plus production-scale mostly-idle WSS tunnels, with no client or backend process sharing
# the gateway container's cgroup or network namespace.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "benchmark-websocket-isolated.sh requires Linux Docker (Ubuntu 24 is the reference host)." >&2
  exit 1
fi

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_cmd docker
require_cmd go
require_cmd grep

cpuset_cpu_count() {
  local cpuset="$1" count=0 item first last
  local items=()
  IFS=',' read -r -a items <<<"$cpuset"
  for item in "${items[@]}"; do
    item="${item//[[:space:]]/}"
    if [[ "$item" =~ ^([0-9]+)-([0-9]+)$ ]]; then
      first="${BASH_REMATCH[1]}"
      last="${BASH_REMATCH[2]}"
      (( last >= first )) || return 1
      count=$((count + last - first + 1))
    elif [[ "$item" =~ ^[0-9]+$ ]]; then
      count=$((count + 1))
    else
      return 1
    fi
  done
  (( count > 0 )) || return 1
  printf '%s\n' "$count"
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
  online="$(getconf _NPROCESSORS_ONLN)"
  [[ "$online" =~ ^[1-9][0-9]*$ ]] || {
    echo "cannot determine online CPU count" >&2
    return 1
  }
  max_cpu=$((online - 1))
  declare -A owner=()
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
      if [[ -n "${owner[$cpu]:-}" ]]; then
        echo "$role cpuset '$set' overlaps ${owner[$cpu]} on CPU $cpu; benchmark roles must be disjoint" >&2
        return 1
      fi
      owner[$cpu]="$role"
    done <<<"$ids"
  done
}

# The gateway is the unit under test: exactly four CPUs and 8GiB. Backends and
# clients use disjoint CPU sets so an echo loop or connection opener cannot be
# mistaken for gateway capacity. Clients deliberately receive more CPU than
# the 4c unit under test: a saturated load generator invalidates a gateway
# result. A 16-core / 32GiB+ benchmark host is advised.
GATEWAY_CPUSET="${GATEWAY_CPUSET:-0-3}"
NGINX_WORKERS="${NGINX_WORKERS:-$(cpuset_cpu_count "$GATEWAY_CPUSET")}"
BACKEND_CPUSET="${BACKEND_CPUSET:-4-7}"
CLIENT_CPUSET="${CLIENT_CPUSET:-8-15}"
CAPACITY_CLIENT_CPUSET="${CAPACITY_CLIENT_CPUSET:-$CLIENT_CPUSET}"
GATEWAY_MEMORY="${GATEWAY_MEMORY:-8g}"
BACKEND_MEMORY="${BACKEND_MEMORY:-2g}"
CLIENT_MEMORY="${CLIENT_MEMORY:-2g}"
NOFILE_LIMIT="${NOFILE_LIMIT:-300000}"
ACTIVE_CONNECTIONS="${ACTIVE_CONNECTIONS:-4096}"
ACTIVE_DURATION_SECS="${ACTIVE_DURATION_SECS:-30}"
ACTIVE_SAMPLE_AFTER_SECS="${ACTIVE_SAMPLE_AFTER_SECS:-5}"
ACTIVE_PAYLOAD_BYTES="${ACTIVE_PAYLOAD_BYTES:-256}"
ACTIVE_MESSAGE_INTERVAL_MICROS="${ACTIVE_MESSAGE_INTERVAL_MICROS:-0}"
CAPACITY_CONNECTIONS="${CAPACITY_CONNECTIONS:-20000}"
CAPACITY_CLIENTS="${CAPACITY_CLIENTS:-5}"
CAPACITY_HOLD_SECS="${CAPACITY_HOLD_SECS:-120}"
CAPACITY_SETTLE_SECS="${CAPACITY_SETTLE_SECS:-90}"
CAPACITY_SAMPLE_AFTER_SECS="${CAPACITY_SAMPLE_AFTER_SECS:-5}"
CAPACITY_CONNECT_WORKERS="${CAPACITY_CONNECT_WORKERS:-256}"
STRICT_GATE="${STRICT_GATE:-1}"
RUN_ACTIVE="${RUN_ACTIVE:-1}"
RUN_CAPACITY="${RUN_CAPACITY:-1}"
BENCH_SUBNET="${BENCH_SUBNET:-172.30.0.0/16}"
NGINX_BACKEND_BASE_IP="${NGINX_BACKEND_BASE_IP:-172.30.10}"
NGINX_GATEWAY_IP="${NGINX_GATEWAY_IP:-172.30.20.20}"
NGINX_CLIENT_BASE_IP="${NGINX_CLIENT_BASE_IP:-172.30.30}"
PROXYSSS_BACKEND_BASE_IP="${PROXYSSS_BACKEND_BASE_IP:-172.30.110}"
PROXYSSS_GATEWAY_IP="${PROXYSSS_GATEWAY_IP:-172.30.120.20}"
PROXYSSS_CLIENT_BASE_IP="${PROXYSSS_CLIENT_BASE_IP:-172.30.130}"
BUILD_PROFILE="${BUILD_PROFILE:-release-fast}"
FORCE_BUILD="${FORCE_BUILD:-0}"
IMAGE="${IMAGE:-proxysss-isolated-ws-bench:local}"
NGINX_VERSION="${NGINX_VERSION:-1.31.2}"
TLS_KEY_TYPE="${TLS_KEY_TYPE:-ecdsa}"
REFRESH_BASE_IMAGE="${REFRESH_BASE_IMAGE:-0}"
RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
RUN_ORDER="${RUN_ORDER:-nginx proxysss}"
RUN_DIR="${BENCH_ROOT:-$ROOT/.benchmark}/runs/isolated-websocket/$RUN_ID"
CONTEXT_DIR="$RUN_DIR/image-context"
BENCH_HELPER_BIN="$RUN_DIR/benchmark-helper"
NETWORK="proxysss-ws-isolated-$RUN_ID"
PREFIX="proxysss-ws-isolated-$RUN_ID"
PROXY_BIN="${PROXY_BIN:-$ROOT/target/$BUILD_PROFILE/proxysss}"

if [[ "$RUN_ACTIVE" != "1" && "$RUN_CAPACITY" != "1" ]]; then
  echo "at least one of RUN_ACTIVE or RUN_CAPACITY must be 1" >&2
  exit 1
fi
if [[ "$RUN_CAPACITY" == "1" ]] && (( CAPACITY_CLIENTS < 2 )); then
  echo "CAPACITY_CLIENTS must be at least 2; use several client source addresses for a high-capacity test." >&2
  exit 1
fi
if [[ "$RUN_CAPACITY" == "1" ]] && (( CAPACITY_CONNECTIONS % CAPACITY_CLIENTS != 0 )); then
  echo "CAPACITY_CONNECTIONS must divide evenly by CAPACITY_CLIENTS for an auditable capacity total." >&2
  exit 1
fi
if [[ "$RUN_CAPACITY" == "1" ]] && (( CAPACITY_CONNECTIONS / CAPACITY_CLIENTS > 25000 )); then
  echo "each client would open more than 25k sockets; increase CAPACITY_CLIENTS to stay within common Linux ephemeral-port ranges." >&2
  exit 1
fi
if [[ "$TLS_KEY_TYPE" != "ecdsa" && "$TLS_KEY_TYPE" != "rsa" ]]; then
  echo "TLS_KEY_TYPE must be ecdsa or rsa" >&2
  exit 1
fi
if ! [[ "$NGINX_WORKERS" =~ ^[1-9][0-9]*$ ]]; then
  echo "NGINX_WORKERS must be a positive integer" >&2
  exit 1
fi
validate_role_cpusets

if [[ "$FORCE_BUILD" == "1" || ! -x "$PROXY_BIN" ]]; then
  cargo build --profile "$BUILD_PROFILE" --locked
fi

mkdir -p "$RUN_DIR" "$CONTEXT_DIR"
go build -o "$BENCH_HELPER_BIN" "$ROOT/scripts/benchmark-helper.go"
cp "$PROXY_BIN" "$CONTEXT_DIR/proxysss"
cp "$ROOT/docker/isolated-websocket-bench.Dockerfile" "$CONTEXT_DIR/Dockerfile"
docker_build_args=(--build-arg "NGINX_VERSION=$NGINX_VERSION" -t "$IMAGE")
if [[ "$REFRESH_BASE_IMAGE" == "1" ]]; then
  docker_build_args=(--pull "${docker_build_args[@]}")
fi
docker build "${docker_build_args[@]}" "$CONTEXT_DIR" >/dev/null

cleanup() {
  set +e
  docker ps -aq --filter "name=^/${PREFIX}" | xargs -r docker rm -f >/dev/null 2>&1
  docker network rm "$NETWORK" >/dev/null 2>&1
}
trap cleanup EXIT

docker network create --driver bridge --subnet "$BENCH_SUBNET" "$NETWORK" >/dev/null

cat >"$RUN_DIR/proxysss.yaml" <<YAML
config_version: 1
logging:
  access_log: false
http:
  plain_bind: 0.0.0.0:18083
  tls_bind: 0.0.0.0:18443
  h3_bind: ''
  tls:
    mode: manual
    cert_path: /run/proxysss/bench.crt
    key_path: /run/proxysss/bench.key
    generate_self_signed_if_missing: false
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
    traffic_profile: small
    adaptive_system: true
    socket_extreme: true
    log_on_start: true
  hot_reload:
    enabled: false
affinity:
  enabled: false
load_balance:
  algorithm: round_robin
  retries:
    enabled: false
  passive_health:
    enabled: false
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: game-wss
        path_prefix: /ws
        upstream: ws://${PROXYSSS_BACKEND_BASE_IP}.1:18192
        upstreams:
          - ws://${PROXYSSS_BACKEND_BASE_IP}.1:18192
          - ws://${PROXYSSS_BACKEND_BASE_IP}.2:18192
          - ws://${PROXYSSS_BACKEND_BASE_IP}.3:18192
          - ws://${PROXYSSS_BACKEND_BASE_IP}.4:18192
        forward_headers: true
YAML

cat >"$RUN_DIR/nginx.conf" <<NGINX
user www-data;
worker_processes ${NGINX_WORKERS};
worker_rlimit_nofile ${NOFILE_LIMIT};
events {
    use epoll;
    worker_connections 65535;
    multi_accept on;
}
http {
    access_log off;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 30s;
    keepalive_requests 1000;
    upstream game_ws {
        server ${NGINX_BACKEND_BASE_IP}.1:18192 max_fails=2 fail_timeout=10s;
        server ${NGINX_BACKEND_BASE_IP}.2:18192 max_fails=2 fail_timeout=10s;
        server ${NGINX_BACKEND_BASE_IP}.3:18192 max_fails=2 fail_timeout=10s;
        server ${NGINX_BACKEND_BASE_IP}.4:18192 max_fails=2 fail_timeout=10s;
        keepalive 256;
        keepalive_timeout 60s;
    }
    server {
        listen 0.0.0.0:18081 backlog=65536 reuseport;
        location /ws/ {
            proxy_pass http://game_ws;
            proxy_http_version 1.1;
            proxy_set_header Upgrade \$http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_connect_timeout 2s;
            proxy_read_timeout 30s;
            proxy_send_timeout 10s;
            proxy_next_upstream_tries 2;
            proxy_buffering off;
        }
    }
    server {
        listen 0.0.0.0:18441 ssl backlog=65536 reuseport;
        http2 on;
        ssl_certificate /run/nginx/bench.crt;
        ssl_certificate_key /run/nginx/bench.key;
        ssl_session_cache shared:SSL:20m;
        ssl_session_timeout 1h;
        location /ws/ {
            proxy_pass http://game_ws;
            proxy_http_version 1.1;
            proxy_set_header Upgrade \$http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_connect_timeout 2s;
            proxy_read_timeout 30s;
            proxy_send_timeout 10s;
            proxy_next_upstream_tries 2;
            proxy_buffering off;
        }
    }
}
NGINX

start_backends() {
  local kind="$1" backend_base
  if [[ "$kind" == "proxysss" ]]; then
    backend_base="$PROXYSSS_BACKEND_BASE_IP"
  else
    backend_base="$NGINX_BACKEND_BASE_IP"
  fi
  for index in 1 2 3 4; do
    docker run -d --name "$PREFIX-backend-$index" \
      --network "$NETWORK" --ip "${backend_base}.${index}" \
      --cpuset-cpus "$BACKEND_CPUSET" --memory "$BACKEND_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      "$IMAGE" proxysss demo ws-echo --listen 0.0.0.0:18192 >/dev/null
  done
}

stop_backends() {
  for index in 1 2 3 4; do
    docker rm -f "$PREFIX-backend-$index" >/dev/null 2>&1 || true
  done
}

create_gateway() {
  local kind="$1" name="$PREFIX-gateway-$kind"
  local gateway_ip
  if [[ "$kind" == "proxysss" ]]; then
    gateway_ip="$PROXYSSS_GATEWAY_IP"
  else
    gateway_ip="$NGINX_GATEWAY_IP"
  fi
  if [[ "$kind" == "proxysss" ]]; then
    docker create --name "$name" --network "$NETWORK" --ip "$gateway_ip" \
      --cpuset-cpus "$GATEWAY_CPUSET" --memory "$GATEWAY_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      --sysctl net.core.somaxconn=65535 \
      --env "TLS_KEY_TYPE=$TLS_KEY_TYPE" \
      "$IMAGE" /bin/bash -ec '
        mkdir -p /run/proxysss
        if [[ "$TLS_KEY_TYPE" == "ecdsa" ]]; then
          openssl ecparam -name prime256v1 -genkey -noout \
            -out /run/proxysss/bench.key
          openssl req -x509 -new -sha256 -days 1 \
            -subj /CN=proxysss-isolated-bench \
            -key /run/proxysss/bench.key -out /run/proxysss/bench.crt >/dev/null 2>&1
        else
          openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
            -subj /CN=proxysss-isolated-bench \
            -keyout /run/proxysss/bench.key -out /run/proxysss/bench.crt >/dev/null 2>&1
        fi
        exec proxysss -config /etc/proxysss.yaml
      ' >/dev/null
    docker cp "$RUN_DIR/proxysss.yaml" "$name:/etc/proxysss.yaml"
  else
    docker create --name "$name" --network "$NETWORK" --ip "$gateway_ip" \
      --cpuset-cpus "$GATEWAY_CPUSET" --memory "$GATEWAY_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      --sysctl net.core.somaxconn=65535 \
      --env "TLS_KEY_TYPE=$TLS_KEY_TYPE" \
      "$IMAGE" /bin/bash -ec '
        mkdir -p /run/nginx
        if [[ "$TLS_KEY_TYPE" == "ecdsa" ]]; then
          openssl ecparam -name prime256v1 -genkey -noout \
            -out /run/nginx/bench.key
          openssl req -x509 -new -sha256 -days 1 \
            -subj /CN=proxysss-isolated-bench \
            -key /run/nginx/bench.key -out /run/nginx/bench.crt >/dev/null 2>&1
        else
          openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
            -subj /CN=proxysss-isolated-bench \
            -keyout /run/nginx/bench.key -out /run/nginx/bench.crt >/dev/null 2>&1
        fi
        nginx -t -c /etc/nginx.conf
        exec nginx -c /etc/nginx.conf -g "daemon off;"
      ' >/dev/null
    docker cp "$RUN_DIR/nginx.conf" "$name:/etc/nginx.conf"
  fi
  docker start "$name" >/dev/null
}

wait_wss() {
  local kind="$1" port="$2"
  local gateway_ip
  if [[ "$kind" == "proxysss" ]]; then
    gateway_ip="$PROXYSSS_GATEWAY_IP"
  else
    gateway_ip="$NGINX_GATEWAY_IP"
  fi
  for _ in $(seq 1 60); do
    local output
    output="$(docker run --rm --network "$NETWORK" "$IMAGE" \
      proxysss bench websocket --url "wss://${gateway_ip}:${port}/ws/" \
      --connections 1 --duration-secs 1 --payload-bytes 16 --insecure 2>&1 || true)"
    local parsed
    parsed="$(printf '%s\n' "$output" | "$BENCH_HELPER_BIN" parse-bench \
      --scenario readiness --gateway "$kind" --protocol websocket \
      --target "wss://${gateway_ip}:${port}/ws/" --concurrency 1 --duration 1)"
    if grep -Eq '"success":[1-9][0-9]*' <<<"$parsed"; then
      return 0
    fi
    sleep 0.5
  done
  echo "gateway WSS listener did not become ready on ${port}" >&2
  docker inspect --format '{{.State.Status}} exit={{.State.ExitCode}} error={{.State.Error}}' \
    "$PREFIX-gateway-$kind" 2>&1 | tee "$RUN_DIR/${kind}-gateway-state.txt" >&2 || true
  docker logs "$PREFIX-gateway-$kind" 2>&1 | tee "$RUN_DIR/${kind}-gateway-start.log" >&2 || true
  return 1
}

run_active() {
  local kind="$1" port="$2" output="$RUN_DIR/${kind}-active.txt"
  local name="$PREFIX-${kind}-active"
  local gateway_ip client_base_ip
  if [[ "$kind" == "proxysss" ]]; then
    gateway_ip="$PROXYSSS_GATEWAY_IP"
    client_base_ip="$PROXYSSS_CLIENT_BASE_IP"
  else
    gateway_ip="$NGINX_GATEWAY_IP"
    client_base_ip="$NGINX_CLIENT_BASE_IP"
  fi
  local bench_args=(
    --url "wss://${gateway_ip}:${port}/ws/"
    --connections "$ACTIVE_CONNECTIONS"
    --duration-secs "$ACTIVE_DURATION_SECS"
    --payload-bytes "$ACTIVE_PAYLOAD_BYTES"
    --insecure
  )
  if (( ACTIVE_MESSAGE_INTERVAL_MICROS > 0 )); then
    bench_args+=(--message-interval-micros "$ACTIVE_MESSAGE_INTERVAL_MICROS")
  fi
  docker run -d --name "$name" --network "$NETWORK" --ip "${client_base_ip}.10" \
    --cpuset-cpus "$CLIENT_CPUSET" --memory "$CLIENT_MEMORY" \
    --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
    "$IMAGE" proxysss bench websocket "${bench_args[@]}" >/dev/null
  sleep "$ACTIVE_SAMPLE_AFTER_SECS"
  docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}} {{.PIDs}}' \
    "$PREFIX-gateway-$kind" "$name" | tee "$RUN_DIR/${kind}-active-stats.txt" || true
  docker wait "$name" >/dev/null
  docker logs "$name" | tee "$output"
  docker rm "$name" >/dev/null
}

run_capacity() {
  local kind="$1" port="$2" per_client=$((CAPACITY_CONNECTIONS / CAPACITY_CLIENTS))
  local gateway_ip client_base_ip
  if [[ "$kind" == "proxysss" ]]; then
    gateway_ip="$PROXYSSS_GATEWAY_IP"
    client_base_ip="$PROXYSSS_CLIENT_BASE_IP"
  else
    gateway_ip="$NGINX_GATEWAY_IP"
    client_base_ip="$NGINX_CLIENT_BASE_IP"
  fi
  local ids=()
  for index in $(seq 1 "$CAPACITY_CLIENTS"); do
    local name="$PREFIX-${kind}-capacity-${index}"
    local client_ip="${client_base_ip}.$((100 + index))"
    ids+=("$name")
    docker run -d --name "$name" --network "$NETWORK" --ip "$client_ip" \
      --cpuset-cpus "$CAPACITY_CLIENT_CPUSET" --memory "$CLIENT_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      "$IMAGE" proxysss bench websocket \
        --url "wss://${gateway_ip}:${port}/ws/" \
        --connections "$per_client" --duration-secs "$CAPACITY_HOLD_SECS" \
        --hold-connections --connect-workers "$CAPACITY_CONNECT_WORKERS" \
        --connect-timeout-ms 10000 --connect-retries 2 --insecure >/dev/null
  done

  local sample_after="$CAPACITY_SAMPLE_AFTER_SECS"
  if (( sample_after > CAPACITY_SETTLE_SECS )); then
    sample_after="$CAPACITY_SETTLE_SECS"
  fi
  if (( sample_after > 0 )); then
    sleep "$sample_after"
    docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}} {{.PIDs}}' \
      "$PREFIX-gateway-$kind" "${ids[@]}" \
      | tee "$RUN_DIR/${kind}-capacity-ramp-stats.txt" || true
  fi
  local remaining_settle=$((CAPACITY_SETTLE_SECS - sample_after))
  if (( remaining_settle > 0 )); then
    sleep "$remaining_settle"
  fi
  docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}} {{.PIDs}}' \
    "$PREFIX-gateway-$kind" | tee "$RUN_DIR/${kind}-capacity-gateway-stats.txt"

  local outputs=()
  for name in "${ids[@]}"; do
    docker wait "$name" >/dev/null
    local output="$RUN_DIR/${name}.txt"
    docker logs "$name" | tee "$output"
    outputs+=("$output")
    docker rm "$name" >/dev/null
  done
  local aggregate_args=(aggregate-isolated-wss-capacity --expected "$CAPACITY_CONNECTIONS")
  for output in "${outputs[@]}"; do
    aggregate_args+=(--input "$output")
  done
  "$BENCH_HELPER_BIN" "${aggregate_args[@]}" | tee "$RUN_DIR/${kind}-capacity-total.txt"
}

run_gateway_suite() {
  local kind="$1" port="$2"
  # Recreate backend network namespaces for every gateway. Reusing them makes
  # the second gateway inherit tens of thousands of server-side TCP states
  # from the first run and biases handshake/tail latency by execution order.
  start_backends "$kind"
  create_gateway "$kind"
  wait_wss "$kind" "$port"
  if [[ "$RUN_ACTIVE" == "1" ]]; then
    run_active "$kind" "$port"
  fi
  if [[ "$RUN_CAPACITY" == "1" ]]; then
    run_capacity "$kind" "$port"
  fi
  docker rm -f "$PREFIX-gateway-$kind" >/dev/null
  stop_backends
}

for kind in $RUN_ORDER; do
  case "$kind" in
    nginx) run_gateway_suite nginx 18441 ;;
    proxysss) run_gateway_suite proxysss 18443 ;;
    *) echo "RUN_ORDER accepts only nginx and proxysss, got: $kind" >&2; exit 1 ;;
  esac
done

for required_kind in nginx proxysss; do
  if [[ "$RUN_ACTIVE" == "1" && ! -f "$RUN_DIR/${required_kind}-active.txt" ]]; then
    echo "RUN_ORDER must run nginx and proxysss exactly once" >&2
    exit 1
  fi
  if [[ "$RUN_CAPACITY" == "1" && ! -f "$RUN_DIR/${required_kind}-capacity-total.txt" ]]; then
    echo "RUN_ORDER must run nginx and proxysss exactly once" >&2
    exit 1
  fi
done

cat >"$RUN_DIR/run-metadata.txt" <<EOF
gateway_cpuset=$GATEWAY_CPUSET
gateway_memory=$GATEWAY_MEMORY
nginx_workers=$NGINX_WORKERS
backend_count=4
backend_cpuset=$BACKEND_CPUSET
backend_memory_each=$BACKEND_MEMORY
client_cpuset=$CLIENT_CPUSET
capacity_client_cpuset=$CAPACITY_CLIENT_CPUSET
capacity_sample_after_secs=$CAPACITY_SAMPLE_AFTER_SECS
capacity_clients=$CAPACITY_CLIENTS
active_connections=$ACTIVE_CONNECTIONS
active_payload_bytes=$ACTIVE_PAYLOAD_BYTES
active_message_interval_micros=$ACTIVE_MESSAGE_INTERVAL_MICROS
active_duration_secs=$ACTIVE_DURATION_SECS
capacity_connections=$CAPACITY_CONNECTIONS
capacity_hold_secs=$CAPACITY_HOLD_SECS
tls_key_type=$TLS_KEY_TYPE
nginx_version=$NGINX_VERSION
run_order=$RUN_ORDER
role_isolation=docker-cgroup-cpuset-network-namespace
nginx_ip_tuple=$NGINX_BACKEND_BASE_IP/$NGINX_GATEWAY_IP/$NGINX_CLIENT_BASE_IP
proxysss_ip_tuple=$PROXYSSS_BACKEND_BASE_IP/$PROXYSSS_GATEWAY_IP/$PROXYSSS_CLIENT_BASE_IP
run_active=$RUN_ACTIVE
run_capacity=$RUN_CAPACITY
EOF

summary_args=(
  summarize-isolated-wss
  --run-dir "$RUN_DIR"
  --out-json "$RUN_DIR/summary.json"
  --out-markdown "$RUN_DIR/summary.md"
  --require-active="$RUN_ACTIVE"
  --require-capacity="$RUN_CAPACITY"
)
if [[ "$RUN_ACTIVE" == "1" ]]; then
  if (( ACTIVE_MESSAGE_INTERVAL_MICROS > 0 )); then
    summary_args+=(--gate-active-ops=false --gate-active-latency=true)
  else
    summary_args+=(--gate-active-ops=true --gate-active-latency=false)
  fi
fi
if [[ "$STRICT_GATE" == "1" ]]; then
  summary_args+=(--strict)
fi
"$BENCH_HELPER_BIN" "${summary_args[@]}"

echo "results: $RUN_DIR"
