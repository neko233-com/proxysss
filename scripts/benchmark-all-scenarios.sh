#!/usr/bin/env bash
# Linux/Ubuntu production gateway benchmark matrix: proxysss vs nginx where nginx
# has an equivalent open-source core module path. The default release gate is a
# mixed multi-proxy load focused on game/WebSocket/TCP/UDP long-connection
# paths. Critical realtime paths use a fairness floor by default because
# proxysss carries built-in policy surfaces that nginx often needs modules or
# extra config to match. Static, reverse proxy, and generic SSE still run
# together with a soft floor; bulk/TLS static are diagnostic unless explicitly
# promoted to a gate. New API, KCP, and QCP protocol-specific wrappers are not
# part of this nginx-comparable benchmark matrix.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "benchmark-all-scenarios.sh is a Linux-only production performance gate" >&2
  echo "Run it on Ubuntu/Linux or inside the Ubuntu 24 benchmark container." >&2
  exit 1
fi

CONCURRENCY_SET="${CONCURRENCY+x}"
STATIC_LARGE_CONCURRENCY_SET="${STATIC_LARGE_CONCURRENCY+x}"
HTTPS_CONCURRENCY_SET="${HTTPS_CONCURRENCY+x}"
STREAM_CONNECTIONS_SET="${STREAM_CONNECTIONS+x}"
SSE_CONCURRENCY_SET="${SSE_CONCURRENCY+x}"
DURATION_SECS_SET="${DURATION_SECS+x}"
MIN_RATIO_SET="${MIN_RATIO+x}"
CRITICAL_RATIO_SET="${CRITICAL_RATIO+x}"
AGGREGATE_RATIO_SET="${AGGREGATE_RATIO+x}"
DIAGNOSTIC_SCENARIOS_SET="${DIAGNOSTIC_SCENARIOS+x}"
WEBSOCKET_ERROR_TOLERANCE_SET="${WEBSOCKET_ERROR_TOLERANCE+x}"
SSE_ERROR_TOLERANCE_SET="${SSE_ERROR_TOLERANCE+x}"
FAST_GATE_RATIO_SET="${FAST_GATE_RATIO+x}"
CPU_CORES="${CPU_CORES:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || nproc 2>/dev/null || echo 1)}"
CPU_CORES="${CPU_CORES:-1}"
if ! [[ "$CPU_CORES" =~ ^[1-9][0-9]*$ ]]; then
  echo "CPU_CORES must be a positive integer" >&2
  exit 1
fi
CONCURRENCY="${CONCURRENCY:-$((CPU_CORES * 16))}"
STATIC_LARGE_CONCURRENCY="${STATIC_LARGE_CONCURRENCY:-$CPU_CORES}"
HTTPS_CONCURRENCY="${HTTPS_CONCURRENCY:-$((CPU_CORES * 4))}"
STREAM_CONNECTIONS="${STREAM_CONNECTIONS:-$((CPU_CORES * 4))}"
SSE_CONCURRENCY="${SSE_CONCURRENCY:-$CPU_CORES}"
SSE_MAX_CHUNKS="${SSE_MAX_CHUNKS:-1}"
DURATION_SECS="${DURATION_SECS:-30}"
UDP_TIMEOUT_MS="${UDP_TIMEOUT_MS:-7000}"
NGINX_VERSION="${NGINX_VERSION:-1.31.2}"
QUICK="${QUICK:-0}"
HTTPS_HTTP1_ONLY="${HTTPS_HTTP1_ONLY:-0}"
MIN_RATIO="${MIN_RATIO:-0.50}"
CRITICAL_RATIO="${CRITICAL_RATIO:-0.97}"
CRITICAL_SCENARIOS="${CRITICAL_SCENARIOS:-websocket-long-connection game-long-connection tcp-stream udp-stream}"
DIAGNOSTIC_SCENARIOS="${DIAGNOSTIC_SCENARIOS:-static-large https-static-small}"
WEBSOCKET_ERROR_TOLERANCE="${WEBSOCKET_ERROR_TOLERANCE:-4}"
SSE_ERROR_TOLERANCE="${SSE_ERROR_TOLERANCE:-1}"
UDP_ERROR_TOLERANCE_SET="${UDP_ERROR_TOLERANCE+x}"
UDP_ERROR_TOLERANCE="${UDP_ERROR_TOLERANCE:-4}"
FAST_GATE="${FAST_GATE:-0}"
FAST_GATE_RATIO="${FAST_GATE_RATIO:-$CRITICAL_RATIO}"
FAST_GATE_CONCURRENCY="${FAST_GATE_CONCURRENCY:-$((CPU_CORES * 16))}"
FAST_GATE_HTTPS_CONCURRENCY="${FAST_GATE_HTTPS_CONCURRENCY:-$((CPU_CORES * 4))}"
FAST_GATE_STATIC_LARGE_CONCURRENCY="${FAST_GATE_STATIC_LARGE_CONCURRENCY:-$CPU_CORES}"
FAST_GATE_STREAM_CONNECTIONS="${FAST_GATE_STREAM_CONNECTIONS:-$((CPU_CORES * 4))}"
FAST_GATE_SSE_CONCURRENCY="${FAST_GATE_SSE_CONCURRENCY:-$CPU_CORES}"
FAST_GATE_DURATION_SECS="${FAST_GATE_DURATION_SECS:-4}"
FAST_GATE_SCENARIOS="${FAST_GATE_SCENARIOS:-$CRITICAL_SCENARIOS}"
SCENARIO_FILTER="${SCENARIO_FILTER:-}"
MIXED_MATRIX="${MIXED_MATRIX:-1}"
AGGREGATE_RATIO="${AGGREGATE_RATIO:-0.97}"
STRICT_SUPERIORITY="${STRICT_SUPERIORITY:-0}"
MAX_LATENCY_RATIO="${MAX_LATENCY_RATIO:-1.0}"
REQUIRE_LATENCY_PERCENTILES="${REQUIRE_LATENCY_PERCENTILES:-$STRICT_SUPERIORITY}"
REQUIRE_ZERO_ERRORS="${REQUIRE_ZERO_ERRORS:-$STRICT_SUPERIORITY}"
GATE_LATENCY="${GATE_LATENCY:-0}"
TRAFFIC_PROFILE="${TRAFFIC_PROFILE:-small}"

