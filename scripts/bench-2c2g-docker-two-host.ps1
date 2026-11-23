param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,
    [int]$Rounds = 3,
    [int]$Workers = 64,
    [int]$WarmupSecs = 2,
    [int]$DurationSecs = 5,
    [string]$ServerImage = 'nginx:1.28',
    [string]$ClientImage = 'debian:bookworm-slim',
    [string]$Tag = 'two-host',
    [string]$TrafficProfile = 'small',
    [switch]$SkipClean,
    [switch]$KeepContainers
)

$ErrorActionPreference = 'Stop'
# Native stderr from Docker/load generator must not terminate PowerShell 7
# before the exit code is checked.
$PSNativeCommandUseErrorActionPreference = $false
$repo = Split-Path -Parent $PSScriptRoot
if (-not $SkipClean) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repo 'scripts\clean-test-cache.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'clean-test-cache.ps1 failed' }
}
if (-not (Test-Path -LiteralPath $Binary)) { throw "binary not found: $Binary" }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$started = Get-Date

# Docker Desktop refuses docker cp sources under the system TEMP profile;
# keep the staging tree on the project drive (outside clean-test-cache scope).
$tmp = Join-Path $repo '.bench-two-host'
if (Test-Path -LiteralPath $tmp) {
    try { Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction Stop } catch { Write-Host "temp cleanup partial (locked handles): $($_.Exception.Message)" }
}
New-Item -ItemType Directory -Force -Path (Join-Path $tmp 'www\rp'), (Join-Path $tmp 'bin'), (Join-Path $tmp 'results') | Out-Null

$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$small = New-Object byte[] 1024
$rng.GetBytes($small)
[IO.File]::WriteAllBytes((Join-Path $tmp 'www\small.bin'), $small)
Copy-Item (Join-Path $tmp 'www\small.bin') (Join-Path $tmp 'www\rp\small.bin')
$large = New-Object byte[] 65536
$rng.GetBytes($large)
[IO.File]::WriteAllBytes((Join-Path $tmp 'www\large.bin'), $large)

$backendConfig = @'
worker_processes 1;
worker_rlimit_nofile 8192;
error_log /dev/stderr error;
pid /tmp/backend.pid;
events { worker_connections 4096; use epoll; multi_accept on; }
http {
    access_log off;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    server_tokens off;
    server { listen 19081; root /bench/www; index index.html; }
}
'@
[IO.File]::WriteAllText((Join-Path $tmp 'backend.conf'), $backendConfig)

$nginxConfig = @'
worker_processes 2;
worker_rlimit_nofile 8192;
error_log /dev/stderr error;
pid /tmp/gateway.pid;
events { worker_connections 4096; use epoll; multi_accept on; }
http {
    access_log off;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    keepalive_requests 100000;
    server_tokens off;
    gzip off;
    open_file_cache max=1024 inactive=30s;
    open_file_cache_valid 30s;
    open_file_cache_min_uses 1;
    open_file_cache_errors on;
    upstream bench_backend { server 127.0.0.1:19081; keepalive 128; }
    server {
        listen 19080;
        location /rp/ {
            proxy_pass http://bench_backend;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host $host;
            proxy_buffering on;
        }
        location / { root /bench/www; index index.html; }
    }
}
'@
[IO.File]::WriteAllText((Join-Path $tmp 'nginx.conf'), $nginxConfig)

$proxysssConfig = @'
http:
  plain_bind: 0.0.0.0:19080
  tls_bind: ''
  h3_bind: ''
logging:
  level: error
  access_log: false
  access_log_path: /bench/logs/proxysss-access.log
  error_log_path: /bench/logs/proxysss-error.log
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
  performance:
    enabled: true
    traffic_profile: small
load_balance:
  active_health:
    enabled: false
  passive_health:
    enabled: false
  retries:
    enabled: false
services:
  static_sites:
    - name: bench
      path_prefix: /
      root: /bench/www
      index_files: [index.html]
  reverse_proxy:
    routes:
      - name: rp
        path_prefix: /rp
        upstream: http://127.0.0.1:19081
