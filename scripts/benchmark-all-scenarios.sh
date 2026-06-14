#!/usr/bin/env bash
# Linux/Ubuntu production gateway benchmark matrix: proxysss vs nginx where nginx
# has an equivalent open-source core module path. The default release gate is a
# mixed multi-proxy load focused on game/WebSocket/TCP/UDP long-connection
# paths. Critical realtime paths use a fairness floor by default because
# proxysss carries built-in policy surfaces that nginx often needs modules or
# extra config to match. Static, reverse proxy, and SSE still run together with
# a soft floor; bulk/TLS static are diagnostic unless explicitly promoted to a
# gate.
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
CPU_CORES="${CPU_CORES:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || nproc 2>/dev/null || echo 1)}"
CPU_CORES="${CPU_CORES:-1}"
CONCURRENCY="${CONCURRENCY:-$((CPU_CORES * 16))}"
STATIC_LARGE_CONCURRENCY="${STATIC_LARGE_CONCURRENCY:-$CPU_CORES}"
HTTPS_CONCURRENCY="${HTTPS_CONCURRENCY:-$((CPU_CORES * 4))}"
STREAM_CONNECTIONS="${STREAM_CONNECTIONS:-$((CPU_CORES * 4))}"
SSE_CONCURRENCY="${SSE_CONCURRENCY:-$CPU_CORES}"
SSE_MAX_CHUNKS="${SSE_MAX_CHUNKS:-1}"
DURATION_SECS="${DURATION_SECS:-30}"
UDP_TIMEOUT_MS="${UDP_TIMEOUT_MS:-7000}"
NGINX_VERSION="${NGINX_VERSION:-1.31.0}"
QUICK="${QUICK:-0}"
HTTPS_HTTP1_ONLY="${HTTPS_HTTP1_ONLY:-0}"
MIN_RATIO="${MIN_RATIO:-0.50}"
CRITICAL_RATIO="${CRITICAL_RATIO:-0.97}"
CRITICAL_SCENARIOS="${CRITICAL_SCENARIOS:-websocket-long-connection game-long-connection tcp-stream udp-stream kcp-style-udp}"
DIAGNOSTIC_SCENARIOS="${DIAGNOSTIC_SCENARIOS:-static-large https-static-small}"
WEBSOCKET_ERROR_TOLERANCE="${WEBSOCKET_ERROR_TOLERANCE:-4}"
FAST_GATE="${FAST_GATE:-0}"
FAST_GATE_RATIO="${FAST_GATE_RATIO:-$CRITICAL_RATIO}"
FAST_GATE_CONCURRENCY="${FAST_GATE_CONCURRENCY:-$((CPU_CORES * 16))}"
FAST_GATE_HTTPS_CONCURRENCY="${FAST_GATE_HTTPS_CONCURRENCY:-$((CPU_CORES * 4))}"
FAST_GATE_STATIC_LARGE_CONCURRENCY="${FAST_GATE_STATIC_LARGE_CONCURRENCY:-$CPU_CORES}"
FAST_GATE_STREAM_CONNECTIONS="${FAST_GATE_STREAM_CONNECTIONS:-$((CPU_CORES * 4))}"
FAST_GATE_SSE_CONCURRENCY="${FAST_GATE_SSE_CONCURRENCY:-$CPU_CORES}"
FAST_GATE_DURATION_SECS="${FAST_GATE_DURATION_SECS:-4}"
FAST_GATE_SCENARIOS="${FAST_GATE_SCENARIOS:-$CRITICAL_SCENARIOS}"
MIXED_MATRIX="${MIXED_MATRIX:-1}"
AGGREGATE_RATIO="${AGGREGATE_RATIO:-0.97}"

if [[ "$QUICK" == "1" ]]; then
  [[ -z "$CONCURRENCY_SET" ]] && CONCURRENCY=$((CPU_CORES * 16))
  [[ -z "$STATIC_LARGE_CONCURRENCY_SET" ]] && STATIC_LARGE_CONCURRENCY=$CPU_CORES
  [[ -z "$HTTPS_CONCURRENCY_SET" ]] && HTTPS_CONCURRENCY=$((CPU_CORES * 4))
  [[ -z "$STREAM_CONNECTIONS_SET" ]] && STREAM_CONNECTIONS=$((CPU_CORES * 4))
  [[ -z "$SSE_CONCURRENCY_SET" ]] && SSE_CONCURRENCY=$CPU_CORES
  [[ -z "$DURATION_SECS_SET" ]] && DURATION_SECS=10