case "$TRAFFIC_PROFILE" in
  small|balanced|bulk) ;;
  *)
    echo "TRAFFIC_PROFILE must be small, balanced, or bulk" >&2
    exit 1
    ;;
esac

if [[ "$STRICT_SUPERIORITY" == "1" ]]; then
  [[ -z "$MIN_RATIO_SET" ]] && MIN_RATIO=1.0
  [[ -z "$CRITICAL_RATIO_SET" ]] && CRITICAL_RATIO=1.0
  [[ -z "$AGGREGATE_RATIO_SET" ]] && AGGREGATE_RATIO=1.0
  [[ -z "$DIAGNOSTIC_SCENARIOS_SET" ]] && DIAGNOSTIC_SCENARIOS=""
  [[ -z "$WEBSOCKET_ERROR_TOLERANCE_SET" ]] && WEBSOCKET_ERROR_TOLERANCE=0
  [[ -z "$SSE_ERROR_TOLERANCE_SET" ]] && SSE_ERROR_TOLERANCE=0
  [[ -z "$UDP_ERROR_TOLERANCE_SET" ]] && UDP_ERROR_TOLERANCE=0
  [[ -z "$FAST_GATE_RATIO_SET" ]] && FAST_GATE_RATIO=1.0
fi

if [[ "$QUICK" == "1" ]]; then
  [[ -z "$CONCURRENCY_SET" ]] && CONCURRENCY=$((CPU_CORES * 16))
  [[ -z "$STATIC_LARGE_CONCURRENCY_SET" ]] && STATIC_LARGE_CONCURRENCY=$CPU_CORES
  [[ -z "$HTTPS_CONCURRENCY_SET" ]] && HTTPS_CONCURRENCY=$((CPU_CORES * 4))
  [[ -z "$STREAM_CONNECTIONS_SET" ]] && STREAM_CONNECTIONS=$((CPU_CORES * 4))
  [[ -z "$SSE_CONCURRENCY_SET" ]] && SSE_CONCURRENCY=$CPU_CORES
  [[ -z "$DURATION_SECS_SET" ]] && DURATION_SECS=10
fi

if [[ "$REQUIRE_ZERO_ERRORS" != "1" && -z "$UDP_ERROR_TOLERANCE_SET" && ! -e /proc/sys/net/core/rmem_max ]]; then
  UDP_ERROR_TOLERANCE=16
  echo "==> /proc/sys/net/core/rmem_max is unavailable; using Docker/WSL UDP error tolerance +$UDP_ERROR_TOLERANCE" >&2
fi

BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
VENDOR_DIR="$BENCH_ROOT/vendors"
RUN_DIR="$BENCH_ROOT/runs/all-scenarios"
WWW_DIR="$RUN_DIR/www"
PROXY_DIR="$RUN_DIR/proxysss"
BENCH_HELPER_SRC="$ROOT/scripts/benchmark-helper.go"
BENCH_HELPER_BIN="$RUN_DIR/benchmark-helper"
PID_FILE="$RUN_DIR/pids.txt"
NGINX_PID_FILE="$RUN_DIR/nginx.pid"
RESULTS_FILE="$RUN_DIR/results.json"
SUMMARY_MD="$RUN_DIR/summary.md"
SUMMARY_HTML="$RUN_DIR/summary.html"
NGINX_START_LOG="$RUN_DIR/nginx-start.log"
BUILD_PROFILE_WAS_SET="${BUILD_PROFILE+x}"
BUILD_PROFILE="${BUILD_PROFILE:-release}"
if [[ "$QUICK" == "1" && -z "$BUILD_PROFILE_WAS_SET" ]]; then
  BUILD_PROFILE="release-fast"
fi
DEFAULT_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
PROXY_BIN="${PROXY_BIN:-$DEFAULT_TARGET_DIR/$BUILD_PROFILE/proxysss}"
FORCE_BUILD="${FORCE_BUILD:-0}"

