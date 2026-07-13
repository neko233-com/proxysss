#!/usr/bin/env bash
# Three-role Linux WSS evidence harness.
#
# Run this *on the client/load-generator host*. It stages the same proxysss
# binary on independent gateway and backend hosts, alternates nginx/proxysss,
# records host fingerprints plus raw client output, and fails unless each WSS
# metric strictly wins. It deliberately does not pretend that Docker role
# isolation proves NIC, IRQ, RSS, or cross-host latency behavior.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

[[ "$(uname -s)" == "Linux" ]] || {
  echo "benchmark-cross-host-wss.sh must run from a Linux client host" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}
for command in ssh scp go curl sha256sum awk; do require_cmd "$command"; done

RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
GATEWAY_HOST="${GATEWAY_HOST:?set GATEWAY_HOST to the independent gateway SSH host}"
BACKEND_HOST="${BACKEND_HOST:?set BACKEND_HOST to the independent backend SSH host}"
BACKEND_ADDR="${BACKEND_ADDR:?set BACKEND_ADDR to the backend address reachable from the gateway}"
GATEWAY_ADDR="${GATEWAY_ADDR:?set GATEWAY_ADDR to the gateway address reachable from this client host}"
SSH_OPTS="${SSH_OPTS:--o BatchMode=yes}"
REPETITIONS="${REPETITIONS:-4}"
CONNECTIONS="${CONNECTIONS:-4096}"
CAPACITY_CONNECTIONS="${CAPACITY_CONNECTIONS:-20000}"
DURATION_SECS="${DURATION_SECS:-30}"
CAPACITY_HOLD_SECS="${CAPACITY_HOLD_SECS:-60}"
CAPACITY_CONNECT_WORKERS="${CAPACITY_CONNECT_WORKERS:-256}"
NOFILE_LIMIT="${NOFILE_LIMIT:-300000}"
GATEWAY_CPUSET="${GATEWAY_CPUSET:-0-3}"
# Memory is evidence, not an arbitrary admission test. The default retains
# cgroup accounting without a cap; set 8G (or another validated envelope) when
# the release under test has an explicit memory budget.
GATEWAY_MEMORY_MAX="${GATEWAY_MEMORY_MAX:-infinity}"
RESOURCE_SAMPLE_AFTER_SECS="${RESOURCE_SAMPLE_AFTER_SECS:-5}"
NGINX_WORKERS="${NGINX_WORKERS:-4}"
NGINX_BIN="${NGINX_BIN:-/usr/local/nginx/sbin/nginx}"
BUILD_PROFILE="${BUILD_PROFILE:-release-fast}"
BUILD_NATIVE="${BUILD_NATIVE:-0}"
PROXY_BIN="${PROXY_BIN:-$ROOT/target/$BUILD_PROFILE/proxysss}"
BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
RUN_DIR="$BENCH_ROOT/runs/cross-host-wss/$RUN_ID"
REMOTE_ROOT="${REMOTE_ROOT:-/tmp/proxysss-cross-host-$RUN_ID}"
GATEWAY_PORT="${GATEWAY_PORT:-18443}"
BACKEND_PORT="${BACKEND_PORT:-18192}"