fi

BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
VENDOR_DIR="$BENCH_ROOT/vendors"
RUN_DIR="$BENCH_ROOT/runs/all-scenarios"
WWW_DIR="$RUN_DIR/www"
PROXY_DIR="$RUN_DIR/proxysss"
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
      kill "$pid" 2>/dev/null || true
    done < "$PID_FILE"
  fi
  pkill -f "$RUN_DIR/nginx.conf" 2>/dev/null || true
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_cmd python3
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
      --without-http_rewrite_module >/dev/null
    make -j"$(nproc)" >/dev/null
    make install >/dev/null
  )
  rm -rf "$BUILD_DIR"
fi

cat >"$WWW_DIR/small.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>small bench</title></head><body><h1>proxysss small static benchmark</h1><p>same payload for proxysss and nginx.</p></body></html>
HTML
printf 'hot-update-v1\n' >"$WWW_DIR/hot.dat"
python3 - "$WWW_DIR/large.bin" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).write_bytes((b"proxysss-large-static-benchmark\n" * 4096) * 128)
PY

mkdir -p "$RUN_DIR/certs"
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$RUN_DIR/certs/bench.key" \
  -out "$RUN_DIR/certs/bench.crt" \
  -subj "/CN=localhost" \
  -days 1 >/dev/null 2>&1