'@.Replace('traffic_profile: small', "traffic_profile: $TrafficProfile")
[IO.File]::WriteAllText((Join-Path $tmp 'proxysss.yaml'), $proxysssConfig)

$runScript = @'
#!/bin/sh
set -eu
nginx -c /bench/backend.conf
sleep 1
if [ "$1" = "nginx" ]; then
    exec nginx -c /bench/nginx.conf -g 'daemon off;'
else
    exec /bench/proxysss -config /bench/proxysss.yaml
fi
'@
[IO.File]::WriteAllText((Join-Path $tmp 'run.sh'), $runScript.Replace("`r`n", "`n"))

$go = Get-Command go -ErrorAction Stop
$loader = Join-Path $tmp 'bin\bench-fast-http'
$oldGoos = $env:GOOS
$oldGoarch = $env:GOARCH
$oldCgo = $env:CGO_ENABLED
$env:GOOS = 'linux'
$env:GOARCH = 'amd64'
$env:CGO_ENABLED = '0'
& $go.Source build -trimpath -ldflags '-s -w' -o $loader (Join-Path $repo 'scripts\bench-fast-http.go')
$goExit = $LASTEXITCODE
$env:GOOS = $oldGoos
$env:GOARCH = $oldGoarch
$env:CGO_ENABLED = $oldCgo
if ($goExit -ne 0) { throw 'failed to build Linux benchmark loader' }

$network = 'proxysss-bench-two-host'
$networkExists = docker network ls --filter "name=^${network}$" --format '{{.Name}}'
if (-not $networkExists) {
    docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to create Docker network $network" }
}

$volume = 'proxysss-bench-data'
docker volume create $volume | Out-Null
if ($LASTEXITCODE -ne 0) { throw "failed to create Docker volume $volume" }

$serverNginx = 'proxysss-bench-server-nginx'
$serverProxysss = 'proxysss-bench-server-proxysss'
$client = 'proxysss-bench-client'
$prep = 'proxysss-bench-prep'
docker rm -f $serverNginx $serverProxysss $client $prep 2>$null | Out-Null

# Docker Desktop occasionally returns a transient "Access is denied." for
# exec/cp right after container start; retry without touching measurement.
function Invoke-DockerRetry([scriptblock]$Op, [string]$What) {
    for ($attempt = 1; $attempt -le 4; $attempt++) {
        & $Op | Out-Null
        if ($LASTEXITCODE -eq 0) { return }
        Start-Sleep -Seconds 1
    }
    throw "failed after retries: $What"
}

# Populate a named volume through a short-lived helper container. Docker cp
# cannot create the parent directory inside a never-started container.
docker create --name $prep --network $network --restart no -v "${volume}:/bench" alpine:3.21 sleep infinity | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to create benchmark prep container' }
docker start $prep | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to start benchmark prep container' }
Invoke-DockerRetry { docker exec $prep sh -c 'mkdir -p /bench/www /bench/logs /bench/results' } 'prepare benchmark volume'
Invoke-DockerRetry { docker cp $Binary "${prep}:/bench/proxysss" } 'copy gateway binary'
Invoke-DockerRetry { docker cp (Join-Path $tmp 'backend.conf') "${prep}:/bench/backend.conf" } 'copy backend.conf'
Invoke-DockerRetry { docker cp (Join-Path $tmp 'nginx.conf') "${prep}:/bench/nginx.conf" } 'copy nginx.conf'
Invoke-DockerRetry { docker cp (Join-Path $tmp 'proxysss.yaml') "${prep}:/bench/proxysss.yaml" } 'copy proxysss.yaml'
Invoke-DockerRetry { docker cp (Join-Path $tmp 'run.sh') "${prep}:/bench/run.sh" } 'copy run.sh'
Invoke-DockerRetry { docker cp (Join-Path $tmp 'www\.') "${prep}:/bench/www" } 'copy www'
Invoke-DockerRetry { docker exec $prep sh -c 'chmod +x /bench/proxysss' } 'chmod benchmark binary'
docker stop $prep | Out-Null
docker rm $prep | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to populate benchmark volume' }

