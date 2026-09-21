#!/usr/bin/env bash
set -euo pipefail
[[ "$PWD" == /work && -f /work/Cargo.toml ]]
[[ "$(uname -sm)" == 'Linux x86_64' ]]
. /etc/os-release
[[ "$ID" == ubuntu && "$VERSION_ID" == 24.04 ]]
repeat="${VERIFY_REPEAT:-2}"
[[ "$repeat" =~ ^([1-9]|10)$ ]]
config="${VERIFY_CONFIG:-examples/all-scenarios.example.yaml}"
[[ "$config" != /* && "$config" != *'..'* && -f "$config" ]]
report=/work/.tmp/docker-scenarios
[[ ! -L /work/.tmp && ! -L "$report" && ! -L /work/.tmp/tests && ! -L /work/.tmp/docker-toolchain ]]
rm -rf -- /work/.tmp/docker-scenarios /work/.tmp/docker-toolchain
mkdir -p "$report" /work/.tmp/docker-toolchain /work/.tmp/go /work/.tmp/tools
export TMPDIR=/work/.tmp/docker-toolchain TEMP=/work/.tmp/docker-toolchain TMP=/work/.tmp/docker-toolchain
export GOCACHE=/work/.cache/go-linux-build GOPATH=/work/.cache/go GOMODCACHE=/work/.cache/go-mod GOTMPDIR=/work/.tmp/go
# The fixture root is an owned anonymous Docker volume. Keep the mountpoint;
# deleting it fails with EBUSY. --rm removes the volume after the container exits.
clear_tests() {
  [[ -d /work/.tmp/tests && ! -L /work/.tmp/tests ]]
  mountpoint -q /work/.tmp/tests
  find /work/.tmp/tests -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +
}
cleanup() { clear_tests; rm -rf -- /work/.tmp/docker-toolchain /work/.tmp/platform-performance/linux/work; rm -f -- /work/.tmp/platform-performance-linux.lock; }
printf 'verification container OS: %s %s %s\n' "$ID" "$VERSION_ID" "$(uname -m)" | tee "$report/environment.txt"
mountpoint -q /work/.tmp/tests || { echo 'Missing isolated fixture volume' >&2; exit 1; }
fixture_fs="$(stat -f -c %T /work/.tmp/tests)"
printf 'fixture filesystem: %s; source filesystem: %s\n' "$fixture_fs" "$(stat -f -c %T /work)" | tee -a "$report/environment.txt"
case "$fixture_fs" in 9p|v9fs|fuseblk|cifs|smb2) echo 'Linux fixtures require a native Docker volume, not a host-sharing filesystem' >&2; exit 1;; esac
trap cleanup EXIT
bash -n /work/scripts/verify-docker-scenarios.sh /work/scripts/verify-docker-container.sh
go run /work/scripts/verify-docs.go
go test /work/scripts/verify-docs.go /work/scripts/verify-docs_test.go
go test /work/scripts/benchmark-helper.go /work/scripts/benchmark-helper_test.go
# The reused benchmark image has a minimal Rust toolchain. Install only missing
# check components in this disposable container; no extra containers or images.
if ! cargo fmt --version >/dev/null 2>&1 || ! cargo clippy --version >/dev/null 2>&1; then
  rustup component add rustfmt clippy
fi
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
for ((pass=1; pass<=repeat; pass++)); do
  clear_tests
  cargo test --locked 2>&1 | tee "$report/tests-$pass.log"
  clear_tests
done
cargo build --locked
binary=/work/.benchmark/linux-target/debug/proxysss
go run /work/scripts/verify-linux-cpu/main.go --binary "$binary" | tee "$report/cpu-confinement.txt"
"$binary" -c "$config" check-config
"$binary" -c examples/security-performance.example.yaml check-config
"$binary" -c "$config" config explain > "$report/explain.txt"
"$binary" -c "$config" config routes > "$report/routes.txt"
"$binary" -c "$config" config security > "$report/security.yaml"
"$binary" -c "$config" config performance > "$report/performance.yaml"
"$binary" config capabilities > "$report/capabilities.txt"
"$binary" config nginx-parity --format yaml > "$report/nginx-parity.yaml"
grep -q 'service discovery : enabled=true, registries=3, mappings=3' "$report/explain.txt"
for capability in 'large file range downloads' 'service discovery registries' 'waf hotlink crawler controls' 'static origin security' 'safe directory index' 'cdn origin and ipv6 edge'; do grep -q "$capability" "$report/capabilities.txt"; done
grep -q 'api gateway policy chain' "$report/nginx-parity.yaml"
grep -q 'mapping api-from-consul registry=consul-main service=spring-api' "$report/routes.txt"
if [[ "${VERIFY_PERFORMANCE:-0}" == 1 ]]; then
  cargo build --profile release-fast --locked
  go build -o /work/.tmp/tools/platform-performance-linux /work/scripts/platform-performance/main.go /work/scripts/platform-performance/process_unix.go
  /work/.tmp/tools/platform-performance-linux -binary /work/.benchmark/linux-target/release-fast/proxysss -seconds 4 -repetitions 4 | tee "$report/mixed-performance.log"
fi
echo 'Single-container verification complete; cleanup follows'