cat >"$RUN_DIR/sse-upstream.py" <<'PY'
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        return

    def do_GET(self):
        self._handle()

    def do_POST(self):
        length = int(self.headers.get("content-length", "0") or "0")
        if length:
            self.rfile.read(length)
        self._handle()

    def _handle(self):
        if self.path.startswith("/v1/chat/completions") or self.path.startswith("/sse"):
            self.send_response(200)
            self.send_header("content-type", "text/event-stream")
            self.send_header("cache-control", "no-cache")
            self.send_header("connection", "close")
            self.end_headers()
            for idx in range(8):
                payload = {
                    "id": "chatcmpl-proxysss-bench",
                    "object": "chat.completion.chunk",
                    "choices": [{"index": 0, "delta": {"content": f"token-{idx}"}}],
                }
                self.wfile.write(b"data: " + json.dumps(payload).encode() + b"\n\n")
                self.wfile.flush()
                time.sleep(0.002)
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return
        body = json.dumps({"ok": True, "path": self.path}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

port = int(sys.argv[1])
ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY

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
    traffic_profile: small
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
  ai_proxy:
    enabled: true
    routes:
      - name: new-api-sse
        provider: new-api
        path_prefix: /v1
        upstream: http://127.0.0.1:18191
        rewrite_base_path: /v1
        forward_headers: false
        emit_metadata_headers: false
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
user root;
worker_processes  auto;
events {
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
    upstream ai_sse {
        server 127.0.0.1:18191;
        keepalive 128;
    }
    upstream ws_echo {
        server 127.0.0.1:18192;
        keepalive 128;
    }
    server {
        listen 127.0.0.1:18081;
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
        location /v1/ {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_buffering off;
            proxy_pass http://ai_sse/v1/;
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
        listen 127.0.0.1:18441 ssl http2;
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
        location /v1/ {
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_buffering off;
            proxy_pass http://ai_sse/v1/;
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
        listen 127.0.0.1:18202;
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
  python3 - "$scenario" "$gateway" "$protocol" "$target" "$bench_concurrency" "$DURATION_SECS" "$output" <<'PY'
import json, re, sys
scenario, gateway, protocol, target, concurrency, duration, output = sys.argv[1:8]
row = {
    "scenario": scenario,
    "gateway": gateway,
    "name": f"{scenario}:{gateway}",
    "protocol": protocol,
    "target": target,
    "concurrency": int(concurrency),
    "duration_secs": int(duration),
    "success": 0,
    "errors": 0,
    "ops_per_sec": 0.0,
    "throughput_mib_s": 0.0,
    "latency_p50_ms": None,
    "latency_p95_ms": None,
    "latency_p99_ms": None,
}
patterns = {
    "success": r"success\s+:\s+(\d+)",
    "errors": r"errors\s+:\s+(\d+)",
    "ops_per_sec": r"ops/sec\s+:\s+([\d.]+)",
    "throughput_mib_s": r"throughput\s+:\s+([\d.]+)\s+MiB/s",
    "latency_p50_ms": r"latency p50\s+:\s+([\d.]+)\s+ms",
    "latency_p95_ms": r"latency p95\s+:\s+([\d.]+)\s+ms",
    "latency_p99_ms": r"latency p99\s+:\s+([\d.]+)\s+ms",
}
for key, pattern in patterns.items():
    match = re.search(pattern, output)
    if match:
        value = match.group(1)
        row[key] = float(value) if "." in value else int(value)
print(json.dumps(row))
PY
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

SSE_UPSTREAM_PID=""

start_sse_backend() {
  python3 "$RUN_DIR/sse-upstream.py" 18191 >/dev/null 2>&1 &
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
wait_http "http://127.0.0.1:18083/v1/chat/completions"
wait_http "http://127.0.0.1:18081/v1/chat/completions"
for _ in 1 2; do
  timeout 8 curl -fsS --no-buffer "http://127.0.0.1:18083/v1/chat/completions" >/dev/null 2>&1 || true
  timeout 8 curl -fsS --no-buffer "http://127.0.0.1:18081/v1/chat/completions" >/dev/null 2>&1 || true
done
warm_ws_gateway "ws://127.0.0.1:18083/ws/"
warm_ws_gateway "ws://127.0.0.1:18081/ws/"

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
      new-api-sse)
        QUICK_ROWS+=("$(run_sse_bench new-api-sse nginx http://127.0.0.1:18081/v1/chat/completions)")
        restart_sse_backend
        warm_sse_gateway "http://127.0.0.1:18083/v1/chat/completions"
        QUICK_ROWS+=("$(run_sse_bench new-api-sse proxysss http://127.0.0.1:18083/v1/chat/completions)")
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

  python3 - "$FAST_GATE_RATIO" "${QUICK_ROWS[@]}" <<'PY'
import json, sys
from collections import defaultdict
min_ratio = float(sys.argv[1])
rows = [json.loads(item) for item in sys.argv[2:]]
by_scenario = defaultdict(dict)
for row in rows:
    by_scenario[row["scenario"]][row["gateway"]] = row
failures = []
for scenario, gateways in sorted(by_scenario.items()):
    proxy = gateways.get("proxysss", {})
    nginx = gateways.get("nginx", {})
    proxy_ops = float(proxy.get("ops_per_sec") or 0)
    nginx_ops = float(nginx.get("ops_per_sec") or 0)
    ratio = proxy_ops / nginx_ops if nginx_ops else 0.0
    print(f"quick gate {scenario}: proxysss={proxy_ops:.2f} nginx={nginx_ops:.2f} ratio={ratio:.3f}x")
    if int(proxy.get("errors") or 0) or int(nginx.get("errors") or 0):
        failures.append(f"{scenario} errors proxysss={proxy.get('errors')} nginx={nginx.get('errors')}")
    elif ratio < min_ratio:
        failures.append(f"{scenario} ratio={ratio:.3f}x < {min_ratio:.2f}x")
if failures:
    raise SystemExit("quick benchmark gate failed; deep matrix skipped: " + "; ".join(failures))
print("quick benchmark gate passed; starting deep matrix")
PY
fi

append_deep_matrix_serial() {
  RESULT_ROWS+=("$(run_http_bench static-small nginx http://127.0.0.1:18081/bench/small.html)")
  RESULT_ROWS+=("$(run_http_bench static-small proxysss http://127.0.0.1:18083/bench/small.html)")
  RESULT_ROWS+=("$(run_http_bench static-large nginx http://127.0.0.1:18081/bench/large.bin)")
  RESULT_ROWS+=("$(run_http_bench static-large proxysss http://127.0.0.1:18083/bench/large.bin)")
  RESULT_ROWS+=("$(run_http_bench cdn-hot-update nginx http://127.0.0.1:18081/bench/hot.dat)")
  RESULT_ROWS+=("$(run_http_bench cdn-hot-update proxysss http://127.0.0.1:18083/bench/hot.dat)")
  RESULT_ROWS+=("$(run_http_bench https-static-small nginx https://127.0.0.1:18441/bench/small.html)")
  RESULT_ROWS+=("$(run_http_bench https-static-small proxysss https://127.0.0.1:18443/bench/small.html)")
  RESULT_ROWS+=("$(run_http_bench reverse-proxy nginx http://127.0.0.1:18081/proxy/ping)")
  RESULT_ROWS+=("$(run_http_bench reverse-proxy proxysss http://127.0.0.1:18083/proxy/ping)")
  RESULT_ROWS+=("$(run_sse_bench new-api-sse nginx http://127.0.0.1:18081/v1/chat/completions)")
  restart_sse_backend
  warm_sse_gateway "http://127.0.0.1:18083/v1/chat/completions"
  RESULT_ROWS+=("$(run_sse_bench new-api-sse proxysss http://127.0.0.1:18083/v1/chat/completions)")
  RESULT_ROWS+=("$(run_websocket_bench websocket-long-connection nginx ws://127.0.0.1:18081/ws/ 256)")
  RESULT_ROWS+=("$(run_websocket_bench websocket-long-connection proxysss ws://127.0.0.1:18083/ws/ 256)")
  RESULT_ROWS+=("$(run_tcp_bench game-long-connection nginx 127.0.0.1:18202 256)")
  RESULT_ROWS+=("$(run_tcp_bench game-long-connection proxysss 127.0.0.1:18200 256)")
  RESULT_ROWS+=("$(run_tcp_bench tcp-stream nginx 127.0.0.1:18202 1024)")
  RESULT_ROWS+=("$(run_tcp_bench tcp-stream proxysss 127.0.0.1:18200 1024)")
  RESULT_ROWS+=("$(run_udp_bench udp-stream nginx 127.0.0.1:18302)")
  RESULT_ROWS+=("$(run_udp_bench udp-stream proxysss 127.0.0.1:18300)")
  RESULT_ROWS+=("$(run_udp_bench kcp-style-udp nginx 127.0.0.1:18302 1200)")
  RESULT_ROWS+=("$(run_udp_bench kcp-style-udp proxysss 127.0.0.1:18300 1200)")
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
    new-api-sse-nginx new-api-sse-proxysss
    websocket-long-connection-nginx websocket-long-connection-proxysss
    game-long-connection-nginx game-long-connection-proxysss
    tcp-stream-nginx tcp-stream-proxysss
    udp-stream-nginx udp-stream-proxysss
    kcp-style-udp-nginx kcp-style-udp-proxysss
  )

  launch_bench() {
    local name="$1"
    shift
    ("$@" >"$row_dir/$name.json") &
    pids+=("$!")
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
  launch_bench static-small-nginx run_http_bench static-small nginx http://127.0.0.1:18081/bench/small.html
  launch_bench static-large-nginx run_http_bench static-large nginx http://127.0.0.1:18081/bench/large.bin
  launch_bench cdn-hot-update-nginx run_http_bench cdn-hot-update nginx http://127.0.0.1:18081/bench/hot.dat
  launch_bench https-static-small-nginx run_http_bench https-static-small nginx https://127.0.0.1:18441/bench/small.html
  launch_bench reverse-proxy-nginx run_http_bench reverse-proxy nginx http://127.0.0.1:18081/proxy/ping
  launch_bench new-api-sse-nginx run_sse_bench new-api-sse nginx http://127.0.0.1:18081/v1/chat/completions
  launch_bench websocket-long-connection-nginx run_websocket_bench websocket-long-connection nginx ws://127.0.0.1:18081/ws/ 256
  launch_bench game-long-connection-nginx run_tcp_bench game-long-connection nginx 127.0.0.1:18202 256
  launch_bench tcp-stream-nginx run_tcp_bench tcp-stream nginx 127.0.0.1:18202 1024
  launch_bench udp-stream-nginx run_udp_bench udp-stream nginx 127.0.0.1:18302
  launch_bench kcp-style-udp-nginx run_udp_bench kcp-style-udp nginx 127.0.0.1:18302 1200
  wait_for_group
  restart_sse_backend
  warm_sse_gateway "http://127.0.0.1:18083/v1/chat/completions"

  echo "=== mixed matrix: proxysss scenarios running concurrently ===" >&2
  launch_bench static-small-proxysss run_http_bench static-small proxysss http://127.0.0.1:18083/bench/small.html
  launch_bench static-large-proxysss run_http_bench static-large proxysss http://127.0.0.1:18083/bench/large.bin
  launch_bench cdn-hot-update-proxysss run_http_bench cdn-hot-update proxysss http://127.0.0.1:18083/bench/hot.dat
  launch_bench https-static-small-proxysss run_http_bench https-static-small proxysss https://127.0.0.1:18443/bench/small.html
  launch_bench reverse-proxy-proxysss run_http_bench reverse-proxy proxysss http://127.0.0.1:18083/proxy/ping
  launch_bench new-api-sse-proxysss run_sse_bench new-api-sse proxysss http://127.0.0.1:18083/v1/chat/completions
  launch_bench websocket-long-connection-proxysss run_websocket_bench websocket-long-connection proxysss ws://127.0.0.1:18083/ws/ 256
  launch_bench game-long-connection-proxysss run_tcp_bench game-long-connection proxysss 127.0.0.1:18200 256
  launch_bench tcp-stream-proxysss run_tcp_bench tcp-stream proxysss 127.0.0.1:18200 1024
  launch_bench udp-stream-proxysss run_udp_bench udp-stream proxysss 127.0.0.1:18300
  launch_bench kcp-style-udp-proxysss run_udp_bench kcp-style-udp proxysss 127.0.0.1:18300 1200
  wait_for_group

  local name
  for name in "${ordered[@]}"; do
    RESULT_ROWS+=("$(cat "$row_dir/$name.json")")
  done
}

if [[ "$MIXED_MATRIX" == "1" ]]; then
  append_deep_matrix_mixed
else
  append_deep_matrix_serial
fi

python3 - "$RESULTS_FILE" "${RESULT_ROWS[@]}" <<'PY'
import json, sys
path = sys.argv[1]
rows = [json.loads(item) for item in sys.argv[2:]]
with open(path, "w", encoding="utf-8") as handle:
    json.dump(rows, handle, indent=2)
    handle.write("\n")
PY

python3 - "$RESULTS_FILE" "$SUMMARY_MD" "$SUMMARY_HTML" "$MIN_RATIO" "$CRITICAL_RATIO" "$CRITICAL_SCENARIOS" "$DIAGNOSTIC_SCENARIOS" "$WEBSOCKET_ERROR_TOLERANCE" "$AGGREGATE_RATIO" "$MIXED_MATRIX" "$CPU_CORES" "$CONCURRENCY" "$HTTPS_CONCURRENCY" "$STATIC_LARGE_CONCURRENCY" "$SSE_CONCURRENCY" "$STREAM_CONNECTIONS" <<'PY'
import html, json, sys
from collections import defaultdict
results_path, md_path, html_path = sys.argv[1], sys.argv[2], sys.argv[3]
min_ratio, critical_ratio = float(sys.argv[4]), float(sys.argv[5])
critical_scenarios = set(sys.argv[6].split())
diagnostic_scenarios = set(sys.argv[7].split())
websocket_error_tolerance = int(sys.argv[8])
aggregate_ratio = float(sys.argv[9])
mixed_matrix = sys.argv[10] == "1"
cpu_cores, http_concurrency, https_concurrency, static_large_concurrency, sse_concurrency, stream_connections = sys.argv[11:17]
rows = json.load(open(results_path, encoding="utf-8"))
by_scenario = defaultdict(dict)
for row in rows:
    by_scenario[row["scenario"]][row["gateway"]] = row
errors = []
for scenario, gateways in by_scenario.items():
    proxy = gateways.get("proxysss", {})
    nginx = gateways.get("nginx", {})
    proxy_errors = int(proxy.get("errors") or 0)
    nginx_errors = int(nginx.get("errors") or 0)
    protocol = proxy.get("protocol") or nginx.get("protocol")
    if protocol == "udp":
        if proxy_errors > nginx_errors + 2:
            errors.append(f"{scenario} udp errors proxysss={proxy_errors} nginx={nginx_errors}")
    elif protocol == "websocket":
        if proxy_errors > nginx_errors + websocket_error_tolerance:
            errors.append(f"{scenario} websocket errors proxysss={proxy_errors} nginx={nginx_errors}")
    elif proxy_errors or nginx_errors:
        errors.append(f"{scenario} errors proxysss={proxy_errors} nginx={nginx_errors}")

lines = [
    "# proxysss all-scenarios benchmark",
    "",
    f"- Matrix mode: `{'mixed concurrent' if mixed_matrix else 'serial diagnostic'}`",
    f"- Detected CPU cores: `{cpu_cores}`",
    f"- Auto concurrency: HTTP `{http_concurrency}`, HTTPS `{https_concurrency}`, static-large `{static_large_concurrency}`, SSE `{sse_concurrency}`, TCP/UDP/WebSocket `{stream_connections}`",
    f"- Non-critical minimum proxysss/nginx ops ratio: `{min_ratio:.2f}` except diagnostic scenarios `{', '.join(sorted(diagnostic_scenarios))}`",
    f"- WebSocket reconnect/error tolerance: `proxysss <= nginx + {websocket_error_tolerance}`",
    f"- Critical long-connection fair ratio gate: `{critical_ratio:.2f}` for `{', '.join(sorted(critical_scenarios))}`",
    f"- Aggregate mixed-load fair ratio gate: `{aggregate_ratio:.2f}`",
    f"- Result file: `{results_path}`",
    "",
    "| Scenario | proxysss ops/s | nginx ops/s | Ratio | proxysss p95 ms | nginx p95 ms | Errors |",
    "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
]
ratios = []
proxy_total_ops = 0.0
nginx_total_ops = 0.0
for scenario in sorted(by_scenario):
    proxy = by_scenario[scenario].get("proxysss")
    nginx = by_scenario[scenario].get("nginx")
    proxy_ops = float((proxy or {}).get("ops_per_sec") or 0)
    nginx_ops = float((nginx or {}).get("ops_per_sec") or 0)
    ratio = proxy_ops / nginx_ops if nginx_ops > 0 else 0.0
    proxy_total_ops += proxy_ops
    nginx_total_ops += nginx_ops
    ratios.append((scenario, ratio))
    err = int((proxy or {}).get("errors") or 0) + int((nginx or {}).get("errors") or 0)
    lines.append(
        f"| {scenario} | {proxy_ops:.2f} | {nginx_ops:.2f} | {ratio:.3f}x | "
        f"{float((proxy or {}).get('latency_p95_ms') or 0):.3f} | "
        f"{float((nginx or {}).get('latency_p95_ms') or 0):.3f} | {err} |"
    )

aggregate = proxy_total_ops / nginx_total_ops if nginx_total_ops > 0 else 0.0
lines.extend([
    "",
    f"- Aggregate proxysss ops/s: `{proxy_total_ops:.2f}`",
    f"- Aggregate nginx ops/s: `{nginx_total_ops:.2f}`",
    f"- Aggregate proxysss/nginx ratio: `{aggregate:.3f}x`",
])

with open(md_path, "w", encoding="utf-8") as handle:
    handle.write("\n".join(lines) + "\n")

body_rows = "\n".join(
    "<tr>"
    f"<td>{html.escape(scenario)}</td>"
    f"<td>{by_scenario[scenario].get('proxysss', {}).get('ops_per_sec', 0):.2f}</td>"
    f"<td>{by_scenario[scenario].get('nginx', {}).get('ops_per_sec', 0):.2f}</td>"
    f"<td>{ratio:.3f}x</td>"
    "</tr>"
    for scenario, ratio in ratios
)
with open(html_path, "w", encoding="utf-8") as handle:
    handle.write(f"<!doctype html><meta charset='utf-8'><title>proxysss all-scenarios benchmark</title><h1>proxysss all-scenarios benchmark</h1><table><thead><tr><th>Scenario</th><th>proxysss ops/s</th><th>nginx ops/s</th><th>ratio</th></tr></thead><tbody>{body_rows}</tbody></table>")

failures = [
    f"{scenario} ratio={ratio:.3f}"
    for scenario, ratio in ratios
    if scenario not in diagnostic_scenarios and ratio < min_ratio
]
aggregate_failure = mixed_matrix and aggregate < aggregate_ratio
critical_failures = [
    f"{scenario} ratio={ratio:.3f} < {critical_ratio:.2f}"
    for scenario, ratio in ratios
    if scenario in critical_scenarios and ratio < critical_ratio
]
if errors:
    raise SystemExit("benchmark errors: " + "; ".join(errors))
if critical_failures:
    raise SystemExit("critical fair benchmark ratio gate failed: " + "; ".join(critical_failures))
if aggregate_failure:
    raise SystemExit(f"aggregate mixed fair benchmark ratio gate failed: {aggregate:.3f} < {aggregate_ratio:.2f}")
if failures:
    raise SystemExit("benchmark ratio gate failed: " + "; ".join(failures))
print(f"all-scenarios benchmark gate passed ({len(rows)} rows, aggregate ratio {aggregate:.3f}x)")
PY

echo "results saved to $RESULTS_FILE"
echo "summary markdown: $SUMMARY_MD"
echo "summary html:     $SUMMARY_HTML"