# Both server candidates share the same populated volume but always use their
# own 2 CPU / 2 GiB cgroup. Only one server is running at a time.
docker create --name $serverNginx --network $network --cpus=2 --memory=2g --restart no -v "${volume}:/bench" $ServerImage sh /bench/run.sh nginx | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to create nginx server container' }
docker create --name $serverProxysss --network $network --cpus=2 --memory=2g --restart no -v "${volume}:/bench" $ServerImage sh /bench/run.sh proxysss | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to create proxysss server container' }
docker create --name $client --network $network --cpus=2 --memory=2g --restart no $ClientImage sleep infinity | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to create client container' }

docker start $client | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to start client container' }
Invoke-DockerRetry { docker exec $client sh -c 'mkdir -p /bench/results' } 'client results dir'
Invoke-DockerRetry { docker cp $loader "${client}:/bench/bench-fast-http" } 'copy loader'
Invoke-DockerRetry { docker exec $client sh -c 'chmod +x /bench/bench-fast-http' } 'chmod loader'

$evidence = Join-Path $repo ("performance-evidence\2c2g-http-docker-two-host-{0}" -f $Tag)
New-Item -ItemType Directory -Force -Path (Join-Path $evidence 'raw') | Out-Null

function Get-CgroupValue([string]$Name, [string]$Path) {
    for ($i = 0; $i -lt 3; $i++) {
        $value = (docker exec $Name sh -c "cat $Path" 2>$null | Out-String).Trim()
        if ($LASTEXITCODE -eq 0 -and $value) { return $value }
        Start-Sleep -Milliseconds 500
    }
    return ''
}

function Get-ExecValue([string]$Name, [string]$Command) {
    for ($i = 0; $i -lt 3; $i++) {
        $value = (docker exec $Name sh -c $Command 2>$null | Out-String).Trim()
        if ($LASTEXITCODE -eq 0 -and $value) { return $value }
        Start-Sleep -Milliseconds 500
    }
    return ''
}

function Get-ContainerInfo([string]$Name) {
    return [ordered]@{
        cpu_max = Get-CgroupValue $Name '/sys/fs/cgroup/cpu.max'
        memory_max = Get-CgroupValue $Name '/sys/fs/cgroup/memory.max'
        cpu_stat = Get-CgroupValue $Name '/sys/fs/cgroup/cpu.stat'
        memory_peak = Get-CgroupValue $Name '/sys/fs/cgroup/memory.peak'
        nproc = Get-ExecValue $Name 'nproc'
        os_release = Get-CgroupValue $Name '/etc/os-release'
    }
}

$serverNginxInfo = $null
$serverProxysssInfo = $null
docker start $serverNginx | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to start nginx server for environment capture' }
Start-Sleep -Seconds 1
$serverNginxInfo = Get-ContainerInfo $serverNginx
docker stop $serverNginx | Out-Null
docker start $serverProxysss | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'failed to start proxysss server for environment capture' }
Start-Sleep -Seconds 1
$serverProxysssInfo = Get-ContainerInfo $serverProxysss
docker stop $serverProxysss | Out-Null
$clientInfo = Get-ContainerInfo $client
$envLines = @(
    "date=$((Get-Date).ToString('yyyy-MM-dd HH:mm:ss zzz'))",
    "binary=$Binary",
    "server_image=$ServerImage",
    "client_image=$ClientImage",
    "network=$network",
    "server_nginx_name=$serverNginx",
    "server_proxysss_name=$serverProxysss",
    "client_name=$client",
    'server_nginx_cpu_max=' + $serverNginxInfo.cpu_max,
    'server_nginx_memory_max=' + $serverNginxInfo.memory_max,
    'server_nginx_memory_peak=' + $serverNginxInfo.memory_peak,
    'server_nginx_os_release=' + ($serverNginxInfo.os_release -replace "`n", ' '),
    'server_proxysss_cpu_max=' + $serverProxysssInfo.cpu_max,
    'server_proxysss_memory_max=' + $serverProxysssInfo.memory_max,
    'server_proxysss_memory_peak=' + $serverProxysssInfo.memory_peak,
    'server_proxysss_os_release=' + ($serverProxysssInfo.os_release -replace "`n", ' '),
    'client_cpu_max=' + $clientInfo.cpu_max,
    'client_memory_max=' + $clientInfo.memory_max,
    'client_memory_peak=' + $clientInfo.memory_peak,
    'client_os_release=' + ($clientInfo.os_release -replace "`n", ' '),
    'server_nginx_nproc=' + $serverNginxInfo.nproc,
    'server_proxysss_nproc=' + $serverProxysssInfo.nproc,
    'client_nproc=' + $clientInfo.nproc
)
$envLines | Set-Content -LiteralPath (Join-Path $evidence 'env.txt') -Encoding utf8

