param(
    [Parameter(Position = 0)]
    [string]$Version = "latest"
)

$ErrorActionPreference = 'Stop'

$BinaryName = "proxysss"
$Repo = "neko233-com/proxysss"
$InstallDir = Join-Path $env:LOCALAPPDATA $BinaryName

function Get-NormalizedVersion([string]$Value) {
    $v = $Value.Trim()
    while ($v.StartsWith('v') -or $v.StartsWith('V')) { $v = $v.Substring(1) }
    return $v
}

function Get-ArchName {
    if ([Environment]::Is64BitOperatingSystem) {
        if ($env:PROCESSOR_ARCHITECTURE -match 'ARM64') {
            return 'arm64'
        }
        return 'amd64'
    }
    return 'amd64'
}

function Test-PathInUserPath([string]$Dir) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) { return $false }

    $normalizedDir = (Resolve-Path -LiteralPath $Dir).Path.TrimEnd('\\')
    foreach ($entry in $userPath -split ';') {
        if ([string]::IsNullOrWhiteSpace($entry)) { continue }
        try {
            $normalizedEntry = (Resolve-Path -LiteralPath $entry -ErrorAction Stop).Path.TrimEnd('\\')
            if ($normalizedEntry -ieq $normalizedDir) { return $true }
        } catch {
            if ($entry.TrimEnd('\\') -ieq $normalizedDir) { return $true }
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
    irm https://deno.land/install.ps1 | iex

    $denoBin = Join-Path $env:USERPROFILE ".deno\bin"
    if (Test-Path $denoBin) {
        Add-ToUserPath $denoBin | Out-Null
        $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                    [Environment]::GetEnvironmentVariable("Path", "User")
    }
}

$arch = Get-ArchName
$asset = "${BinaryName}-windows-${arch}.exe"

if ($Version -eq "latest" -or [string]::IsNullOrWhiteSpace($Version)) {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
    $versionLabel = "latest"
} else {
    $Version = Get-NormalizedVersion $Version
    $url = "https://github.com/$Repo/releases/download/v$Version/$asset"
    $versionLabel = "v$Version"
}

$dest = Join-Path $InstallDir "$BinaryName.exe"
Write-Host "Installing ${BinaryName} $versionLabel for windows/$arch..."
Write-Host "Downloading $url..."

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Invoke-WebRequest -Uri $url -OutFile $dest

if (Add-ToUserPath $InstallDir) {
    Write-Host "Added $InstallDir to user PATH."
}

Notify-PathChanged
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

Install-DenoIfMissing

& $dest init
& $dest check-config
& $dest service install

Write-Host ""
Write-Host "Installed to $dest"
Write-Host "Gateway port: 23380 (TCP for HTTP/1.1 + HTTP/2, UDP for HTTP/3)"
