#!/usr/bin/env bash
# Compare static-file throughput: proxysss vs nginx vs caddy.
# All downloads and run artifacts stay under .benchmark/ (gitignored).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONCURRENCY="${CONCURRENCY:-512}"
DURATION_SECS="${DURATION_SECS:-30}"
NGINX_VERSION="${NGINX_VERSION:-1.31.0}"
CADDY_VERSION="${CADDY_VERSION:-2.11.3}"
QUICK="${QUICK:-0}"

if [[ "$QUICK" == "1" ]]; then
  CONCURRENCY=128
  DURATION_SECS=10
fi

BENCH_ROOT="${BENCH_ROOT:-$ROOT/.benchmark}"
VENDOR_DIR="$BENCH_ROOT/vendors"
RUN_DIR="$BENCH_ROOT/runs/latest"
WWW_DIR="$RUN_DIR/www"
PID_FILE="$RUN_DIR/pids.txt"
RESULTS_FILE="$RUN_DIR/results.json"
PROXY_BIN="$ROOT/target/release/proxysss"

stop_bench_processes() {
  if [[ -f "$PID_FILE" ]]; then
    while read -r pid; do
      kill "$pid" 2>/dev/null || true
    done < "$PID_FILE"
  fi
  pkill -f "$RUN_DIR/nginx.conf" 2>/dev/null || true
  pkill -f "caddy file-server --listen 127.0.0.1:18082" 2>/dev/null || true
}

if [[ ! -x "$PROXY_BIN" ]]; then
  cargo build --release --locked
fi

stop_bench_processes
rm -rf "$RUN_DIR"
mkdir -p "$VENDOR_DIR" "$WWW_DIR"

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

CADDY_TARBALL="$VENDOR_DIR/caddy_${CADDY_VERSION}_linux_amd64.tar.gz"
CADDY_BIN="$VENDOR_DIR/caddy"
if [[ ! -x "$CADDY_BIN" ]]; then
  echo "==> downloading caddy $CADDY_VERSION"
  if [[ ! -f "$CADDY_TARBALL" ]]; then
    curl -fsSL "https://github.com/caddyserver/caddy/releases/download/v${CADDY_VERSION}/caddy_${CADDY_VERSION}_linux_amd64.tar.gz" -o "$CADDY_TARBALL"
  fi
  tar -xzf "$CADDY_TARBALL" -C "$VENDOR_DIR" caddy
  chmod +x "$CADDY_BIN"
fi

cat >"$WWW_DIR/index.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>gateway bench</title></head><body><h1>gateway benchmark</h1><p>same static payload for proxysss nginx caddy.</p></body></html>
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
  echo ""
  echo "=== $name c$CONCURRENCY d${DURATION_SECS}s ==="
  local output
  output="$("$PROXY_BIN" bench http --url "$url" --concurrency "$CONCURRENCY" --duration-secs "$DURATION_SECS" 2>&1)"
  echo "$output"
  python3 - "$name" "$url" "$CONCURRENCY" "$DURATION_SECS" "$output" <<'PY'
import json, re, sys
name, url, concurrency, duration, output = sys.argv[1:6]
row = {
    "name": name,
    "url": url,
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
    if not match:
        continue
    value = match.group(1)
    row[key] = float(value) if "." in value else int(value)
print(json.dumps(row))
PY
}

trap 'stop_bench_processes' EXIT

"$PROXY_BIN" -config "$PROXY_DIR/proxysss.yaml" >/dev/null 2>&1 &
echo $! >"$PID_FILE"
"$NGINX_BIN" -p "$NGINX_PREFIX" -c "$RUN_DIR/nginx.conf" >/dev/null 2>&1 &
echo $! >>"$PID_FILE"
"$CADDY_BIN" file-server --listen 127.0.0.1:18082 --root "$WWW_DIR" >/dev/null 2>&1 &
echo $! >>"$PID_FILE"

TARGETS=(
  "proxysss|http://127.0.0.1:18083/bench/index.html"
  "nginx|http://127.0.0.1:18081/bench/index.html"
  "caddy|http://127.0.0.1:18082/index.html"
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

python3 - "$RESULTS_FILE" "${RESULT_ROWS[@]}" <<'PY'
import json, sys
path = sys.argv[1]
rows = [json.loads(item) for item in sys.argv[2:]]
with open(path, "w", encoding="utf-8") as handle:
    json.dump(rows, handle, indent=2)
    handle.write("\n")
PY

echo ""
echo "=== throughput summary (ops/sec) ==="
python3 - "$RESULTS_FILE" <<'PY'
import json, sys
rows = json.load(open(sys.argv[1], encoding="utf-8"))
rows.sort(key=lambda row: row.get("ops_per_sec", 0), reverse=True)
print(f"{'name':<10} {'ops/s':>12} {'MiB/s':>10} {'p50ms':>8} {'errors':>8}")
for row in rows:
    print(f"{row['name']:<10} {row.get('ops_per_sec', 0):>12.2f} {row.get('throughput_mib_s', 0):>10.2f} {row.get('latency_p50_ms') or 0:>8.2f} {row.get('errors', 0):>8}")
PY

echo "results saved to $RESULTS_FILE"
echo "vendor binaries cached under $VENDOR_DIR (gitignored)"

python3 "$ROOT/scripts/benchmark-report.py" --results "$RESULTS_FILE" --out-dir "$RUN_DIR" --concurrency "$CONCURRENCY" --duration-secs "$DURATION_SECS"
python3 "$ROOT/scripts/compare-report.py" --binary "$PROXY_BIN" --benchmark "$RESULTS_FILE" --out-dir "$RUN_DIR"
echo "benchmark report markdown: $RUN_DIR/report.md"
echo "benchmark report html:     $RUN_DIR/report.html"

bash "$ROOT/scripts/benchmark-gate-check.sh" "$RESULTS_FILE"
