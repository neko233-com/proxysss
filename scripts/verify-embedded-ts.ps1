<#
.SYNOPSIS
    Verifies the embedded TypeScript engine in a freshly built proxysss binary.

.DESCRIPTION
    Proves the single-binary, no-external-deno architecture:
      1. The binary contains no bundled deno runtime and needs none on PATH.
      2. `script eval` transpiles and runs TypeScript in-process (QuickJS).
      3. A running gateway routes HTTP through a TypeScript plugin.
      4. A buggy plugin (throw) is isolated: normal proxy traffic still works.

    Run after `cargo build --release`. Exits non-zero on the first failure.

.PARAMETER Binary
    Path to the proxysss(.exe) to verify. Defaults to target/release.
#>
param(
    [string]$Binary = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Binary) {
    $exe = if ($IsWindows -or $env:OS -eq "Windows_NT") { "proxysss.exe" } else { "proxysss" }
    $Binary = Join-Path $repoRoot "target/release/$exe"
}
if (-not (Test-Path -LiteralPath $Binary)) {
    throw "proxysss binary not found at $Binary; run 'cargo build --release' first"
}
$Binary = (Resolve-Path -LiteralPath $Binary).Path

function Write-Step([string]$Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Fail([string]$Message) { throw "VERIFY FAILED: $Message" }

# Remove any deno on PATH for the duration of this verification so we prove the
# embedded engine needs no external runtime.
$env:Path = ($env:Path -split [IO.Path]::PathSeparator | Where-Object { $_ -notmatch 'deno' }) -join [IO.Path]::PathSeparator

$work = Join-Path ([IO.Path]::GetTempPath()) ("proxysss-verify-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $work | Out-Null

try {
    # ----------------------------------------------------------------------
    Write-Step "1/5 binary reports a version and ships no bundled deno runtime"
    $version = (& $Binary --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { Fail "proxysss --version exited $LASTEXITCODE" }
    Write-Host "    $version"

    $bundleDir = Split-Path -Parent $Binary
    $strayDeno = Get-ChildItem -LiteralPath $bundleDir -Recurse -Filter "deno*" -ErrorAction SilentlyContinue
    if ($strayDeno) { Fail "found bundled deno runtime next to the binary: $($strayDeno.FullName)" }

    # ----------------------------------------------------------------------
    Write-Step "2/5 embedded engine evaluates TypeScript in-process (no deno on PATH)"
    $cfg = Join-Path $work "proxysss.yaml"
    & $Binary print-default-config --format yaml | Set-Content -LiteralPath $cfg -Encoding utf8
    $evalOut = (& $Binary script --config $cfg eval "const n: number = 20; console.log('eval=' + (n + 2));" 2>&1 | Out-String)
    if ($evalOut -notmatch "eval=22") { Fail "embedded TypeScript eval did not produce expected output. Got: $evalOut" }
    Write-Host "    embedded TypeScript eval OK"

    # ----------------------------------------------------------------------
    Write-Step "3/5 initialize a gateway project with the default house script + plugins"
    $proj = Join-Path $work "site"
    & $Binary init --dir $proj --overwrite | Out-Null
    if (-not (Test-Path (Join-Path $proj "gateway.ts"))) { Fail "init did not write gateway.ts" }
    if (-not (Test-Path (Join-Path $proj "plugins"))) { Fail "init did not create plugins directory" }

    # Add a plugin that overrides routing, plus a buggy plugin that always throws.
    $pluginDir = Join-Path $proj "plugins"
    @'
export default {
  name: "verify-route",
  priority: 500,
  access(message: { ctx: { path?: string } }) {
    if ((message.ctx.path ?? "").startsWith("/verify")) {
      return { upstream: "proxysss://healthz" };
    }
  },
};
'@ | Set-Content -LiteralPath (Join-Path $pluginDir "verify-route.ts") -Encoding utf8

    @'
export default {
  name: "verify-broken",
  priority: 900,
  access() { throw new Error("intentional plugin failure"); },
};
'@ | Set-Content -LiteralPath (Join-Path $pluginDir "verify-broken.ts") -Encoding utf8

    # Write a config that enables scripting + plugins on a private port.
    $port = 18987
    $projFwd = $proj.Replace('\', '/')
    @"
config_version: 1
logging:
  access_log: false
http:
  plain_bind: 127.0.0.1:$port
  tls_bind: ''
  h3_bind: ''
script:
  enabled: true
  entry: gateway.ts
  cwd: '$projFwd'
  timeout_ms: 500
plugins:
  enabled: true
  auto_load_dir: '$projFwd/plugins'
admin:
  enabled: false
runtime:
  hot_reload:
    enabled: false
"@ | Set-Content -LiteralPath (Join-Path $proj "proxysss.yaml") -Encoding utf8

    # ----------------------------------------------------------------------
    Write-Step "4/5 start the gateway and route an HTTP request through the TypeScript plugin"
    $logFile = Join-Path $work "proxysss.log"
    $serverConfig = Join-Path $proj "proxysss.yaml"
    $proc = Start-Process -FilePath $Binary -ArgumentList @("run", "--config", $serverConfig) `
        -RedirectStandardOutput $logFile -RedirectStandardError (Join-Path $work "proxysss.err.log") `
        -PassThru -WindowStyle Hidden

    try {
        $base = "http://127.0.0.1:$port"
        $ready = $false
        for ($i = 0; $i -lt 50; $i++) {
            Start-Sleep -Milliseconds 200
            try {
                $welcome = Invoke-WebRequest -Uri "$base/" -UseBasicParsing -TimeoutSec 2
                if ($welcome.StatusCode -ge 200) { $ready = $true; break }
            } catch { }
        }
        if (-not $ready) { Fail "gateway did not become ready on $base" }
        Write-Host "    gateway is serving on $base"

        # Plugin-routed path: verify-route maps /verify -> internal healthz.
        $verify = Invoke-WebRequest -Uri "$base/verify" -UseBasicParsing -TimeoutSec 5
        if ($verify.StatusCode -ne 200) { Fail "plugin-routed /verify returned $($verify.StatusCode)" }
        Write-Host "    TypeScript plugin routing OK (/verify -> $($verify.StatusCode))"

        # ------------------------------------------------------------------
        Write-Step "5/5 a throwing plugin is isolated: normal proxy traffic still works"
        # verify-broken throws on every access hook (highest priority); the
        # welcome page and plugin routing must still succeed.
        $again = Invoke-WebRequest -Uri "$base/verify" -UseBasicParsing -TimeoutSec 5
        if ($again.StatusCode -ne 200) { Fail "routing broke after a plugin threw ($($again.StatusCode))" }
        $welcomeAfter = Invoke-WebRequest -Uri "$base/" -UseBasicParsing -TimeoutSec 5
        if ($welcomeAfter.StatusCode -ne 200) { Fail "welcome page broke after a plugin threw ($($welcomeAfter.StatusCode))" }
        Write-Host "    plugin failure isolation OK (proxy unaffected by throwing plugin)"
    }
    finally {
        if ($proc -and -not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }

    Write-Host ""
    Write-Host "ALL EMBEDDED TYPESCRIPT ENGINE CHECKS PASSED" -ForegroundColor Green
    Write-Host "proxysss is a single self-contained binary: no external deno required." -ForegroundColor Green
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
