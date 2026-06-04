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

function Get-InstalledProxysssVersion {
    try {
        $text = (& proxysss --version 2>$null | Out-String).Trim()
        if ($LASTEXITCODE -eq 0 -and $text -match "([0-9]+\.[0-9]+\.[0-9]+)") {
            return $matches[1]
        }
    } catch {
    }
    return $null
}

function Assert-InstalledWindowsAsset([string]$ExpectedVersion) {
    $installed = Get-InstalledProxysssVersion
    if ($installed -eq $ExpectedVersion) {
        return
    }

    $installPath = Join-Path $env:LOCALAPPDATA "proxysss\proxysss.exe"
    if (!(Test-Path -LiteralPath $installPath)) {
        throw "installed proxysss.exe not found at $installPath"
    }

    $tmp = Join-Path $env:TEMP "proxysss-$ExpectedVersion-windows-amd64.exe"
    $assetTag = "v$ExpectedVersion"
    $url = "https://github.com/$Repo/releases/download/$assetTag/proxysss-windows-amd64.exe"
    try {
        Invoke-WebRequest -Uri $url -OutFile $tmp
    } catch {
        $assetName = "proxysss-windows-amd64.exe"
        gh release download $assetTag --repo $Repo --pattern $assetName --dir $env:TEMP --clobber
        $downloaded = Join-Path $env:TEMP $assetName
        Move-Item -Force $downloaded $tmp
    }

    $installedHash = (Get-FileHash -LiteralPath $installPath -Algorithm SHA256).Hash
    $expectedHash = (Get-FileHash -LiteralPath $tmp -Algorithm SHA256).Hash
    if ($installedHash -ne $expectedHash) {
        throw "installed binary hash mismatch for $assetTag"
    }
}

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
Assert-InstalledWindowsAsset $VersionNumber

if ($PreviousVersion) {
    $previousNumber = $PreviousVersion.TrimStart("v", "V")
    Write-Host "Downgrading to $PreviousVersion..."
    powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action downgrade -Version $PreviousVersion -SkipInit -NoServiceRestart
    Assert-InstalledWindowsAsset $previousNumber

    Write-Host "Restoring $Tag..."
    powershell -NoProfile -ExecutionPolicy Bypass -File $install -Action upgrade -Version $Tag -SkipInit -NoServiceRestart
    Assert-InstalledWindowsAsset $VersionNumber
}

Write-Host "Release install checks OK."
