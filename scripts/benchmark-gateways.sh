#!/usr/bin/env bash
# Compare static-file throughput: proxysss vs nginx.
# All downloads and run artifacts stay under .benchmark/ (gitignored).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONCURRENCY="${CONCURRENCY:-512}"
DURATION_SECS="${DURATION_SECS:-30}"
NGINX_VERSION="${NGINX_VERSION:-1.31.0}"
QUICK="${QUICK:-0}"

if [[ "$QUICK" == "1" ]]; then
  CONCURRENCY=128
  DURATION_SECS=10
fi

BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
VENDOR_DIR="$BENCH_ROOT/vendors"
RUN_DIR="$BENCH_ROOT/runs/latest"
WWW_DIR="$RUN_DIR/www"
BENCH_HELPER_SRC="$ROOT/scripts/benchmark-helper.go"
BENCH_HELPER_BIN="$RUN_DIR/benchmark-helper"
PID_FILE="$RUN_DIR/pids.txt"
RESULTS_FILE="$RUN_DIR/results.json"
BUILD_PROFILE_WAS_SET="${BUILD_PROFILE+x}"
BUILD_PROFILE="${BUILD_PROFILE:-release}"
if [[ "$QUICK" == "1" && -z "$BUILD_PROFILE_WAS_SET" ]]; then
  BUILD_PROFILE="release-fast"
fi
PROXY_BIN="${PROXY_BIN:-$ROOT/target/$BUILD_PROFILE/proxysss}"

stop_bench_processes() {
  if [[ -f "$PID_FILE" ]]; then
    while read -r pid; do
      kill "$pid" 2>/dev/null || true
    done < "$PID_FILE"
  fi
  pkill -f "$RUN_DIR/nginx.conf" 2>/dev/null || true
}

if [[ ! -x "$PROXY_BIN" ]]; then
  cargo build --profile "$BUILD_PROFILE" --locked
fi
command -v go >/dev/null 2>&1 || {
  echo "missing required command: go" >&2
  exit 1
}

stop_bench_processes
rm -rf "$RUN_DIR"
mkdir -p "$VENDOR_DIR" "$WWW_DIR"
go build -o "$BENCH_HELPER_BIN" "$BENCH_HELPER_SRC"

NGINX_TARBALL="$VENDOR_DIR/nginx-$NGINX_VERSION.tar.gz"
NGINX_PREFIX="$VENDOR_DIR/nginx-$NGINX_VERSION"
NGINX_BIN="$NGINX_PREFIX/sbin/nginx"
if [[ ! -x "$NGINX_BIN" ]]; then
  echo "==> building nginx $NGINX_VERSION into $NGINX_PREFIX"
  if [[ ! -f "$NGINX_TARBALL" ]]; then
    curl -fsSL "https://nginx.org/download/nginx-$NGINX_VERSION.tar.gz" -o "$NGINX_TARBALL"
  fi
  BUILD_DIR="$VENDOR_DIR/nginx-build-$NGINX_VERSION"
  rm -rf "$BUILD_DIR"
  mkdir -p "$BUILD_DIR"
  tar -xzf "$NGINX_TARBALL" -C "$BUILD_DIR" --strip-components=1
  (
    cd "$BUILD_DIR"
    ./configure --prefix="$NGINX_PREFIX" --with-http_v2_module --with-threads --with-file-aio >/dev/null
    make -j"$(nproc)" >/dev/null
    make install >/dev/null
  )
  rm -rf "$BUILD_DIR"
fi

cat >"$WWW_DIR/index.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>gateway bench</title></head><body><h1>gateway benchmark</h1><p>same static payload for proxysss and nginx.</p></body></html>
HTML

PROXY_DIR="$RUN_DIR/proxysss"
"$PROXY_BIN" init --dir "$PROXY_DIR" --overwrite >/dev/null
cat >"$PROXY_DIR/proxysss.yaml" <<YAML
config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:18083
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
services:
  static_sites:
    - name: bench
      path_prefix: /bench
      root: '$WWW_DIR'
      index_files: [index.html]
      autoindex: false
YAML

"$PROXY_BIN" -config "$PROXY_DIR/proxysss.yaml" check-config

cat >"$RUN_DIR/nginx.conf" <<NGINX
worker_processes  1;
events { worker_connections  4096; }
http {
    access_log off;
    sendfile on;
    server {
        listen 127.0.0.1:18081;
        location /bench/ {
            alias $WWW_DIR/;
            index index.html;
        }
    }
}
NGINX

wait_http() {
  local url="$1"
  for _ in $(seq 1 80); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  echo "not ready: $url" >&2
  return 1
}

run_bench() {
  local name="$1" url="$2"
  echo "" >&2
  echo "=== $name c$CONCURRENCY d${DURATION_SECS}s ===" >&2
  local output
  output="$("$PROXY_BIN" bench http --url "$url" --concurrency "$CONCURRENCY" --duration-secs "$DURATION_SECS" 2>&1)"
  echo "$output" >&2
  printf '%s' "$output" | "$BENCH_HELPER_BIN" parse-bench \
    --gateway "$name" \
    --protocol http \
    --target "$url" \
    --concurrency "$CONCURRENCY" \
    --duration "$DURATION_SECS"
}

trap 'stop_bench_processes' EXIT

"$PROXY_BIN" -config "$PROXY_DIR/proxysss.yaml" >/dev/null 2>&1 &
echo $! >"$PID_FILE"
"$NGINX_BIN" -p "$NGINX_PREFIX" -c "$RUN_DIR/nginx.conf" >/dev/null 2>&1 &
echo $! >>"$PID_FILE"

TARGETS=(
  "proxysss|http://127.0.0.1:18083/bench/index.html"
  "nginx|http://127.0.0.1:18081/bench/index.html"
)

for target in "${TARGETS[@]}"; do
  wait_http "${target#*|}"
done

RESULT_ROWS=()
for target in "${TARGETS[@]}"; do
  name="${target%%|*}"
  url="${target#*|}"
  row="$(run_bench "$name" "$url")"
  RESULT_ROWS+=("$row")
  sleep 2
done

RESULT_ROWS_FILE="$RUN_DIR/results.jsonl"
printf '%s\n' "${RESULT_ROWS[@]}" >"$RESULT_ROWS_FILE"
"$BENCH_HELPER_BIN" write-json-array --in "$RESULT_ROWS_FILE" --out "$RESULTS_FILE"

echo ""
echo "=== throughput summary (ops/sec) ==="
"$BENCH_HELPER_BIN" print-results-summary --results "$RESULTS_FILE"

echo "results saved to $RESULTS_FILE"
echo "vendor binaries cached under $VENDOR_DIR (gitignored)"

"$BENCH_HELPER_BIN" write-gateway-report --results "$RESULTS_FILE" --out-dir "$RUN_DIR" --concurrency "$CONCURRENCY" --duration "$DURATION_SECS"
"$BENCH_HELPER_BIN" write-gateway-compare --results "$RESULTS_FILE" --out-dir "$RUN_DIR" --binary "$PROXY_BIN"

bash "$ROOT/scripts/benchmark-gate-check.sh" "$RESULTS_FILE"