stop_bench_processes() {
  if [[ -f "$PID_FILE" ]]; then
    while read -r pid; do
      if ! [[ "$pid" =~ ^[1-9][0-9]*$ ]] || [[ ! -r "/proc/$pid/exe" ]]; then
        continue
      fi
      local executable command_line proxy_executable helper_executable
      executable="$(readlink -f "/proc/$pid/exe" 2>/dev/null || true)"
      command_line="$(tr '\0' ' ' <"/proc/$pid/cmdline" 2>/dev/null || true)"
      proxy_executable="$(readlink -f "$PROXY_BIN" 2>/dev/null || true)"
      helper_executable="$(readlink -f "$BENCH_HELPER_BIN" 2>/dev/null || true)"
      if [[ ( -n "$proxy_executable" && "$executable" == "$proxy_executable" ) \
        || ( -n "$helper_executable" && "$executable" == "$helper_executable" ) \
        || "$command_line" == *"$RUN_DIR/nginx.conf"* ]]; then
        kill "$pid" 2>/dev/null || true
      fi
    done < "$PID_FILE"
    rm -f "$PID_FILE"
  fi
  pkill -f "$RUN_DIR/nginx.conf" 2>/dev/null || true
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_cmd go
require_cmd curl
require_cmd tar
require_cmd make
require_cmd openssl

if [[ "$FORCE_BUILD" == "1" || ! -x "$PROXY_BIN" ]]; then
  cargo build --profile "$BUILD_PROFILE" --locked
fi

stop_bench_processes
rm -rf "$RUN_DIR"
mkdir -p "$VENDOR_DIR" "$WWW_DIR" "$PROXY_DIR"
: >"$PID_FILE"
go build -o "$BENCH_HELPER_BIN" "$BENCH_HELPER_SRC"

NGINX_TARBALL="$VENDOR_DIR/nginx-$NGINX_VERSION.tar.gz"
NGINX_PREFIX="$VENDOR_DIR/nginx-$NGINX_VERSION-all-scenarios"
NGINX_BIN="$NGINX_PREFIX/sbin/nginx"
if [[ -x "$NGINX_BIN" ]] && ! "$NGINX_BIN" -V 2>&1 | grep -q -- '--with-http_ssl_module'; then
  echo "==> cached nginx $NGINX_VERSION lacks http_ssl_module; rebuilding"
  rm -rf "$NGINX_PREFIX"
fi
if [[ ! -x "$NGINX_BIN" ]]; then
  echo "==> building nginx $NGINX_VERSION with http + stream modules into $NGINX_PREFIX"
  if [[ ! -f "$NGINX_TARBALL" ]]; then
    curl -fsSL "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" -o "$NGINX_TARBALL"
  fi
  BUILD_DIR="$VENDOR_DIR/nginx-build-$NGINX_VERSION-all-scenarios"
  rm -rf "$BUILD_DIR"
  mkdir -p "$BUILD_DIR"
  tar -xzf "$NGINX_TARBALL" -C "$BUILD_DIR" --strip-components=1
  (
    cd "$BUILD_DIR"
    ./configure \
      --prefix="$NGINX_PREFIX" \
      --with-http_ssl_module \
      --with-http_v2_module \
      --with-stream \
      --with-stream_ssl_preread_module \
      --with-threads \
      --with-file-aio \
      --without-http_rewrite_module \
      --with-cc-opt='-O3 -fno-plt' \
      --with-ld-opt='-Wl,-O1,--as-needed' >/dev/null
    make -j"$(nproc)" >/dev/null
    make install >/dev/null
  )
  rm -rf "$BUILD_DIR"
fi

cat >"$WWW_DIR/small.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>small bench</title></head><body><h1>proxysss small static benchmark</h1><p>same payload for proxysss and nginx.</p></body></html>
HTML
printf 'hot-update-v1\n' >"$WWW_DIR/hot.dat"
"$BENCH_HELPER_BIN" write-large-file --path "$WWW_DIR/large.bin"

mkdir -p "$RUN_DIR/certs"
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$RUN_DIR/certs/bench.key" \
  -out "$RUN_DIR/certs/bench.crt" \
  -subj "/CN=localhost" \
  -days 1 >/dev/null 2>&1

cat >"$PROXY_DIR/proxysss.yaml" <<YAML
config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:18083
  tls_bind: 127.0.0.1:18443
  h3_bind: ''
  tls:
    mode: manual
    cert_path: '$RUN_DIR/certs/bench.crt'
    key_path: '$RUN_DIR/certs/bench.key'
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
    traffic_profile: $TRAFFIC_PROFILE
    adaptive_system: true
    socket_extreme: true
    log_on_start: true
  hot_reload:
    enabled: false
affinity:
  enabled: false
load_balance:
  retries:
    enabled: false
  passive_health:
    enabled: false
  active_health:
    enabled: false
services:
  reverse_proxy:
    routes:
      - name: http-echo
        path_prefix: /proxy
        upstream: http://127.0.0.1:18190
        strip_prefix: true
        forward_headers: false
      - name: websocket-echo
        path_prefix: /ws
        upstream: ws://127.0.0.1:18192
        forward_headers: false
      - name: generic-sse
        path_prefix: /sse
        upstream: http://127.0.0.1:18191
        forward_headers: false
  static_sites:
    - name: bench
      path_prefix: /bench
      root: '$WWW_DIR'
      index_files: [small.html]
      autoindex: false
tcp:
  listeners:
    - name: tcp-echo
      bind: 127.0.0.1:18200
      upstream: 127.0.0.1:18201
      nodelay: true
      connect_timeout_ms: 1000
udp:
  listeners:
    - name: udp-echo
      bind: 127.0.0.1:18300
      upstream: 127.0.0.1:18301
      session_ttl_secs: 30
      max_associations: 65536
YAML

"$PROXY_BIN" -config "$PROXY_DIR/proxysss.yaml" check-config

cat >"$RUN_DIR/nginx.conf" <<NGINX
user www-data;
worker_processes  ${CPU_CORES};
worker_rlimit_nofile 1048576;
events {
    use epoll;
    worker_connections  65535;
    multi_accept on;
}
http {
    access_log off;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    upstream http_echo {
        server 127.0.0.1:18190;
        keepalive 128;
    }
    upstream generic_sse {
        server 127.0.0.1:18191;
        keepalive 128;
    }
    upstream ws_echo {
        server 127.0.0.1:18192;
        keepalive 128;
    }
    server {
        listen 127.0.0.1:18081 backlog=65536 reuseport;
        location /bench/ {
            alias $WWW_DIR/;
            index small.html;
        }
        location /proxy/ {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host \$host;
            proxy_pass http://http_echo/;
        }
        location /sse {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_buffering off;
            proxy_pass http://generic_sse/sse;
        }
        location /ws/ {
            proxy_http_version 1.1;
            proxy_set_header Upgrade \$http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host \$host;
            proxy_pass http://ws_echo;
        }
    }
    server {
        listen 127.0.0.1:18441 ssl backlog=65536 reuseport;
        http2 on;
        ssl_certificate $RUN_DIR/certs/bench.crt;
        ssl_certificate_key $RUN_DIR/certs/bench.key;
        location /bench/ {
            alias $WWW_DIR/;
            index small.html;
        }
        location /proxy/ {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host \$host;
            proxy_pass http://http_echo/;
        }
        location /sse {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_buffering off;
            proxy_pass http://generic_sse/sse;
        }
        location /ws/ {
            proxy_http_version 1.1;
            proxy_set_header Upgrade \$http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host \$host;
            proxy_pass http://ws_echo;
        }
    }
}
stream {
    upstream tcp_echo {
        server 127.0.0.1:18201;
    }
    server {
        listen 127.0.0.1:18202 backlog=65536 reuseport;
        proxy_pass tcp_echo;
        proxy_connect_timeout 1s;
        proxy_timeout 30s;
        tcp_nodelay on;
    }
    upstream udp_echo {
        server 127.0.0.1:18301;
    }
    server {
        listen 127.0.0.1:18302 udp reuseport;
        proxy_pass udp_echo;
        proxy_responses 1;
        proxy_timeout 30s;
    }
}
NGINX

wait_http() {
  local url="$1"
  for _ in $(seq 1 120); do
    if curl -kfsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  echo "not ready: $url" >&2
  return 1
}

parse_bench_output() {
  local scenario="$1" gateway="$2" protocol="$3" target="$4" bench_concurrency="$5" output="$6"
  printf '%s' "$output" | "$BENCH_HELPER_BIN" parse-bench \
    --scenario "$scenario" \
    --gateway "$gateway" \
    --protocol "$protocol" \
    --target "$target" \
    --concurrency "$bench_concurrency" \
    --duration "$DURATION_SECS"
}

run_http_bench() {
  local scenario="$1" gateway="$2" url="$3"
  local bench_concurrency="$CONCURRENCY"
  if [[ "$scenario" == "static-large" ]]; then
    bench_concurrency="$STATIC_LARGE_CONCURRENCY"
  elif [[ "$scenario" == "https-static-small" ]]; then
    bench_concurrency="$HTTPS_CONCURRENCY"
  fi
  echo "" >&2
  echo "=== $scenario / $gateway HTTP c$bench_concurrency d${DURATION_SECS}s ===" >&2
  local output
  if [[ "$url" == https://* ]]; then
    local tls_args=(--insecure)
    if [[ "$HTTPS_HTTP1_ONLY" == "1" ]]; then
      tls_args+=(--http1-only)
    fi
    output="$("$PROXY_BIN" bench http --url "$url" --concurrency "$bench_concurrency" --duration-secs "$DURATION_SECS" "${tls_args[@]}" 2>&1)"
  else
    output="$("$PROXY_BIN" bench http --url "$url" --concurrency "$bench_concurrency" --duration-secs "$DURATION_SECS" 2>&1)"
  fi
  echo "$output" >&2
  parse_bench_output "$scenario" "$gateway" "http" "$url" "$bench_concurrency" "$output"
}

run_sse_bench() {
  local scenario="$1" gateway="$2" url="$3"
  echo "" >&2
  echo "=== $scenario / $gateway SSE c$SSE_CONCURRENCY d${DURATION_SECS}s ===" >&2
  local output
  if [[ "$url" == https://* ]]; then
    output="$("$PROXY_BIN" bench sse --url "$url" --concurrency "$SSE_CONCURRENCY" --duration-secs "$DURATION_SECS" --max-chunks "$SSE_MAX_CHUNKS" --insecure 2>&1)"
  else
    output="$("$PROXY_BIN" bench sse --url "$url" --concurrency "$SSE_CONCURRENCY" --duration-secs "$DURATION_SECS" --max-chunks "$SSE_MAX_CHUNKS" 2>&1)"
  fi
  echo "$output" >&2
  parse_bench_output "$scenario" "$gateway" "sse" "$url" "$SSE_CONCURRENCY" "$output"
}

run_websocket_bench() {
  local scenario="$1" gateway="$2" url="$3" payload_bytes="${4:-256}"
  echo "" >&2
  echo "=== $scenario / $gateway WebSocket c$STREAM_CONNECTIONS d${DURATION_SECS}s ===" >&2
  local output
  output="$("$PROXY_BIN" bench websocket --url "$url" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes "$payload_bytes" 2>&1)"
  echo "$output" >&2
  parse_bench_output "$scenario" "$gateway" "websocket" "$url" "$STREAM_CONNECTIONS" "$output"
}

run_tcp_bench() {
  local scenario="$1" gateway="$2" addr="$3" payload_bytes="${4:-1024}"
  echo "" >&2
  echo "=== $scenario / $gateway TCP c$STREAM_CONNECTIONS d${DURATION_SECS}s ===" >&2
  local output
  output="$("$PROXY_BIN" bench tcp --addr "$addr" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes "$payload_bytes" 2>&1)"
  echo "$output" >&2
  parse_bench_output "$scenario" "$gateway" "tcp" "$addr" "$STREAM_CONNECTIONS" "$output"
}

run_udp_bench() {
  local scenario="$1" gateway="$2" addr="$3" payload_bytes="${4:-512}"
  echo "" >&2
  echo "=== $scenario / $gateway UDP c$STREAM_CONNECTIONS d${DURATION_SECS}s ===" >&2
  local output
  output="$("$PROXY_BIN" bench udp --addr "$addr" --connections "$STREAM_CONNECTIONS" --duration-secs "$DURATION_SECS" --payload-bytes "$payload_bytes" --timeout-ms "$UDP_TIMEOUT_MS" 2>&1)"
  echo "$output" >&2
  parse_bench_output "$scenario" "$gateway" "udp" "$addr" "$STREAM_CONNECTIONS" "$output"
}

scenario_enabled() {
  local scenario="$1"
  [[ -z "$SCENARIO_FILTER" || " $SCENARIO_FILTER " == *" $scenario "* ]]
}

SSE_UPSTREAM_PID=""

start_sse_backend() {
  "$BENCH_HELPER_BIN" serve-sse --listen 127.0.0.1:18191 --chunks 1 >/dev/null 2>&1 &
  SSE_UPSTREAM_PID="$!"
  echo "$SSE_UPSTREAM_PID" >>"$PID_FILE"
  wait_http "http://127.0.0.1:18191/sse"
}

restart_sse_backend() {
  if [[ -n "$SSE_UPSTREAM_PID" ]]; then
    kill "$SSE_UPSTREAM_PID" 2>/dev/null || true
    wait "$SSE_UPSTREAM_PID" 2>/dev/null || true
  fi
  start_sse_backend
}

warm_sse_gateway() {
  local url="$1"
  for _ in 1 2; do
    timeout 8 curl -fsS --no-buffer "$url" >/dev/null 2>&1 || true
  done
}

warm_ws_gateway() {
  local url="$1"
  "$PROXY_BIN" bench websocket --url "$url" --connections 1 --duration-secs 1 --payload-bytes 16 >/dev/null 2>&1 || true
}

warm_tcp_gateway() {
  local addr="$1"
  local payload_bytes="${2:-256}"
  "$PROXY_BIN" bench tcp --addr "$addr" --connections 1 --duration-secs 1 --payload-bytes "$payload_bytes" >/dev/null 2>&1 || true
}

warm_udp_gateway() {
  local addr="$1"
  local payload_bytes="${2:-512}"
  "$PROXY_BIN" bench udp --addr "$addr" --connections 1 --duration-secs 1 --payload-bytes "$payload_bytes" --timeout-ms "$UDP_TIMEOUT_MS" >/dev/null 2>&1 || true
}

trap 'stop_bench_processes' EXIT

"$PROXY_BIN" demo http-echo --listen 127.0.0.1:18190 >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
start_sse_backend
"$PROXY_BIN" demo ws-echo --listen 127.0.0.1:18192 >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
"$PROXY_BIN" demo tcp-echo --listen 127.0.0.1:18201 >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
"$PROXY_BIN" demo udp-echo --listen 127.0.0.1:18301 >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
"$PROXY_BIN" -config "$PROXY_DIR/proxysss.yaml" >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
"$NGINX_BIN" -p "$NGINX_PREFIX/" -c "$RUN_DIR/nginx.conf" -g "pid $NGINX_PID_FILE; error_log $RUN_DIR/nginx-error.log notice;" >"$NGINX_START_LOG" 2>&1
for _ in $(seq 1 40); do
  if [[ -f "$NGINX_PID_FILE" ]]; then
    cat "$NGINX_PID_FILE" >>"$PID_FILE"
    break
  fi
  sleep 0.1
done
if [[ ! -f "$NGINX_PID_FILE" ]]; then
  echo "nginx did not create pid file: $NGINX_PID_FILE" >&2
  "$NGINX_BIN" -p "$NGINX_PREFIX/" -t -c "$RUN_DIR/nginx.conf" >&2 || true
  cat "$NGINX_START_LOG" >&2 || true
  cat "$RUN_DIR/nginx-error.log" >&2 || true
  exit 1
fi

wait_http "http://127.0.0.1:18083/bench/small.html"
wait_http "http://127.0.0.1:18081/bench/small.html"
wait_http "http://127.0.0.1:18083/bench/large.bin"
wait_http "http://127.0.0.1:18081/bench/large.bin"
wait_http "http://127.0.0.1:18083/proxy/ping"
wait_http "http://127.0.0.1:18081/proxy/ping"
wait_http "https://127.0.0.1:18443/bench/small.html"
wait_http "https://127.0.0.1:18441/bench/small.html"
wait_http "http://127.0.0.1:18083/sse"
wait_http "http://127.0.0.1:18081/sse"
for _ in 1 2; do
  timeout 8 curl -fsS --no-buffer "http://127.0.0.1:18083/sse" >/dev/null 2>&1 || true
  timeout 8 curl -fsS --no-buffer "http://127.0.0.1:18081/sse" >/dev/null 2>&1 || true
done
warm_ws_gateway "ws://127.0.0.1:18083/ws/"
warm_ws_gateway "ws://127.0.0.1:18081/ws/"
warm_tcp_gateway "127.0.0.1:18200" 256
warm_tcp_gateway "127.0.0.1:18202" 256
warm_tcp_gateway "127.0.0.1:18200" 1024
warm_tcp_gateway "127.0.0.1:18202" 1024
warm_udp_gateway "127.0.0.1:18300" 512
warm_udp_gateway "127.0.0.1:18302" 512
warm_udp_gateway "127.0.0.1:18300" 1200
warm_udp_gateway "127.0.0.1:18302" 1200

sleep 2
printf 'hot-update-v2\n' >"$WWW_DIR/hot.dat"
for url in http://127.0.0.1:18083/bench/hot.dat http://127.0.0.1:18081/bench/hot.dat; do
  for _ in $(seq 1 40); do
    if curl -fsS "$url" | grep -q 'hot-update-v2'; then
      break
    fi
    sleep 0.25
  done
done

RESULT_ROWS=()
if [[ "$FAST_GATE" == "1" ]]; then
  echo ""
  echo "=== quick gate before deep benchmark: ratio >= ${FAST_GATE_RATIO}x, c${FAST_GATE_CONCURRENCY}, https-c${HTTPS_CONCURRENCY}, large-c${FAST_GATE_STATIC_LARGE_CONCURRENCY}, sse-c${FAST_GATE_SSE_CONCURRENCY}, stream-c${FAST_GATE_STREAM_CONNECTIONS}, d${FAST_GATE_DURATION_SECS}s ===" >&2
  SAVED_CONCURRENCY="$CONCURRENCY"
  SAVED_STATIC_LARGE_CONCURRENCY="$STATIC_LARGE_CONCURRENCY"
  SAVED_HTTPS_CONCURRENCY="$HTTPS_CONCURRENCY"
  SAVED_STREAM_CONNECTIONS="$STREAM_CONNECTIONS"
  SAVED_SSE_CONCURRENCY="$SSE_CONCURRENCY"
  SAVED_DURATION_SECS="$DURATION_SECS"
  CONCURRENCY="$FAST_GATE_CONCURRENCY"
  STATIC_LARGE_CONCURRENCY="$FAST_GATE_STATIC_LARGE_CONCURRENCY"
  HTTPS_CONCURRENCY="${FAST_GATE_HTTPS_CONCURRENCY:-$HTTPS_CONCURRENCY}"
  STREAM_CONNECTIONS="$FAST_GATE_STREAM_CONNECTIONS"
  SSE_CONCURRENCY="$FAST_GATE_SSE_CONCURRENCY"
  DURATION_SECS="$FAST_GATE_DURATION_SECS"
  QUICK_ROWS=()
  for scenario in $FAST_GATE_SCENARIOS; do
    case "$scenario" in
      static-small)
        QUICK_ROWS+=("$(run_http_bench static-small nginx http://127.0.0.1:18081/bench/small.html)")
        QUICK_ROWS+=("$(run_http_bench static-small proxysss http://127.0.0.1:18083/bench/small.html)")
        ;;
      static-large)
        QUICK_ROWS+=("$(run_http_bench static-large nginx http://127.0.0.1:18081/bench/large.bin)")
        QUICK_ROWS+=("$(run_http_bench static-large proxysss http://127.0.0.1:18083/bench/large.bin)")
        ;;
      reverse-proxy)
        QUICK_ROWS+=("$(run_http_bench reverse-proxy nginx http://127.0.0.1:18081/proxy/ping)")
        QUICK_ROWS+=("$(run_http_bench reverse-proxy proxysss http://127.0.0.1:18083/proxy/ping)")
        ;;
      https-static-small)
        QUICK_ROWS+=("$(run_http_bench https-static-small nginx https://127.0.0.1:18441/bench/small.html)")
        QUICK_ROWS+=("$(run_http_bench https-static-small proxysss https://127.0.0.1:18443/bench/small.html)")
        ;;
      generic-sse)
        QUICK_ROWS+=("$(run_sse_bench generic-sse nginx http://127.0.0.1:18081/sse)")
        restart_sse_backend
        warm_sse_gateway "http://127.0.0.1:18083/sse"
        QUICK_ROWS+=("$(run_sse_bench generic-sse proxysss http://127.0.0.1:18083/sse)")
        ;;
      websocket-long-connection)
        QUICK_ROWS+=("$(run_websocket_bench websocket-long-connection nginx ws://127.0.0.1:18081/ws/ 256)")
        QUICK_ROWS+=("$(run_websocket_bench websocket-long-connection proxysss ws://127.0.0.1:18083/ws/ 256)")
        ;;
      tcp-stream)
        QUICK_ROWS+=("$(run_tcp_bench tcp-stream nginx 127.0.0.1:18202 1024)")
        QUICK_ROWS+=("$(run_tcp_bench tcp-stream proxysss 127.0.0.1:18200 1024)")
        ;;
      game-long-connection)
        QUICK_ROWS+=("$(run_tcp_bench game-long-connection nginx 127.0.0.1:18202 256)")
        QUICK_ROWS+=("$(run_tcp_bench game-long-connection proxysss 127.0.0.1:18200 256)")
        ;;
      *)
        echo "unknown FAST_GATE_SCENARIOS item: $scenario" >&2
        exit 1
        ;;
    esac
  done
  CONCURRENCY="$SAVED_CONCURRENCY"
  STATIC_LARGE_CONCURRENCY="$SAVED_STATIC_LARGE_CONCURRENCY"
  HTTPS_CONCURRENCY="$SAVED_HTTPS_CONCURRENCY"
  STREAM_CONNECTIONS="$SAVED_STREAM_CONNECTIONS"
  SSE_CONCURRENCY="$SAVED_SSE_CONCURRENCY"
  DURATION_SECS="$SAVED_DURATION_SECS"

  QUICK_ROWS_FILE="$RUN_DIR/quick-rows.jsonl"
  printf '%s\n' "${QUICK_ROWS[@]}" >"$QUICK_ROWS_FILE"
  "$BENCH_HELPER_BIN" quick-gate \
    --min-ratio "$FAST_GATE_RATIO" \
    --max-latency-ratio "$MAX_LATENCY_RATIO" \
    --require-latency-percentiles="$REQUIRE_LATENCY_PERCENTILES" \
    --require-zero-errors="$REQUIRE_ZERO_ERRORS" \
    --strict-superiority="$STRICT_SUPERIORITY" \
    --rows "$QUICK_ROWS_FILE"
