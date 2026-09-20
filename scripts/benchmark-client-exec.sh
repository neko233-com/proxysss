#!/usr/bin/env bash
# Executed after taskset, before exec/warm-up. Record the affinity inherited by
# the actual generator, rather than the enclosing container's CPU allowance.
set -euo pipefail
record="$1"
shift
[[ "$TOKIO_WORKER_THREADS" =~ ^[1-9][0-9]*$ ]]
awk '/^Cpus_allowed_list:/ { print "allowed_cpus=" $2 }' /proc/self/status >"$record"
printf 'runtime_workers=%s\n' "$TOKIO_WORKER_THREADS" >>"$record"
exec "$@"
