param(
    [ValidateRange(1,10)][int]$Repeat = 2,
    [switch]$SkipClippy,
    [switch]$SkipPackage
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
Initialize-ProjectArtifacts
$lockPath = Get-ProjectArtifactPath '.tmp/verify-local.lock'
try { $lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None') }
catch { throw 'Another local verification is running; its artifacts will not be removed.' }
$report = Reset-ProjectArtifactDirectory '.tmp/verification/latest'

function Invoke-Checked([string]$Name, [scriptblock]$Command) {
    Write-Host "==> $Name"
    # Windows PowerShell 5 wraps native stderr as ErrorRecord, including Cargo's
    # successful progress messages. Judge native commands by their actual exit code.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $Command 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath (Join-Path $report "$Name.log")
        $commandExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($commandExit -ne 0) { throw "$Name failed (exit $commandExit)" }
}

Push-Location $script:ProjectRoot
try {
    & (Join-Path $PSScriptRoot 'verify-artifact-policy.ps1')
    Invoke-Checked 'fmt' { cargo fmt --all -- --check }
    if (-not $SkipClippy) { Invoke-Checked 'clippy' { cargo clippy --workspace --all-targets --locked -- -D warnings } }
    $passes = @()
    for ($pass = 1; $pass -le $Repeat; $pass++) {
        $sandbox = Reset-ProjectArtifactDirectory '.tmp/tests'
        try {
            Invoke-Checked "tests-$pass" { cargo test --workspace --all-targets --locked }
            $passes += @{ pass = $pass; passed = $true }
        } finally {
            Reset-ProjectArtifactDirectory '.tmp/tests' | Out-Null
        }
        if ((Get-ChildItem -LiteralPath $sandbox -Force | Measure-Object).Count -ne 0) {
            throw 'Test sandbox was not empty after cleanup'
        }
    }
    if (-not $SkipPackage) {
        & (Join-Path $PSScriptRoot 'package-local.ps1')
        if ($LASTEXITCODE -ne 0) { throw 'Local packaging failed' }
    }
    @{ passes = $passes; test_sandbox_empty = $true; project = $script:ProjectRoot } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $report 'result.json') -Encoding utf8
    Write-Host "Verification passed; report: $report"
} finally {
    Pop-Location
    $lock.Dispose()
}
