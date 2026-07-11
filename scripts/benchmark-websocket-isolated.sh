#!/usr/bin/env bash
# Role-isolated Docker benchmark for a single 4c/8GiB WebSocket/WSS gateway.
#
# This is intentionally separate from benchmark-all-scenarios.sh: the latter is
# the broad mixed nginx-parity release matrix, while this harness verifies the
# production game-gateway question directly: active WSS echo latency/throughput
# plus 100k mostly-idle WSS tunnels, with no client or backend process sharing
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
require_cmd awk
require_cmd sed

# The gateway is the unit under test: exactly four CPUs and 8GiB. Backends and
# clients use disjoint CPU sets so an echo loop or connection opener cannot be
# mistaken for gateway capacity. A 12-core / 32GiB+ benchmark host is advised.
GATEWAY_CPUSET="${GATEWAY_CPUSET:-0-3}"
BACKEND_CPUSET="${BACKEND_CPUSET:-4-7}"
CLIENT_CPUSET="${CLIENT_CPUSET:-8-11}"
GATEWAY_MEMORY="${GATEWAY_MEMORY:-8g}"
BACKEND_MEMORY="${BACKEND_MEMORY:-2g}"
CLIENT_MEMORY="${CLIENT_MEMORY:-2g}"
NOFILE_LIMIT="${NOFILE_LIMIT:-300000}"
ACTIVE_CONNECTIONS="${ACTIVE_CONNECTIONS:-4096}"
ACTIVE_DURATION_SECS="${ACTIVE_DURATION_SECS:-30}"
ACTIVE_PAYLOAD_BYTES="${ACTIVE_PAYLOAD_BYTES:-256}"
CAPACITY_CONNECTIONS="${CAPACITY_CONNECTIONS:-100000}"
CAPACITY_CLIENTS="${CAPACITY_CLIENTS:-5}"
CAPACITY_HOLD_SECS="${CAPACITY_HOLD_SECS:-120}"
CAPACITY_SETTLE_SECS="${CAPACITY_SETTLE_SECS:-90}"
CAPACITY_CONNECT_WORKERS="${CAPACITY_CONNECT_WORKERS:-256}"
BENCH_SUBNET="${BENCH_SUBNET:-172.30.0.0/16}"
BACKEND_BASE_IP="${BACKEND_BASE_IP:-172.30.10}"
GATEWAY_IP="${GATEWAY_IP:-172.30.20.20}"
CLIENT_BASE_IP="${CLIENT_BASE_IP:-172.30.30}"
BUILD_PROFILE="${BUILD_PROFILE:-release-fast}"
FORCE_BUILD="${FORCE_BUILD:-0}"
IMAGE="${IMAGE:-proxysss-isolated-ws-bench:local}"
RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
RUN_DIR="${BENCH_ROOT:-$ROOT/.benchmark}/runs/isolated-websocket/$RUN_ID"
CONTEXT_DIR="$RUN_DIR/image-context"
NETWORK="proxysss-ws-isolated-$RUN_ID"
PREFIX="proxysss-ws-isolated-$RUN_ID"
PROXY_BIN="${PROXY_BIN:-$ROOT/target/$BUILD_PROFILE/proxysss}"

if (( CAPACITY_CLIENTS < 2 )); then
  echo "CAPACITY_CLIENTS must be at least 2; use several client source addresses for a 100k capacity test." >&2
  exit 1
fi
if (( CAPACITY_CONNECTIONS % CAPACITY_CLIENTS != 0 )); then
  echo "CAPACITY_CONNECTIONS must divide evenly by CAPACITY_CLIENTS for an auditable capacity total." >&2
  exit 1
fi
if (( CAPACITY_CONNECTIONS / CAPACITY_CLIENTS > 25000 )); then
  echo "each client would open more than 25k sockets; increase CAPACITY_CLIENTS to stay within common Linux ephemeral-port ranges." >&2
  exit 1
