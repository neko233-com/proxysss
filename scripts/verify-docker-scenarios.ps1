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
    if ($LASTEXITCODE -ne 0) {
        throw "Docker scenario image build failed with exit code $LASTEXITCODE"
    }

    # Keep the command body in one bash script inside the container. That avoids
    # PowerShell quoting differences changing the exact verification sequence.
    $containerScript = @"
set -euo pipefail

cargo test --locked

cargo build --locked
proxysss_bin=/target/debug/proxysss
`$proxysss_bin -config $Config check-config
`$proxysss_bin -config $Config config explain | tee /tmp/proxysss-explain.txt
`$proxysss_bin -config $Config config routes | tee /tmp/proxysss-routes.txt
`$proxysss_bin config capabilities | tee /tmp/proxysss-capabilities.txt
`$proxysss_bin config nginx-parity --format yaml | tee /tmp/proxysss-nginx-parity.yaml

grep -q 'service discovery : enabled=true, registries=3, mappings=3' /tmp/proxysss-explain.txt
grep -q 'large file range downloads' /tmp/proxysss-capabilities.txt
grep -q 'service discovery registries' /tmp/proxysss-capabilities.txt
grep -q 'waf hotlink crawler controls' /tmp/proxysss-capabilities.txt
grep -q 'cdn origin and ipv6 edge' /tmp/proxysss-capabilities.txt
grep -q 'api gateway policy chain' /tmp/proxysss-nginx-parity.yaml
grep -q 'large file range downloads' /tmp/proxysss-nginx-parity.yaml
grep -q 'mapping api-from-consul registry=consul-main service=spring-api' /tmp/proxysss-routes.txt
"@
    # Windows here-strings use CRLF. bash receives the script as an argument,
    # so Git's checkout normalization cannot remove the carriage returns.
    $containerScript = $containerScript.Replace("`r", "")

    docker run --rm `
        -e CARGO_TARGET_DIR=/target `
        -v proxysss-scenario-target:/target `
        -v "${repoRoot}:/work" `
        -w /work `
        $Image `
        bash -lc $containerScript
    if ($LASTEXITCODE -ne 0) {
        throw "Docker scenario verification failed with exit code $LASTEXITCODE"
    }

    Write-Host "proxysss Docker scenario verification passed" -ForegroundColor Green
}
finally {
    Pop-Location
}
