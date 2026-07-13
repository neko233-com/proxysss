#!/usr/bin/env bash
# Repeated, order-balanced Linux WSS production gate.
#
# Active-session and connection-capacity scales are separated so a large idle
# hold does not contaminate steady-state message latency. Every scale runs an
# four-run Latin-square sequence, recreates backend/network namespaces per
# gateway, and is summarized/gated by the native Go benchmark helper using
# medians. Run order and address/hash assignment are balanced independently.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "benchmark-websocket-production-gate.sh requires Linux" >&2
  exit 1
fi

command -v docker >/dev/null 2>&1 || {
  echo "missing required command: docker" >&2
  exit 1
}

REPETITIONS="${REPETITIONS:-4}"
ACTIVE_SCALES="${ACTIVE_SCALES:-256 1024 4096}"
CAPACITY_SCALES="${CAPACITY_SCALES:-5000 10000 20000}"
ACTIVE_DURATION_SECS="${ACTIVE_DURATION_SECS:-30}"
ACTIVE_PAYLOAD_BYTES="${ACTIVE_PAYLOAD_BYTES:-256}"
LATENCY_TARGET_OPS="${LATENCY_TARGET_OPS:-40000}"
LATENCY_LARGE_SCALE_MIN_CONNECTIONS="${LATENCY_LARGE_SCALE_MIN_CONNECTIONS:-4096}"
LATENCY_LARGE_SCALE_TARGET_OPS="${LATENCY_LARGE_SCALE_TARGET_OPS:-30000}"
LATENCY_MIN_ACHIEVEMENT_PERCENT="${LATENCY_MIN_ACHIEVEMENT_PERCENT:-98}"
CAPACITY_HOLD_SECS="${CAPACITY_HOLD_SECS:-60}"
CAPACITY_SETTLE_SECS="${CAPACITY_SETTLE_SECS:-60}"
CAPACITY_CONNECT_WORKERS="${CAPACITY_CONNECT_WORKERS:-256}"
# Default to memory observation.  Set an explicit value only when the declared
# production envelope includes a real cgroup budget for load generators.
ACTIVE_CLIENT_MEMORY="${ACTIVE_CLIENT_MEMORY:-}"
CAPACITY_CLIENT_MEMORY="${CAPACITY_CLIENT_MEMORY:-}"
STRICT_GATE="${STRICT_GATE:-1}"
RUN_ACTIVE_MATRIX="${RUN_ACTIVE_MATRIX:-1}"
RUN_SATURATION_MATRIX="${RUN_SATURATION_MATRIX:-1}"
RUN_LATENCY_MATRIX="${RUN_LATENCY_MATRIX:-1}"
RUN_CAPACITY_MATRIX="${RUN_CAPACITY_MATRIX:-1}"
RUN_ID="${RUN_ID:-$(date +%Y%m%d-%H%M%S)-$$}"
PROXY_BIN="${PROXY_BIN:-$ROOT/target/release-fast/proxysss}"
BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
BENCH_SUBNET="${BENCH_SUBNET:-172.30.0.0/16}"
# benchmark-websocket-isolated.sh assigns fixed role addresses within a /16.
# Keep every Latin-square tuple inside the caller-selected subnet rather than
# silently retaining 172.30.* when BENCH_SUBNET is overridden to avoid a
# stale Docker network collision.
[[ "$BENCH_SUBNET" =~ ^[0-9]{1,3}\.[0-9]{1,3}\.0\.0/16$ ]] || {
  echo "BENCH_SUBNET must be an IPv4 /16 such as 172.30.0.0/16" >&2
  exit 1
}
BENCH_IP_PREFIX="${BENCH_IP_PREFIX:-${BENCH_SUBNET%%.0.0/16}}"
[[ "$BENCH_IP_PREFIX" =~ ^[0-9]{1,3}\.[0-9]{1,3}$ ]] || {
  echo "BENCH_IP_PREFIX must contain the first two IPv4 octets" >&2
  exit 1
}
OUTPUT_DIR="$BENCH_ROOT/runs/isolated-websocket-production/$RUN_ID"
HELPER="$OUTPUT_DIR/benchmark-helper"
PREBUILT_BENCH_HELPER="${PREBUILT_BENCH_HELPER:-}"
if [[ -n "$PREBUILT_BENCH_HELPER" ]]; then
  HELPER="$PREBUILT_BENCH_HELPER"