fi

if [[ "$FORCE_BUILD" == "1" || ! -x "$PROXY_BIN" ]]; then
  cargo build --profile "$BUILD_PROFILE" --locked
fi

mkdir -p "$RUN_DIR" "$CONTEXT_DIR"
cp "$PROXY_BIN" "$CONTEXT_DIR/proxysss"
cp "$ROOT/docker/isolated-websocket-bench.Dockerfile" "$CONTEXT_DIR/Dockerfile"
docker build --pull -t "$IMAGE" "$CONTEXT_DIR" >/dev/null

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
        upstream: ws://${BACKEND_BASE_IP}.1:18192
        upstreams:
          - ws://${BACKEND_BASE_IP}.1:18192
          - ws://${BACKEND_BASE_IP}.2:18192
          - ws://${BACKEND_BASE_IP}.3:18192
          - ws://${BACKEND_BASE_IP}.4:18192
        forward_headers: false
YAML

cat >"$RUN_DIR/nginx.conf" <<NGINX
user www-data;
worker_processes auto;
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
        least_conn;
        server ${BACKEND_BASE_IP}.1:18192 max_fails=2 fail_timeout=10s;
        server ${BACKEND_BASE_IP}.2:18192 max_fails=2 fail_timeout=10s;
        server ${BACKEND_BASE_IP}.3:18192 max_fails=2 fail_timeout=10s;
        server ${BACKEND_BASE_IP}.4:18192 max_fails=2 fail_timeout=10s;
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

for index in 1 2 3 4; do
  docker run -d --name "$PREFIX-backend-$index" \
    --network "$NETWORK" --ip "${BACKEND_BASE_IP}.${index}" \
    --cpuset-cpus "$BACKEND_CPUSET" --memory "$BACKEND_MEMORY" \
    --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
    "$IMAGE" proxysss demo ws-echo --listen 0.0.0.0:18192 >/dev/null
done

create_gateway() {
  local kind="$1" name="$PREFIX-gateway-$kind"
  if [[ "$kind" == "proxysss" ]]; then
    docker create --name "$name" --network "$NETWORK" --ip "$GATEWAY_IP" \
      --cpuset-cpus "$GATEWAY_CPUSET" --memory "$GATEWAY_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      --sysctl net.core.somaxconn=65535 \
      "$IMAGE" /bin/bash -ec '
        mkdir -p /run/proxysss
        openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
          -subj /CN=proxysss-isolated-bench \
          -keyout /run/proxysss/bench.key -out /run/proxysss/bench.crt >/dev/null 2>&1
        exec proxysss -config /etc/proxysss.yaml
      ' >/dev/null
    docker cp "$RUN_DIR/proxysss.yaml" "$name:/etc/proxysss.yaml"
  else
    docker create --name "$name" --network "$NETWORK" --ip "$GATEWAY_IP" \
      --cpuset-cpus "$GATEWAY_CPUSET" --memory "$GATEWAY_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      --sysctl net.core.somaxconn=65535 \
      "$IMAGE" /bin/bash -ec '
        mkdir -p /run/nginx
        openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
          -subj /CN=proxysss-isolated-bench \
          -keyout /run/nginx/bench.key -out /run/nginx/bench.crt >/dev/null 2>&1
        nginx -t -c /etc/nginx.conf
        exec nginx -c /etc/nginx.conf -g "daemon off;"
      ' >/dev/null
    docker cp "$RUN_DIR/nginx.conf" "$name:/etc/nginx.conf"
  fi
  docker start "$name" >/dev/null
}

