param(
    [switch]$SkipClean,
    [switch]$Release,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

# Run the native Windows test suite without relying on the caller's shell
# having loaded a Visual Studio developer environment. Projects are still
# cleaned first so a stale target/ cannot masquerade as a fresh run.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$repo = Split-Path -Parent $PSScriptRoot

function Find-VcVars64 {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $install = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath 2>$null
        if ($install) {
            $candidate = Join-Path $install.Trim() 'VC\Auxiliary\Build\vcvars64.bat'
            if (Test-Path -LiteralPath $candidate) { return $candidate }
        }
    }
    $roots = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\BuildTools'),
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\Community'),
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\Professional'),
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\2022\Enterprise'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\2022\BuildTools'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\18\Community')
    )
    foreach ($root in $roots) {
        $candidate = Join-Path $root 'VC\Auxiliary\Build\vcvars64.bat'
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    return $null
}

function Import-VcVars([string]$VcVars) {
    Write-Host "Loading MSVC environment: $VcVars"
    $command = '"{0}" >nul 2>&1 && set' -f $VcVars
    $lines = & cmd.exe /d /c $command
    if ($LASTEXITCODE -ne 0) { throw "vcvars64.bat failed: $VcVars" }
    foreach ($line in $lines) {
        if ($line -match '^([^=]+)=(.*)$') {
            Set-Item -LiteralPath ("env:{0}" -f $matches[1]) -Value $matches[2]
        }
    }
}

$vcvars = Find-VcVars64
if ($vcvars) {
    Import-VcVars $vcvars
} else {
    Write-Warning 'vcvars64.bat not found; falling back to the current PATH.'
}

$cl = Get-Command cl.exe -ErrorAction SilentlyContinue
$link = Get-Command link.exe -ErrorAction SilentlyContinue
if (-not $cl -or -not $link) {
    throw 'MSVC cl.exe/link.exe not on PATH. Install Visual Studio Build Tools with the C++ workload.'
}
if ($link.Source -notmatch '\\VC\\Tools\\MSVC\\') {
    throw "link.exe resolves to $($link.Source), not the MSVC linker. Remove the shadowing tool from PATH or rerun this script."
}

if (-not $SkipClean) {
    & pwsh -NoProfile -File (Join-Path $repo 'scripts\clean-test-cache.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'clean-test-cache.ps1 failed' }
}

Write-Host ("cl:   {0}" -f $cl.Source)
Write-Host ("link: {0}" -f $link.Source)

$cargo = Get-Command cargo.exe -ErrorAction Stop
$cargoCommandArgs = @('test', '--locked')
if ($Release) { $cargoCommandArgs += '--release' }
$cargoCommandArgs += $CargoArgs
Write-Host ("cargo: {0} {1}" -f $cargo.Source, ($cargoCommandArgs -join ' '))
& $cargo.Source @cargoCommandArgs
exit $LASTEXITCODE