fi

append_deep_matrix_serial() {
  if scenario_enabled static-small; then
    RESULT_ROWS+=("$(run_http_bench static-small nginx http://127.0.0.1:18081/bench/small.html)")
    RESULT_ROWS+=("$(run_http_bench static-small proxysss http://127.0.0.1:18083/bench/small.html)")
  fi
  if scenario_enabled static-large; then
    RESULT_ROWS+=("$(run_http_bench static-large nginx http://127.0.0.1:18081/bench/large.bin)")
    RESULT_ROWS+=("$(run_http_bench static-large proxysss http://127.0.0.1:18083/bench/large.bin)")
  fi
  if scenario_enabled cdn-hot-update; then
    RESULT_ROWS+=("$(run_http_bench cdn-hot-update nginx http://127.0.0.1:18081/bench/hot.dat)")
    RESULT_ROWS+=("$(run_http_bench cdn-hot-update proxysss http://127.0.0.1:18083/bench/hot.dat)")
  fi
  if scenario_enabled https-static-small; then
    RESULT_ROWS+=("$(run_http_bench https-static-small nginx https://127.0.0.1:18441/bench/small.html)")
    RESULT_ROWS+=("$(run_http_bench https-static-small proxysss https://127.0.0.1:18443/bench/small.html)")
  fi
  if scenario_enabled reverse-proxy; then
    RESULT_ROWS+=("$(run_http_bench reverse-proxy nginx http://127.0.0.1:18081/proxy/ping)")
    RESULT_ROWS+=("$(run_http_bench reverse-proxy proxysss http://127.0.0.1:18083/proxy/ping)")
  fi
  if scenario_enabled generic-sse; then
    RESULT_ROWS+=("$(run_sse_bench generic-sse nginx http://127.0.0.1:18081/sse)")
    restart_sse_backend
    warm_sse_gateway "http://127.0.0.1:18083/sse"
    RESULT_ROWS+=("$(run_sse_bench generic-sse proxysss http://127.0.0.1:18083/sse)")
  fi
  if scenario_enabled websocket-long-connection; then
    RESULT_ROWS+=("$(run_websocket_bench websocket-long-connection nginx ws://127.0.0.1:18081/ws/ 256)")
    RESULT_ROWS+=("$(run_websocket_bench websocket-long-connection proxysss ws://127.0.0.1:18083/ws/ 256)")
  fi
  if scenario_enabled game-long-connection; then
    RESULT_ROWS+=("$(run_tcp_bench game-long-connection nginx 127.0.0.1:18202 256)")
    RESULT_ROWS+=("$(run_tcp_bench game-long-connection proxysss 127.0.0.1:18200 256)")
  fi
  if scenario_enabled tcp-stream; then
    RESULT_ROWS+=("$(run_tcp_bench tcp-stream nginx 127.0.0.1:18202 1024)")
    RESULT_ROWS+=("$(run_tcp_bench tcp-stream proxysss 127.0.0.1:18200 1024)")
  fi
  if scenario_enabled udp-stream; then
    RESULT_ROWS+=("$(run_udp_bench udp-stream nginx 127.0.0.1:18302)")
    RESULT_ROWS+=("$(run_udp_bench udp-stream proxysss 127.0.0.1:18300)")
  fi
}

