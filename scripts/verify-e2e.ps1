<#
.SYNOPSIS
    Runs integration E2E checks with a fresh init config (no legacy APPDATA config).

.DESCRIPTION
    1. cargo integration_e2e tests (HTTP reverse proxy, WebSocket, HTTP/3 bind validation)
    2. embedded TypeScript verification
    3. fresh-init gateway smoke on ephemeral ports
#>
param(
    [string]$Binary = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Binary) {
    $exe = if ($IsWindows -or $env:OS -eq "Windows_NT") { "proxysss.exe" } else { "proxysss" }
    $Binary = Join-Path $repoRoot "target/release/$exe"
}
if (-not (Test-Path -LiteralPath $Binary)) {
    Write-Host "==> building release binary"
    Push-Location $repoRoot
    try {
        cargo build --release --locked
    } finally {
        Pop-Location
    }
}
$Binary = (Resolve-Path -LiteralPath $Binary).Path

function Write-Step([string]$Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Fail([string]$Message) { throw "VERIFY E2E FAILED: $Message" }

Write-Step "1/3 cargo integration_e2e tests"
Push-Location $repoRoot
try {
    cargo test --release integration_e2e -- --nocapture
    if ($LASTEXITCODE -ne 0) { Fail "integration_e2e tests failed" }
} finally {
    Pop-Location
}

Write-Step "2/3 embedded TypeScript engine verification"
& (Join-Path $repoRoot "scripts/verify-embedded-ts.ps1") -Binary $Binary

Write-Step "3/3 fresh-init gateway smoke (no legacy config)"
$work = Join-Path ([IO.Path]::GetTempPath()) ("proxysss-e2e-smoke-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    & $Binary init --dir $work --overwrite | Out-Null
    $port = 19001
    $cfg = Join-Path $work "proxysss.yaml"
    $content = Get-Content $cfg -Raw
    $content = $content -replace 'plain_bind: 0\.0\.0\.0:80', "plain_bind: 127.0.0.1:$port"
    $content = $content -replace 'tls_bind: 0\.0\.0\.0:443', "tls_bind: ''"
    $content = $content -replace 'h3_bind: 0\.0\.0\.0:443', "h3_bind: ''"
    $content = $content -replace 'bind: 127\.0\.0\.1:7777', 'bind: 127.0.0.1:19002'
    $content | Set-Content $cfg -Encoding utf8

    & $Binary -config $cfg check-config
    if ($LASTEXITCODE -ne 0) { Fail "check-config failed on fresh init config" }

    $proc = Start-Process -FilePath $Binary -ArgumentList @("-config", $cfg) -PassThru -WindowStyle Hidden
    try {
        $base = "http://127.0.0.1:$port"
        $ready = $false
        for ($i = 0; $i -lt 50; $i++) {
            Start-Sleep -Milliseconds 200
            try {
                $welcome = Invoke-WebRequest -Uri "$base/" -UseBasicParsing -TimeoutSec 2
                if ($welcome.StatusCode -ge 200) { $ready = $true; break }
            } catch { }
        }
        if (-not $ready) { Fail "fresh-init gateway did not become ready on $base" }

        $metrics = Invoke-WebRequest -Uri "$base/metrics" -UseBasicParsing -TimeoutSec 5
        if ($metrics.StatusCode -ne 200) { Fail "/metrics returned $($metrics.StatusCode)" }
        Write-Host "    fresh-init gateway OK (welcome + metrics)"
    } finally {
        if ($proc -and -not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "ALL E2E VERIFICATION CHECKS PASSED" -ForegroundColor Green
