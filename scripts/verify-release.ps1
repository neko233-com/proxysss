param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$PreviousVersion = "",
    [switch]$SkipInstallChecks
)

$ErrorActionPreference = "Stop"
$Repo = "neko233-com/proxysss"
$Tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
$VersionNumber = $Tag.TrimStart("v", "V")

function Require-Command([string]$Name) {
    if (!(Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name not found"
    }
}

Require-Command gh

$assets = @(
    "proxysss-windows-amd64.exe",
    "proxysss-windows-arm64.exe",
    "proxysss-linux-amd64",
    "proxysss-linux-arm64",
    "proxysss-darwin-amd64",
    "proxysss-darwin-arm64"
)

Write-Host "Checking GitHub release $Repo@$Tag..."
$releaseJson = gh release view $Tag --repo $Repo --json tagName,isDraft,isPrerelease,assets | ConvertFrom-Json
if ($releaseJson.tagName -ne $Tag) {
    throw "release tag mismatch: $($releaseJson.tagName)"
}
if ($releaseJson.isDraft) {
    throw "release is draft"
}

$actualAssets = @($releaseJson.assets | ForEach-Object { $_.name })
foreach ($asset in $assets) {
    if ($actualAssets -notcontains $asset) {
        throw "missing release asset: $asset"
    }
}
Write-Host "Release assets OK."

if ($SkipInstallChecks) {
    exit 0
}

$install = Join-Path $PSScriptRoot "install.ps1"

if ($PreviousVersion) {
    Write-Host "Installing previous version $PreviousVersion..."
    powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action install -Version $PreviousVersion -AllowDowngrade -SkipInit -NoServiceRestart
}

Write-Host "Upgrading to $Tag..."
powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action upgrade -Version $Tag -SkipInit -NoServiceRestart
$upgraded = (& proxysss --version | Out-String).Trim()
if ($upgraded -notmatch [regex]::Escape($VersionNumber)) {
    throw "upgrade verification failed: $upgraded"
}

if ($PreviousVersion) {
    $previousNumber = $PreviousVersion.TrimStart("v", "V")
    Write-Host "Downgrading to $PreviousVersion..."
    powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action downgrade -Version $PreviousVersion -SkipInit -NoServiceRestart
    $downgraded = (& proxysss --version | Out-String).Trim()
    if ($downgraded -notmatch [regex]::Escape($previousNumber)) {
        throw "downgrade verification failed: $downgraded"
    }

    Write-Host "Restoring $Tag..."
    powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action upgrade -Version $Tag -SkipInit -NoServiceRestart
}

Write-Host "Release install checks OK."