append_deep_matrix_mixed() {
  local row_dir="$RUN_DIR/mixed-rows"
  rm -rf "$row_dir"
  mkdir -p "$row_dir"
  local pids=()
  local ordered=(
    static-small-nginx static-small-proxysss
    static-large-nginx static-large-proxysss
    cdn-hot-update-nginx cdn-hot-update-proxysss
    https-static-small-nginx https-static-small-proxysss
    reverse-proxy-nginx reverse-proxy-proxysss
    generic-sse-nginx generic-sse-proxysss
    websocket-long-connection-nginx websocket-long-connection-proxysss
    game-long-connection-nginx game-long-connection-proxysss
    tcp-stream-nginx tcp-stream-proxysss
    udp-stream-nginx udp-stream-proxysss
  )

  launch_bench() {
    local name="$1"
    shift
    ("$@" >"$row_dir/$name.json") &
    pids+=("$!")
  }

  launch_scenario_bench() {
    local scenario="$1"
    shift
    if scenario_enabled "$scenario"; then
      launch_bench "$@"
    fi
  }

  wait_for_group() {
    local failed=0
    local pid
    for pid in "${pids[@]}"; do
      if ! wait "$pid"; then
        failed=1
      fi
    done
    pids=()
    if [[ "$failed" != "0" ]]; then
      echo "mixed benchmark worker failed" >&2
      exit 1
    fi
  }

  echo "=== mixed matrix: nginx scenarios running concurrently ===" >&2
  launch_scenario_bench static-small static-small-nginx run_http_bench static-small nginx http://127.0.0.1:18081/bench/small.html
  launch_scenario_bench static-large static-large-nginx run_http_bench static-large nginx http://127.0.0.1:18081/bench/large.bin
  launch_scenario_bench cdn-hot-update cdn-hot-update-nginx run_http_bench cdn-hot-update nginx http://127.0.0.1:18081/bench/hot.dat
  launch_scenario_bench https-static-small https-static-small-nginx run_http_bench https-static-small nginx https://127.0.0.1:18441/bench/small.html
  launch_scenario_bench reverse-proxy reverse-proxy-nginx run_http_bench reverse-proxy nginx http://127.0.0.1:18081/proxy/ping
  launch_scenario_bench generic-sse generic-sse-nginx run_sse_bench generic-sse nginx http://127.0.0.1:18081/sse
  launch_scenario_bench websocket-long-connection websocket-long-connection-nginx run_websocket_bench websocket-long-connection nginx ws://127.0.0.1:18081/ws/ 256
  launch_scenario_bench game-long-connection game-long-connection-nginx run_tcp_bench game-long-connection nginx 127.0.0.1:18202 256
  launch_scenario_bench tcp-stream tcp-stream-nginx run_tcp_bench tcp-stream nginx 127.0.0.1:18202 1024
  launch_scenario_bench udp-stream udp-stream-nginx run_udp_bench udp-stream nginx 127.0.0.1:18302
  wait_for_group
  restart_sse_backend
  warm_sse_gateway "http://127.0.0.1:18083/sse"

  echo "=== mixed matrix: proxysss scenarios running concurrently ===" >&2
  launch_scenario_bench static-small static-small-proxysss run_http_bench static-small proxysss http://127.0.0.1:18083/bench/small.html
  launch_scenario_bench static-large static-large-proxysss run_http_bench static-large proxysss http://127.0.0.1:18083/bench/large.bin
  launch_scenario_bench cdn-hot-update cdn-hot-update-proxysss run_http_bench cdn-hot-update proxysss http://127.0.0.1:18083/bench/hot.dat
  launch_scenario_bench https-static-small https-static-small-proxysss run_http_bench https-static-small proxysss https://127.0.0.1:18443/bench/small.html
  launch_scenario_bench reverse-proxy reverse-proxy-proxysss run_http_bench reverse-proxy proxysss http://127.0.0.1:18083/proxy/ping
  launch_scenario_bench generic-sse generic-sse-proxysss run_sse_bench generic-sse proxysss http://127.0.0.1:18083/sse
  launch_scenario_bench websocket-long-connection websocket-long-connection-proxysss run_websocket_bench websocket-long-connection proxysss ws://127.0.0.1:18083/ws/ 256
  launch_scenario_bench game-long-connection game-long-connection-proxysss run_tcp_bench game-long-connection proxysss 127.0.0.1:18200 256
  launch_scenario_bench tcp-stream tcp-stream-proxysss run_tcp_bench tcp-stream proxysss 127.0.0.1:18200 1024
  launch_scenario_bench udp-stream udp-stream-proxysss run_udp_bench udp-stream proxysss 127.0.0.1:18300
  wait_for_group

  local name
  for name in "${ordered[@]}"; do
    if [[ -f "$row_dir/$name.json" ]]; then
      RESULT_ROWS+=("$(cat "$row_dir/$name.json")")
    fi
  done
}

