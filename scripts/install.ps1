param(
    [Parameter(Position = 0)]
    [string]$Version = "latest",

    [ValidateSet("install", "update", "upgrade", "downgrade")]
    [string]$Action = "install",

    [switch]$AllowDowngrade,
    [switch]$NoServiceRestart,
    [switch]$SkipInit,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$BinaryName = "proxysss"
$Repo = "neko233-com/proxysss"
$InstallDir = Join-Path $env:LOCALAPPDATA $BinaryName
$Dest = Join-Path $InstallDir "$BinaryName.exe"

function Get-NormalizedVersion([string]$Value) {
    $v = $Value.Trim()
    while ($v.StartsWith("v") -or $v.StartsWith("V")) { $v = $v.Substring(1) }
    return $v
}

function Get-ArchName {
    if ([Environment]::Is64BitOperatingSystem) {
        if ($env:PROCESSOR_ARCHITECTURE -match "ARM64") {
            return "arm64"
        }
        return "amd64"
    }
    return "amd64"
}

function Test-PathInUserPath([string]$Dir) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) { return $false }

    $normalizedDir = (Resolve-Path -LiteralPath $Dir).Path.TrimEnd("\\")
    foreach ($entry in $userPath -split ";") {
        if ([string]::IsNullOrWhiteSpace($entry)) { continue }
        try {
            $normalizedEntry = (Resolve-Path -LiteralPath $entry -ErrorAction Stop).Path.TrimEnd("\\")
            if ($normalizedEntry -ieq $normalizedDir) { return $true }
        } catch {
            if ($entry.TrimEnd("\\") -ieq $normalizedDir) { return $true }
        }
    }

    return $false
}

function Add-ToUserPath([string]$Dir) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $normalizedDir = (Resolve-Path -LiteralPath $Dir).Path

    if (Test-PathInUserPath $normalizedDir) { return $false }

    $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) {
        $normalizedDir
    } else {
        "$normalizedDir;$userPath"
    }

    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    return $true
}

function Notify-PathChanged {
    $signature = @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@

    try {
        Add-Type -MemberDefinition $signature -Name NativeMethods -Namespace Win32 -ErrorAction Stop
        $null = [UIntPtr]::Zero
        [Win32.NativeMethods]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, "Environment", 2, 5000, [ref]$null) | Out-Null
    } catch {
    }
}

function Install-DenoIfMissing {
    if (Get-Command deno -ErrorAction SilentlyContinue) {
        return
    }

    Write-Host "Deno not found, installing Deno for TypeScript script runtime..."
    if ($DryRun) {
        Write-Host "[dry-run] irm https://deno.land/install.ps1 | iex"
        return
    }

    irm https://deno.land/install.ps1 | iex

    $denoBin = Join-Path $env:USERPROFILE ".deno\bin"
    if (Test-Path $denoBin) {
        Add-ToUserPath $denoBin | Out-Null
        $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                    [Environment]::GetEnvironmentVariable("Path", "User")
    }
}

function Get-InstalledVersion([string]$ExePath) {
    if (!(Test-Path $ExePath)) {
        return $null
    }

    try {
        $output = & $ExePath --version 2>$null
        if ($LASTEXITCODE -ne 0) {
            return $null
        }
        $text = ($output | Out-String).Trim()
        $match = [regex]::Match($text, "([0-9]+\.[0-9]+\.[0-9]+)")
        if ($match.Success) {
            return $match.Groups[1].Value
        }
    } catch {
        return $null
    }

    return $null
}

function Get-LatestReleaseVersion {
    $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "proxysss-install-script" }
        if ($null -ne $response.tag_name) {
            return (Get-NormalizedVersion $response.tag_name)
        }
    } catch {
        Write-Host "warning: failed to query latest release version from GitHub API" -ForegroundColor Yellow
    }
    return $null
}

function Compare-VersionNullable([string]$Left, [string]$Right) {
    if ([string]::IsNullOrWhiteSpace($Left) -or [string]::IsNullOrWhiteSpace($Right)) {
        return $null
    }
    return ([version]$Left).CompareTo([version]$Right)
}

function Test-ServiceTaskExists {
    schtasks /Query /TN $BinaryName *> $null
    return ($LASTEXITCODE -eq 0)
}

