param(
    [switch]$DryRun,
    [switch]$KeepTmp,
    [switch]$KeepDist,
    [switch]$KeepDocker
)

# Strict project-local test/build cache cleanup. This intentionally does not
# touch the global Cargo registry, shared benchmark roots, or unrelated Docker
# resources: proxysss can otherwise retain 100 GB+ in target/ and .benchmark/.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
$root = $script:ProjectRoot

function Get-DirectorySize([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return 0 }
    $sum = (Get-ChildItem -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue |
        Measure-Object Length -Sum).Sum
    if ($null -eq $sum) { return 0 }
    return $sum
}

function Remove-SafeDirectory([string]$RelativePath) {
    $path = Get-ProjectArtifactPath $RelativePath
    if (-not (Test-Path -LiteralPath $path)) { return }
    if ($DryRun) {
        Write-Host "would remove $RelativePath"
        return
    }
    Remove-Item -LiteralPath $path -Recurse -Force
}

$before = Get-PSDrive -Name ([IO.Path]::GetPathRoot($root).TrimEnd(':', '\'))
Write-Host ("Project cache cleanup in {0}" -f $root)

$directories = @('target', '.benchmark', '.cache')
if (-not $KeepDist) { $directories += 'dist' }
foreach ($relative in $directories) {
    Remove-SafeDirectory $relative
}

# Verification reports live under .tmp/verification; keep them only when the
# caller explicitly asks for post-run cleanup.
$tmp = Get-ProjectArtifactPath '.tmp'
if (Test-Path -LiteralPath $tmp) {
    if ($KeepTmp) {
        Get-ChildItem -LiteralPath $tmp -Force | Where-Object { $_.Name -ne 'verification' } |
            ForEach-Object {
                if ($DryRun) { Write-Host ("would remove .tmp/{0}" -f $_.Name) }
                else { Remove-Item -LiteralPath $_.FullName -Recurse -Force }
            }
        if (-not $DryRun) {
            $latest = Join-Path $tmp 'verification/latest'
            if (Test-Path -LiteralPath $latest) {
                Get-ChildItem -LiteralPath (Join-Path $tmp 'verification') -Force |
                    Where-Object { $_.Name -ne 'latest' } |
                    ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force }
            }
        }
    } else {
        Remove-SafeDirectory '.tmp'
    }
}

# One-off benchmark/CI scratch directories at the repository root.
$rootScratch = @('.tmp-bench-logs', '.tmp-gh-benchmark', '.tmp-gh-benchmark-latest', '.tmp-gh-benchmark-latest2', 'logs')
foreach ($relative in $rootScratch) {
    $path = [IO.Path]::GetFullPath((Join-Path $root $relative))
    if (-not $path.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Cleanup path escaped project root: $path"
    }
    if (Test-Path -LiteralPath $path) {
        if ($DryRun) { Write-Host "would remove $relative" }
        else { Remove-Item -LiteralPath $path -Recurse -Force }
    }
}

# Ignored root-level test helpers and logs. Never remove tracked files.
Get-ChildItem -LiteralPath $root -File -Force -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^(\.tmp-|tmp-|benchmark-helper)' } |
    ForEach-Object {
        $relative = $_.Name
        git -C $root check-ignore -q -- $relative
        $ignored = $LASTEXITCODE -eq 0
        if (-not $ignored) { return }
        if ($DryRun) { Write-Host "would remove $relative" }
        else { Remove-Item -LiteralPath $_.FullName -Force }
    }

if (-not $KeepDocker) {
    $container = 'proxysss-verify'
    $exists = docker ps -aq --filter "name=^/${container}$" 2>$null
    if ($LASTEXITCODE -eq 0 -and $exists) {
        $label = (docker inspect $container --format '{{index .Config.Labels "com.proxysss.owner"}}' 2>$null)
        if ($label -eq 'proxysss-project-verification') {
            if ($DryRun) { Write-Host "would remove container $container" }
            else {
                docker rm -f $container | Out-Null
                Write-Host "removed container $container"
            }
        }
    }
    foreach ($volume in @('proxysss-perf', 'proxysss-release-verify', 'proxysss-build-cache', 'proxysss-test-tmp', 'proxysss-arm-target', 'proxysss-musl-test-tmp')) {
        $existsVolume = docker volume ls --format '{{.Name}}' 2>$null | Where-Object { $_ -eq $volume }
        if ($existsVolume) {
            if ($DryRun) { Write-Host "would remove volume $volume" }
            else {
                docker volume rm $volume 2>$null | Out-Null
                if ($LASTEXITCODE -eq 0) { Write-Host "removed volume $volume" }
            }
        }
    }
}

if ($DryRun) { return }

$targetSize = Get-DirectorySize (Join-Path $root 'target')
$benchSize = Get-DirectorySize (Join-Path $root '.benchmark')
if ($targetSize -gt 1GB -or $benchSize -gt 1GB) {
    throw ("Cache cleanup did not reduce target/.benchmark below 1 GB (target={0:N2} GB, benchmark={1:N2} GB)" -f ($targetSize / 1GB), ($benchSize / 1GB))
}
$tmpSize = Get-DirectorySize (Join-Path $root '.tmp')
$after = Get-PSDrive -Name ([IO.Path]::GetPathRoot($root).TrimEnd(':', '\'))
Write-Host ("Cache cleanup complete: target={0:N2} GB benchmark={1:N2} GB tmp={2:N2} GB free={3:N2} GB" -f ($targetSize / 1GB), ($benchSize / 1GB), ($tmpSize / 1GB), ($after.Free / 1GB))