$scenarios = @(
    @{ name = 'static-1k'; path = '/small.bin'; bytes = 1024 },
    @{ name = 'static-64k'; path = '/large.bin'; bytes = 65536 },
    @{ name = 'proxy-1k'; path = '/rp/small.bin'; bytes = 1024 }
)
$candidates = @('nginx', 'proxysss')
$results = @()

function Wait-Ready([string]$HostName) {
    for ($i = 0; $i -lt 30; $i++) {
        Start-Sleep -Milliseconds 500
        $oldEap = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $probe = & docker exec $client /bench/bench-fast-http `
            -url ("http://{0}:19080/small.bin" -f $HostName) `
            -workers 1 -warmup 0 -duration 1 -expected-bytes 1024 `
            -prewarm-timeout-ms 500 -out '/tmp/probe.json' 2>$null
        $probeExit = $LASTEXITCODE
        $ErrorActionPreference = $oldEap
        if ($probeExit -eq 0) { return }
        if ($i -eq 29) { Write-Host ($probe | Out-String); throw "$HostName did not become ready" }
    }
}

function Start-Server([string]$Candidate) {
    $name = if ($Candidate -eq 'nginx') { $serverNginx } else { $serverProxysss }
    docker start $name | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to start $name" }
    Wait-Ready $name
}

function Stop-Server {
    docker stop -t 2 $serverNginx $serverProxysss 2>$null | Out-Null
}

try {
    foreach ($round in 1..$Rounds) {
        $order = if ($round % 2 -eq 1) { @('nginx', 'proxysss') } else { @('proxysss', 'nginx') }
        foreach ($candidate in $order) {
            Write-Host ("[{0}/{1}] {2} via Docker bridge" -f $round, $Rounds, $candidate)
            Start-Server $candidate
            $hostName = if ($candidate -eq 'nginx') { $serverNginx } else { $serverProxysss }
            foreach ($scenario in $scenarios) {
                $file = "{0}-{1}-r{2}.json" -f $candidate, $scenario.name, $round
                $containerPath = "/bench/results/$file"
                & docker exec -e GOMAXPROCS=2 $client /bench/bench-fast-http `
                    -label ("{0}-{1}-r{2}" -f $candidate, $scenario.name, $round) `
                    -url ("http://{0}:19080{1}" -f $hostName, $scenario.path) `
                    -workers $Workers -warmup $WarmupSecs -duration $DurationSecs `
                    -expected-bytes $scenario.bytes -out $containerPath
                if ($LASTEXITCODE -ne 0) { throw ("{0} {1} failed" -f $candidate, $scenario.name) }
                docker cp "${client}:$containerPath" (Join-Path $evidence ("raw\" + $file)) | Out-Null
                $results += Get-Content -LiteralPath (Join-Path $evidence ("raw\" + $file)) -Raw | ConvertFrom-Json
            }
            Stop-Server
        }
    }

    $summary = @(
        '# 2C2G Docker two-host HTTP benchmark',
        '',
        ('- Date: {0}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')),
        ('- Candidate: {0}' -f $Binary),
        ('- Server: Docker container `{0}`, 2 CPU / 2 GiB cgroup; backend nginx runs in the same server container' -f $ServerImage),
        ('- Client: separate Docker container `{0}`, 2 CPU / 2 GiB cgroup, Go HTTP/1.1 keep-alive, {1} preconnected connections' -f $ClientImage, $Workers),
        ('- Network: Docker bridge `{0}`; client connects to the server container DNS name, no published port and no Windows host client' -f $network),
        ('- Server cgroup: cpu.max={0}, memory.max={1}' -f $serverProxysssInfo.cpu_max, $serverProxysssInfo.memory_max),
        ('- Client cgroup: cpu.max={0}, memory.max={1}' -f $clientInfo.cpu_max, $clientInfo.memory_max),
        ('- Samples: {0} rounds x {1}s warm-up + {2}s measurement; median RPS, worst error count reported' -f $Rounds, $WarmupSecs, $DurationSecs),
        '- Scope: static 1 KiB, static 64 KiB, 1 KiB reverse proxy only. This is a two-container Docker bridge comparison, not a physical two-host or cross-host production claim.',
        ''
    )

    function Median([double[]]$values) {
        $sorted = $values | Sort-Object
        $n = $sorted.Count
        if ($n -eq 0) { return 0 }
        if ($n % 2 -eq 1) { return [double]$sorted[[int]($n / 2)] }
        return ([double]$sorted[$n / 2 - 1] + [double]$sorted[$n / 2]) / 2
    }

    $summary += '| Scenario | nginx RPS | proxysss RPS | Ratio | nginx p50/p95/p99 ms | proxysss p50/p95/p99 ms | nginx errors | proxysss errors |'
    $summary += '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |'
    foreach ($scenario in $scenarios) {
        $nginxRows = $results | Where-Object { $_.Label -like "nginx-$($scenario.name)-*" }
        $proxysssRows = $results | Where-Object { $_.Label -like "proxysss-$($scenario.name)-*" }
        $nginxRps = Median @($nginxRows | ForEach-Object { [double]$_.RPS })
        $proxysssRps = Median @($proxysssRows | ForEach-Object { [double]$_.RPS })
        $ratio = if ($nginxRps -gt 0) { $proxysssRps / $nginxRps } else { 0 }
        $nginxP50 = Median @($nginxRows | ForEach-Object { [double]$_.p50_ms })
        $nginxP95 = Median @($nginxRows | ForEach-Object { [double]$_.p95_ms })
        $nginxP99 = Median @($nginxRows | ForEach-Object { [double]$_.p99_ms })
        $proxysssP50 = Median @($proxysssRows | ForEach-Object { [double]$_.p50_ms })
        $proxysssP95 = Median @($proxysssRows | ForEach-Object { [double]$_.p95_ms })
        $proxysssP99 = Median @($proxysssRows | ForEach-Object { [double]$_.p99_ms })
        $nginxErrors = ($nginxRows | Measure-Object -Property errors -Maximum).Maximum
        $proxysssErrors = ($proxysssRows | Measure-Object -Property errors -Maximum).Maximum
        $summary += ("| {0} | {1:N0} | {2:N0} | {3:N3}x | {4:N3} / {5:N3} / {6:N3} | {7:N3} / {8:N3} / {9:N3} | {10:N0} | {11:N0} |" -f `
            $scenario.name, $nginxRps, $proxysssRps, $ratio, $nginxP50, $nginxP95, $nginxP99, $proxysssP50, $proxysssP95, $proxysssP99, $nginxErrors, $proxysssErrors)
    }

    $summary | Set-Content -LiteralPath (Join-Path $evidence 'summary.md') -Encoding utf8
    $summary | Write-Host
    Write-Host ("Evidence: {0}" -f $evidence)
    Write-Host ("Elapsed: {0:N1}s" -f ((Get-Date) - $started).TotalSeconds)
}
finally {
    if (-not $KeepContainers) {
        docker rm -f $serverNginx $serverProxysss $client 2>$null | Out-Null
    }
}
