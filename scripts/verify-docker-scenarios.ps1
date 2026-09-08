param(
    [string]$Image = 'proxysss-ubuntu24-amd64-bench:local',
    [string]$Config = 'examples/all-scenarios.example.yaml',
    [ValidateRange(1,10)][int]$Repeat = 2,
    [switch]$Performance
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
Initialize-ProjectArtifacts
$repoRoot = $script:ProjectRoot
$container = 'proxysss-verify'
$owner = 'proxysss-project-verification'
$lockPath = Get-ProjectArtifactPath '.tmp/docker-verify.lock'
try { $lock = [IO.File]::Open($lockPath, 'OpenOrCreate', 'ReadWrite', 'None') }
catch { throw 'Another Docker verification is running.' }
Push-Location $repoRoot
$owned = $false
try {
    $configPath = [IO.Path]::GetFullPath((Join-Path $repoRoot $Config))
    if (-not $configPath.StartsWith($repoRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $configPath)) { throw 'Verification config must exist inside this project' }
    $configRelative = $configPath.Substring($repoRoot.Length + 1).Replace('\','/')
    $exists = docker ps -aq --filter "name=^/${container}$"
    if ($LASTEXITCODE -ne 0) { throw 'Docker is unavailable' }
    if ($exists) {
        $inspect = docker inspect $container | ConvertFrom-Json
        $label = $inspect[0].Config.Labels.'com.proxysss.owner'
        if ($label -ne $owner) { throw "Container name $container belongs to another task; refusing to delete it" }
        docker rm -f $container | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Previous verification container cleanup failed' }
    }
    $owned = $true
    Reset-ProjectArtifactDirectory '.tmp/docker-scenarios' | Out-Null
    $imageId = docker image ls -q $Image
    if (-not $imageId) {
        docker build -f docker/ubuntu24-bench.Dockerfile -t $Image .
        if ($LASTEXITCODE -ne 0) { throw 'Verification image build failed' }
    }
    # OS probes, tests and mixed diagnostics share one fixed-name container.
    $performanceValue = if ($Performance) { '1' } else { '0' }
    docker run --name $container --label "com.proxysss.owner=$owner" --rm `
        -e CARGO_HOME=/work/.cache/cargo -e CARGO_TARGET_DIR=/work/.benchmark/linux-target `
        -e "VERIFY_CONFIG=$configRelative" -e "VERIFY_REPEAT=$Repeat" -e "VERIFY_PERFORMANCE=$performanceValue" `
        -v "${repoRoot}:/work" -w /work $Image bash /work/scripts/verify-docker-container.sh
    if ($LASTEXITCODE -ne 0) { throw "Docker verification failed with exit code $LASTEXITCODE" }
    Write-Host 'proxysss Docker scenario verification passed'
} finally {
    if ($owned) {
        $remaining = docker ps -aq --filter "name=^/${container}$"
        if ($remaining) { docker rm -f $container | Out-Null }
    }
    Pop-Location
    $lock.Dispose()
}
