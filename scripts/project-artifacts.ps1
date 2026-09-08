# Shared artifact locations; dot-source from local build/verification entry points.
$script:ProjectRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))

function Get-ProjectArtifactPath([string]$RelativePath) {
    if ([IO.Path]::IsPathRooted($RelativePath) -or $RelativePath -match '(^|[\\/])\.\.([\\/]|$)') {
        throw "Artifact path must be relative to the project: $RelativePath"
    }
    if ($RelativePath -notmatch '^(\.tmp|\.cache|target|dist|\.benchmark)[\\/].+') {
        throw "Artifact path must be a child of a known artifact directory: $RelativePath"
    }
    $path = [IO.Path]::GetFullPath((Join-Path $script:ProjectRoot $RelativePath))
    if (-not $path.StartsWith($script:ProjectRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Artifact path escapes the project: $path"
    }
    $current = $path
    while ($current -and $current -ne $script:ProjectRoot) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Artifact path crosses a link: $current" }
        }
        $current = Split-Path -Parent $current
    }
    return $path
}

function Reset-ProjectArtifactDirectory([string]$RelativePath) {
    $path = Get-ProjectArtifactPath $RelativePath
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
    New-Item -ItemType Directory -Path $path -Force | Out-Null
    return $path
}

function Initialize-ProjectArtifacts {
    $locations = @{
        TEMP = '.tmp/toolchain'; TMP = '.tmp/toolchain'; TMPDIR = '.tmp/toolchain'
        CARGO_HOME = '.cache/cargo'; CARGO_TARGET_DIR = 'target/local'
        GOCACHE = '.cache/go-build'; GOPATH = '.cache/go'; GOMODCACHE = '.cache/go-mod'; GOTMPDIR = '.tmp/go'
    }
    # Reuse the repository's existing Rust target directory, avoiding a second full build cache.
    $locations.Remove('CARGO_TARGET_DIR')
    foreach ($name in $locations.Keys) {
        $path = Get-ProjectArtifactPath $locations[$name]
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        [Environment]::SetEnvironmentVariable($name, $path, 'Process')
    }
    $env:CARGO_TARGET_DIR = Join-Path $script:ProjectRoot 'target'
}