[[ "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "RUN_ID contains unsafe characters" >&2; exit 1; }
[[ "$REMOTE_ROOT" == "/tmp/proxysss-cross-host-$RUN_ID" ]] || {
  echo "REMOTE_ROOT must be the run-specific /tmp/proxysss-cross-host-$RUN_ID path" >&2
  exit 1
}
[[ "$GATEWAY_HOST" != "$BACKEND_HOST" && "$GATEWAY_ADDR" != "$BACKEND_ADDR" ]] || {
  echo "gateway and backend must be distinct SSH hosts and reachable addresses" >&2
  exit 1
}
[[ "$REPETITIONS" =~ ^[1-9][0-9]*$ && "$REPETITIONS" -ge 4 && $((REPETITIONS % 2)) -eq 0 ]] || {
  echo "REPETITIONS must be an even integer >= 4 to balance gateway order" >&2
  exit 1
}
for value in CONNECTIONS CAPACITY_CONNECTIONS DURATION_SECS CAPACITY_HOLD_SECS CAPACITY_CONNECT_WORKERS NOFILE_LIMIT NGINX_WORKERS GATEWAY_PORT BACKEND_PORT RESOURCE_SAMPLE_AFTER_SECS; do
  [[ "${!value}" =~ ^[1-9][0-9]*$ ]] || { echo "$value must be positive" >&2; exit 1; }
done
if [[ "$BUILD_NATIVE" != "0" && "$BUILD_NATIVE" != "1" ]]; then
  echo "BUILD_NATIVE must be 0 or 1" >&2
  exit 1
fi

cpuset_cpu_count() {
  local cpuset="$1" item first last count=0
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

GATEWAY_CPU_COUNT="$(cpuset_cpu_count "$GATEWAY_CPUSET")" || {
  echo "GATEWAY_CPUSET must be a non-empty Linux CPU list" >&2
  exit 1
}
[[ "$GATEWAY_CPU_COUNT" == "$NGINX_WORKERS" ]] || {
  echo "NGINX_WORKERS must equal GATEWAY_CPUSET core count for a fair envelope" >&2
  exit 1
}

read -r -a SSH_ARGS <<<"$SSH_OPTS"
ssh_run() { ssh "${SSH_ARGS[@]}" "$1" "${@:2}"; }
scp_to() { scp "${SSH_ARGS[@]}" "$1" "$2:$REMOTE_ROOT/$3"; }
machine_id_hash() { sha256sum /etc/machine-id | awk '{print $1}'; }

for host in "$GATEWAY_HOST" "$BACKEND_HOST"; do
  ssh_run "$host" "test \"\$(uname -s)\" = Linux && command -v openssl >/dev/null && command -v sha256sum >/dev/null && test \"\$(ulimit -Hn)\" -ge '$NOFILE_LIMIT'"
done
ssh_run "$GATEWAY_HOST" "command -v systemd-run >/dev/null && systemctl show-environment >/dev/null && test \"\$(nproc)\" -ge '$GATEWAY_CPU_COUNT'"
CLIENT_MACHINE_ID_HASH="$(machine_id_hash)"
GATEWAY_MACHINE_ID_HASH="$(ssh_run "$GATEWAY_HOST" "sha256sum /etc/machine-id | awk '{print \$1}'")"
BACKEND_MACHINE_ID_HASH="$(ssh_run "$BACKEND_HOST" "sha256sum /etc/machine-id | awk '{print \$1}'")"
[[ "$CLIENT_MACHINE_ID_HASH" != "$GATEWAY_MACHINE_ID_HASH" && "$CLIENT_MACHINE_ID_HASH" != "$BACKEND_MACHINE_ID_HASH" && "$GATEWAY_MACHINE_ID_HASH" != "$BACKEND_MACHINE_ID_HASH" ]] || {
  echo "client, gateway, and backend must be three distinct Linux machines" >&2
  exit 1
}
ulimit -n "$NOFILE_LIMIT" 2>/dev/null || true
[[ "$(ulimit -n)" -ge "$CAPACITY_CONNECTIONS" ]] || {
  echo "client ulimit -n must be at least CAPACITY_CONNECTIONS=$CAPACITY_CONNECTIONS" >&2
  exit 1
}

if [[ "$BUILD_NATIVE" == "1" ]]; then
  [[ "$PROXY_BIN" == "$ROOT/target/$BUILD_PROFILE/proxysss" ]] || {
    echo "BUILD_NATIVE=1 requires the default PROXY_BIN; prebuild and pass an explicit binary otherwise" >&2
    exit 1
  }
  RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native" cargo build --profile "$BUILD_PROFILE" --locked
elif [[ ! -x "$PROXY_BIN" ]]; then
  cargo build --profile "$BUILD_PROFILE" --locked
fi

mkdir -p "$RUN_DIR"
HELPER="$RUN_DIR/benchmark-helper"
go build -o "$HELPER" "$ROOT/scripts/benchmark-helper.go"
LOCAL_SHA="$(sha256sum "$PROXY_BIN" | awk '{print $1}')"

prepare_remote() {
  local host="$1"
  ssh_run "$host" "test ! -e '$REMOTE_ROOT' && install -d -m 0755 '$REMOTE_ROOT'"
  scp_to "$PROXY_BIN" "$host" proxysss
  ssh_run "$host" "chmod 0755 '$REMOTE_ROOT/proxysss'; sha256sum '$REMOTE_ROOT/proxysss'"
}
prepare_remote "$GATEWAY_HOST" | tee "$RUN_DIR/gateway-proxy-sha256.txt"
prepare_remote "$BACKEND_HOST" | tee "$RUN_DIR/backend-proxy-sha256.txt"
grep -q "$LOCAL_SHA" "$RUN_DIR/gateway-proxy-sha256.txt"
grep -q "$LOCAL_SHA" "$RUN_DIR/backend-proxy-sha256.txt"

ssh_run "$GATEWAY_HOST" "uname -a; lscpu; '$NGINX_BIN' -V 2>&1" >"$RUN_DIR/gateway-host.txt"
ssh_run "$BACKEND_HOST" "uname -a; lscpu" >"$RUN_DIR/backend-host.txt"
uname -a >"$RUN_DIR/client-host.txt"
lscpu >>"$RUN_DIR/client-host.txt"

cat >"$RUN_DIR/proxysss.yaml" <<YAML
config_version: 1
logging: { access_log: false }
http:
  plain_bind: ''
  tls_bind: 0.0.0.0:${GATEWAY_PORT}
  h3_bind: ''
  tls:
    mode: manual
    cert_path: ${REMOTE_ROOT}/bench.crt
    key_path: ${REMOTE_ROOT}/bench.key
script: { enabled: false }
plugins: { enabled: false }
admin: { enabled: false }
runtime:
  performance: { enabled: true, profile: edge, traffic_profile: small, adaptive_system: true, socket_extreme: true }
  hot_reload: { enabled: false }
affinity: { enabled: false }
load_balance:
  retries: { enabled: false }
  passive_health: { enabled: false }
  active_health: { enabled: false }
services:
  reverse_proxy:
    routes:
      - name: websocket-echo
        path_prefix: /ws
        upstream: ws://${BACKEND_ADDR}:${BACKEND_PORT}
        forward_headers: false
YAML
cat >"$RUN_DIR/nginx.conf" <<NGINX
worker_processes ${NGINX_WORKERS};
pid ${REMOTE_ROOT}/nginx.pid;
events { use epoll; worker_connections 65535; multi_accept on; }
http {
  access_log off;
  tcp_nodelay on;
  upstream ws_echo { server ${BACKEND_ADDR}:${BACKEND_PORT}; keepalive 256; }
  server {
    listen 0.0.0.0:${GATEWAY_PORT} ssl backlog=65536 reuseport;
    http2 on;
    ssl_certificate ${REMOTE_ROOT}/bench.crt;
    ssl_certificate_key ${REMOTE_ROOT}/bench.key;
    ssl_session_cache shared:SSL:20m;
    location /ws/ {
      proxy_http_version 1.1;
      proxy_set_header Upgrade \$http_upgrade;
      proxy_set_header Connection "upgrade";
      proxy_set_header Host \$host;
      proxy_pass http://ws_echo;
    }
  }
}
NGINX
scp_to "$RUN_DIR/proxysss.yaml" "$GATEWAY_HOST" proxysss.yaml
scp_to "$RUN_DIR/nginx.conf" "$GATEWAY_HOST" nginx.conf

backend_pid=""
gateway_pid=""
gateway_unit=""
stop_processes() {
  if [[ -n "$gateway_unit" ]]; then ssh_run "$GATEWAY_HOST" "systemctl stop '$gateway_unit' 2>/dev/null || true" || true; fi
  if [[ -n "$backend_pid" ]]; then ssh_run "$BACKEND_HOST" "kill '$backend_pid' 2>/dev/null || true" || true; fi
}
trap stop_processes EXIT

start_backend() {
  backend_pid="$(ssh_run "$BACKEND_HOST" "cd '$REMOTE_ROOT' && ulimit -n '$NOFILE_LIMIT' && nohup ./proxysss demo ws-echo --listen 0.0.0.0:$BACKEND_PORT >backend.log 2>&1 & echo \$!")"
  [[ "$backend_pid" =~ ^[0-9]+$ ]] || { echo "backend failed to start" >&2; exit 1; }
}
start_gateway() {
  local kind="$1"
  gateway_unit="proxysss-bench-${RUN_ID}-${kind}"
  ssh_run "$GATEWAY_HOST" "cd '$REMOTE_ROOT' && openssl ecparam -name prime256v1 -genkey -noout -out bench.key && openssl req -x509 -new -sha256 -days 1 -subj /CN=proxysss-cross-host -key bench.key -out bench.crt >/dev/null 2>&1"
  if [[ "$kind" == proxysss ]]; then
    ssh_run "$GATEWAY_HOST" "systemd-run --quiet --unit '$gateway_unit' --collect --property 'AllowedCPUs=$GATEWAY_CPUSET' --property 'MemoryMax=$GATEWAY_MEMORY_MAX' --property 'LimitNOFILE=$NOFILE_LIMIT' --working-directory '$REMOTE_ROOT' /bin/bash -lc 'exec ./proxysss -config proxysss.yaml'"
  else
    ssh_run "$GATEWAY_HOST" "systemd-run --quiet --unit '$gateway_unit' --collect --property 'AllowedCPUs=$GATEWAY_CPUSET' --property 'MemoryMax=$GATEWAY_MEMORY_MAX' --property 'LimitNOFILE=$NOFILE_LIMIT' --working-directory '$REMOTE_ROOT' /bin/bash -lc 'exec \"$NGINX_BIN\" -c nginx.conf -g \"daemon off;\"'"
  fi
  gateway_pid="0"
  for _ in $(seq 1 20); do
    gateway_pid="$(ssh_run "$GATEWAY_HOST" "systemctl show '$gateway_unit' --property MainPID --value")"
    [[ "$gateway_pid" =~ ^[1-9][0-9]*$ ]] && break
    sleep 0.1
  done
  [[ "$gateway_pid" =~ ^[1-9][0-9]*$ ]] || { echo "$kind systemd unit did not start" >&2; exit 1; }
  for _ in $(seq 1 60); do
    if curl --silent --show-error --insecure --connect-timeout 1 "https://${GATEWAY_ADDR}:${GATEWAY_PORT}/ws/" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "$kind did not become reachable" >&2
  ssh_run "$GATEWAY_HOST" "journalctl --no-pager -u '$gateway_unit' -n 100 2>/dev/null || true" >&2 || true
  exit 1
}
stop_gateway() {
  ssh_run "$GATEWAY_HOST" "systemctl stop '$gateway_unit'"
  gateway_pid=""
  gateway_unit=""
}

sample_gateway_resources() {
  local phase="$1" kind="$2"
  ssh_run "$GATEWAY_HOST" "systemctl show '$gateway_unit' --property MainPID --property MemoryCurrent --property MemoryPeak --property CPUUsageNSec --property AllowedCPUs --property MemoryMax" >"$RUN_DIR/${phase}-${kind}-cgroup.txt"
}

record_capacity_memory_cost() {
  local iteration="$1" kind="$2" sample="$RUN_DIR/capacity-r${iteration}-final-${kind}-cgroup.txt"
  local current peak measured
  current="$(awk -F= '$1 == "MemoryCurrent" {print $2}' "$sample")"
  peak="$(awk -F= '$1 == "MemoryPeak" {print $2}' "$sample")"
  measured="$peak"
  [[ "$measured" =~ ^[0-9]+$ ]] || measured="$current"
  [[ "$measured" =~ ^[0-9]+$ ]] || return 0
  printf 'repetition=%s gateway=%s memory_bytes=%s bytes_per_connection=%s requested_connections=%s\n' \
    "$iteration" "$kind" "$measured" "$((measured / CAPACITY_CONNECTIONS))" "$CAPACITY_CONNECTIONS" \
    >>"$RUN_DIR/capacity-memory-per-connection.txt"
}

run_active() {
  local phase="$1" kind="$2" interval="${3:-}"
  start_gateway "$kind"
  local output="$RUN_DIR/${phase}-${kind}.txt"
  local args=(bench websocket --url "wss://${GATEWAY_ADDR}:${GATEWAY_PORT}/ws/" --connections "$CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes 256 --insecure)
  [[ -z "$interval" ]] || args+=(--message-interval-micros "$interval")
  "$PROXY_BIN" "${args[@]}" >"$output" 2>&1 &
  local client_pid=$!
  sleep "$RESOURCE_SAMPLE_AFTER_SECS"
  sample_gateway_resources "$phase" "$kind"
  wait "$client_pid"
  cat "$output"
  stop_gateway
}

run_capacity() {
  local kind="$1" iteration="$2" output="$RUN_DIR/capacity-r${iteration}-${kind}.txt"
  start_gateway "$kind"
  "$PROXY_BIN" bench websocket --url "wss://${GATEWAY_ADDR}:${GATEWAY_PORT}/ws/" --connections "$CAPACITY_CONNECTIONS" --hold-connections --connect-workers "$CAPACITY_CONNECT_WORKERS" --duration-secs "$CAPACITY_HOLD_SECS" --insecure >"$output" 2>&1 &
  local client_pid=$!
  sleep "$RESOURCE_SAMPLE_AFTER_SECS"
  sample_gateway_resources "capacity-r${iteration}" "$kind"
  wait "$client_pid"
  sample_gateway_resources "capacity-r${iteration}-final" "$kind"
  record_capacity_memory_cost "$iteration" "$kind"
  cat "$output"
  stop_gateway
  local requested opened failed
  requested="$(awk -F: '/connections requested/ {gsub(/ /, "", $2); print $2}' "$output")"
  opened="$(awk -F: '/connections opened/ {gsub(/ /, "", $2); print $2}' "$output")"
  failed="$(awk -F: '/connections failed/ {gsub(/ /, "", $2); print $2}' "$output")"
  [[ "$requested" == "$CAPACITY_CONNECTIONS" && "$opened" == "$CAPACITY_CONNECTIONS" && "$failed" == "0" ]] || {
    echo "$kind capacity failed: requested=$requested opened=$opened failed=$failed" >&2
    exit 1
  }
}

capacity_metric() {
  local file="$1" label="$2"
  awk -F: -v label="$label" '$1 ~ label {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); split($2, value, " "); print value[1]; exit}' "$file"
}
strictly_greater() { awk -v left="$1" -v right="$2" 'BEGIN { exit !(left > right) }'; }
strictly_less() { awk -v left="$1" -v right="$2" 'BEGIN { exit !(left < right) }'; }
compare_capacity_iteration() {
  local iteration="$1" metric nginx proxy
  for metric in 'open rate' 'handshake p50' 'handshake p95' 'handshake p99'; do
    nginx="$(capacity_metric "$RUN_DIR/capacity-r${iteration}-nginx.txt" "$metric")"
    proxy="$(capacity_metric "$RUN_DIR/capacity-r${iteration}-proxysss.txt" "$metric")"
    [[ "$nginx" =~ ^[0-9]+([.][0-9]+)?$ && "$proxy" =~ ^[0-9]+([.][0-9]+)?$ ]] || {
      echo "missing $metric capacity metric in repetition $iteration" >&2
      exit 1
    }
    if [[ "$metric" == 'open rate' ]]; then
      strictly_greater "$proxy" "$nginx" || {
        echo "capacity repetition $iteration $metric did not strictly beat nginx: proxysss=$proxy nginx=$nginx" >&2
        exit 1
      }
    else
      strictly_less "$proxy" "$nginx" || {
        echo "capacity repetition $iteration $metric did not strictly beat nginx: proxysss=$proxy nginx=$nginx" >&2
        exit 1
      }
    fi
    printf 'repetition=%s metric=%s proxysss=%s nginx=%s\n' "$iteration" "$metric" "$proxy" "$nginx" >>"$RUN_DIR/capacity-strict-comparison.txt"
  done
}

start_backend
declare -a SATURATION_ROWS=() LATENCY_ROWS=()
for repetition in $(seq 1 "$REPETITIONS"); do
  if (( repetition % 2 )); then order="nginx proxysss"; else order="proxysss nginx"; fi
  for kind in $order; do
    run_active "saturation-r${repetition}" "$kind"
    row="$("$HELPER" parse-bench --scenario websocket-long-connection --gateway "$kind" --protocol websocket --target "wss://${GATEWAY_ADDR}:${GATEWAY_PORT}/ws/" --concurrency "$CONNECTIONS" --duration "$DURATION_SECS" <"$RUN_DIR/saturation-r${repetition}-${kind}.txt")"
    SATURATION_ROWS+=("$row")
  done
done
printf '%s\n' "${SATURATION_ROWS[@]}" >"$RUN_DIR/saturation.jsonl"
"$HELPER" aggregate-bench-medians --in "$RUN_DIR/saturation.jsonl" --out "$RUN_DIR/saturation.json"
"$HELPER" write-all-scenarios-summary --results "$RUN_DIR/saturation.json" --md "$RUN_DIR/saturation.md" --html "$RUN_DIR/saturation.html" --min-ratio 1.0 --critical-ratio 1.0 --aggregate-ratio 1.0 --max-latency-ratio 1.0 --require-zero-errors=true --gate-ops=true --gate-latency=false --strict-superiority=true --mixed-matrix=false --samples-per-gateway "$REPETITIONS" --phase=cross-host-saturation

"$HELPER" write-equal-load-plan --results "$RUN_DIR/saturation.json" --out "$RUN_DIR/equal-load-plan.txt" --fraction 0.70
interval="$(awk -F'|' '$1 == "websocket-long-connection" {print $2}' "$RUN_DIR/equal-load-plan.txt")"
[[ "$interval" =~ ^[1-9][0-9]*$ ]] || { echo "could not calculate equal-load interval" >&2; exit 1; }
for repetition in $(seq 1 "$REPETITIONS"); do
  if (( repetition % 2 )); then order="proxysss nginx"; else order="nginx proxysss"; fi
  for kind in $order; do
    run_active "equal-r${repetition}" "$kind" "$interval"
    row="$("$HELPER" parse-bench --scenario websocket-long-connection --gateway "$kind" --protocol websocket --target "wss://${GATEWAY_ADDR}:${GATEWAY_PORT}/ws/" --concurrency "$CONNECTIONS" --duration "$DURATION_SECS" <"$RUN_DIR/equal-r${repetition}-${kind}.txt")"
    LATENCY_ROWS+=("$row")
  done
done
printf '%s\n' "${LATENCY_ROWS[@]}" >"$RUN_DIR/equal-load.jsonl"
"$HELPER" aggregate-bench-medians --in "$RUN_DIR/equal-load.jsonl" --out "$RUN_DIR/equal-load.json"
"$HELPER" write-all-scenarios-summary --results "$RUN_DIR/equal-load.json" --md "$RUN_DIR/equal-load.md" --html "$RUN_DIR/equal-load.html" --min-ratio 1.0 --critical-ratio 1.0 --aggregate-ratio 1.0 --max-latency-ratio 1.0 --require-latency-percentiles=true --require-zero-errors=true --gate-ops=false --gate-latency=true --min-target-achievement=0.98 --strict-superiority=true --mixed-matrix=false --samples-per-gateway "$REPETITIONS" --phase=cross-host-equal-load

for repetition in $(seq 1 "$REPETITIONS"); do
  if (( repetition % 2 )); then order="nginx proxysss"; else order="proxysss nginx"; fi
  for kind in $order; do run_capacity "$kind" "$repetition"; done
  compare_capacity_iteration "$repetition"
done

ssh_run "$GATEWAY_HOST" "journalctl --no-pager -u 'proxysss-bench-${RUN_ID}-proxysss' -n 500 2>/dev/null || true" >"$RUN_DIR/gateway-proxysss.log" || true
ssh_run "$GATEWAY_HOST" "journalctl --no-pager -u 'proxysss-bench-${RUN_ID}-nginx' -n 500 2>/dev/null || true" >"$RUN_DIR/gateway-nginx.log" || true
scp "${SSH_ARGS[@]}" "$BACKEND_HOST:$REMOTE_ROOT/backend.log" "$RUN_DIR/backend.log" 2>/dev/null || true
cat >"$RUN_DIR/run-metadata.txt" <<EOF
run_id=$RUN_ID
roles=client-local,gateway=$GATEWAY_HOST,backend=$BACKEND_HOST
gateway_addr=$GATEWAY_ADDR
backend_addr=$BACKEND_ADDR
proxy_sha256=$LOCAL_SHA
nginx_bin=$NGINX_BIN
connections=$CONNECTIONS
capacity_connections=$CAPACITY_CONNECTIONS
capacity_connect_workers=$CAPACITY_CONNECT_WORKERS
nofile_limit=$NOFILE_LIMIT
gateway_cpuset=$GATEWAY_CPUSET
gateway_cpu_count=$GATEWAY_CPU_COUNT
gateway_memory_max=$GATEWAY_MEMORY_MAX
memory_gate=report-current-peak-and-per-connection-cost
client_machine_id_hash=$CLIENT_MACHINE_ID_HASH
gateway_machine_id_hash=$GATEWAY_MACHINE_ID_HASH
backend_machine_id_hash=$BACKEND_MACHINE_ID_HASH
repetitions=$REPETITIONS
method=separate-linux-hosts-systemd-cgroup-balanced-order-median-saturation-equal-load-strict-p50-p95-p99-zero-errors-capacity
EOF
echo "cross-host WSS gate passed: $RUN_DIR"
