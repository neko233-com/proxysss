#!/bin/bash
set -euo pipefail
cd /tmp/bench2c2g
stop_gateway() {
  fuser -k -TERM 19080/tcp 2>/dev/null || true
  sleep 1
  if ss -ltn | grep -q ':19080 '; then fuser -k -KILL 19080/tcp 2>/dev/null || true; sleep 1; fi
  rm -f logs/nginx.pid logs/proxysss.pid
}
start_nginx() {
  stop_gateway
  prlimit --as=2147483648 /tmp/nginx128/sbin/nginx -c /tmp/bench2c2g/nginx28-final.conf 2>>logs/nginx-error.log
  sleep 1
  curl -sf -o /dev/null http://127.0.0.1:19080/small.bin
}
start_ps() {
  stop_gateway
  nohup prlimit --as=2147483648 ./proxysss-v1315 -c /tmp/bench2c2g/proxysss.yaml > logs/proxysss.out 2>&1 &
  echo $! > logs/proxysss.pid
  for i in $(seq 1 20); do curl -sf -o /dev/null http://127.0.0.1:19080/small.bin && return 0; sleep 0.5; done
  return 1
}
run_suite() {
  local gw=$1 round=$2
  echo "$gw round $round" | tee -a final.log
  ./bench-fast-http -url http://127.0.0.1:19080/small.bin -label "$gw-static-1k-r$round" -workers 64 -warmup 2 -duration 5 -expected-bytes 1024 -out "final-r${round}-${gw}-static-1k.json"
  ./bench-fast-http -url http://127.0.0.1:19080/large.bin -label "$gw-static-64k-r$round" -workers 64 -warmup 2 -duration 5 -expected-bytes 65536 -out "final-r${round}-${gw}-static-64k.json"
  ./bench-fast-http -url http://127.0.0.1:19080/rp/small.bin -label "$gw-proxy-1k-r$round" -workers 64 -warmup 2 -duration 5 -expected-bytes 1024 -out "final-r${round}-${gw}-proxy-1k.json"
}
rm -f final.log
for round in 1 2 3; do
  if [ $((round % 2)) -eq 1 ]; then
    start_nginx; run_suite nginx128 "$round"
    start_ps; run_suite proxysss "$round"
  else
    start_ps; run_suite proxysss "$round"
    start_nginx; run_suite nginx128 "$round"
  fi
done
stop_gateway