if [[ "$MIXED_MATRIX" == "1" ]]; then
  append_deep_matrix_mixed
else
  append_deep_matrix_serial
fi

RESULT_ROWS_FILE="$RUN_DIR/results.jsonl"
printf '%s\n' "${RESULT_ROWS[@]}" >"$RESULT_ROWS_FILE"
"$BENCH_HELPER_BIN" write-json-array --in "$RESULT_ROWS_FILE" --out "$RESULTS_FILE"

MIXED_MATRIX_BOOL=false
if [[ "$MIXED_MATRIX" == "1" ]]; then
  MIXED_MATRIX_BOOL=true
fi

STRICT_SUPERIORITY_BOOL=false
if [[ "$STRICT_SUPERIORITY" == "1" ]]; then
  STRICT_SUPERIORITY_BOOL=true
fi

REQUIRE_LATENCY_PERCENTILES_BOOL=false
if [[ "$REQUIRE_LATENCY_PERCENTILES" == "1" ]]; then
  REQUIRE_LATENCY_PERCENTILES_BOOL=true
fi

REQUIRE_ZERO_ERRORS_BOOL=false
if [[ "$REQUIRE_ZERO_ERRORS" == "1" ]]; then
  REQUIRE_ZERO_ERRORS_BOOL=true
fi

