param(
    [int]$Concurrency = 512,
    [int]$DurationSecs = 30,
    [string]$NginxVersion = "1.31.0",
    [string]$CaddyVersion = "2.11.3"
)

$ErrorActionPreference = "Stop"

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

$Root = (Resolve-Path ".").Path
$ReleaseProxysss = Join-Path $Root "target\release\proxysss.exe"
if (-not (Test-Path $ReleaseProxysss)) {
    cargo build --release
}

$Vendor = Join-Path $Root "target\bench-vendors"
$BenchRoot = Join-Path $Root "target\gateway-bench"
$Www = Join-Path $BenchRoot "www"
$PidFile = Join-Path $BenchRoot "pids.txt"

Stop-BenchProcesses -PidFile $PidFile
if (Test-Path $BenchRoot) {
    Remove-Item -Recurse -Force $BenchRoot
}
New-Item -ItemType Directory -Path $Vendor,$Www -Force | Out-Null

$NginxZip = Join-Path $Vendor "nginx-$NginxVersion.zip"
$NginxDir = Join-Path $Vendor "nginx-$NginxVersion"
$NginxExe = Join-Path $NginxDir "nginx.exe"
if (-not (Test-Path $NginxZip)) {
    Invoke-WebRequest -Uri "https://nginx.org/download/nginx-$NginxVersion.zip" -OutFile $NginxZip
}
if (-not (Test-Path $NginxExe)) {
    Expand-Archive -Path $NginxZip -DestinationPath $Vendor -Force
}

$CaddyZip = Join-Path $Vendor "caddy_$CaddyVersion`_windows_amd64.zip"
$CaddyExe = Join-Path $Vendor "caddy.exe"
if (-not (Test-Path $CaddyZip)) {
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

& $ReleaseProxysss init --dir (Join-Path $BenchRoot "proxysss") --overwrite | Out-Null
$ProxyConfig = @"
config_version: 1
include:
  enabled: false
  required: false
  files: []
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
  cwd: '$($BenchRoot.Replace('\','/'))/proxysss'
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
$ProxyConfigPath = Join-Path $BenchRoot "proxysss\proxysss.yaml"
$ProxyConfig | Set-Content -Path $ProxyConfigPath -Encoding ascii
& $ReleaseProxysss check-config --config $ProxyConfigPath

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
$NginxConfigPath = Join-Path $BenchRoot "nginx.conf"
$NginxConfig | Set-Content -Path $NginxConfigPath -Encoding ascii

$Proxy = Start-Process -FilePath $ReleaseProxysss -ArgumentList @("run", "--config", $ProxyConfigPath) -WorkingDirectory $Root -WindowStyle Hidden -PassThru
$Nginx = Start-Process -FilePath $NginxExe -ArgumentList @("-p", $NginxDir, "-c", $NginxConfigPath) -WorkingDirectory $NginxDir -WindowStyle Hidden -PassThru
$Caddy = Start-Process -FilePath $CaddyExe -ArgumentList @("file-server", "--listen", "127.0.0.1:18082", "--root", $Www) -WorkingDirectory $Root -WindowStyle Hidden -PassThru
@($Proxy.Id, $Nginx.Id, $Caddy.Id) | Set-Content -Path $PidFile -Encoding ascii

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
        Write-Host "=== $($Target.Name) c$Concurrency d$DurationSecs ==="
        & $ReleaseProxysss bench http --url $Target.Url --concurrency $Concurrency --duration-secs $DurationSecs
        Start-Sleep -Seconds 3
    }
} finally {
    Stop-BenchProcesses -PidFile $PidFile
}