wait_wss() {
  local port="$1"
  for _ in $(seq 1 60); do
    local output
    output="$(docker run --rm --network "$NETWORK" "$IMAGE" \
      proxysss bench websocket --url "wss://${GATEWAY_IP}:${port}/ws/" \
      --connections 1 --duration-secs 1 --payload-bytes 16 --insecure 2>&1 || true)"
    local success
    success="$(printf '%s\n' "$output" | awk -F ': *' '$1 ~ /^success/ { print $2; exit }')"
    if [[ "${success:-0}" =~ ^[1-9][0-9]*$ ]]; then
      return 0
    fi
    sleep 0.5
  done
  echo "gateway WSS listener did not become ready on ${port}" >&2
  return 1
}

metric() {
  local label="$1" file="$2"
  awk -F ': *' -v label="$label" '{ key = $1; sub(/[[:space:]]+$/, "", key); if (key == label) { print $2; exit } }' "$file"
}

run_active() {
  local kind="$1" port="$2" output="$RUN_DIR/${kind}-active.txt"
  docker run --rm --network "$NETWORK" --ip "${CLIENT_BASE_IP}.10" \
    --cpuset-cpus "$CLIENT_CPUSET" --memory "$CLIENT_MEMORY" \
    --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
    "$IMAGE" proxysss bench websocket \
      --url "wss://${GATEWAY_IP}:${port}/ws/" \
      --connections "$ACTIVE_CONNECTIONS" --duration-secs "$ACTIVE_DURATION_SECS" \
      --payload-bytes "$ACTIVE_PAYLOAD_BYTES" --insecure | tee "$output"
}

run_capacity() {
  local kind="$1" port="$2" per_client=$((CAPACITY_CONNECTIONS / CAPACITY_CLIENTS))
  local ids=()
  for index in $(seq 1 "$CAPACITY_CLIENTS"); do
    local name="$PREFIX-${kind}-capacity-${index}"
    local client_ip="${CLIENT_BASE_IP}.$((100 + index))"
    ids+=("$name")
    docker run -d --name "$name" --network "$NETWORK" --ip "$client_ip" \
      --cpuset-cpus "$CLIENT_CPUSET" --memory "$CLIENT_MEMORY" \
      --ulimit "nofile=${NOFILE_LIMIT}:${NOFILE_LIMIT}" \
      "$IMAGE" proxysss bench websocket \
        --url "wss://${GATEWAY_IP}:${port}/ws/" \
        --connections "$per_client" --duration-secs "$CAPACITY_HOLD_SECS" \
        --hold-connections --connect-workers "$CAPACITY_CONNECT_WORKERS" \
        --connect-timeout-ms 10000 --connect-retries 2 --insecure >/dev/null
  done

  sleep "$CAPACITY_SETTLE_SECS"
  docker stats --no-stream --format '{{.Name}} {{.CPUPerc}} {{.MemUsage}} {{.PIDs}}' \
    "$PREFIX-gateway-$kind" | tee "$RUN_DIR/${kind}-capacity-gateway-stats.txt"

  local opened=0 failed=0 attempts=0
  for name in "${ids[@]}"; do
    docker wait "$name" >/dev/null
    local output="$RUN_DIR/${name}.txt"
    docker logs "$name" | tee "$output"
    opened=$((opened + $(metric 'connections opened' "$output" || echo 0)))
    failed=$((failed + $(metric 'connections failed' "$output" || echo 0)))
    attempts=$((attempts + $(metric 'handshake attempts' "$output" || echo 0)))
    docker rm "$name" >/dev/null
  done
  printf 'requested=%s opened=%s failed=%s attempts=%s\n' \
    "$CAPACITY_CONNECTIONS" "$opened" "$failed" "$attempts" | tee "$RUN_DIR/${kind}-capacity-total.txt"
  [[ "$opened" == "$CAPACITY_CONNECTIONS" && "$failed" == "0" ]]
}

run_gateway_suite() {
  local kind="$1" port="$2"
  create_gateway "$kind"
  wait_wss "$port"
  run_active "$kind" "$port"
  run_capacity "$kind" "$port"
  docker rm -f "$PREFIX-gateway-$kind" >/dev/null
}

run_gateway_suite nginx 18441
run_gateway_suite proxysss 18443

