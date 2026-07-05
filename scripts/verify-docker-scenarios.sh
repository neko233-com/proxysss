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
  -v "$repo_root:/work" \
  -w /work \
  "$image" \
  bash -lc "
    set -euo pipefail
    # Focused Rust checks protect the newly declared scenario surfaces without
    # turning default GitHub CI back into a full test/smoke pipeline.
    cargo test --locked static_site_serves_byte_ranges -- --nocapture
    cargo test --locked static_site_rejects_unsatisfiable_byte_range -- --nocapture
    cargo test --locked service_discovery_accepts_registry_mappings -- --nocapture
    cargo test --locked validate_rejects_unknown_service_discovery_registry -- --nocapture
    cargo test --locked integration_deep_static_site_supports_range_downloads -- --nocapture

    # Build the CLI once, then validate that the broad scenario sample is both
    # syntactically valid and visible through agent-friendly inspection commands.
    cargo build --locked
    ./target/debug/proxysss -config $config check-config
    ./target/debug/proxysss -config $config config explain | tee /tmp/proxysss-explain.txt
    ./target/debug/proxysss -config $config config routes | tee /tmp/proxysss-routes.txt
    ./target/debug/proxysss config capabilities | tee /tmp/proxysss-capabilities.txt
    ./target/debug/proxysss config nginx-parity --format yaml | tee /tmp/proxysss-nginx-parity.yaml

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
