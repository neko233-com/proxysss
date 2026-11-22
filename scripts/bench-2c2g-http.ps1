param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,
    [int]$Rounds = 3,
    [int]$Workers = 64,
    [int]$WarmupSecs = 2,
    [int]$DurationSecs = 5,
    [int]$Port = 19080,
    [ValidateSet('small', 'balanced', 'bulk')]
    [string]$TrafficProfile = 'small',
    [string]$Tag = 'local',
    [switch]$SkipClean,
    [switch]$HostNetwork,
    [switch]$KeepContainers
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $SkipClean) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repo 'scripts\clean-test-cache.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'clean-test-cache.ps1 failed' }
}
if (-not (Test-Path -LiteralPath $Binary)) { throw "binary not found: $Binary" }
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$started = Get-Date

$tmp = Join-Path $env:TEMP 'proxysss-2c2g-bench'
if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $tmp 'www\rp') | Out-Null

$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$small = New-Object byte[] 1024
$rng.GetBytes($small)
[IO.File]::WriteAllBytes((Join-Path $tmp 'www\small.bin'), $small)
Copy-Item (Join-Path $tmp 'www\small.bin') (Join-Path $tmp 'www\rp\small.bin')
$large = New-Object byte[] 65536
$rng.GetBytes($large)
[IO.File]::WriteAllBytes((Join-Path $tmp 'www\large.bin'), $large)

$backendUpstream = if ($HostNetwork) { '127.0.0.1:19081' } else { 'bench-backend:80' }
$nginx = @'
worker_processes 2;
worker_rlimit_nofile 8192;
error_log /dev/stderr error;
pid /tmp/nginx.pid;
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
    upstream bench_backend { server __BACKEND__; keepalive 128; }
    server {
        listen 19080;
        location /rp/ {
            proxy_pass http://bench_backend;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host $host;
            proxy_buffering off;
        }
        location / { root /www; index index.html; }
    }
}
'@
[IO.File]::WriteAllText((Join-Path $tmp 'nginx.conf'), $nginx.Replace('__BACKEND__', $backendUpstream))

$proxysss = @'
http:
  plain_bind: 0.0.0.0:19080
  tls_bind: ''
  h3_bind: ''
logging:
  level: error
  access_log: false
  access_log_path: /tmp/proxysss-access.log
  error_log_path: /tmp/proxysss-error.log
admin:
  enabled: false
runtime:
  performance:
    enabled: true
    traffic_profile: __PROFILE__
services:
  static_sites:
    - name: bench
      path_prefix: /
      root: /www
      index_files: [index.html]
  reverse_proxy:
    routes:
      - name: rp
        path_prefix: /rp
        upstream: http://__BACKEND__
'@
[IO.File]::WriteAllText((Join-Path $tmp 'proxysss.yaml'), $proxysss.Replace('__BACKEND__', $backendUpstream).Replace('__PROFILE__', $TrafficProfile))

$backendConfig = @'
worker_processes 1;
error_log /dev/stderr error;
pid /tmp/backend.pid;
events { worker_connections 4096; use epoll; multi_accept on; }
http {
    access_log off;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    server_tokens off;
    server { listen 19081; root /usr/share/nginx/html; }
}
'@
[IO.File]::WriteAllText((Join-Path $tmp 'backend.conf'), $backendConfig)

$go = Get-Command go -ErrorAction Stop
$loader = Join-Path $tmp 'bench-fast-http.exe'
& $go.Source build -trimpath -ldflags '-s -w' -o $loader (Join-Path $repo 'scripts\bench-fast-http.go')
if ($LASTEXITCODE -ne 0) { throw 'failed to build benchmark loader' }

