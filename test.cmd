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
exit /b %ERRORLEVEL%