NGINX_OPS="$(metric 'ops/sec' "$RUN_DIR/nginx-active.txt")"
PROXY_OPS="$(metric 'ops/sec' "$RUN_DIR/proxysss-active.txt")"
NGINX_P50="$(metric 'latency p50' "$RUN_DIR/nginx-active.txt")"
PROXY_P50="$(metric 'latency p50' "$RUN_DIR/proxysss-active.txt")"
NGINX_P95="$(metric 'latency p95' "$RUN_DIR/nginx-active.txt")"
PROXY_P95="$(metric 'latency p95' "$RUN_DIR/proxysss-active.txt")"
OPS_RATIO="$(awk -v proxy="$PROXY_OPS" -v nginx="$NGINX_OPS" 'BEGIN { printf "%.2f", (proxy / nginx - 1) * 100 }')"
P50_RATIO="$(awk -v proxy="${PROXY_P50%% *}" -v nginx="${NGINX_P50%% *}" 'BEGIN { printf "%.2f", (proxy / nginx - 1) * 100 }')"
P95_RATIO="$(awk -v proxy="${PROXY_P95%% *}" -v nginx="${NGINX_P95%% *}" 'BEGIN { printf "%.2f", (proxy / nginx - 1) * 100 }')"

cat >"$RUN_DIR/summary.md" <<EOF
# Isolated WSS benchmark

- gateway container: cpuset `$GATEWAY_CPUSET`, memory `$GATEWAY_MEMORY`
- backend containers: 4, cpuset `$BACKEND_CPUSET`, memory `$BACKEND_MEMORY` each
- client containers: active=1, capacity=`$CAPACITY_CLIENTS`, cpuset `$CLIENT_CPUSET`
- active workload: `$ACTIVE_CONNECTIONS` WSS echo sessions, `$ACTIVE_PAYLOAD_BYTES` bytes, `$ACTIVE_DURATION_SECS` s
- capacity workload: `$CAPACITY_CONNECTIONS` WSS sessions held for `$CAPACITY_HOLD_SECS` s
- TLS: generated self-signed fixture; client uses `--insecure` **only for this benchmark**

| Active WSS metric | nginx | proxysss | proxysss vs nginx |
| --- | ---: | ---: | ---: |
| ops/sec | $NGINX_OPS | $PROXY_OPS | ${OPS_RATIO}% |
| p50 latency | $NGINX_P50 | $PROXY_P50 | ${P50_RATIO}% |
| p95 latency | $NGINX_P95 | $PROXY_P95 | ${P95_RATIO}% |

| WSS capacity | nginx | proxysss |
| --- | ---: | ---: |
| requested | $(cat "$RUN_DIR/nginx-capacity-total.txt" | sed -n 's/.*requested=\([0-9]*\).*/\1/p') | $(cat "$RUN_DIR/proxysss-capacity-total.txt" | sed -n 's/.*requested=\([0-9]*\).*/\1/p') |
| opened | $(cat "$RUN_DIR/nginx-capacity-total.txt" | sed -n 's/.*opened=\([0-9]*\).*/\1/p') | $(cat "$RUN_DIR/proxysss-capacity-total.txt" | sed -n 's/.*opened=\([0-9]*\).*/\1/p') |
| failed | $(cat "$RUN_DIR/nginx-capacity-total.txt" | sed -n 's/.*failed=\([0-9]*\).*/\1/p') | $(cat "$RUN_DIR/proxysss-capacity-total.txt" | sed -n 's/.*failed=\([0-9]*\).*/\1/p') |

This is role/cgroup/network-namespace isolated. For a release claim about physical-machine latency, run the same gateway image on a dedicated gateway Docker host and use independent backend and client Docker hosts; do not treat a single Linux kernel as a WAN or cross-host proof.
EOF

cat "$RUN_DIR/summary.md"
echo "results: $RUN_DIR"
