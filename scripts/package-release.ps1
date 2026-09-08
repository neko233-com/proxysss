param(
    [Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc','x86_64-unknown-linux-gnu','aarch64-unknown-linux-gnu','x86_64-apple-darwin','aarch64-apple-darwin')][string]$Target,
    [Parameter(Mandatory)][ValidatePattern('^(proxysss|deploy)-(windows|linux|darwin)-(amd64|arm64)\.(zip|tar\.gz)$')][string]$Asset
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
$windowsTarget = $Target.EndsWith('-windows-msvc')
if ($windowsTarget -ne $Asset.EndsWith('.zip')) { throw 'Archive format must match the target OS.' }
$binary = if ($windowsTarget) { 'proxysss.exe' } else { 'proxysss' }
$source = Get-ProjectArtifactPath "target/$Target/release/$binary"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing release binary: $source" }
$lockPath = Get-ProjectArtifactPath '.tmp/release-package.lock'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $lockPath) | Out-Null
$lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None')
try {
    $stage = Reset-ProjectArtifactDirectory '.tmp/release-package'
    Copy-Item -LiteralPath $source -Destination $stage
    foreach ($name in @('README.md','README-CN.md','CHANGELOG.md','proxysss-script.d.ts','ts-how-to-use.md','nginx-to-proxysss.md','caddy-to-proxysss.md','benchmark-linux.md','docs','examples','templates')) {
        Copy-Item -LiteralPath (Join-Path $script:ProjectRoot $name) -Destination $stage -Recurse
    }
    $scriptDir = New-Item -ItemType Directory -Path (Join-Path $stage 'scripts')
    foreach ($name in @('install.sh','install.ps1')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $scriptDir
    }
    $stamp = [DateTime]::SpecifyKind([DateTime]'2000-01-01', [DateTimeKind]::Utc)
    Get-ChildItem -LiteralPath $stage -Recurse -Force | ForEach-Object { $_.LastWriteTimeUtc = $stamp }
    $pending = Get-ProjectArtifactPath ".tmp/$Asset"
    $output = Get-ProjectArtifactPath "dist/$Asset"
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
    if (Test-Path -LiteralPath $pending) { Remove-Item -LiteralPath $pending -Force }
    if ($windowsTarget) {
        Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $pending
    } else {
        # Preserve the native Unix executable bit; stage timestamps are stable.
        & tar -czf $pending -C $stage .
        if ($LASTEXITCODE -ne 0) { throw 'Release archive failed.' }
    }
    $entries = & tar -tf $pending
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect release archive.' }
    $entries = $entries | ForEach-Object { $_ -replace '^\./', '' }
    foreach ($required in @($binary,'README.md','README-CN.md','docs/site.css','docs/site.js','templates/admin.html','templates/admin.css','templates/docs.html','CHANGELOG.md','docs/cdn-origin.html','docs/security-performance.html','templates/cdn-origin.example.yaml','examples/security-performance.example.yaml','scripts/install.sh','scripts/install.ps1')) {
        if ($entries -notcontains $required) { throw "Release archive is missing $required" }
    }
    if ($entries -match '(^|/)(\.git|\.ssh|\.cache|\.tmp|target|logs)(/|$)|(^|/)proxysss\.yaml$') {
        throw 'Release archive contains local state.'
    }
    Move-Item -LiteralPath $pending -Destination $output -Force
    Write-Host "Release package: $output"
    Get-FileHash -LiteralPath $output -Algorithm SHA256 | Format-List
} finally {
    if ($pending -and (Test-Path -LiteralPath $pending)) { Remove-Item -LiteralPath $pending -Force }
    Reset-ProjectArtifactDirectory '.tmp/release-package' | Out-Null
    $lock.Dispose()
}
