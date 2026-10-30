param(
    [Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc','aarch64-pc-windows-msvc')][string]$Target,
    [Parameter(Mandatory)][string]$Wix
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
$source = Get-ProjectArtifactPath "target/$Target/release/proxysss.exe"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing Windows binary: $source" }
if (-not (Test-Path -LiteralPath $Wix -PathType Leaf)) { throw 'WiX executable not found.' }
$cargo = Get-Content -LiteralPath (Join-Path $script:ProjectRoot 'Cargo.toml') -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"') { throw 'Cannot read MSI product version.' }
$version = $Matches[1]
$arch = if ($Target.StartsWith('x86_64')) { 'x64' } else { 'arm64' }
$assetArch = if ($arch -eq 'x64') { 'amd64' } else { 'arm64' }
$componentGuid = if ($arch -eq 'x64') { '{62087420-D1BC-4253-8A14-3568703D55BA}' } else { '{8A096964-258A-4C66-B195-45E9C9C33D18}' }
$identity = [Text.Encoding]::UTF8.GetBytes("proxysss-msi|$Target|$version")
$hash = [Security.Cryptography.SHA256]::HashData($identity)
$productCode = ([Guid]::new([byte[]]$hash[0..15])).ToString('B').ToUpperInvariant()
$output = Get-ProjectArtifactPath "dist/proxysss-windows-$assetArch.msi"
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
$work = Get-ProjectArtifactPath ".tmp/msi-$assetArch"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$stampedBinary = Join-Path $work 'proxysss.exe'
Copy-Item -LiteralPath $source -Destination $stampedBinary -Force
& (Join-Path $PSScriptRoot 'stamp-windows-version.ps1') -Binary $stampedBinary -Version $version -WorkDir $work
& $Wix build -arch $arch -d "ProductVersion=$version" -d "BinaryPath=$stampedBinary" -d "ComponentGuid=$componentGuid" -d "ProductCode=$productCode" -d 'InstallFolderName=proxysss' `
    -o $output (Join-Path $script:ProjectRoot 'packaging/windows/proxysss.wxs')
if ($LASTEXITCODE -ne 0) { throw 'MSI build failed.' }
Write-Host "Windows MSI: $output"