fi

if (( REPETITIONS < 4 || REPETITIONS % 4 != 0 )); then
  echo "REPETITIONS must be a multiple of four for a balanced order/address Latin square" >&2
  exit 1
fi
if [[ "$RUN_ACTIVE_MATRIX" != "1" && "$RUN_CAPACITY_MATRIX" != "1" ]]; then
  echo "at least one of RUN_ACTIVE_MATRIX or RUN_CAPACITY_MATRIX must be 1" >&2
  exit 1
fi
if [[ "$RUN_ACTIVE_MATRIX" == "1" && "$RUN_SATURATION_MATRIX" != "1" && "$RUN_LATENCY_MATRIX" != "1" && "$RUN_CAPACITY_MATRIX" != "1" ]]; then
  echo "active matrix selected but both active submatrices are disabled" >&2
  exit 1
fi
if [[ ! -x "$PROXY_BIN" ]]; then
  echo "missing Linux proxysss binary: $PROXY_BIN" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"
if [[ -n "$PREBUILT_BENCH_HELPER" ]]; then
  [[ -x "$PREBUILT_BENCH_HELPER" ]] || {
    echo "PREBUILT_BENCH_HELPER must be an executable Linux benchmark-helper binary" >&2
    exit 1
  }
else
  command -v go >/dev/null 2>&1 || {
    echo "missing required command: go (or set PREBUILT_BENCH_HELPER)" >&2
    exit 1
  }
  go build -o "$HELPER" "$ROOT/scripts/benchmark-helper.go"
fi
export PREBUILT_BENCH_HELPER="$HELPER"

run_order_for_iteration() {
  local iteration="$1"
  if (( iteration % 2 == 1 )); then
    printf '%s\n' "nginx proxysss"
  else
    printf '%s\n' "proxysss nginx"
  fi
}

# Four-run Latin square:
#   r1 AB low/high, r2 BA low/high, r3 AB high/low, r4 BA high/low.
# This makes gateway identity independent of both execution order and Linux
# flow/RSS hashing caused by a particular address tuple.
set_address_assignment_for_iteration() {
  local iteration="$1"
  if (( ((iteration - 1) / 2) % 2 == 0 )); then
    NGINX_BACKEND_BASE_IP="$BENCH_IP_PREFIX.10"
    NGINX_GATEWAY_IP="$BENCH_IP_PREFIX.20.20"
    NGINX_CLIENT_BASE_IP="$BENCH_IP_PREFIX.30"
    PROXYSSS_BACKEND_BASE_IP="$BENCH_IP_PREFIX.110"
    PROXYSSS_GATEWAY_IP="$BENCH_IP_PREFIX.120.20"
    PROXYSSS_CLIENT_BASE_IP="$BENCH_IP_PREFIX.130"
  else
    NGINX_BACKEND_BASE_IP="$BENCH_IP_PREFIX.110"
    NGINX_GATEWAY_IP="$BENCH_IP_PREFIX.120.20"
    NGINX_CLIENT_BASE_IP="$BENCH_IP_PREFIX.130"
    PROXYSSS_BACKEND_BASE_IP="$BENCH_IP_PREFIX.10"
    PROXYSSS_GATEWAY_IP="$BENCH_IP_PREFIX.20.20"
    PROXYSSS_CLIENT_BASE_IP="$BENCH_IP_PREFIX.30"
  fi
}

capacity_clients_for() {
  local connections="$1" clients=2
  while (( connections % clients != 0 || connections / clients > 25000 )); do
    clients=$((clients + 1))
  done
  printf '%s\n' "$clients"
}

summarize_scale() {
  local kind="$1" scale="$2" require_active="$3" require_capacity="$4"
  local gate_active_ops="${5:-true}" gate_active_latency="${6:-true}" min_active_ops="${7:-0}"
  local args=(
    summarize-isolated-wss
    --out-json "$OUTPUT_DIR/${kind}-${scale}.json"
    --out-markdown "$OUTPUT_DIR/${kind}-${scale}.md"
    --require-active="$require_active"
    --require-capacity="$require_capacity"
    --gate-active-ops="$gate_active_ops"
    --gate-active-latency="$gate_active_latency"
    --min-active-ops="$min_active_ops"
  )
  for iteration in $(seq 1 "$REPETITIONS"); do
    args+=(--run-dir "$BENCH_ROOT/runs/isolated-websocket/${RUN_ID}-${kind}-${scale}-r${iteration}")
  done
  if [[ "$STRICT_GATE" == "1" ]]; then
    args+=(--strict)
  fi
  "$HELPER" "${args[@]}" | tee "$OUTPUT_DIR/${kind}-${scale}.console.txt"
}