function Stop-ServiceIfPresent([string]$ExePath) {
    if ($NoServiceRestart) {
        return
    }
    if (!(Test-ServiceTaskExists)) {
        return
    }
    if (!(Test-Path $ExePath)) {
        return
    }

    Write-Host "Stopping existing service before binary replacement..."
    if ($DryRun) {
        Write-Host "[dry-run] $ExePath service stop"
        return
    }

    try {
        & $ExePath service stop | Out-Null
    } catch {
        Write-Host "warning: failed to stop service, continuing with replacement" -ForegroundColor Yellow
    }
}

function Start-ServiceIfPresent([string]$ExePath) {
    if ($NoServiceRestart) {
        return
    }
    if (!(Test-ServiceTaskExists)) {
        return
    }

    Write-Host "Starting service with updated binary..."
    if ($DryRun) {
        Write-Host "[dry-run] $ExePath service start"
        return
    }

    try {
        & $ExePath service start | Out-Null
    } catch {
        Write-Host "warning: failed to start service, run '$BinaryName service start' manually" -ForegroundColor Yellow
    }
}

function Resolve-TargetVersion([string]$RequestedVersion) {
    if ($RequestedVersion -eq "latest" -or [string]::IsNullOrWhiteSpace($RequestedVersion)) {
        return @{ Label = "latest"; Version = (Get-LatestReleaseVersion) }
    }

    $normalized = Get-NormalizedVersion $RequestedVersion
    return @{ Label = "v$normalized"; Version = $normalized }
}

$arch = Get-ArchName
$asset = "${BinaryName}-windows-${arch}.exe"
$target = Resolve-TargetVersion $Version
$currentVersion = Get-InstalledVersion $Dest
$targetVersion = $target.Version
$targetLabel = $target.Label

if ($null -ne $currentVersion) {
    Write-Host "Current installed version: $currentVersion"
} else {
    Write-Host "Current installed version: none"
}

if ($null -ne $targetVersion) {
    Write-Host "Requested target version: $targetVersion ($targetLabel)"
} else {
    Write-Host "Requested target version: unknown ($targetLabel)"
}

$cmp = Compare-VersionNullable $targetVersion $currentVersion
if ($null -ne $cmp) {
    if ($cmp -eq 0) {
        Write-Host "Target version is already installed. Nothing to do."
        exit 0
    }

    if ($cmp -lt 0 -and !$AllowDowngrade -and $Action -ne "downgrade") {
        throw "Requested version $targetVersion is lower than installed version $currentVersion. Use -Action downgrade or -AllowDowngrade."
    }

    if ($cmp -lt 0 -and ($Action -eq "install" -or $Action -eq "upgrade" -or $Action -eq "update")) {
        if ($AllowDowngrade) {
            Write-Host "warning: performing downgrade under -AllowDowngrade" -ForegroundColor Yellow
        } else {
            throw "Action '$Action' does not allow downgrade. Use -Action downgrade."
        }
    }

    if ($cmp -gt 0 -and $Action -eq "downgrade") {
        throw "Action 'downgrade' requires a lower target version than current ($currentVersion)."
    }
}

$url = if ($targetLabel -eq "latest") {
    "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    "https://github.com/$Repo/releases/download/$targetLabel/$asset"
}

Write-Host "Installing action '$Action' for windows/$arch..."
Write-Host "Downloading $url..."

if ($DryRun) {
    Write-Host "[dry-run] mkdir $InstallDir"
    Write-Host "[dry-run] download => $Dest"
    Write-Host "[dry-run] init/check/service install flow"
    exit 0
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Stop-ServiceIfPresent $Dest

$tmp = Join-Path $InstallDir ("$BinaryName.tmp.exe")
Invoke-WebRequest -Uri $url -OutFile $tmp
Move-Item -Force $tmp $Dest

if (Add-ToUserPath $InstallDir) {
    Write-Host "Added $InstallDir to user PATH."
}

Notify-PathChanged
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

Install-DenoIfMissing

if (!$SkipInit) {
    & $Dest init
    & $Dest check-config
}

if (Test-ServiceTaskExists) {
    Start-ServiceIfPresent $Dest
} else {
    & $Dest service install
}

Write-Host ""
Write-Host "Installed to $Dest"
Write-Host "Gateway port: 23380 (TCP for HTTP/1.1 + HTTP/2, UDP for HTTP/3)"
if ($null -ne $targetVersion) {
    Write-Host "Applied version: $targetVersion"
} else {
    Write-Host "Applied version: latest"
}
