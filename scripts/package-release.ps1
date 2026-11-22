param(
    [Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc','x86_64-unknown-linux-musl','aarch64-unknown-linux-musl','x86_64-apple-darwin','aarch64-apple-darwin')][string]$Target,
    [Parameter(Mandatory)][ValidatePattern('^(proxysss|deploy)-(windows|linux|darwin)-(amd64|arm64)\.(zip|tar\.gz)$')][string]$Asset
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
$windowsTarget = $Target.EndsWith('-windows-msvc')
if ($windowsTarget -ne $Asset.EndsWith('.zip')) { throw 'Archive format must match the target OS.' }
$binary = if ($windowsTarget) { 'proxysss.exe' } else { 'proxysss' }
$source = Get-ProjectArtifactPath "target/$Target/release/$binary"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing release binary: $source" }
if ($Target.EndsWith('-linux-musl')) {
    # Linux release assets must run without a host glibc or dynamic loader.
    $readelf = Get-Command readelf -ErrorAction SilentlyContinue
    if (-not $readelf) { throw 'readelf is required to verify static Linux release binaries.' }
    $dynamic = & $readelf.Source -d $source 2>&1
    if ($LASTEXITCODE -ne 0) { throw "readelf failed on $source" }
    if ($dynamic -match 'NEEDED') { throw "Linux release binary has dynamic dependencies:`n$dynamic" }
    $headers = & $readelf.Source -l $source 2>&1
    if ($LASTEXITCODE -ne 0) { throw "readelf program headers failed on $source" }
    if ($headers -match 'INTERP') { throw 'Linux release binary requests a dynamic interpreter.' }
    $file = Get-Command file -ErrorAction SilentlyContinue
    if ($file) {
        $fileInfo = & $file.Source $source 2>&1
        if ($LASTEXITCODE -ne 0 -or ($fileInfo | Out-String) -notmatch 'statically linked|static-pie linked') {
            throw "Linux release binary is not statically linked: $fileInfo"
        }
    }
}
if ($windowsTarget) {
    # Windows release bundles must not require a separately installed VC++
    # redistributable. Build them with -C target-feature=+crt-static and reject
    # any surviving VCRUNTIME/MSVCP/UCRT import before packaging.
    $dumpbinPath = $null
    $dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($dumpbin) { $dumpbinPath = $dumpbin.Source }
    if (-not $dumpbinPath) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path -LiteralPath $vswhere) {
            foreach ($pattern in @(
                'VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe',
                'VC\Tools\MSVC\**\bin\Hostarm64\arm64\dumpbin.exe'
            )) {
                $found = & $vswhere -latest -find $pattern 2>$null | Select-Object -First 1
                if ($found) { $found = $found.Trim() }
                if ($found -and (Test-Path -LiteralPath $found)) {
                    $dumpbinPath = $found
                    break
                }
            }
        }
    }
    if (-not $dumpbinPath) { throw 'dumpbin is required to verify the Windows runtime dependencies.' }
    $deps = & $dumpbinPath /dependents $source 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "dumpbin failed on $source`n$deps" }
    if ($deps -match '(?i)(VCRUNTIME\d*\.dll|MSVCP\d*\.dll|ucrtbase\.dll|api-ms-win-crt-[^\s]+\.dll)') {
        throw "Windows release binary still imports a redistributable runtime:`n$deps"
    }
}
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
    # GitHub's Linux runners package native tar members, but a local Windows
    # rehearsal cannot represent Unix modes. Set them explicitly when chmod is
    # available so official Linux archives always carry 0755 executables.
    if (-not $windowsTarget -and (Get-Command chmod -ErrorAction SilentlyContinue)) {
        & chmod 755 (Join-Path $stage $binary)
        & chmod 755 (Join-Path $stage 'scripts/install.sh')
        if ($LASTEXITCODE -ne 0) { throw 'Failed to set executable permissions in the release stage.' }
    }
    $pending = Get-ProjectArtifactPath ".tmp/$Asset"
    $output = Get-ProjectArtifactPath "dist/$Asset"
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
    if (Test-Path -LiteralPath $pending) { Remove-Item -LiteralPath $pending -Force }
    if ($windowsTarget) {
        # Windows bsdtar produces a smaller ZIP than Compress-Archive and
        # completes roughly twice as fast for the full documentation bundle.
        # Expand-Archive accepts the resulting ./ prefixed entries.
        & tar -a -c -f $pending -C $stage .
        if ($LASTEXITCODE -ne 0) { throw 'Release archive failed.' }
    } else {
        # Preserve the native Unix executable bit; stage timestamps are stable.
        & tar -czf $pending -C $stage .
        if ($LASTEXITCODE -ne 0) { throw 'Release archive failed.' }
    }
    if ($windowsTarget) {
        # Windows tar.exe can reject valid UTF-8 ZIP names under a legacy codepage.
        # Read ZIP metadata with .NET so Chinese documentation is checked faithfully.
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $archive = [IO.Compression.ZipFile]::OpenRead($pending)
        try { $entries = @($archive.Entries | ForEach-Object { $_.FullName }) }
        finally { $archive.Dispose() }
    } else {
        $entries = & tar -tf $pending
        if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect release archive.' }
    }
    $entries = $entries | ForEach-Object { $_ -replace '^\./', '' }
    foreach ($required in @($binary,'README.md','README-CN.md','docs/site.css','docs/site.js','docs/火山引擎-自动化所需权限.md','templates/admin.html','templates/admin.css','templates/docs.html','CHANGELOG.md','docs/cdn-origin.html','docs/security-performance.html','templates/cdn-origin.example.yaml','examples/security-performance.example.yaml','scripts/install.sh','scripts/install.ps1')) {
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