if [[ "$RUN_ACTIVE_MATRIX" == "1" ]]; then
for connections in $ACTIVE_SCALES; do
  if [[ "$RUN_SATURATION_MATRIX" == "1" ]]; then
  echo "==> active WSS scale: $connections connections"
  for iteration in $(seq 1 "$REPETITIONS"); do
    order="$(run_order_for_iteration "$iteration")"
    set_address_assignment_for_iteration "$iteration"
    RUN_ID="${RUN_ID}-active-${connections}-r${iteration}" \
    RUN_ORDER="$order" \
    NGINX_BACKEND_BASE_IP="$NGINX_BACKEND_BASE_IP" NGINX_GATEWAY_IP="$NGINX_GATEWAY_IP" NGINX_CLIENT_BASE_IP="$NGINX_CLIENT_BASE_IP" \
    PROXYSSS_BACKEND_BASE_IP="$PROXYSSS_BACKEND_BASE_IP" PROXYSSS_GATEWAY_IP="$PROXYSSS_GATEWAY_IP" PROXYSSS_CLIENT_BASE_IP="$PROXYSSS_CLIENT_BASE_IP" \
    RUN_ACTIVE=1 RUN_CAPACITY=0 STRICT_GATE=0 \
    ACTIVE_CONNECTIONS="$connections" \
    ACTIVE_DURATION_SECS="$ACTIVE_DURATION_SECS" \
    ACTIVE_PAYLOAD_BYTES="$ACTIVE_PAYLOAD_BYTES" \
    CLIENT_MEMORY="$ACTIVE_CLIENT_MEMORY" \
    PROXY_BIN="$PROXY_BIN" BENCH_ROOT="$BENCH_ROOT" \
      bash "$ROOT/scripts/benchmark-websocket-isolated.sh"
  done
  # Saturation mode compares maximum completed operations. Latency is not a
  # fair gate here because the faster gateway automatically receives a higher
  # closed-loop offered load.
  summarize_scale active "$connections" 1 0 true false 0
  fi

  if [[ "$RUN_LATENCY_MATRIX" == "1" ]]; then
    latency_target_ops="$LATENCY_TARGET_OPS"
    if (( connections >= LATENCY_LARGE_SCALE_MIN_CONNECTIONS )); then
      latency_target_ops="$LATENCY_LARGE_SCALE_TARGET_OPS"
    fi
    interval_micros=$(((connections * 1000000 + latency_target_ops - 1) / latency_target_ops))
    # A finite benchmark window can contain only an integer number of fixed
    # ticks. Gate against the achievable scheduled rate, not the continuous
    # ideal, then require every run to complete at least the declared percent.
    scheduled_ticks=$((ACTIVE_DURATION_SECS * 1000000 / interval_micros))
    scheduled_active_ops=$((scheduled_ticks * connections / ACTIVE_DURATION_SECS))
    min_active_ops=$((scheduled_active_ops * LATENCY_MIN_ACHIEVEMENT_PERCENT / 100))
    echo "==> equal-load WSS latency scale: $connections connections, continuous target ${latency_target_ops} ops/s, scheduled ${scheduled_active_ops} ops/s"
    for iteration in $(seq 1 "$REPETITIONS"); do
      order="$(run_order_for_iteration "$iteration")"
      set_address_assignment_for_iteration "$iteration"
      RUN_ID="${RUN_ID}-latency-${connections}-r${iteration}" \
      RUN_ORDER="$order" \
      NGINX_BACKEND_BASE_IP="$NGINX_BACKEND_BASE_IP" NGINX_GATEWAY_IP="$NGINX_GATEWAY_IP" NGINX_CLIENT_BASE_IP="$NGINX_CLIENT_BASE_IP" \
      PROXYSSS_BACKEND_BASE_IP="$PROXYSSS_BACKEND_BASE_IP" PROXYSSS_GATEWAY_IP="$PROXYSSS_GATEWAY_IP" PROXYSSS_CLIENT_BASE_IP="$PROXYSSS_CLIENT_BASE_IP" \
      RUN_ACTIVE=1 RUN_CAPACITY=0 STRICT_GATE=0 \
      ACTIVE_CONNECTIONS="$connections" \
      ACTIVE_DURATION_SECS="$ACTIVE_DURATION_SECS" \
      ACTIVE_PAYLOAD_BYTES="$ACTIVE_PAYLOAD_BYTES" \
      ACTIVE_MESSAGE_INTERVAL_MICROS="$interval_micros" \
      CLIENT_MEMORY="$ACTIVE_CLIENT_MEMORY" \
      PROXY_BIN="$PROXY_BIN" BENCH_ROOT="$BENCH_ROOT" \
        bash "$ROOT/scripts/benchmark-websocket-isolated.sh"
    done
    # Equal-load mode requires both gateways to sustain the declared rate;
    # then it gates all latency percentiles without demanding unequal ops.
    summarize_scale latency "$connections" 1 0 false true "$min_active_ops"
  fi
