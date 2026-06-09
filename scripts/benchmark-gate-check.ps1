param(
    [Parameter(Mandatory = $true)]
    [string]$ResultsFile,
    [string]$BaselineFile = ""
)

$ErrorActionPreference = "Stop"

if (-not $BaselineFile) {
    $BaselineFile = Join-Path $PSScriptRoot "benchmark-baseline.json"
}
if (-not (Test-Path -LiteralPath $ResultsFile)) {
    throw "benchmark results not found: $ResultsFile"
}
if (-not (Test-Path -LiteralPath $BaselineFile)) {
    throw "benchmark baseline not found: $BaselineFile"
}

$results = Get-Content -LiteralPath $ResultsFile -Raw | ConvertFrom-Json
$baseline = Get-Content -LiteralPath $BaselineFile -Raw | ConvertFrom-Json

$proxysss = $results | Where-Object { $_.name -eq "proxysss" } | Select-Object -First 1
$nginx = $results | Where-Object { $_.name -eq "nginx" } | Select-Object -First 1

if (-not $proxysss) { throw "results missing proxysss entry" }
if (-not $nginx) { throw "results missing nginx entry" }

foreach ($row in $results) {
    if ($row.errors -gt $baseline.max_error_count) {
        throw "benchmark gate failed: $($row.name) reported $($row.errors) errors (max $($baseline.max_error_count))"
    }
}

if ($nginx.ops_per_sec -le 0) {
    throw "benchmark gate failed: nginx ops_per_sec is zero"
}

$ratio = [double]$proxysss.ops_per_sec / [double]$nginx.ops_per_sec
$minRatio = [double]$baseline.min_proxysss_vs_nginx_ops_ratio

Write-Host "benchmark gate: proxysss=$([math]::Round($proxysss.ops_per_sec, 2)) ops/s nginx=$([math]::Round($nginx.ops_per_sec, 2)) ops/s ratio=$([math]::Round($ratio, 3)) min=$minRatio"

if ($ratio -lt $minRatio) {
    throw "benchmark gate failed: proxysss/nginx ops ratio $([math]::Round($ratio, 3)) < required $minRatio"
}

Write-Host "benchmark gate passed" -ForegroundColor Green
