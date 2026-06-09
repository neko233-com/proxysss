<#
.SYNOPSIS
    Full local verification: unit tests, E2E, and optional gateway benchmark.
#>
param(
    [switch]$SkipBench,
    [switch]$QuickBench
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    Write-Host "==> cargo fmt --check" -ForegroundColor Cyan
    cargo fmt --all -- --check

    Write-Host "==> cargo clippy" -ForegroundColor Cyan
    cargo clippy --workspace --all-targets -- -D warnings

    Write-Host "==> cargo test" -ForegroundColor Cyan
    cargo test --workspace --all-targets

    & (Join-Path $repoRoot "scripts/verify-e2e.ps1")

    Write-Host "==> verify-deep (optional full matrix; use scripts/verify-deep.ps1 for benchmark reports)" -ForegroundColor Yellow
    Write-Host "    run: powershell -File scripts/verify-deep.ps1 -QuickBench"

    if (-not $SkipBench) {
        if ($QuickBench) {
            & (Join-Path $repoRoot "scripts/benchmark-gateways.ps1") -Quick
        } else {
            & (Join-Path $repoRoot "scripts/benchmark-gateways.ps1")
        }
    }
} finally {
    Pop-Location
}
