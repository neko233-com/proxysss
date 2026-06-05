param(
    [string]$Proxysss = "proxysss"
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$pidFile = Join-Path $PSScriptRoot ".services.pids"

function Start-ServiceProcess {
    param([string]$Name, [string[]]$CmdArgs)
    $argLine = ($CmdArgs | ForEach-Object {
        if ($null -eq $_ -or "$_" -eq "") { throw "empty argument for $Name" }
        if ("$_" -match '[\s"]') { '"' + ("$_" -replace '"', '\"') + '"' } else { "$_" }
    }) -join ' '
    $proc = Start-Process -FilePath $Proxysss -ArgumentList $argLine -PassThru -WindowStyle Hidden
    Add-Content -Path $pidFile -Value "$Name=$($proc.Id)"
    Write-Host "started $Name pid=$($proc.Id)"
}

if (Test-Path $pidFile) {
    Get-Content $pidFile | ForEach-Object {
        if ($_ -match '=(\d+)$') { Stop-Process -Id $Matches[1] -Force -ErrorAction SilentlyContinue }
    }
    Remove-Item $pidFile -Force
}

New-Item -ItemType Directory -Force -Path "$PSScriptRoot\public", "$PSScriptRoot\webdav" | Out-Null
if (-not (Test-Path "$PSScriptRoot\public\index.html")) {
    @"
<!doctype html><html><body><h1>backend static assets</h1></body></html>
"@ | Set-Content -Encoding utf8 "$PSScriptRoot\public\index.html"
}

Start-ServiceProcess "http-echo" @("demo", "http-echo", "--listen", "127.0.0.1:8081")
Start-ServiceProcess "tcp-echo" @("demo", "tcp-echo", "--listen", "127.0.0.1:7001")
Start-ServiceProcess "udp-echo" @("demo", "udp-echo", "--listen", "127.0.0.1:8101")
Start-ServiceProcess "ftp-echo" @("demo", "tcp-echo", "--listen", "127.0.0.1:2121")

Write-Host ""
Write-Host "Backend services running under $PSScriptRoot"
Write-Host "PIDs: $pidFile"
Write-Host "Start proxysss gateway from default install path, not this directory."
