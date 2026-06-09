<#
.SYNOPSIS
    Super-heavy verification: unit tests, deep integration, E2E scripts, benchmark + reports.

.PARAMETER SkipBench
    Skip throughput benchmark (faster).

.PARAMETER QuickBench
    Run benchmark in quick mode (128c x 10s).
#>
param(
    [switch]$SkipBench,
    [switch]$QuickBench,
    [string]$Binary = "",
    [string]$ReportDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$startedAt = Get-Date

if (-not $ReportDir) {
    $ReportDir = Join-Path $repoRoot ".benchmark\verify-deep\latest"
}
if (Test-Path $ReportDir) {
    Remove-Item -Recurse -Force $ReportDir
}
New-Item -ItemType Directory -Path $ReportDir -Force | Out-Null

$results = [System.Collections.Generic.List[object]]::new()

function Add-Result {
    param(
        [string]$Category,
        [string]$Name,
        [bool]$Passed,
        [string]$Detail = ""
    )
    $script:results.Add([pscustomobject]@{
            category = $Category
            name     = $Name
            status   = if ($Passed) { "PASS" } else { "FAIL" }
            detail   = $Detail
        })
    if (-not $Passed) {
        throw "VERIFY DEEP FAILED [$Category] $Name${Detail}"
    }
}

function Write-Step([string]$Message) {
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

Push-Location $repoRoot
try {
    Write-Step "1/7 cargo fmt --check"
    cargo fmt --all -- --check
    Add-Result "quality" "rustfmt" $true

    Write-Step "2/7 cargo clippy"
    cargo clippy --workspace --all-targets -- -D warnings
    Add-Result "quality" "clippy" $true

    Write-Step "3/7 cargo test (full unit + integration suite)"
    cargo test --workspace --all-targets
    if ($LASTEXITCODE -ne 0) { Add-Result "tests" "cargo test" $false "exit $LASTEXITCODE" }
    Add-Result "tests" "cargo test" $true

    Write-Step "4/7 integration_e2e"
    cargo test integration_e2e -- --nocapture
    if ($LASTEXITCODE -ne 0) { Add-Result "integration" "integration_e2e" $false }
    Add-Result "integration" "integration_e2e" $true

    Write-Step "5/7 integration_deep"
    cargo test integration_deep -- --nocapture
    if ($LASTEXITCODE -ne 0) { Add-Result "integration" "integration_deep" $false }
    Add-Result "integration" "integration_deep" $true

    Write-Step "6/7 verify-e2e (embedded TS + fresh-init gateway)"
    & (Join-Path $repoRoot "scripts/verify-e2e.ps1") -Binary $Binary
    Add-Result "scripts" "verify-e2e" $true

    if (-not $SkipBench) {
        Write-Step "7/7 benchmark-gateways + HTML/MD reports"
        if ($QuickBench) {
            & (Join-Path $repoRoot "scripts/benchmark-gateways.ps1") -Quick
        } else {
            & (Join-Path $repoRoot "scripts/benchmark-gateways.ps1")
        }
        $benchRun = Join-Path $repoRoot ".benchmark\runs\latest"
        Copy-Item (Join-Path $benchRun "results.json") (Join-Path $ReportDir "benchmark-results.json") -Force
        Copy-Item (Join-Path $benchRun "report.md") (Join-Path $ReportDir "benchmark-report.md") -Force
        Copy-Item (Join-Path $benchRun "report.html") (Join-Path $ReportDir "benchmark-report.html") -Force
        Add-Result "benchmark" "throughput gate" $true (Get-Content (Join-Path $benchRun "report.md") -TotalCount 8 -Raw)
    } else {
        Write-Host "==> 7/7 benchmark skipped (-SkipBench)"
        Add-Result "benchmark" "throughput" $true "skipped"
    }
}
finally {
    Pop-Location
}

$endedAt = Get-Date
$duration = [math]::Round(($endedAt - $startedAt).TotalSeconds, 1)

$md = @(
    "# proxysss deep verification report",
    "",
    "- Started: $($startedAt.ToString('u'))",
    "- Finished: $($endedAt.ToString('u'))",
    "- Duration: ${duration}s",
    "- Report dir: ``$ReportDir``",
    "",
    "## Matrix",
    "",
    "| Category | Check | Status | Detail |",
    "| --- | --- | --- | --- |"
)
foreach ($row in $results) {
    $detail = ($row.detail -replace '\|', '/').Replace("`n", " ")
    $md += "| $($row.category) | $($row.name) | $($row.status) | $detail |"
}
$md += ""
$md += "## Artifacts"
$md += ""
$md += "- ``verify-report.md`` (this file)"
$md += "- ``verify-report.html``"
if (-not $SkipBench) {
    $md += "- ``benchmark-report.md`` / ``benchmark-report.html``"
    $md += "- ``benchmark-results.json``"
}
$mdText = ($md -join "`n") + "`n"
$mdPath = Join-Path $ReportDir "verify-report.md"
$mdText | Set-Content -Path $mdPath -Encoding utf8

$tableRows = ($results | ForEach-Object {
        $detail = ($_.detail -replace '&', '&amp;' -replace '<', '&lt;' -replace '>', '&gt;')
        $statusClass = if ($_.status -eq "PASS") { "pass" } else { "fail" }
        "      <tr><td>$($_.category)</td><td>$($_.name)</td><td class='$statusClass'>$($_.status)</td><td>$detail</td></tr>"
    }) -join "`n"

$benchLink = ""
if (-not $SkipBench) {
    $benchLink = "<p>Benchmark tables: <a href='benchmark-report.html'>benchmark-report.html</a> · <a href='benchmark-report.md'>benchmark-report.md</a></p>"
}

$html = @"
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>proxysss deep verification</title>
  <style>
    body { font-family: Segoe UI, sans-serif; margin: 2rem; background: #0f172a; color: #e2e8f0; }
    table { border-collapse: collapse; width: 100%; background: #111827; }
    th, td { border: 1px solid #334155; padding: 0.6rem 0.75rem; text-align: left; }
    th { background: #1e293b; }
    .pass { color: #34d399; font-weight: 700; }
    .fail { color: #f87171; font-weight: 700; }
    a { color: #5eead4; }
  </style>
</head>
<body>
  <h1>proxysss deep verification</h1>
  <p>Duration: ${duration}s · $($results.Count) checks · all passed</p>
  $benchLink
  <table>
    <thead><tr><th>Category</th><th>Check</th><th>Status</th><th>Detail</th></tr></thead>
    <tbody>
$tableRows
    </tbody>
  </table>
</body>
</html>
"@
$htmlPath = Join-Path $ReportDir "verify-report.html"
$html | Set-Content -Path $htmlPath -Encoding utf8

Write-Host ""
Write-Host "ALL DEEP VERIFICATION CHECKS PASSED" -ForegroundColor Green
Write-Host "verify report markdown: $mdPath"
Write-Host "verify report html:     $htmlPath"
