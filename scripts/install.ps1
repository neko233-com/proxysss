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

try {
    $tls12 = [Net.SecurityProtocolType]::Tls12
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor $tls12
} catch {
}

$BinaryName = "proxysss"
$Repo = "neko233-com/proxysss"
$InstallDir = Join-Path $env:LOCALAPPDATA $BinaryName
$Dest = Join-Path $InstallDir "$BinaryName.exe"

function Get-ProxysssConfigDir {
    Join-Path $env:APPDATA $BinaryName
}

function Get-BundleAssetName([string]$Arch) {
    "${BinaryName}-windows-${Arch}.zip"
}

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

function Add-BinaryLink([string]$Source, [string]$TargetDir) {
    $linkPath = Join-Path $TargetDir "$BinaryName.exe"
    if (Test-Path -LiteralPath $linkPath) {
        Remove-Item -LiteralPath $linkPath -Force
    }

    try {
        New-Item -ItemType HardLink -Path $linkPath -Target $Source -Force | Out-Null
        return $linkPath
    } catch {
        Copy-Item -LiteralPath $Source -Destination $linkPath -Force
        return $linkPath
    }
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

function Compare-VersionNullable([string]$Left, [string]$Right) {
    if ([string]::IsNullOrWhiteSpace($Left) -or [string]::IsNullOrWhiteSpace($Right)) {
        return $null
    }
    return ([version]$Left).CompareTo([version]$Right)
}

function Test-ServiceTaskExists {
    $getScheduledTask = Get-Command Get-ScheduledTask -ErrorAction SilentlyContinue
    if ($null -ne $getScheduledTask) {
        try {
            $task = Get-ScheduledTask -TaskName $BinaryName -ErrorAction SilentlyContinue
            if ($null -ne $task) {
                return $true
            }
        } catch {
        }
    }

    cmd /c "schtasks /Query /TN \"$BinaryName\" >nul 2>&1"
    if ($LASTEXITCODE -eq 0) {
        return $true
    }

    try {
        $runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
        $runValue = (Get-ItemProperty -LiteralPath $runKey -Name $BinaryName -ErrorAction SilentlyContinue).$BinaryName
        return -not [string]::IsNullOrWhiteSpace($runValue)
    } catch {
        return $false
    }
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
        Start-Process -FilePath $ExePath -ArgumentList @("service", "start") -WorkingDirectory (Get-ProxysssConfigDir) -WindowStyle Hidden | Out-Null
    } catch {
        Write-Host "warning: failed to start service, run '$BinaryName service start' manually" -ForegroundColor Yellow
    }
}

function Resolve-TargetVersion([string]$RequestedVersion) {
    if ($RequestedVersion -eq "latest" -or [string]::IsNullOrWhiteSpace($RequestedVersion)) {
        return @{ Label = "latest"; Version = $null }
    }

    $normalized = Get-NormalizedVersion $RequestedVersion
    return @{ Label = "v$normalized"; Version = $normalized }
}

