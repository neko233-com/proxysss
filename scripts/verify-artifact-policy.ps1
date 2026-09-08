param([switch]$Package)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
Initialize-ProjectArtifacts
foreach ($invalid in @('../outside', '.tmp/../outside', 'docs/source', 'C:\outside', 'target')) {
    $rejected = $false
    try { Get-ProjectArtifactPath $invalid | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Unsafe artifact path was accepted: $invalid" }
}
$sandbox = Reset-ProjectArtifactDirectory '.tmp/artifact-policy'
try {
    [IO.File]::WriteAllText((Join-Path $sandbox 'first.txt'), 'first run')
    Reset-ProjectArtifactDirectory '.tmp/artifact-policy' | Out-Null
    Reset-ProjectArtifactDirectory '.tmp/artifact-policy' | Out-Null
    if ((Get-ChildItem -LiteralPath $sandbox -Force | Measure-Object).Count -ne 0) { throw 'Repeated cleanup left artifacts' }
    if ($env:OS -eq 'Windows_NT') {
        $source = Join-Path $sandbox 'owned-target'
        $link = Join-Path $sandbox 'junction'
        New-Item -ItemType Directory -Path $source | Out-Null
        [IO.File]::WriteAllText((Join-Path $source 'sentinel.txt'), 'preserve')
        New-Item -ItemType Junction -Path $link -Target $source | Out-Null
        try {
            $rejected = $false
            try { Reset-ProjectArtifactDirectory '.tmp/artifact-policy/junction' | Out-Null } catch { $rejected = $true }
            if (-not $rejected) { throw 'Cleanup followed a junction' }
            if (-not (Test-Path -LiteralPath (Join-Path $source 'sentinel.txt'))) { throw 'Cleanup removed the junction target' }
        } finally {
            # Delete only this test-owned junction itself, never recurse through it.
            if ((Get-Item -LiteralPath $link -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { [IO.Directory]::Delete($link) }
        }
    }
} finally { Reset-ProjectArtifactDirectory '.tmp/artifact-policy' | Out-Null }

if ($Package) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $previousHash = $null
    for ($pass = 1; $pass -le 2; $pass++) {
        & (Join-Path $PSScriptRoot 'package-local.ps1')
        $archive = Get-ProjectArtifactPath 'dist/proxysss-local.zip'
        $zip = [IO.Compression.ZipFile]::OpenRead($archive)
        try {
            $names = @($zip.Entries | ForEach-Object { $_.FullName.Replace('\','/') })
            foreach ($required in @('proxysss.exe','README.md','docs/cdn-origin.html','templates/cdn-origin.example.yaml','docs/security-performance.html','templates/security-performance.example.yaml')) {
                if ($names -notcontains $required) { throw "Package is missing $required" }
            }
            if ($names | Where-Object { $_ -match '(^|/)(\.git|\.ssh|\.tmp|\.cache|target|logs|certs)(/|$)' -or $_ -eq 'proxysss.yaml' }) {
                throw 'Package contains local state or secrets'
            }
        } finally { $zip.Dispose() }
        $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        if ($previousHash -and $hash -ne $previousHash) { throw 'Unchanged repeated packaging produced different bytes' }
        $previousHash = $hash
        if ((Get-ChildItem -LiteralPath (Get-ProjectArtifactPath '.tmp/package-local') -Force | Measure-Object).Count -ne 0) { throw 'Package staging was not cleaned' }
    }
    Write-Host "Repeated package SHA256: $previousHash"
}
Write-Host 'Artifact path, cleanup and idempotence checks passed'
