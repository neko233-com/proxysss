param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,
    [int]$HttpPort = 18081,
    [int]$TlsPort = 18443,
    [int]$AdminPort = 17977,
    [switch]$KeepContainers
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath $Binary)) {
    throw "binary not found: $Binary"
}
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$started = Get-Date

$tmp = Join-Path $env:TEMP 'proxysss-fast-verify'
if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $tmp 'www') | Out-Null

$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$small = New-Object byte[] 1024
$rng.GetBytes($small)
[IO.File]::WriteAllBytes((Join-Path $tmp 'www\small.bin'), $small)

$config = @'
http:
  plain_bind: 0.0.0.0:18080
  tls_bind: 0.0.0.0:18443
  tls:
    mode: self_signed
    generate_self_signed_if_missing: true
    cert_path: /bench/certs/proxysss-cert.pem
    key_path: /bench/certs/proxysss-key.pem
logging:
  level: error
  access_log: false
  access_log_path: /bench/logs/access.log
  error_log_path: /bench/logs/error.log
admin:
  enabled: true
  bind: 0.0.0.0:17977
  loopback_only: false
  enable_write_ops: false
runtime:
  performance:
    enabled: true
    traffic_profile: small
services:
  static_sites:
    - name: verify
      path_prefix: /
      root: /bench/www
      index_files: [index.html]
'@
[IO.File]::WriteAllText((Join-Path $tmp 'proxysss.yaml'), $config)

function Invoke-CurlNumber([string]$Description, [string[]]$Arguments) {
    $value = & curl.exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Description failed with curl exit $LASTEXITCODE" }
    return $value
}

function Assert-Http200([string]$Description, [string]$Url, [switch]$Insecure) {
    $args = @('-sS', '-o', 'NUL', '-w', '%{http_code}')
    if ($Insecure) { $args = @('-k') + $args }
    $code = Invoke-CurlNumber $Description ($args + $Url)
    if ($code -ne '200') { throw "$Description expected HTTP 200, got $code" }
}

function Test-Distribution([string]$Name, [string]$Image, [int]$Index) {
    $container = "bench-verify-$index"
    docker rm -f $container 2>$null | Out-Null
    docker run -d --name $container --cpus=1 --memory=512m -p "${HttpPort}:18080" -p "${TlsPort}:18443" -p "${AdminPort}:17977" $Image sleep infinity | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "$Name container failed to start (port ${HttpPort} may be busy)" }
    docker exec $container mkdir -p /bench/www /bench/certs /bench/logs | Out-Null
    docker cp $Binary "${container}:/bench/proxysss" | Out-Null
    docker cp (Join-Path $tmp 'proxysss.yaml') "${container}:/bench/proxysss.yaml" | Out-Null
    docker cp (Join-Path $tmp 'www\.') "${container}:/bench/www" | Out-Null
    docker exec $container chmod +x /bench/proxysss | Out-Null

    $version = [string](docker exec $container /bench/proxysss --version)
    $version = $version.Trim()
    if ($LASTEXITCODE -ne 0 -or $version -notmatch 'proxysss 1\.') {
        throw "$Name --version failed: $version"
    }
    docker exec $container /bench/proxysss check-config --local-config /bench/proxysss.yaml | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "$Name check-config failed" }

    docker exec -d $container /bench/proxysss -config /bench/proxysss.yaml | Out-Null
    $ready = $false
    for ($i = 0; $i -lt 80; $i++) {
        Start-Sleep -Milliseconds 300
        $code = & curl.exe -s -o NUL -w '%{http_code}' "http://127.0.0.1:${HttpPort}/small.bin" 2>$null
        if ($LASTEXITCODE -eq 0 -and $code -eq '200') { $ready = $true; break }
    }
    if (-not $ready) {
        docker logs $container --tail 40 | Write-Host
        throw "$Name gateway did not become ready"
    }

    Assert-Http200 "$Name HTTP" "http://127.0.0.1:${HttpPort}/small.bin"
    $head = Invoke-CurlNumber "$Name HEAD" @('-sS', '-I', '-o', 'NUL', '-w', '%{http_code}', "http://127.0.0.1:${HttpPort}/small.bin")
    if ($head -ne '200') { throw "$Name HEAD expected 200, got $head" }
    $rangeFile = Join-Path $tmp 'range.bin'
    & curl.exe -sS -r 0-99 -o $rangeFile "http://127.0.0.1:${HttpPort}/small.bin" | Out-Null
    if ($LASTEXITCODE -ne 0 -or (Get-Item -LiteralPath $rangeFile).Length -ne 100) {
        throw "$Name Range expected 100 bytes"
    }
    $rangeCode = Invoke-CurlNumber "$Name Range status" @('-sS', '-r', '0-99', '-o', 'NUL', '-w', '%{http_code}', "http://127.0.0.1:${HttpPort}/small.bin")
    if ($rangeCode -ne '206') { throw "$Name Range expected 206, got $rangeCode" }
    Assert-Http200 "$Name HTTPS" "https://127.0.0.1:${TlsPort}/small.bin" -Insecure
    Assert-Http200 "$Name healthz" "http://127.0.0.1:${AdminPort}/healthz"

    Write-Host ("PASS {0}: {1}; HTTP/HTTPS/Range/HEAD/healthz OK" -f $Name, $version)
    if (-not $KeepContainers) { docker rm -f $container 2>$null | Out-Null }
}

try {
    Write-Host 'Binary checks'
    $inspect = docker run --rm -v "${Binary}:/bin/proxysss:ro" proxysss-musl-builder:local sh -c 'file /bin/proxysss; if readelf -d /bin/proxysss | grep -q NEEDED; then echo unexpected NEEDED; exit 1; fi; if readelf -l /bin/proxysss | grep -q INTERP; then echo unexpected INTERP; exit 1; fi; echo static-ok'
    $inspect
    if ($LASTEXITCODE -ne 0 -or ($inspect -join "`n") -notmatch 'static-ok') { throw 'binary static-link check failed' }

    Test-Distribution 'Debian 12' 'debian:bookworm-slim' 1
    Test-Distribution 'Ubuntu 24.04 LTS' 'ubuntu:24.04' 2
    $elapsed = (Get-Date) - $started
    Write-Host ("FAST VERIFY PASS in {0:N1}s" -f $elapsed.TotalSeconds)
}
finally {
    if (-not $KeepContainers) {
        docker rm -f bench-verify-1 bench-verify-2 2>$null | Out-Null
    }
}
