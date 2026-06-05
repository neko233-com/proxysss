param(
    [string]$HttpBase = "http://127.0.0.1",
    [string]$HttpsBase = "https://127.0.0.1",
    [string]$AdminBase = "http://127.0.0.1:7777",
    [switch]$InsecureHttps
)

$ErrorActionPreference = "Continue"

function Test-Url {
    param([string]$Name, [string]$Url, [switch]$Insecure, [string]$Method = "GET")
    try {
        $args = @("-sS", "-o", "NUL", "-w", "%{http_code}")
        if ($Insecure) { $args += "-k" }
        if ($Method -ne "GET") { $args += "-X", $Method }
        $args += $Url
        $code = & curl.exe @args
        if ($code -match "^2|^3") { Write-Host "[ok] $Name ($code)" }
        else { Write-Host "[fail] $Name ($code)" }
    } catch {
        Write-Host "[fail] $Name ($($_.Exception.Message))"
    }
}

Write-Host "proxysss proxy verification"
Write-Host "http=$HttpBase https=$HttpsBase admin=$AdminBase"
Write-Host ""

Test-Url "welcome-http" "$HttpBase/"
if ($InsecureHttps) {
    Test-Url "welcome-https" "$HttpsBase/" -Insecure
}
Test-Url "http-echo" "$HttpBase/echo/lab"
if ($InsecureHttps) {
    Test-Url "https-echo" "$HttpsBase/echo/lab" -Insecure
}
Test-Url "healthz" "$HttpBase/healthz"
Test-Url "static" "$HttpBase/assets/"
Test-Url "webdav" "$HttpBase/dav/" -Method "PROPFIND"
Test-Url "metrics" "$HttpBase/metrics"
Test-Url "admin" "$AdminBase/healthz"

Write-Host ""
Write-Host "Tip: if your config uses custom ports, pass -HttpBase/-HttpsBase/-AdminBase."
