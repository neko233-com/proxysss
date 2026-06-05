$ErrorActionPreference = "Continue"

function Test-Url {
    param([string]$Name, [string]$Url, [switch]$Insecure)
    try {
        if ($Insecure) {
            curl.exe -k -sS -o NUL -w "%{http_code}" $Url | Out-Null
            $code = curl.exe -k -sS -o NUL -w "%{http_code}" $Url
        } else {
            $code = curl.exe -sS -o NUL -w "%{http_code}" $Url
        }
        if ($code -match "^2|^3") {
            Write-Host "[ok] $Name -> $Url ($code)"
        } else {
            Write-Host "[fail] $Name -> $Url ($code)"
        }
    } catch {
        Write-Host "[fail] $Name -> $Url ($($_.Exception.Message))"
    }
}

Write-Host "proxysss lab verification"
Test-Url "welcome-http" "http://127.0.0.1/"
Test-Url "welcome-https" "https://127.0.0.1/" -Insecure
Test-Url "http-echo" "http://127.0.0.1/echo/lab"
Test-Url "https-echo" "https://127.0.0.1/echo/lab" -Insecure
Test-Url "healthz" "http://127.0.0.1/healthz"
Test-Url "static" "http://127.0.0.1/assets/"
try {
    $code = curl.exe -sS -o NUL -w "%{http_code}" -X PROPFIND "http://127.0.0.1/dav/"
    if ($code -eq "207") { Write-Host "[ok] webdav -> PROPFIND http://127.0.0.1/dav/ ($code)" }
    else { Write-Host "[fail] webdav -> PROPFIND http://127.0.0.1/dav/ ($code)" }
} catch {
    Write-Host "[fail] webdav -> $($_.Exception.Message)"
}
Test-Url "metrics" "http://127.0.0.1/metrics"
Test-Url "admin" "http://127.0.0.1:7777/healthz"

Write-Host ""
Write-Host "Manual stream checks:"
Write-Host "  TCP: proxysss demo tcp-echo --listen 127.0.0.1:5999  (then telnet/nc 127.0.0.1 26379)"
Write-Host "  UDP: proxysss demo udp-echo --listen 127.0.0.1:5998  (then send to 127.0.0.1:2053)"
Write-Host "  FTP passthrough uses TCP echo on 127.0.0.1:2121 via port 21"
