#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="${PROXYSSS_VERIFY_IMAGE:-proxysss-ubuntu24-scenarios}"
config="examples/all-scenarios.example.yaml"

cd "$repo_root"

# Reuse the Ubuntu 24 benchmark image so scenario validation runs in the same
# Linux family used by the production performance gate.
docker build -f docker/ubuntu24-bench.Dockerfile -t "$image" .

docker run --rm \
  -e CARGO_TARGET_DIR=/target \
  -v proxysss-scenario-target:/target \
  -v "$repo_root:/work" \
  -w /work \
  "$image" \
  bash -lc "
    set -euo pipefail
    # One full pass covers the scenario surfaces and avoids repeated Linux
    # recompilation when the checkout is bind-mounted from Windows.
    cargo test --locked

    # Build the CLI once, then validate that the broad scenario sample is both
    # syntactically valid and visible through agent-friendly inspection commands.
    cargo build --locked
    proxysss_bin=/target/debug/proxysss
    \$proxysss_bin -config $config check-config
    \$proxysss_bin -config $config config explain | tee /tmp/proxysss-explain.txt
    \$proxysss_bin -config $config config routes | tee /tmp/proxysss-routes.txt
    \$proxysss_bin config capabilities | tee /tmp/proxysss-capabilities.txt
    \$proxysss_bin config nginx-parity --format yaml | tee /tmp/proxysss-nginx-parity.yaml

    # Grep the exact operator-facing surfaces that must not regress silently:
    # static Range, discovery mappings, WAF/plugin boundary, CDN/IPv6, and the
    # policy-chain parity declaration.
    grep -q 'service discovery : enabled=true, registries=3, mappings=3' /tmp/proxysss-explain.txt
    grep -q 'large file range downloads' /tmp/proxysss-capabilities.txt
    grep -q 'service discovery registries' /tmp/proxysss-capabilities.txt
    grep -q 'waf hotlink crawler controls' /tmp/proxysss-capabilities.txt
    grep -q 'cdn origin and ipv6 edge' /tmp/proxysss-capabilities.txt
    grep -q 'api gateway policy chain' /tmp/proxysss-nginx-parity.yaml
    grep -q 'large file range downloads' /tmp/proxysss-nginx-parity.yaml
    grep -q 'mapping api-from-consul registry=consul-main service=spring-api' /tmp/proxysss-routes.txt
  "

echo "proxysss Docker scenario verification passed"
