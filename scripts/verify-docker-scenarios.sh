#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
image="${PROXYSSS_VERIFY_IMAGE:-proxysss-ubuntu24-amd64-bench:local}"
container=proxysss-verify
owner=proxysss-project-verification
mkdir -p "$root/.tmp"
lock="$root/.tmp/docker-verify-shell.lock"
[[ ! -L "$root/.tmp" && ! -L "$lock" ]]
if [[ -d "$lock" ]]; then
  previous_pid="$(cat "$lock/pid" 2>/dev/null || true)"
  if [[ "$previous_pid" =~ ^[0-9]+$ ]] && kill -0 "$previous_pid" 2>/dev/null; then echo 'Another Docker verification is running' >&2; exit 1; fi
  rm -f -- "$lock/pid"
  rmdir "$lock"
fi
mkdir "$lock"
echo $$ > "$lock/pid"
owned=0
cleanup() {
  if [[ "$owned" == 1 ]]; then docker rm -f "$container" >/dev/null 2>&1 || true; fi
  rm -f -- "$lock/pid"
  rmdir "$lock"
}
trap cleanup EXIT
if [[ -n "$(docker ps -aq --filter "name=^/${container}$")" ]]; then
  [[ "$(docker inspect --format '{{index .Config.Labels "com.proxysss.owner"}}' "$container")" == "$owner" ]] || { echo 'Container name belongs to another task' >&2; exit 1; }
  docker rm -f "$container" >/dev/null
fi
owned=1
if ! docker image inspect "$image" >/dev/null 2>&1; then docker build -f docker/ubuntu24-bench.Dockerfile -t "$image" .; fi
mount_root="$root"
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) mount_root="$(cygpath -m "$root")"; export MSYS2_ARG_CONV_EXCL='*';; esac
docker run --name "$container" --label "com.proxysss.owner=$owner" --rm \
  -e CARGO_HOME=/work/.cache/cargo -e CARGO_TARGET_DIR=/work/.benchmark/linux-target \
  -e "VERIFY_CONFIG=${VERIFY_CONFIG:-examples/all-scenarios.example.yaml}" \
  -e "VERIFY_REPEAT=${VERIFY_REPEAT:-2}" -e "VERIFY_PERFORMANCE=${VERIFY_PERFORMANCE:-0}" \
  -v "$mount_root:/work" -w /work "$image" bash /work/scripts/verify-docker-container.sh
echo 'proxysss Docker scenario verification passed'
