# Cross-check the real platform adapter without requiring an Apple SDK for the
# gateway's C dependencies. This does not build or run the full macOS gateway.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'project-artifacts.ps1')
Initialize-ProjectArtifacts
$stage = Reset-ProjectArtifactDirectory '.tmp/macos-adapter-check'
$report = Get-ProjectArtifactPath '.tmp/verification/macos-adapter'
New-Item -ItemType Directory -Force -Path $report | Out-Null
$version = ((rustc --version) -split ' ')[1]
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Use a stable Rust toolchain for cross-checking' }
$sysroot = Get-ProjectArtifactPath ".cache/macos-sysroot/$version"
New-Item -ItemType Directory -Force -Path $sysroot | Out-Null
$previousFlags = $env:CARGO_ENCODED_RUSTFLAGS
try {
    $sourceRoot = (Join-Path $script:ProjectRoot 'src').Replace('\','/')
    $manifest = @'
[package]
name = "proxysss-macos-adapter-check"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
anyhow = "1"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
socket2 = "0.6"
tokio = { version = "1", features = ["full"] }
libc = "0.2"
dashmap = "6"
tracing = "0.1"
'@
    [IO.File]::WriteAllText((Join-Path $stage 'Cargo.toml'), $manifest, [Text.UTF8Encoding]::new($false))
    New-Item -ItemType Directory -Path (Join-Path $stage 'src') | Out-Null
    $lib = @"
#![allow(dead_code)]
#[path = "$sourceRoot/config/performance.rs"]
mod config;
#[path = "$sourceRoot/linux_tune/mod.rs"]
mod linux_tune;
#[path = "$sourceRoot/runtime_tuning.rs"]
mod runtime_tuning;
"@
    [IO.File]::WriteAllText((Join-Path $stage 'src/lib.rs'), $lib, [Text.UTF8Encoding]::new($false))
    foreach ($target in @('x86_64-apple-darwin','aarch64-apple-darwin')) {
        $name = "rust-std-$version-$target"
        $archive = Get-ProjectArtifactPath ".cache/rust-components/$name.tar.xz"
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $archive) | Out-Null
        if (-not (Test-Path -LiteralPath (Join-Path $sysroot "lib/rustlib/$target/lib"))) {
            if (-not (Test-Path -LiteralPath $archive)) { Invoke-WebRequest -UseBasicParsing "https://static.rust-lang.org/dist/$name.tar.xz" -OutFile $archive }
            $checksumBody = (Invoke-WebRequest -UseBasicParsing "https://static.rust-lang.org/dist/$name.tar.xz.sha256").Content
            if ($checksumBody -is [byte[]]) { $checksumBody = [Text.Encoding]::UTF8.GetString($checksumBody) }
            $expected = ($checksumBody -split '\s+')[0]
            if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected) { throw "Rust component checksum mismatch: $target" }
            tar -xf $archive -C $sysroot --strip-components 2 "$name/rust-std-$target/lib"
            if ($LASTEXITCODE -ne 0) { throw "Rust component extraction failed: $target" }
        }
        $env:CARGO_ENCODED_RUSTFLAGS = '--sysroot' + [char]0x1f + $sysroot
        $previousPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        cargo check --manifest-path (Join-Path $stage 'Cargo.toml') --target $target 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath (Join-Path $report "$target.log")
        $code = $LASTEXITCODE
        $ErrorActionPreference = $previousPreference
        if ($code -ne 0) { throw "macOS adapter check failed: $target" }
    }
    Write-Host 'macOS x86_64/arm64 adapter cross-check passed; native runtime and full SDK-linked binary remain unverified'
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $previousFlags
    Reset-ProjectArtifactDirectory '.tmp/macos-adapter-check' | Out-Null
}
