param(
    [string]$Proxysss = "proxysss",
    [string]$Config = "$PSScriptRoot\proxysss.yaml",
    [switch]$WithGateway
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
$pidFile = Join-Path $PSScriptRoot ".lab.pids"

function Start-LabProcess {
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
        if ($_ -match '=(\d+)$') {
            Stop-Process -Id $Matches[1] -Force -ErrorAction SilentlyContinue
        }
    }
    Remove-Item $pidFile -Force
}

New-Item -ItemType Directory -Force -Path "$PSScriptRoot\logs", "$PSScriptRoot\webdav", "$PSScriptRoot\public", "$PSScriptRoot\certs", "$PSScriptRoot\plugins" | Out-Null
if (-not (Test-Path "$PSScriptRoot\public\index.html")) {
    @"
<!doctype html><html><body><h1>proxysss lab assets</h1></body></html>
"@ | Set-Content -Encoding utf8 "$PSScriptRoot\public\index.html"
}

if (-not (Test-Path "$PSScriptRoot\certs\proxysss-cert.pem")) {
    & $Proxysss cert-bootstrap --dir $PSScriptRoot | Out-Null
}
& $Proxysss check-config --config $Config

Start-LabProcess "lab-http-echo" @("demo", "http-echo", "--listen", "127.0.0.1:8081")
Start-LabProcess "lab-tcp-echo" @("demo", "tcp-echo", "--listen", "127.0.0.1:7001")
Start-LabProcess "lab-udp-echo" @("demo", "udp-echo", "--listen", "127.0.0.1:8101")
Start-LabProcess "lab-ftp-echo" @("demo", "tcp-echo", "--listen", "127.0.0.1:2121")

if ($WithGateway) {
    Start-LabProcess "lab-proxysss" @("run", "--config", $Config)
}

Write-Host ""
Write-Host "Lab backends running (PIDs in $pidFile)."
if (-not $WithGateway) {
    Write-Host "Start gateway:"
    Write-Host "  $Proxysss run --config $Config"
    Write-Host "Or rerun: .\start-lab.ps1 -WithGateway"
}