GATE_LATENCY_BOOL=false
if [[ "$GATE_LATENCY" == "1" ]]; then
  GATE_LATENCY_BOOL=true
fi

"$BENCH_HELPER_BIN" write-all-scenarios-summary \
  --results "$RESULTS_FILE" \
  --md "$SUMMARY_MD" \
  --html "$SUMMARY_HTML" \
  --min-ratio "$MIN_RATIO" \
  --critical-ratio "$CRITICAL_RATIO" \
  --critical-scenarios "$CRITICAL_SCENARIOS" \
  --diagnostic-scenarios "$DIAGNOSTIC_SCENARIOS" \
  --sse-error-tolerance "$SSE_ERROR_TOLERANCE" \
  --websocket-error-tolerance "$WEBSOCKET_ERROR_TOLERANCE" \
  --udp-error-tolerance "$UDP_ERROR_TOLERANCE" \
  --aggregate-ratio "$AGGREGATE_RATIO" \
  --max-latency-ratio "$MAX_LATENCY_RATIO" \
  --require-latency-percentiles="$REQUIRE_LATENCY_PERCENTILES_BOOL" \
  --require-zero-errors="$REQUIRE_ZERO_ERRORS_BOOL" \
  --gate-latency="$GATE_LATENCY_BOOL" \
  --strict-superiority="$STRICT_SUPERIORITY_BOOL" \
  --mixed-matrix="$MIXED_MATRIX_BOOL" \
  --cpu-cores "$CPU_CORES" \
  --traffic-profile "$TRAFFIC_PROFILE" \
  --http-concurrency "$CONCURRENCY" \
  --https-concurrency "$HTTPS_CONCURRENCY" \
  --static-large-concurrency "$STATIC_LARGE_CONCURRENCY" \
  --sse-concurrency "$SSE_CONCURRENCY" \
  --stream-connections "$STREAM_CONNECTIONS"

echo "results saved to $RESULTS_FILE"
echo "summary markdown: $SUMMARY_MD"
echo "summary html:     $SUMMARY_HTML"
