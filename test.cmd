@echo off
setlocal

echo [proxysss] cargo fmt --check
cargo fmt --all -- --check
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] cargo clippy
cargo clippy --workspace --all-targets -- -D warnings
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] cargo test
cargo test --workspace --all-targets
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] verify-e2e (fresh init, no legacy config)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\verify-e2e.ps1"
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] integration_deep
cargo test integration_deep
exit /b %ERRORLEVEL%
