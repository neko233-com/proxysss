<#
.SYNOPSIS
    Docker Ubuntu 24 validation for the broad proxysss gateway scenario surface.

.DESCRIPTION
    This PowerShell wrapper mirrors scripts/verify-docker-scenarios.sh for
    Windows operators. It validates the all-scenarios YAML, static Range
    downloads, service discovery config, CLI capability output, and nginx-parity
    declarations inside the Ubuntu 24 Docker image used by benchmark work.
#>
param(
    [string]$Image = "proxysss-ubuntu24-scenarios",
    [string]$Config = "examples/all-scenarios.example.yaml"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Resolve-Path -LiteralPath $repoRoot

Push-Location $repoRoot
try {
    # Build the same Ubuntu 24 toolchain image used by the Linux benchmark
    # helpers, so scenario validation runs in a production-like Linux family.
    docker build -f docker/ubuntu24-bench.Dockerfile -t $Image .

    # Keep the command body in one bash script inside the container. That avoids
    # PowerShell quoting differences changing the exact verification sequence.
    $containerScript = @"
set -euo pipefail

cargo test --locked static_site_serves_byte_ranges -- --nocapture
cargo test --locked static_site_rejects_unsatisfiable_byte_range -- --nocapture
cargo test --locked service_discovery_accepts_registry_mappings -- --nocapture
cargo test --locked validate_rejects_unknown_service_discovery_registry -- --nocapture
cargo test --locked integration_deep_static_site_supports_range_downloads -- --nocapture

cargo build --locked
./target/debug/proxysss -config $Config check-config
./target/debug/proxysss -config $Config config explain | tee /tmp/proxysss-explain.txt
./target/debug/proxysss -config $Config config routes | tee /tmp/proxysss-routes.txt
./target/debug/proxysss config capabilities | tee /tmp/proxysss-capabilities.txt
./target/debug/proxysss config nginx-parity --format yaml | tee /tmp/proxysss-nginx-parity.yaml

grep -q 'service discovery : enabled=true, registries=3, mappings=3' /tmp/proxysss-explain.txt
grep -q 'large file range downloads' /tmp/proxysss-capabilities.txt
grep -q 'service discovery registries' /tmp/proxysss-capabilities.txt
grep -q 'waf hotlink crawler controls' /tmp/proxysss-capabilities.txt
grep -q 'cdn origin and ipv6 edge' /tmp/proxysss-capabilities.txt
grep -q 'api gateway policy chain' /tmp/proxysss-nginx-parity.yaml
grep -q 'large file range downloads' /tmp/proxysss-nginx-parity.yaml
grep -q 'mapping api-from-consul registry=consul-main service=spring-api' /tmp/proxysss-routes.txt
"@

    docker run --rm `
        -v "${repoRoot}:/work" `
        -w /work `
        $Image `
        bash -lc $containerScript

    Write-Host "proxysss Docker scenario verification passed" -ForegroundColor Green
}
finally {
    Pop-Location
}