$network = docker network ls --filter 'name=^bench-net$' --format '{{.Name}}'
if (-not $network) {
    docker network create bench-net | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'failed to create bench-net' }
}
docker rm -f bench-backend bench-nginx bench-proxysss 2>$null | Out-Null
if ($HostNetwork) {
    docker create --name bench-backend --network host --cpus=1 --memory=512m --restart no nginx:alpine | Out-Null
    docker cp (Join-Path $tmp 'backend.conf') 'bench-backend:/etc/nginx/nginx.conf' | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-backend:/usr/share/nginx/html' | Out-Null
    docker start bench-backend | Out-Null

    docker create --name bench-nginx --network host --cpus=2 --memory=2g --restart no nginx:alpine | Out-Null
    docker cp (Join-Path $tmp 'nginx.conf') 'bench-nginx:/etc/nginx/nginx.conf' | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-nginx:/www' | Out-Null

    docker create --name bench-proxysss --network host --cpus=2 --memory=2g --restart no alpine:3.21 sleep infinity | Out-Null
    docker cp $Binary 'bench-proxysss:/proxysss' | Out-Null
    docker cp (Join-Path $tmp 'proxysss.yaml') 'bench-proxysss:/proxysss.yaml' | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-proxysss:/www' | Out-Null
} else {
    docker run -d --name bench-backend --network bench-net --cpus=1 --memory=512m nginx:alpine | Out-Null
    docker exec bench-backend mkdir -p /usr/share/nginx/html/rp | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-backend:/usr/share/nginx/html' | Out-Null

    docker create --name bench-nginx --network bench-net --cpus=2 --memory=2g --restart no -p "${Port}:19080" nginx:alpine | Out-Null
    docker cp (Join-Path $tmp 'nginx.conf') 'bench-nginx:/etc/nginx/nginx.conf' | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-nginx:/www' | Out-Null

    docker create --name bench-proxysss --network bench-net --cpus=2 --memory=2g --restart no -p "${Port}:19080" alpine:3.21 sleep infinity | Out-Null
    docker cp $Binary 'bench-proxysss:/proxysss' | Out-Null
    docker cp (Join-Path $tmp 'proxysss.yaml') 'bench-proxysss:/proxysss.yaml' | Out-Null
    docker cp (Join-Path $tmp 'www\.') 'bench-proxysss:/www' | Out-Null
}

docker stop bench-nginx bench-proxysss 2>$null | Out-Null
Start-Sleep -Seconds 1

$evidence = Join-Path $repo ("performance-evidence\2c2g-http-{0}" -f $Tag)
New-Item -ItemType Directory -Force -Path $evidence | Out-Null
$results = @()

function Wait-Ready([string]$Name) {
    for ($i = 0; $i -lt 80; $i++) {
        Start-Sleep -Milliseconds 250
        $code = & curl.exe -s -o NUL -w '%{http_code}' "http://127.0.0.1:${Port}/small.bin" 2>$null
        if ($LASTEXITCODE -eq 0 -and $code -eq '200') { return }
    }
    docker logs $Name --tail 40 | Write-Host
    throw "$Name did not become ready"
}

function Start-Gateway([string]$Name) {
    docker stop bench-nginx bench-proxysss 2>$null | Out-Null
    if ($Name -eq 'nginx') {
        docker start bench-nginx | Out-Null
    } else {
        docker start bench-proxysss | Out-Null
        docker exec bench-proxysss chmod +x /proxysss | Out-Null
        docker exec bench-proxysss sh -c "pkill -f '/proxysss' || true" | Out-Null
        docker exec -d bench-proxysss /proxysss -config /proxysss.yaml | Out-Null
    }
    Wait-Ready $Name
}

function Stop-Gateway {
    docker stop bench-nginx bench-proxysss 2>$null | Out-Null
}

$scenarios = @(
    @{ name = 'static-1k'; path = '/small.bin'; bytes = 1024 },
    @{ name = 'static-64k'; path = '/large.bin'; bytes = 65536 },
    @{ name = 'proxy-1k'; path = '/rp/small.bin'; bytes = 1024 }
)
$candidates = @('nginx', 'proxysss')