done
fi

if [[ "$RUN_CAPACITY_MATRIX" == "1" ]]; then
for connections in $CAPACITY_SCALES; do
  clients="$(capacity_clients_for "$connections")"
  echo "==> capacity WSS scale: $connections connections across $clients clients"
  for iteration in $(seq 1 "$REPETITIONS"); do
    order="$(run_order_for_iteration "$iteration")"
    set_address_assignment_for_iteration "$iteration"
    RUN_ID="${RUN_ID}-capacity-${connections}-r${iteration}" \
    RUN_ORDER="$order" \
    NGINX_BACKEND_BASE_IP="$NGINX_BACKEND_BASE_IP" NGINX_GATEWAY_IP="$NGINX_GATEWAY_IP" NGINX_CLIENT_BASE_IP="$NGINX_CLIENT_BASE_IP" \
    PROXYSSS_BACKEND_BASE_IP="$PROXYSSS_BACKEND_BASE_IP" PROXYSSS_GATEWAY_IP="$PROXYSSS_GATEWAY_IP" PROXYSSS_CLIENT_BASE_IP="$PROXYSSS_CLIENT_BASE_IP" \
    RUN_ACTIVE=0 RUN_CAPACITY=1 STRICT_GATE=0 \
    CAPACITY_CONNECTIONS="$connections" CAPACITY_CLIENTS="$clients" \
    CAPACITY_HOLD_SECS="$CAPACITY_HOLD_SECS" \
    CAPACITY_SETTLE_SECS="$CAPACITY_SETTLE_SECS" \
    CAPACITY_CONNECT_WORKERS="$CAPACITY_CONNECT_WORKERS" \
    CLIENT_MEMORY="$CAPACITY_CLIENT_MEMORY" \
    PROXY_BIN="$PROXY_BIN" BENCH_ROOT="$BENCH_ROOT" \
      bash "$ROOT/scripts/benchmark-websocket-isolated.sh"
  done
  summarize_scale capacity "$connections" 0 1 false false 0
done
fi

cat >"$OUTPUT_DIR/README.txt" <<EOF
run_id=$RUN_ID
repetitions=$REPETITIONS
active_scales=$ACTIVE_SCALES
capacity_scales=$CAPACITY_SCALES
active_payload_bytes=$ACTIVE_PAYLOAD_BYTES
latency_target_ops=$LATENCY_TARGET_OPS
latency_large_scale_min_connections=$LATENCY_LARGE_SCALE_MIN_CONNECTIONS
latency_large_scale_target_ops=$LATENCY_LARGE_SCALE_TARGET_OPS
latency_min_achievement_percent=$LATENCY_MIN_ACHIEVEMENT_PERCENT
tls_key_type=ecdsa
nginx_build=mainline-1.31.2-O3-fno-plt
gate=strict-median-saturation-throughput-equal-load-every-run-target-p50-p95-p99-capacity-zero-errors
role_isolation=docker-cgroup-cpuset-network-namespace
confounder_balance=four-run-latin-square-order-and-address-assignment
EOF

if [[ "$STRICT_GATE" == "1" ]]; then
  echo "production WSS scale gate passed: $OUTPUT_DIR"
else
  echo "production WSS scale matrix completed (strict gate disabled): $OUTPUT_DIR"
fi