function Download-FileWithFallback([string]$Url, [string]$OutFile, [string]$ReleaseTag, [string]$AssetName) {
    $lastError = $null

    for ($attempt = 1; $attempt -le 3; $attempt++) {
        try {
            Invoke-WebRequest -Uri $Url -OutFile $OutFile
            if (Test-ValidDownloadedFile $OutFile) {
                return
            }
            throw "downloaded file is incomplete"
        } catch {
            $lastError = $_
            Write-Host "warning: download attempt $attempt failed: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    }

    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($null -ne $curl) {
        & curl.exe -L --retry 3 --connect-timeout 20 -o $OutFile $Url
        if ($LASTEXITCODE -eq 0 -and (Test-ValidDownloadedFile $OutFile)) {
            return
        }
    }

    $gh = Get-Command gh -ErrorAction SilentlyContinue
    if ($null -ne $gh) {
        try {
            $downloadDir = Split-Path -Parent $OutFile
            if ($ReleaseTag -eq "latest") {
                & gh release download --repo $Repo --pattern $AssetName --dir $downloadDir --clobber
            } else {
                & gh release download $ReleaseTag --repo $Repo --pattern $AssetName --dir $downloadDir --clobber
            }
            $downloaded = Join-Path $downloadDir $AssetName
            if ($LASTEXITCODE -eq 0 -and (Test-ValidDownloadedFile $downloaded)) {
                Move-Item -Force $downloaded $OutFile
                return
            }
        } catch {
            Write-Host "warning: gh release download failed: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    }

    throw "download failed after retries: $($lastError.Exception.Message)"
}

function Test-ValidDownloadedFile([string]$Path) {
    if (!(Test-Path $Path)) {
        return $false
    }

    $file = Get-Item $Path
    if ($file.Length -lt 1024) {
        return $false
    }

    return $true
}

function Install-BinaryFile([string]$Source, [string]$Destination) {
    if (!(Test-Path -LiteralPath $Source)) {
        throw "source binary does not exist: $Source"
    }

    $backup = "$Destination.install-backup"
    Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue

    $hasBackup = $false
    if (Test-Path -LiteralPath $Destination) {
        try {
            Move-Item -LiteralPath $Destination -Destination $backup -Force
            $hasBackup = $true
        } catch {
            throw "failed to replace existing $BinaryName.exe at $Destination. Stop running proxysss processes or services and retry. Details: $($_.Exception.Message)"
        }
    }

    try {
        Move-Item -LiteralPath $Source -Destination $Destination -Force
        Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
    } catch {
        if ($hasBackup -and !(Test-Path -LiteralPath $Destination) -and (Test-Path -LiteralPath $backup)) {
            Move-Item -LiteralPath $backup -Destination $Destination -Force
        }
        throw "failed to install new $BinaryName.exe to $Destination. Existing binary was restored if possible. Details: $($_.Exception.Message)"
    }
}

$arch = Get-ArchName
$asset = Get-BundleAssetName $arch
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
    Write-Host "[dry-run] download and extract => $asset"
    Write-Host "[dry-run] install => $Dest"
    Write-Host "[dry-run] init/check/service install flow"
    exit 0
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Stop-ServiceIfPresent $Dest

$tmpRoot = Join-Path $InstallDir "bundle-tmp"
$archivePath = Join-Path $InstallDir $asset
$extractPath = Join-Path $tmpRoot "extract"
Remove-Item -LiteralPath $tmpRoot -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $extractPath | Out-Null

Download-FileWithFallback -Url $url -OutFile $archivePath -ReleaseTag $targetLabel -AssetName $asset
Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force

$bundleExe = Join-Path $extractPath "$BinaryName.exe"
if (!(Test-Path $bundleExe)) {
    throw "release bundle is missing $BinaryName.exe"
}

Install-BinaryFile -Source $bundleExe -Destination $Dest

Remove-Item -LiteralPath $archivePath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $tmpRoot -Recurse -Force -ErrorAction SilentlyContinue

$pathCandidates = @(
    (Join-Path $env:USERPROFILE ".local\bin"),
    (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links"),
    (Join-Path $env:USERPROFILE "go\bin")
)

$linked = $false
foreach ($candidate in $pathCandidates) {
    if (-not (Test-Path -LiteralPath $candidate)) { continue }
    if (-not (Test-PathInUserPath $candidate)) { continue }

    $linkPath = Add-BinaryLink -Source $Dest -TargetDir $candidate
    Write-Host "Linked $linkPath -> $Dest"
    $linked = $true
    break
}

if (-not $linked) {
    if (Add-ToUserPath $InstallDir) {
        Write-Host "Added $InstallDir to user PATH."
    } else {
        Write-Host "$InstallDir is already in user PATH."
    }
}

Notify-PathChanged
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

if (!$SkipInit) {
    & $Dest init
    & $Dest check-config
}

if (Test-ServiceTaskExists) {
    Start-ServiceIfPresent $Dest
} else {
    try {
        & $Dest service install
    } catch {
        Write-Host "warning: service install failed (likely requires administrator). Binary installation is complete." -ForegroundColor Yellow
        Write-Host "warning: run PowerShell as Administrator and execute '$Dest service install' to enable startup service." -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "Installed to $Dest"
Write-Host "Gateway ports: 80 (HTTP), 443 (HTTPS + HTTP/3)"
if ($null -ne $targetVersion) {
    Write-Host "Applied version: $targetVersion"
} else {
    Write-Host "Applied version: latest"
}

$global:LASTEXITCODE = 0
