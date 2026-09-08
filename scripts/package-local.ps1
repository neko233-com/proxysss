param([ValidateSet('debug','release-fast','release')][string]$Profile = 'release-fast')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
Initialize-ProjectArtifacts
$lockPath = Get-ProjectArtifactPath '.tmp/package-local.lock'
try { $lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None') }
catch { throw 'Another local package build is running.' }
Push-Location $script:ProjectRoot
try {
    if ($Profile -eq 'debug') { cargo build --locked } else { cargo build --locked --profile $Profile }
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    $stage = Reset-ProjectArtifactDirectory '.tmp/package-local'
    $output = Get-ProjectArtifactPath 'dist/proxysss-local.zip'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
    $exe = if ($env:OS -eq 'Windows_NT') { 'proxysss.exe' } else { 'proxysss' }
    Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR "$Profile/$exe") -Destination $stage
    foreach ($name in @('README.md','CHANGELOG.md','proxysss-script.d.ts','docs','examples','templates')) {
        Copy-Item -LiteralPath (Join-Path $script:ProjectRoot $name) -Destination $stage -Recurse
    }
    # Stable entry times make unchanged repeated builds produce identical archives.
    $stamp = [DateTime]::SpecifyKind([DateTime]'2000-01-01', [DateTimeKind]::Utc)
    Get-ChildItem -LiteralPath $stage -Recurse -Force | ForEach-Object { $_.LastWriteTimeUtc = $stamp }
    $pending = Get-ProjectArtifactPath '.tmp/package-local.zip'
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $pending -Force
    # Publish only a complete archive, preserving the previous package on failure.
    Move-Item -LiteralPath $pending -Destination $output -Force
    Write-Host "Local package: $output"
} finally {
    Reset-ProjectArtifactDirectory '.tmp/package-local' | Out-Null
    $pending = Get-ProjectArtifactPath '.tmp/package-local.zip'
    if (Test-Path -LiteralPath $pending) { Remove-Item -LiteralPath $pending -Force }
    Pop-Location
    $lock.Dispose()
}