try {
    foreach ($round in 1..$Rounds) {
        $order = if ($round % 2 -eq 1) { $candidates } else { $candidates | ForEach-Object { $_ } | Sort-Object -Descending }
        foreach ($candidate in $order) {
            Write-Host ("[{0}/{1}] {2}" -f $round, $Rounds, $candidate)
            Start-Gateway $candidate
            foreach ($scenario in $scenarios) {
                $out = Join-Path $evidence ("{0}-{1}-r{2}.json" -f $candidate, $scenario.name, $round)
                & $loader -label ("{0}-{1}-r{2}" -f $candidate, $scenario.name, $round) `
                    -url ("http://127.0.0.1:{0}{1}" -f $Port, $scenario.path) `
                    -workers $Workers -warmup $WarmupSecs -duration $DurationSecs `
                    -expected-bytes $scenario.bytes -out $out
                if ($LASTEXITCODE -ne 0) { throw ("{0} {1} failed" -f $candidate, $scenario.name) }
                $results += Get-Content -LiteralPath $out -Raw | ConvertFrom-Json
            }
            Stop-Gateway
        }
    }

    $topology = if ($HostNetwork) {
        'Docker Desktop host networking, Windows client connects directly without the published-port proxy'
    } else {
        'Docker Desktop published port; Windows client crosses the Docker port proxy'
    }
    $summary = @('# 2C2G HTTP benchmark', '',
        ("- Date: {0}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')),
        ("- Candidate: {0}" -f $Binary),
        ("- Gateway: Docker Desktop same-host, 2 CPU / 2 GiB per candidate; backend 1 CPU / 512 MiB"),
        ("- proxysss traffic_profile: {0}" -f $TrafficProfile),
        ("- Network: {0}" -f $topology),
        ("- Client: Windows host Go loader, HTTP/1.1 keep-alive, {0} preconnected connections per scenario" -f $Workers),
        ("- Samples: {0} rounds x {1}s warm-up + {2}s measurement; report uses the median RPS" -f $Rounds, $WarmupSecs, $DurationSecs),
        '- Scope: static 1 KiB, static 64 KiB, 1 KiB reverse proxy only. This is a diagnostic comparison, not cross-host or full mixed-load evidence.', '',
        '| Scenario | nginx RPS | proxysss RPS | Ratio | nginx p50/p99 ms | proxysss p50/p99 ms | nginx errors | proxysss errors |',
        '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')

    function Median([double[]]$values) {
        $sorted = $values | Sort-Object
        $n = $sorted.Count
        if ($n -eq 0) { return 0 }
        if ($n % 2 -eq 1) { return [double]$sorted[[int]($n / 2)] }
        return ([double]$sorted[$n / 2 - 1] + [double]$sorted[$n / 2]) / 2
    }

    foreach ($scenario in $scenarios) {
        $nginxRows = $results | Where-Object { $_.Label -like "nginx-$($scenario.name)-*" }
        $proxysssRows = $results | Where-Object { $_.Label -like "proxysss-$($scenario.name)-*" }
        $nginxRps = Median @($nginxRows | ForEach-Object { [double]$_.RPS })
        $proxysssRps = Median @($proxysssRows | ForEach-Object { [double]$_.RPS })
        $ratio = if ($nginxRps -gt 0) { $proxysssRps / $nginxRps } else { 0 }
        $nginxP50 = Median @($nginxRows | ForEach-Object { [double]$_.p50_ms })
        $nginxP99 = Median @($nginxRows | ForEach-Object { [double]$_.p99_ms })
        $proxysssP50 = Median @($proxysssRows | ForEach-Object { [double]$_.p50_ms })
        $proxysssP99 = Median @($proxysssRows | ForEach-Object { [double]$_.p99_ms })
        $nginxErrors = ($nginxRows | Measure-Object -Property errors -Maximum).Maximum
        $proxysssErrors = ($proxysssRows | Measure-Object -Property errors -Maximum).Maximum
        $summary += ("| {0} | {1:N0} | {2:N0} | {3:N3}x | {4:N3} / {5:N3} | {6:N3} / {7:N3} | {8:N0} | {9:N0} |" -f `
            $scenario.name, $nginxRps, $proxysssRps, $ratio, $nginxP50, $nginxP99, $proxysssP50, $proxysssP99, $nginxErrors, $proxysssErrors)
    }

    $summary | Set-Content -LiteralPath (Join-Path $evidence 'summary.md') -Encoding utf8
    $summary | Write-Host
    Write-Host ("Evidence: {0}" -f $evidence)
    Write-Host ("Elapsed: {0:N1}s" -f ((Get-Date) - $started).TotalSeconds)
}
finally {
    if (-not $KeepContainers) {
        docker rm -f bench-backend bench-nginx bench-proxysss 2>$null | Out-Null
    }
}
