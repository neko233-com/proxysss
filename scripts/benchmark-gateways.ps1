param(
    [int]$Concurrency = 512,
    [int]$DurationSecs = 30,
    [string]$NginxVersion = "1.31.0",
    [string]$CaddyVersion = "2.11.3",
    [switch]$Quick,
    [switch]$SkipGate
)

$ErrorActionPreference = "Stop"

if ($Quick) {
    $Concurrency = 128
    $DurationSecs = 10
}

function Stop-BenchProcesses {
    param([string]$PidFile)
    if (Test-Path $PidFile) {
        Get-Content $PidFile | ForEach-Object {
            $proc = Get-Process -Id ([int]$_) -ErrorAction SilentlyContinue
            if ($proc) {
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            }
        }
    }
    Get-Process nginx,caddy -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
}

function Wait-HttpReady {
    param([string]$Url)
    for ($i = 0; $i -lt 80; $i++) {
        try {
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2
            if ($response.StatusCode -eq 200) {
                return
            }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "not ready: $Url"
}

function Invoke-GatewayBench {
    param(
        [string]$Binary,
        [string]$Url,
        [int]$Concurrency,
        [int]$DurationSecs
    )
    $output = & $Binary bench http --url $Url --concurrency $Concurrency --duration-secs $DurationSecs 2>&1 | Out-String
    Write-Host $output
    $result = [ordered]@{
        url = $Url
        concurrency = $Concurrency
        duration_secs = $DurationSecs
        success = 0
        errors = 0
        ops_per_sec = 0.0
        throughput_mib_s = 0.0
        latency_p50_ms = $null
        latency_p95_ms = $null
        latency_p99_ms = $null
    }
    if ($output -match 'success\s+:\s+(\d+)') { $result.success = [int]$Matches[1] }
    if ($output -match 'errors\s+:\s+(\d+)') { $result.errors = [int]$Matches[1] }
    if ($output -match 'ops/sec\s+:\s+([\d.]+)') { $result.ops_per_sec = [double]$Matches[1] }
    if ($output -match 'throughput\s+:\s+([\d.]+)\s+MiB/s') { $result.throughput_mib_s = [double]$Matches[1] }
    if ($output -match 'latency p50\s+:\s+([\d.]+)\s+ms') { $result.latency_p50_ms = [double]$Matches[1] }
    if ($output -match 'latency p95\s+:\s+([\d.]+)\s+ms') { $result.latency_p95_ms = [double]$Matches[1] }
    if ($output -match 'latency p99\s+:\s+([\d.]+)\s+ms') { $result.latency_p99_ms = [double]$Matches[1] }
    return [pscustomobject]$result
}

$Root = (Resolve-Path ".").Path
$ReleaseProxysss = Join-Path $Root "target\release\proxysss.exe"
if (-not (Test-Path $ReleaseProxysss)) {
    cargo build --release --locked
}

# All vendor downloads and per-run artifacts stay under .benchmark/ (gitignored).
$BenchRoot = Join-Path $Root ".benchmark"
$Vendor = Join-Path $BenchRoot "vendors"
$RunDir = Join-Path $BenchRoot "runs\latest"
$Www = Join-Path $RunDir "www"
$PidFile = Join-Path $RunDir "pids.txt"
$ResultsFile = Join-Path $RunDir "results.json"

Stop-BenchProcesses -PidFile $PidFile
if (Test-Path $RunDir) {
    Remove-Item -Recurse -Force $RunDir
}
New-Item -ItemType Directory -Path $Vendor,$Www -Force | Out-Null

$NginxZip = Join-Path $Vendor "nginx-$NginxVersion.zip"
$NginxDir = Join-Path $Vendor "nginx-$NginxVersion"
$NginxExe = Join-Path $NginxDir "nginx.exe"
if (-not (Test-Path $NginxZip)) {
    Write-Host "==> downloading nginx $NginxVersion to $Vendor"
    Invoke-WebRequest -Uri "https://nginx.org/download/nginx-$NginxVersion.zip" -OutFile $NginxZip
}
if (-not (Test-Path $NginxExe)) {
    Expand-Archive -Path $NginxZip -DestinationPath $Vendor -Force
}

$CaddyZip = Join-Path $Vendor "caddy_$CaddyVersion`_windows_amd64.zip"
$CaddyExe = Join-Path $Vendor "caddy.exe"
if (-not (Test-Path $CaddyZip)) {
    Write-Host "==> downloading caddy $CaddyVersion to $Vendor"
    $gh = Get-Command gh -ErrorAction SilentlyContinue
    if ($gh) {
        gh release download "v$CaddyVersion" -R caddyserver/caddy -p "caddy_$CaddyVersion`_windows_amd64.zip" -D $Vendor
    } else {
        Invoke-WebRequest -Uri "https://github.com/caddyserver/caddy/releases/download/v$CaddyVersion/caddy_$CaddyVersion`_windows_amd64.zip" -OutFile $CaddyZip
    }
}
if (-not (Test-Path $CaddyExe)) {
    Expand-Archive -Path $CaddyZip -DestinationPath $Vendor -Force
}

"<!doctype html><html><head><meta charset=`"utf-8`"><title>gateway bench</title></head><body><h1>gateway benchmark</h1><p>same static payload for proxysss nginx caddy.</p></body></html>" |
    Set-Content -Path (Join-Path $Www "index.html") -Encoding ascii

& $ReleaseProxysss init --dir (Join-Path $RunDir "proxysss") --overwrite | Out-Null
$ProxyConfig = @"
config_version: 1
logging:
  access_log: false
  access_log_path: logs/access.log
  error_log_path: logs/error.log
http:
  plain_bind: 127.0.0.1:18083
  tls_bind: ''
  h3_bind: ''
script:
  enabled: false
  cwd: '$($RunDir.Replace('\','/'))/proxysss'
plugins:
  enabled: false
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
services:
  static_sites:
    - name: bench
      path_prefix: /bench
      root: '$($Www.Replace('\','/'))'
      index_files: [index.html]
      autoindex: false
"@
$ProxyConfigPath = Join-Path $RunDir "proxysss\proxysss.yaml"
$ProxyConfig | Set-Content -Path $ProxyConfigPath -Encoding ascii
& $ReleaseProxysss -config $ProxyConfigPath check-config

$NginxConfig = @"
worker_processes  1;
events { worker_connections  4096; }
http {
    access_log off;
    sendfile on;
    server {
        listen 127.0.0.1:18081;
        location /bench/ {
            alias $($Www.Replace('\','/'))/;
            index index.html;
        }
    }
}
"@
$NginxConfigPath = Join-Path $RunDir "nginx.conf"
$NginxConfig | Set-Content -Path $NginxConfigPath -Encoding ascii

$Proxy = Start-Process -FilePath $ReleaseProxysss -ArgumentList @("-config", $ProxyConfigPath) -WorkingDirectory $Root -WindowStyle Hidden -PassThru
$Nginx = Start-Process -FilePath $NginxExe -ArgumentList @("-p", $NginxDir, "-c", $NginxConfigPath) -WorkingDirectory $NginxDir -WindowStyle Hidden -PassThru
$Caddy = Start-Process -FilePath $CaddyExe -ArgumentList @("file-server", "--listen", "127.0.0.1:18082", "--root", $Www) -WorkingDirectory $Root -WindowStyle Hidden -PassThru
@($Proxy.Id, $Nginx.Id, $Caddy.Id) | Set-Content -Path $PidFile -Encoding ascii

$Results = @()
try {
    $Targets = @(
        @{ Name = "proxysss"; Url = "http://127.0.0.1:18083/bench/index.html" },
        @{ Name = "nginx"; Url = "http://127.0.0.1:18081/bench/index.html" },
        @{ Name = "caddy"; Url = "http://127.0.0.1:18082/index.html" }
    )

    foreach ($Target in $Targets) {
        Wait-HttpReady -Url $Target.Url
    }

    foreach ($Target in $Targets) {
        Write-Host ""
        Write-Host "=== $($Target.Name) c$Concurrency d${DurationSecs}s ===" -ForegroundColor Cyan
        $bench = Invoke-GatewayBench -Binary $ReleaseProxysss -Url $Target.Url -Concurrency $Concurrency -DurationSecs $DurationSecs
        $bench | Add-Member -NotePropertyName name -NotePropertyValue $Target.Name -Force
        $Results += $bench
        Start-Sleep -Seconds 2
    }
} finally {
    Stop-BenchProcesses -PidFile $PidFile
}

$Results | ConvertTo-Json -Depth 4 | Set-Content -Path $ResultsFile -Encoding utf8

Write-Host ""
Write-Host "=== throughput summary (ops/sec) ===" -ForegroundColor Green
$Results | Sort-Object ops_per_sec -Descending | Format-Table name, ops_per_sec, throughput_mib_s, latency_p50_ms, latency_p95_ms, success, errors -AutoSize
Write-Host "results saved to $ResultsFile"
Write-Host "vendor binaries cached under $Vendor (gitignored under .benchmark/)"

$reportScript = Join-Path $PSScriptRoot "benchmark-report.py"
$compareScript = Join-Path $PSScriptRoot "compare-report.py"
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command py -ErrorAction SilentlyContinue }
if (-not $python) { throw "python not found; install Python 3 to generate benchmark reports" }
& $python.Source $reportScript --results $ResultsFile --out-dir $RunDir --concurrency $Concurrency --duration-secs $DurationSecs
& $python.Source $compareScript --binary $ReleaseProxysss --benchmark $ResultsFile --out-dir $RunDir
if ($LASTEXITCODE -ne 0) { throw "compare-report.py failed with exit code $LASTEXITCODE" }

Write-Host "benchmark report markdown: $(Join-Path $RunDir 'report.md')"
Write-Host "benchmark report html:     $(Join-Path $RunDir 'report.html')"
Write-Host "nginx compare markdown:    $(Join-Path $RunDir 'nginx-compare.md')"
Write-Host "nginx compare html:        $(Join-Path $RunDir 'nginx-compare.html')"

if (-not $SkipGate) {
    & (Join-Path $PSScriptRoot "benchmark-gate-check.ps1") -ResultsFile $ResultsFile
}
